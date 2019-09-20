use std::cell::Cell;
use std::fmt;
use std::ops::Deref;

use crate::taggedptr::{FatPtr, TaggedPtr, Value};

/// Type that provides a generic anchor for mutator timeslice lifetimes
pub trait MutatorScope {}

/// A pointer type encapsulating `FatPtr` with scope limited by `MutatorScopeGuard` such that a
/// `Value` instance can safely be derived and accessed. This type is neccessary to derive
/// `Value`s from
#[derive(Copy, Clone)]
pub struct ScopedPtr<'guard> {
    ptr: TaggedPtr,
    value: Value<'guard>,
}

impl<'guard> ScopedPtr<'guard> {
    pub fn new(guard: &'guard dyn MutatorScope, ptr: TaggedPtr) -> ScopedPtr<'guard> {
        ScopedPtr {
            ptr: ptr,
            value: FatPtr::from(ptr).as_value(guard),
        }
    }

    pub fn value(&self) -> Value<'guard> {
        self.value
    }
}

impl<'guard> Deref for ScopedPtr<'guard> {
    type Target = Value<'guard>;

    fn deref(&self) -> &Value<'guard> {
        &self.value
    }
}

impl<'guard> fmt::Display for ScopedPtr<'guard> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.value.fmt(f)
    }
}

impl<'guard> fmt::Debug for ScopedPtr<'guard> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.value.fmt(f)
    }
}

impl<'guard> PartialEq for ScopedPtr<'guard> {
    fn eq(&self, rhs: &ScopedPtr<'guard>) -> bool {
        self.ptr == rhs.ptr
    }
}

/// A wrapper around `TaggedPtr` for storing pointers in data structures with interior mutability,
/// allowing pointers to be updated to point at different target objects.
#[derive(Clone)]
pub struct CellPtr {
    inner: Cell<TaggedPtr>,
}

impl CellPtr {
    /// Construct a new Nil CellPtr instance
    pub fn new_nil() -> CellPtr {
        CellPtr {
            inner: Cell::new(TaggedPtr::nil()),
        }
    }

    /// Construct a new CellPtr from a ScopedPtr
    pub fn new_with(source: ScopedPtr) -> CellPtr {
        CellPtr {
            inner: Cell::new(TaggedPtr::from(source.ptr)),
        }
    }

    /// Return the pointer as a `ScopedPtr` type that carries a copy of the `TaggedPtr` and
    /// a `Value` type for both copying and access convenience
    pub fn get<'scope>(&self, guard: &'scope dyn MutatorScope) -> ScopedPtr<'scope> {
        ScopedPtr::new(guard, self.inner.get())
    }

    /// This returns the pointer as a `Value` type, given a mutator scope for safety
    pub fn get_value<'scope>(&self, guard: &'scope dyn MutatorScope) -> Value<'scope> {
        FatPtr::from(self.inner.get()).as_value(guard)
    }

    /// Set this pointer to point at the same object as a given `ScopedPtr` instance
    pub fn set(&self, source: ScopedPtr) {
        self.inner.set(TaggedPtr::from(source.ptr))
    }

    /// Take the pointer of another `CellPtr` and set this instance to point at that object too
    pub fn copy_from(&self, other: &CellPtr) {
        self.inner.set(other.inner.get());
    }

    /// Return true if the pointer is nil
    pub fn is_nil(&self) -> bool {
        self.inner.get().is_nil()
    }
}
