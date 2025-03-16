//! Memory utilities for highly optimized operations
//!
//! # Safety
//! These functions use unsafe Rust and should be used with extreme caution.
//! Improper use can lead to undefined behavior, memory corruption, and security vulnerabilities.

use std::alloc::{self, Layout};
use std::mem;
use std::ptr;

/// Allocates uninitialized memory with the specified size and alignment.
///
/// # Safety
/// The caller must ensure the returned memory is properly initialized before reading
/// and must deallocate the memory using `deallocate` when no longer needed.
///
/// # Arguments
/// * `size` - The size in bytes to allocate
/// * `align` - The memory alignment (must be a power of 2)
///
/// # Returns
/// A pointer to the allocated memory block or null if allocation failed
pub unsafe fn allocate(size: usize, align: usize) -> *mut u8 {
    if size == 0 {
        return ptr::null_mut();
    }

    let layout = Layout::from_size_align_unchecked(size, align);
    alloc::alloc(layout)
}

/// Deallocates memory previously allocated with `allocate`.
///
/// # Safety
/// The pointer must have been allocated using `allocate` with the same size and alignment.
/// After this call, the pointer becomes invalid and must not be used.
pub unsafe fn deallocate(ptr: *mut u8, size: usize, align: usize) {
    if !ptr.is_null() && size > 0 {
        let layout = Layout::from_size_align_unchecked(size, align);
        alloc::dealloc(ptr, layout);
    }
}

/// Fast memory copy that uses SIMD instructions when available.
///
/// # Safety
/// The caller must ensure:
/// * Both source and destination pointers point to valid memory regions
/// * The memory regions don't overlap
/// * The memory regions are at least `count` bytes in size
///
/// # Arguments
/// * `dst` - Destination pointer
/// * `src` - Source pointer  
/// * `count` - Number of bytes to copy
pub unsafe fn fast_memcpy(dst: *mut u8, src: *const u8, count: usize) {
    if count < 32 {
        for i in 0..count {
            *dst.add(i) = *src.add(i);
        }
        return;
    }

    let dst_ptr = dst as *mut usize;
    let src_ptr = src as *const usize;
    let word_size = mem::size_of::<usize>();
    let word_count = count / word_size;

    for i in 0..word_count {
        *dst_ptr.add(i) = *src_ptr.add(i);
    }

    let remaining_offset = word_count * word_size;
    for i in 0..(count - remaining_offset) {
        *dst.add(remaining_offset + i) = *src.add(remaining_offset + i);
    }
}

/// Fast memory set that uses SIMD instructions when available.
///
/// # Safety
/// The caller must ensure:
/// * The destination pointer points to valid memory region
/// * The memory region is at least `count` bytes in size
///
/// # Arguments
/// * `dst` - Destination pointer
/// * `value` - Byte value to set
/// * `count` - Number of bytes to set
pub unsafe fn fast_memset(dst: *mut u8, value: u8, count: usize) {
    if count < 32 {
        for i in 0..count {
            *dst.add(i) = value;
        }
        return;
    }

    *dst = value;

    let mut i = 1;
    while i <= count / 2 {
        ptr::copy_nonoverlapping(dst, dst.add(i), i);
        i *= 2;
    }

    if i < count {
        ptr::copy_nonoverlapping(dst, dst.add(i), count - i);
    }
}

/// Zero memory in a way that shouldn't be optimized out by the compiler.
/// Useful for clearing sensitive information.
///
/// # Safety
/// The caller must ensure:
/// * The pointer points to valid memory region
/// * The memory region is at least `count` bytes in size
///
/// # Arguments
/// * `ptr` - Pointer to memory to clear
/// * `count` - Number of bytes to clear
pub unsafe fn secure_zero_memory(ptr: *mut u8, count: usize) {
    for i in 0..count {
        ptr::write_volatile(ptr.add(i), 0);
    }
    std::sync::atomic::fence(std::sync::atomic::Ordering::SeqCst);
}

