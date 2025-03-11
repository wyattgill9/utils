use std::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};

pub struct LockFreeStack<T> {
    top: AtomicPtr<Node<T>>,
}

struct Node<T> {
    value: T,
    next: *mut Node<T>,
}

impl<T> LockFreeStack<T> {
    pub fn new() -> Self {
        LockFreeStack {
            top: AtomicPtr::new(ptr::null_mut()),
        }
    }

    #[inline(always)]
    pub fn push(&self, value: T) {
        let new_node = Box::into_raw(Box::new(Node {
            value,
            next: ptr::null_mut(),
        }));

        loop {
            let top = self.top.load(Ordering::Acquire);
            unsafe { (*new_node).next = top };

            if self
                .top
                .compare_exchange_weak(top, new_node, Ordering::Release, Ordering::Relaxed)
                .is_ok()
            {
                break;
            }

            core::hint::spin_loop();
        }
    }

    #[inline(always)]
    pub fn pop(&self) -> Option<T> {
        loop {
            let top = self.top.load(Ordering::Acquire);
            if top.is_null() {
                return None;
            }

            let next = unsafe { (*top).next };

            if self
                .top
                .compare_exchange_weak(top, next, Ordering::Release, Ordering::Relaxed)
                .is_ok()
            {
                let value = unsafe { ptr::read(&(*top).value) };
                unsafe { drop(Box::from_raw(top)) };
                return Some(value);
            }

            core::hint::spin_loop();
        }
    }
}

impl<T> Drop for LockFreeStack<T> {
    fn drop(&mut self) {
        while self.pop().is_some() {}
    }
}
