use std::cmp::{Eq, PartialEq};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

/// Universal pointer type with a lifetime restricted to the Heap that
/// instantiated it.
pub struct Ptr<'heap, T, A: 'heap + Heap> {
    ptr: *mut T,
    _marker: PhantomData<&'heap A>
}


impl<'heap, T, A: 'heap + Heap> Ptr<'heap, T, A> {
    /// Pointer identity comparison
    pub fn is<'anyheap, B: 'anyheap + Heap>(&self, other: Ptr<'anyheap, T, B>) -> bool {
        self.ptr == other.ptr
    }
}


impl<'heap, T, A: 'heap + Heap> Deref for Ptr<'heap, T, A> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.ptr }
    }
}


impl<'heap, T, A: 'heap + Heap> DerefMut for Ptr<'heap, T, A> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.ptr }
    }
}


impl<'heap, T: Hash, A: 'heap + Heap> Hash for Ptr<'heap, T, A> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Self::deref(self).hash(state);
    }
}


impl<'heap, T, A: 'heap + Heap> PartialEq for Ptr<'heap, T, A> {
    fn eq(&self, other: &Ptr<'heap, T, A>) -> bool {
        self.ptr == other.ptr
    }
}


impl<'heap, T, A: 'heap + Heap> Eq for Ptr<'heap, T, A> {}


impl<'heap, T, A: 'heap + Heap> Copy for Ptr<'heap, T, A> {}


// We don't want to force A to be Clone, so we can't #[derive(Copy, Clone)]
impl<'heap, T, A: 'heap + Heap> Clone for Ptr<'heap, T, A> {
    fn clone(&self) -> Ptr<'heap, T, A> {
        Ptr {
            ptr: self.ptr,
            _marker: PhantomData
        }
    }
}
