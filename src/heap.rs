use std::cell::Cell;
use std::mem;
use std::ptr;

use memalloc::{allocate, deallocate};
use taggedptr::TaggedPtr;


/// A memory error type, encompassing all memory related errors at this
/// time.
pub enum MemError {
    OOM
}


/// A heap trait
pub trait Heap {
    fn alloc<T>(&self, object: T) -> Result<TaggedPtr, MemError>;
    fn alloc_static<T>(&self, object: T) -> Result<TaggedPtr, MemError>;
    fn lookup_symbol(&self, name: &str) -> Result<TaggedPtr, MemError>;
}


/// A fixed-size block of contiguous bytes type that implements the Heap
/// traits. When it's full, it panics. Does not garbage collect.
pub struct Arena {
    buffer: *mut u8,
    size: usize,
    bump: Cell<usize>,
}


impl Arena {
    pub fn new(size: usize) -> Result<Arena, MemError> {
        let buffer = unsafe { allocate(size) };

        if buffer == ptr::null_mut() {
            Err(MemError::OOM)
        } else {
            Ok(
                Arena {
                    buffer: buffer,
                    size: size,
                    bump: Cell::new(0),
                }
            )
        }
    }

    // Allocate a new object and return it's pointer
    fn inner_alloc<T>(&self, object: T) -> Result<*mut T, MemError> {
        let next_bump = self.bump.get() + mem::size_of::<T>();

        if next_bump > self.size {
            Err(MemError::OOM)
        } else {

            let ptr = unsafe {
                let p = self.buffer.offset(self.bump.get() as isize) as *mut T;
                ptr::write(p, object);
                p
            };

            self.bump.set(next_bump);

            Ok(ptr)
        }
    }
}


impl Heap for Arena {
    fn alloc<Pair>(&self, object: Pair) -> Result<TaggedPtr, MemError> {
        Err(MemError::OOM)
    }

    fn alloc_static<T>(&self, object: T) -> Result<TaggedPtr, MemError> {
        Err(MemError::OOM)
    }

    fn lookup_symbol(&self, name: &str) -> Result<TaggedPtr, MemError> {
        Err(MemError::OOM)
    }
}


impl Drop for Arena {
    fn drop(&mut self) {
        unsafe { deallocate(self.buffer, self.size) };
    }
}


#[cfg(test)]
mod test {

    use super::*;

    struct Thing {
        a: u8,
        b: u16,
        c: u32,
        d: u64,
    }

    impl Thing {
        fn new() -> Thing {
            Thing {
                a: 1,
                b: 2,
                c: 3,
                d: 4,
            }
        }

        fn check(&self) -> bool {
            self.a == 1 && self.b == 2 && self.c == 3 && self.d == 4
        }
    }


    #[test]
    fn test_alloc_struct() {
        let mem = Arena::new(1024);
        let ptr = mem.alloc(Thing::new());
        assert!(ptr.check());
    }

    #[test]
    #[should_panic]
    fn test_out_of_memory() {
        let mem = Arena::new(1024);
        loop {
            let _ptr = mem.alloc(Thing::new());
        }
    }
}
