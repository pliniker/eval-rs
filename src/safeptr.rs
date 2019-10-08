use std::cell::Cell;
use std::fmt;
use std::ops::Deref;

use stickyimmix::RawPtr;

use crate::taggedptr::{FatPtr, TaggedPtr, Value};

/// Type that provides a generic anchor for mutator timeslice lifetimes
pub trait MutatorScope {}


#[derive(Copy, Clone)]
struct ScopedPtr<'scope, T: Sized> {
    value: &'scope T,
}

impl<'scope, T: Sized> ScopedPtr<'scope, T> {}

#[derive(Clone)]
struct CellPtr<T: Sized> {
    inner: Cell<RawPtr<T>>,
}

impl<T: Sized> CellPtr<T> {
    /// Construct a new CellPtr from a ScopedPtr
    pub fn new_with(source: ScopedPtr<T>) -> CellPtr<T> {
        CellPtr {
            inner: Cell::new(TaggedPtr::from(source.ptr)),
        }
    }

    pub fn get<'scope>(&self, guard: &'scope dyn MutatorScope) -> ScopedPtr<'scope, T> {
        ScopedPtr::new(guard, self.inner.get())
    }

    pub fn set(&self, source: ScopedPtr<T>) {
        self.inner.set(TaggedPtr::from(source.ptr))
    }

    pub fn copy_from(&self, other: &CellPtr<T>) {
        self.inner.set(other.inner.get());
    }
}


/// A pointer type with scope limited by `MutatorScopeGuard` such that a `Value` instance can
/// safely be derived and accessed. This type is neccessary to derive `Value`s from
#[derive(Copy, Clone)]
pub struct TaggedScopedPtr<'guard> {
    ptr: TaggedPtr,
    value: Value<'guard>,
}

impl<'guard> TaggedScopedPtr<'guard> {
    pub fn new(guard: &'guard dyn MutatorScope, ptr: TaggedPtr) -> TaggedScopedPtr<'guard> {
        TaggedScopedPtr {
            ptr: ptr,
            value: FatPtr::from(ptr).as_value(guard),
        }
    }

    pub fn value(&self) -> Value<'guard> {
        self.value
    }
}

impl<'guard> Deref for TaggedScopedPtr<'guard> {
    type Target = Value<'guard>;

    fn deref(&self) -> &Value<'guard> {
        &self.value
    }
}

impl<'guard> fmt::Display for TaggedScopedPtr<'guard> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.value.fmt(f)
    }
}

impl<'guard> fmt::Debug for TaggedScopedPtr<'guard> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.value.fmt(f)
    }
}

impl<'guard> PartialEq for TaggedScopedPtr<'guard> {
    fn eq(&self, rhs: &TaggedScopedPtr<'guard>) -> bool {
        self.ptr == rhs.ptr
    }
}

/// A wrapper around `TaggedPtr` for storing pointers in data structures with interior mutability,
/// allowing pointers to be updated to point at different target objects.
#[derive(Clone)]
pub struct TaggedCellPtr {
    inner: Cell<TaggedPtr>,
}

impl TaggedCellPtr {
    /// Construct a new Nil TaggedCellPtr instance
    pub fn new_nil() -> TaggedCellPtr {
        TaggedCellPtr {
            inner: Cell::new(TaggedPtr::nil()),
        }
    }

    /// Construct a new TaggedCellPtr from a TaggedScopedPtr
    pub fn new_with(source: TaggedScopedPtr) -> TaggedCellPtr {
        TaggedCellPtr {
            inner: Cell::new(TaggedPtr::from(source.ptr)),
        }
    }

    /// Return the pointer as a `TaggedScopedPtr` type that carries a copy of the `TaggedPtr` and
    /// a `Value` type for both copying and access convenience
    pub fn get<'scope>(&self, guard: &'scope dyn MutatorScope) -> TaggedScopedPtr<'scope> {
        TaggedScopedPtr::new(guard, self.inner.get())
    }

    /// This returns the pointer as a `Value` type, given a mutator scope for safety
    pub fn get_value<'scope>(&self, guard: &'scope dyn MutatorScope) -> Value<'scope> {
        FatPtr::from(self.inner.get()).as_value(guard)
    }

    /// Set this pointer to point at the same object as a given `TaggedScopedPtr` instance
    pub fn set(&self, source: TaggedScopedPtr) {
        self.inner.set(TaggedPtr::from(source.ptr))
    }

    /// Take the pointer of another `TaggedCellPtr` and set this instance to point at that object too
    pub fn copy_from(&self, other: &TaggedCellPtr) {
        self.inner.set(other.inner.get());
    }

    /// Return true if the pointer is nil
    pub fn is_nil(&self) -> bool {
        self.inner.get().is_nil()
    }
}
