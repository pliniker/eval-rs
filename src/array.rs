use std::ptr::NonNull;

use crate::error::RuntimeError;
use crate::memory::MutatorView;

/// Fundamental array type on which other variable-length types are built.
/// Analagous to RawVec.
pub struct Array<T: Sized> {
    capacity: u32,
    ptr: Option<NonNull<T>>,
}

impl<T: Sized> Array<T> {
    pub fn new() -> Array<T> {
        Array {
            capacity: 0,
            ptr: None,
        }
    }

    pub fn with_capacity<'scope>(
        &self,
        mem: &'scope MutatorView,
        capacity: u32,
    ) -> Result<Array<T>, RuntimeError> {
        Ok(Array {
            capacity: capacity,
            ptr: NonNull::new(mem.alloc_array(capacity)?.as_ptr() as *mut T),
        })
    }

    // fn allocate(capacity: u32)
    // fn grow(capacity: u32)
    // fn shrink(capacity: u32)

    //fn as_ptr(&self) -> *const T {
    //    self.ptr.as_ptr()
    //}
}
