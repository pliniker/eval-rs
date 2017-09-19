use std::cell::Cell;
use std::cmp::{Eq, PartialEq};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::mem;
use std::ops::{Deref, DerefMut};
use std::ptr;

use memalloc::{allocate, deallocate};


// we need two allocators:
// 1. movable objects
// 2. immovable objects
//
// Either:
// trait Allocator { fn alloc(); fn alloc_immovable(); }
// heap: Heap;
//
// or
// trait Allocator { fn alloc(); }
// heap: MovableHeap; heap: StaticHeap;
// (two parameters everywhere)

/// An allocator trait that is expected throughout the source code. This should
/// serve to abstract any allocator backing, allowing easier experimentation.
pub trait Allocator {
    fn alloc<T>(&self, object: T) -> Ptr<T, Self> where Self: Sized;
}


// A trait that describes heap operations
pub trait Heap<A: Allocator> {
    fn alloc<T>(&self, object: T) -> Ptr<T, A> where Self: Sized;
    fn alloc_immovable<T>(&self, object: T) -> Ptr<T, A> where Self: Sized;
}


pub struct Ptr<'heap, T, A: 'heap + Allocator> {
    ptr: *mut T,
    _marker: PhantomData<&'heap A>
}


impl<'heap, T, A: 'heap + Allocator> Ptr<'heap, T, A> {
    /// Pointer identity comparison
    pub fn is<'anyheap, B: 'anyheap + Allocator>(&self, other: Ptr<'anyheap, T, B>) -> bool {
        self.ptr == other.ptr
    }
}


impl<'heap, T, A: 'heap + Allocator> Deref for Ptr<'heap, T, A> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.ptr }
    }
}


impl<'heap, T, A: 'heap + Allocator> DerefMut for Ptr<'heap, T, A> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.ptr }
    }
}


impl<'heap, T: Hash, A: 'heap + Allocator> Hash for Ptr<'heap, T, A> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Self::deref(self).hash(state);
    }
}


impl<'heap, T, A: 'heap + Allocator> PartialEq for Ptr<'heap, T, A> {
    fn eq(&self, other: &Ptr<'heap, T, A>) -> bool {
        self.ptr == other.ptr
    }
}


impl<'heap, T, A: 'heap + Allocator> Eq for Ptr<'heap, T, A> {}


impl<'heap, T, A: 'heap + Allocator> Copy for Ptr<'heap, T, A> {}


// We don't want to force A to be Clone, so we can't #[derive(Copy, Clone)]
impl<'heap, T, A: 'heap + Allocator> Clone for Ptr<'heap, T, A> {
    fn clone(&self) -> Ptr<'heap, T, A> {
        Ptr {
            ptr: self.ptr,
            _marker: PhantomData
        }
    }
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
