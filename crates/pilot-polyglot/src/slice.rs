//! Milestone 35.3 & 42.1: Zero-Marshalling SliceBuffer for Cross-Language Interop.

use std::fs::File;
use std::io::{Read, Write};

#[repr(C)]
pub struct SliceBuffer<T> {
    pub ptr: *mut T,
    pub len: usize,
    pub capacity: usize,
}

unsafe impl<T: Send> Send for SliceBuffer<T> {}
unsafe impl<T: Sync> Sync for SliceBuffer<T> {}

impl<T> SliceBuffer<T> {
    pub fn new() -> Self {
        Self {
            ptr: core::ptr::null_mut(),
            len: 0,
            capacity: 0,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let mut v = Vec::with_capacity(capacity);
        let ptr = v.as_mut_ptr();
        core::mem::forget(v);
        Self {
            ptr,
            len: 0,
            capacity,
        }
    }

    pub fn from_vec(mut v: Vec<T>) -> Self {
        let ptr = v.as_mut_ptr();
        let len = v.len();
        let capacity = v.capacity();
        core::mem::forget(v);
        Self { ptr, len, capacity }
    }

    pub fn as_slice(&self) -> &[T] {
        if self.ptr.is_null() || self.len == 0 {
            &[]
        } else {
            unsafe { core::slice::from_raw_parts(self.ptr, self.len) }
        }
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        if self.ptr.is_null() || self.len == 0 {
            &mut []
        } else {
            unsafe { core::slice::from_raw_parts_mut(self.ptr, self.len) }
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn get(&self, idx: usize) -> Option<&T> {
        if idx < self.len && !self.ptr.is_null() {
            Some(unsafe { &*self.ptr.add(idx) })
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, idx: usize) -> Option<&mut T> {
        if idx < self.len && !self.ptr.is_null() {
            Some(unsafe { &mut *self.ptr.add(idx) })
        } else {
            None
        }
    }

    pub fn push(&mut self, item: T) {
        if self.len >= self.capacity {
            let new_cap = if self.capacity == 0 { 4 } else { self.capacity * 2 };
            let mut new_vec = Vec::with_capacity(new_cap);
            let new_ptr = new_vec.as_mut_ptr();
            core::mem::forget(new_vec);

            if !self.ptr.is_null() && self.len > 0 {
                unsafe {
                    core::ptr::copy_nonoverlapping(self.ptr, new_ptr, self.len);
                    drop(Vec::from_raw_parts(self.ptr, self.len, self.capacity));
                }
            }

            self.ptr = new_ptr;
            self.capacity = new_cap;
        }

        unsafe {
            core::ptr::write(self.ptr.add(self.len), item);
        }
        self.len += 1;
    }

    pub fn to_vec(&self) -> Vec<T>
    where
        T: Clone,
    {
        self.as_slice().to_vec()
    }
}

impl<T: Copy> SliceBuffer<T> {
    /// Writes raw buffer bytes to a shared memory file accessible by out-of-process foreign runtimes (Python mmap, etc.).
    pub fn write_to_shared_file(&self, file_path: &str) -> std::io::Result<()> {
        let mut file = File::create(file_path)?;
        let byte_len = self.len * core::mem::size_of::<T>();
        if byte_len > 0 && !self.ptr.is_null() {
            let byte_slice = unsafe {
                core::slice::from_raw_parts(self.ptr as *const u8, byte_len)
            };
            file.write_all(byte_slice)?;
        }
        file.flush()?;
        Ok(())
    }

    /// Reads raw bytes from a shared memory file into a `SliceBuffer<u8>`.
    pub fn read_bytes_from_shared_file(file_path: &str) -> std::io::Result<SliceBuffer<u8>> {
        let mut file = File::open(file_path)?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        Ok(SliceBuffer::from_vec(buf))
    }
}

impl<T> Default for SliceBuffer<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Drop for SliceBuffer<T> {
    fn drop(&mut self) {
        if !self.ptr.is_null() && self.capacity > 0 {
            unsafe {
                drop(Vec::from_raw_parts(self.ptr, self.len, self.capacity));
            }
            self.ptr = core::ptr::null_mut();
            self.len = 0;
            self.capacity = 0;
        }
    }
}
