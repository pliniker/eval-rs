use std::cell::Cell;
use std::cmp::{Eq, PartialEq};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::mem;
use std::ops::{Deref, DerefMut};
use std::ptr;

use memalloc::{allocate, deallocate};


pub struct Ptr<'a, T, A: 'a + Allocator> {
    ptr: *mut T,
    _marker: PhantomData<&'a A>
}


impl<'a, T, A: 'a + Allocator> Ptr<'a, T, A> {
    /// Pointer identity comparison
    pub fn is<'b, B: 'b + Allocator>(&self, other: Ptr<'b, T, B>) -> bool {
        self.ptr == other.ptr
    }
}


impl<'a, T, A: 'a + Allocator> Deref for Ptr<'a, T, A> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.ptr }
    }
}


impl<'a, T, A: 'a + Allocator> DerefMut for Ptr<'a, T, A> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.ptr }
    }
}


impl<'a, T: Hash, A: 'a + Allocator> Hash for Ptr<'a, T, A> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Self::deref(self).hash(state);
    }
}


impl<'a, T, A: 'a + Allocator> PartialEq for Ptr<'a, T, A> {
    fn eq(&self, other: &Ptr<'a, T, A>) -> bool {
        self.ptr == other.ptr
    }
}


impl<'a, T, A: 'a + Allocator> Eq for Ptr<'a, T, A> {}


impl<'a, T, A: 'a + Allocator> Copy for Ptr<'a, T, A> {}


// We don't want to force A to be Clone, so we can't #[derive(Copy, Clone)]
impl<'a, T, A: 'a + Allocator> Clone for Ptr<'a, T, A> {
    fn clone(&self) -> Ptr<'a, T, A> {
        Ptr {
            ptr: self.ptr,
            _marker: PhantomData
        }
    }
}


/// An allocator trait that is expected throughout the source code. This should
/// serve to abstract any allocator backing, allowing easier experimentation.
pub trait Allocator {
    fn alloc<T>(&self, object: T) -> Ptr<T, Self> where Self: Sized;
}


/// A fixed-size block of contiguous bytes type that implements the Allocator
/// trait. When it's full, it panics.
pub struct Arena {
    buffer: *mut u8,
    size: usize,
    bump: Cell<usize>,
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
            bump: Cell::new(0),
        }
    }
}


impl Allocator for Arena {
    fn alloc<T>(&self, object: T) -> Ptr<T, Self> {
        let next_bump = self.bump.get() + mem::size_of::<T>();
        if next_bump > self.size {
            panic!("out of memory");
        }

        let p = unsafe {
            let p = self.buffer.offset(self.bump.get() as isize) as *mut T;
            ptr::write(p, object);
            p
        };

        self.bump.set(next_bump);

        Ptr {
            ptr: p,
            _marker: PhantomData
        }
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
