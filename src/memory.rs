use std::cell::Cell;
use std::cmp::{Eq, PartialEq};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::mem;
use std::ops::{Deref, DerefMut};
use std::ptr;

use memalloc::{allocate, deallocate};



/// An allocator trait that is expected throughout the source code. This should
/// serve to abstract any allocator backing, allowing easier experimentation.
pub trait Allocator {
    /// Allocate and move an object into the space. This object may be garbage
    /// collected or moved.
    fn alloc<T>(&self, object: T) -> Ptr<T, Self> where Self: Sized;

    /// Allocate an object with a lifetime and address that lives as long as the
    /// allocator itself, i.e. is not dynamically garbage collected.
    fn alloc_static<T>(&self, object: T) -> Ptr<T, Self> where Self: Sized;

    // /// Run a garbage collection iteration.
    //fn collect(&mut self);
}


/*
What is it we need?

 * A vm has a stack, code and a pc
 * A stack has a heap
 * A heap has:
 * an allocator
 * a GC

A symbol table has a hashmap and an append-only-heap

1. the stack is the window into the heap
2. an stack context allocates new heap pointers but stores them straight onto the stack

1. a mutator is subservient to the memory structure (stack + heap)
2. to describe heap operations in terms of the heap *owning* it's memory and allocator
3. heap operations can't escape a heap borrow context

heap.mutate(|mem| ptr = mem.alloc())


pub trait StackFrames {}

pub trait Stack {
    type Heap: Allocator;
    type Frames: StackFrames;

    fn mutate<M>(mutator: &mut M) where M: FnMut(&mut Self::Frames, &mut Self::Heap);
}
*/


pub struct Ptr<'storage, T, A: 'storage + Allocator> {
    ptr: *mut T,
    _marker: PhantomData<&'storage A>
}


impl<'storage, T, A: 'storage + Allocator> Ptr<'storage, T, A> {
    /// Pointer identity comparison
    pub fn is<'anyheap, B: 'anyheap + Allocator>(&self, other: Ptr<'anyheap, T, B>) -> bool {
        self.ptr == other.ptr
    }
}


impl<'storage, T, A: 'storage + Allocator> Deref for Ptr<'storage, T, A> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.ptr }
    }
}


impl<'storage, T, A: 'storage + Allocator> DerefMut for Ptr<'storage, T, A> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.ptr }
    }
}


impl<'storage, T: Hash, A: 'storage + Allocator> Hash for Ptr<'storage, T, A> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Self::deref(self).hash(state);
    }
}


impl<'storage, T, A: 'storage + Allocator> PartialEq for Ptr<'storage, T, A> {
    fn eq(&self, other: &Ptr<'storage, T, A>) -> bool {
        self.ptr == other.ptr
    }
}


impl<'storage, T, A: 'storage + Allocator> Eq for Ptr<'storage, T, A> {}


impl<'storage, T, A: 'storage + Allocator> Copy for Ptr<'storage, T, A> {}


// We don't want to force A to be Clone, so we can't #[derive(Copy, Clone)]
impl<'storage, T, A: 'storage + Allocator> Clone for Ptr<'storage, T, A> {
    fn clone(&self) -> Ptr<'storage, T, A> {
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

    // In this implementation, alloc and alloc_static are the same because
    // no moving or collection occurs anyway.
    fn alloc_static<T>(&self, object: T) -> Ptr<T, Self> {
        self.alloc(object)
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
