use std::ptr::NonNull;
use std::slice::from_raw_parts_mut;

use stickyimmix::ArraySize;

use crate::error::RuntimeError;
use crate::memory::MutatorView;

/// Fundamental array type on which other variable-length types are built.
/// Analagous to RawVec.
pub struct RawArray<T: Sized> {
    capacity: ArraySize,
    ptr: Option<NonNull<T>>,
}

impl<T: Sized> RawArray<T> {
    /// Return a RawArray of capacity 0 with no array bytes allocated
    pub fn new() -> RawArray<T> {
        RawArray {
            capacity: 0,
            ptr: None,
        }
    }

    /// Return the capacity of the array in bytes
    pub fn capacity(&self) -> ArraySize {
        self.capacity
    }

    /// Return a RawArray of the given capacity number of bytes allocated
    pub fn with_capacity<'scope>(
        mem: &'scope MutatorView,
        capacity: u32,
    ) -> Result<RawArray<T>, RuntimeError> {
        Ok(RawArray {
            capacity: capacity,
            ptr: NonNull::new(mem.alloc_array(capacity)?.as_ptr() as *mut T),
        })
    }

    /// Resize the array to the new capacity
    /// TODO the inner implementation of this should live in the allocator API to make
    /// better use of optimizations
    pub fn resize<'scope>(
        &mut self,
        mem: &'scope MutatorView,
        new_capacity: u32,
    ) -> Result<(), RuntimeError> {

        // If we're reducing the capacity to 0, simply detach the array pointer
        if new_capacity == 0 {
            self.capacity = 0;
            self.ptr = None;
            return Ok(());
        }

        match self.ptr {
            // If we have capacity, create new capacity and copy over all bytes from the old
            // to the new array
            Some(old_ptr) => {
                let old_ptr = old_ptr.as_ptr();
                let new_ptr = mem.alloc_array(new_capacity)?.as_ptr() as *mut T;

                let (old_slice, new_slice) = unsafe {
                    (
                        from_raw_parts_mut(old_ptr as *mut u8, self.capacity as usize),
                        from_raw_parts_mut(new_ptr as *mut u8, new_capacity as usize),
                    )
                };

                // copy content from old to new array
                for (src, dest) in old_slice.iter().zip(new_slice) {
                    *dest = *src;
                }

                self.ptr = NonNull::new(new_ptr);
                self.capacity = new_capacity;

                Ok(())
            },

            // If we have no capacity, create new blank capacity
            None => {
                *self = Self::with_capacity(mem, new_capacity)?;
                Ok(())
            }
        }
    }

    /// Return a pointer to the array
    pub fn as_ptr(&self) -> Option<*const T> {
        match self.ptr {
            Some(ptr) => Some(ptr.as_ptr()),
            None => None,
        }
    }
}
