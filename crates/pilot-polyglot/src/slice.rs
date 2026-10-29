//! Milestone 35.3: Zero-Marshalling SliceBuffer for Cross-Language Interop.

#[repr(C)]
pub struct SliceBuffer<T> {
    pub ptr: *mut T,
    pub len: usize,
    pub capacity: usize,
}

impl<T> SliceBuffer<T> {
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

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<T> Drop for SliceBuffer<T> {
    fn drop(&mut self) {
        if !self.ptr.is_null() && self.capacity > 0 {
            unsafe {
                drop(Vec::from_raw_parts(self.ptr, self.len, self.capacity));
            }
            self.ptr = core::ptr::null_mut();
        }
    }
}