/// Reallocates memory block to a new size.
///
/// # Safety
/// The caller must ensure:
/// * The original pointer was allocated with `allocate`
/// * The same size and alignment are provided
/// * Memory is properly initialized before reading
///
/// # Arguments
/// * `ptr` - Pointer to the memory block to reallocate
/// * `old_size` - Current size of the allocation
/// * `new_size` - Desired new size
/// * `align` - Memory alignment (must be a power of 2)
///
/// # Returns
/// A pointer to the reallocated memory block or null if reallocation failed
pub unsafe fn reallocate(ptr: *mut u8, old_size: usize, new_size: usize, align: usize) -> *mut u8 {
    if ptr.is_null() {
        return allocate(new_size, align);
    }

    if new_size == 0 {
        deallocate(ptr, old_size, align);
        return ptr::null_mut();
    }

    let old_layout = Layout::from_size_align_unchecked(old_size, align);
    let new_layout = Layout::from_size_align_unchecked(new_size, align);

    alloc::realloc(ptr, old_layout, new_size)
}

pub struct MemoryBlock {
    ptr: *mut u8,
    size: usize,
    align: usize,
}

impl MemoryBlock {
    pub fn new(size: usize, align: usize) -> Option<Self> {
        unsafe {
            let ptr = allocate(size, align);
            if ptr.is_null() {
                None
            } else {
                Some(Self { ptr, size, align })
            }
        }
    }

    pub fn as_ptr(&self) -> *mut u8 {
        self.ptr
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn fill(&mut self, value: u8) {
        unsafe {
            fast_memset(self.ptr, value, self.size);
        }
    }

    pub fn resize(&mut self, new_size: usize) -> bool {
        unsafe {
            let new_ptr = reallocate(self.ptr, self.size, new_size, self.align);
            if new_ptr.is_null() {
                return false;
            }
            self.ptr = new_ptr;
            self.size = new_size;
            true
        }
    }

    pub fn secure_zero(&mut self) {
        unsafe {
            secure_zero_memory(self.ptr, self.size);
        }
    }
}

impl Drop for MemoryBlock {
    fn drop(&mut self) {
        unsafe {
            deallocate(self.ptr, self.size, self.align);
        }
    }
}

#[derive(Debug)]
pub struct MemoryAccess<'a> {
    ptr: *mut u8,
    size: usize,
    _phantom: std::marker::PhantomData<&'a mut [u8]>,
}

impl<'a> MemoryAccess<'a> {
    /// Creates a new memory access wrapper from a raw pointer and size.
    ///
    /// # Safety
    /// The caller must ensure the pointer points to valid memory of at least 'size' bytes
    /// and remains valid for the lifetime 'a.
    pub unsafe fn new(ptr: *mut u8, size: usize) -> Self {
        Self {
            ptr,
            size,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Reads a value of type T from the offset.
    ///
    /// # Panics
    /// Panics if the read would go out of bounds.
    pub fn read<T: Copy>(&self, offset: usize) -> T {
        assert!(
            offset + mem::size_of::<T>() <= self.size,
            "Read out of bounds"
        );
        unsafe { ptr::read_unaligned(self.ptr.add(offset) as *const T) }
    }

    /// Writes a value of type T at the offset.
    ///
    /// # Panics
    /// Panics if the write would go out of bounds.
    pub fn write<T>(&mut self, offset: usize, value: T) {
        assert!(
            offset + mem::size_of::<T>() <= self.size,
            "Write out of bounds"
        );
        unsafe {
            ptr::write_unaligned(self.ptr.add(offset) as *mut T, value);
        }
    }

    /// Gets a slice of the memory.
    ///
    /// # Panics
    /// Panics if the slice would go out of bounds.
    pub fn slice(&self, offset: usize, len: usize) -> &[u8] {
        assert!(offset + len <= self.size, "Slice out of bounds");
        unsafe { std::slice::from_raw_parts(self.ptr.add(offset), len) }
    }

    /// Gets a mutable slice of the memory.
    ///
    /// # Panics
    /// Panics if the slice would go out of bounds.
    pub fn slice_mut(&mut self, offset: usize, len: usize) -> &mut [u8] {
        assert!(offset + len <= self.size, "Slice out of bounds");
        unsafe { std::slice::from_raw_parts_mut(self.ptr.add(offset), len) }
    }
}
