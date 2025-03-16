use std::cell::UnsafeCell;
use std::ptr;
use std::sync::atomic::{fence, AtomicPtr, AtomicUsize, Ordering};

#[allow(dead_code)]
pub struct LockFreeQueue<T> {
    head: AtomicPtr<Node<T>>,
    tail: AtomicPtr<Node<T>>,
    cache_line_pad: [u8; 64],
}

struct Node<T> {
    value: UnsafeCell<Option<T>>,
    next: AtomicPtr<Node<T>>,
}

impl<T> LockFreeQueue<T> {
    pub fn new() -> Self {
        let sentinel = Box::into_raw(Box::new(Node {
            value: UnsafeCell::new(None),
            next: AtomicPtr::new(ptr::null_mut()),
        }));

        LockFreeQueue {
            head: AtomicPtr::new(sentinel),
            tail: AtomicPtr::new(sentinel),
            cache_line_pad: [0; 64],
        }
    }

    #[inline(always)]
    pub fn enqueue(&self, value: T) {
        let new_node = Box::into_raw(Box::new(Node {
            value: UnsafeCell::new(Some(value)),
            next: AtomicPtr::new(ptr::null_mut()),
        }));

        loop {
            let tail = self.tail.load(Ordering::Acquire);
            let tail_next = unsafe { (*tail).next.load(Ordering::Acquire) };

            if tail == self.tail.load(Ordering::Acquire) {
                if tail_next.is_null() {
                    if unsafe {
                        (*tail).next.compare_exchange_weak(
                            tail_next,
                            new_node,
                            Ordering::Release,
                            Ordering::Relaxed,
                        )
                    }
                    .is_ok()
                    {
                        let _ = self.tail.compare_exchange_weak(
                            tail,
                            new_node,
                            Ordering::Release,
                            Ordering::Relaxed,
                        );
                        return;
                    }
                } else {
                    let _ = self.tail.compare_exchange_weak(
                        tail,
                        tail_next,
                        Ordering::Release,
                        Ordering::Relaxed,
                    );
                }
            }
            core::hint::spin_loop();
        }
    }

    #[inline(always)]
    pub fn dequeue(&self) -> Option<T> {
        loop {
            let head = self.head.load(Ordering::Acquire);
            let tail = self.tail.load(Ordering::Acquire);
            let head_next = unsafe { (*head).next.load(Ordering::Acquire) };

            if head == self.head.load(Ordering::Acquire) {
                if head == tail {
                    if head_next.is_null() {
                        return None;
                    }
                    let _ = self.tail.compare_exchange_weak(
                        tail,
                        head_next,
                        Ordering::Release,
                        Ordering::Relaxed,
                    );
                } else {
                    if self
                        .head
                        .compare_exchange_weak(
                            head,
                            head_next,
                            Ordering::Release,
                            Ordering::Relaxed,
                        )
                        .is_ok()
                    {
                        let value = unsafe { ptr::read(&(*head_next).value) }.into_inner();

                        unsafe { drop(Box::from_raw(head)) };

                        return value;
                    }
                }
            }
            core::hint::spin_loop();
        }
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        let head = self.head.load(Ordering::Acquire);
        let head_next = unsafe { (*head).next.load(Ordering::Acquire) };
        head_next.is_null()
    }
}

impl<T> Drop for LockFreeQueue<T> {
    fn drop(&mut self) {
        while self.dequeue().is_some() {}

        let sentinel = self.head.load(Ordering::Relaxed);
        unsafe { drop(Box::from_raw(sentinel)) };
    }
}

#[allow(dead_code)]
pub struct BoundedLockFreeQueue<T> {
    buffer: *mut Node<T>,
    capacity: usize,
    head: AtomicUsize,
    tail: AtomicUsize,
    cache_line_pad: [u8; 64],
}

impl<T> BoundedLockFreeQueue<T> {
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.next_power_of_two();

        let mut buffer = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            buffer.push(Node {
                value: UnsafeCell::new(None),
                next: AtomicPtr::new(ptr::null_mut()),
            });
        }

        let buffer = buffer.leak().as_mut_ptr();

        BoundedLockFreeQueue {
            buffer,
            capacity,
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            cache_line_pad: [0; 64],
        }
    }

    #[inline(always)]
    pub fn enqueue(&self, value: T) -> Result<(), T> {
        let mask = self.capacity - 1;
        let mut tail = self.tail.load(Ordering::Relaxed);

        loop {
            let head = self.head.load(Ordering::Acquire);

            if (tail - head) >= self.capacity {
                return Err(value);
            }

            if self
                .tail
                .compare_exchange_weak(tail, tail + 1, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                let index = tail & mask;
                let node = unsafe { &*self.buffer.add(index) };

                unsafe { *node.value.get() = Some(value) };

                fence(Ordering::Release);

                return Ok(());
            }

            tail = self.tail.load(Ordering::Relaxed);
            core::hint::spin_loop();
        }
    }

    #[inline(always)]
    pub fn dequeue(&self) -> Option<T> {
        let mask = self.capacity - 1;
        let mut head = self.head.load(Ordering::Relaxed);

        loop {
            let tail = self.tail.load(Ordering::Acquire);

            if head >= tail {
                return None;
            }

            if self
                .head
                .compare_exchange_weak(head, head + 1, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                let index = head & mask;
                let node = unsafe { &*self.buffer.add(index) };

                fence(Ordering::Acquire);

                let value = unsafe { (*node.value.get()).take() };

                return value;
            }

            head = self.head.load(Ordering::Relaxed);
            core::hint::spin_loop();
        }
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.head.load(Ordering::Acquire) >= self.tail.load(Ordering::Acquire)
    }

    #[inline(always)]
    pub fn is_full(&self) -> bool {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);
        (tail - head) >= self.capacity
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);
        tail.saturating_sub(head)
    }

    #[inline(always)]
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

unsafe impl<T: Send> Send for BoundedLockFreeQueue<T> {}
unsafe impl<T: Send> Sync for BoundedLockFreeQueue<T> {}

impl<T> Drop for BoundedLockFreeQueue<T> {
    fn drop(&mut self) {
        while self.dequeue().is_some() {}

        unsafe {
            Vec::from_raw_parts(self.buffer, self.capacity, self.capacity);
        }
    }
}

unsafe impl<T: Send> Send for LockFreeQueue<T> {}
unsafe impl<T: Send> Sync for LockFreeQueue<T> {}
