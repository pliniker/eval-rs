use std::mem;
use std::ops::{Deref, DerefMut};
use std::ptr;

use memalloc::{allocate, deallocate};


// A raw pointer abstraction. This *should* be lifetime-tied to an Arena, but
// I don't know how to do that without proliferating lifetime annotation
// *everywhere* and complicating things horribly.
pub struct Ptr<T> {
    ptr: *mut T,
}


impl<T> Ptr<T> {
    /// Pointer identity comparison
    pub fn is(&self, other: Ptr<T>) -> bool {
        self.ptr == other.ptr
    }
}


impl<T> Deref for Ptr<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.ptr }
    }
}


impl<T> DerefMut for Ptr<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.ptr }
    }
}


impl<T> Copy for Ptr<T> {}


impl<T> Clone for Ptr<T> {
    fn clone(&self) -> Ptr<T> {
        Ptr { ptr: self.ptr }
    }
}


/// An allocator trait that is expected throughout the source code. This should
/// serve to abstract any allocator backing, allowing easier experimentation.
pub trait Allocator {
    fn alloc<T>(&mut self, object: T) -> Ptr<T>;
}


/// A fixed-size block of contiguous bytes type that implements the Allocator
/// trait. When it's full, it panics.
pub struct Arena {
    buffer: *mut u8,
    size: usize,
    bump: usize,
}


impl Arena {
    pub fn new(size: usize) -> Arena {
        let buffer = unsafe { allocate(size) };

        if buffer == ptr::null_mut() {
            panic!("could not allocate memory!");
        }

        Arena {
            buffer: buffer,
            size: size,
            bump: 0,
        }
    }
}

impl Allocator for Arena {
    fn alloc<T>(&mut self, object: T) -> Ptr<T> {
        let next_bump = self.bump + mem::size_of::<T>();
        if next_bump > self.size {
            panic!("out of memory");
        }

        let p = unsafe {
            let p = self.buffer.offset(self.bump as isize) as *mut T;
            ptr::write(p, object);
            p
        };

        self.bump = next_bump;

        Ptr { ptr: p }
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
        let mut mem = Arena::new(1024);
        let ptr = mem.alloc(Thing::new());
        assert!(ptr.check());
    }

    #[test]
    #[should_panic]
    fn test_out_of_memory() {
        let mut mem = Arena::new(1024);
        loop {
            let _ptr = mem.alloc(Thing::new());
        }
    }
}
