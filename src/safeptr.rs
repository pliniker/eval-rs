use std::cell::Cell;
use std::fmt;
use std::marker::PhantomData;
use std::ops::Deref;

use stickyimmix::RawPtr;

use crate::heap::Environment;
use crate::taggedptr::{FatPtr, TaggedPtr, Value};

/// Type that provides a generic anchor for mutator timeslice lifetimes
pub trait MutatorScope {}

/// A thing to limit moveability and lifetime of ScopedPtr pointers; also the mutator's view into
/// an allocation API. The lifetime of an instance of this type must be shared via the
/// `MutatorScope` trait as `guard: &'scope MutatorScope`. This parameter exists soley to enforce
/// the lifetime limit on accessing GC-managed objects and should be optimized out.
pub struct MutatorScopeGuard<'env> {
    env: &'env Environment,
}

impl<'env> MutatorScopeGuard<'env> {
    pub fn new(env: &'env Environment) -> MutatorScopeGuard {
        MutatorScopeGuard { env }
    }

    pub fn get_reg(&self, reg: usize) -> ScopedPtr<'_> {
        ScopedPtr::new(self, self.env.get_reg(reg))
    }

    pub fn set_reg(&self, reg: usize, ptr: ScopedPtr<'_>) {
        self.env.set_reg(reg, ptr.ptr);
    }

    pub fn alloc<T>(&self, object: T) -> ScopedPtr<'_>
    where
        FatPtr: From<RawPtr<T>>,
    {
        ScopedPtr::new(self, self.env.alloc(object))
    }

    pub fn alloc_into_reg<T>(&self, reg: usize, object: T) -> ScopedPtr<'_>
    where
        FatPtr: From<RawPtr<T>>,
    {
        ScopedPtr::new(self, self.env.alloc_into_reg(reg, object))
    }
}

impl<'env> MutatorScope for MutatorScopeGuard<'env> {}

/// A pointer type encapsulating `FatPtr` with scope limited by `MutatorScopeGuard` such that a
/// `Value` instance can safely be derived and accessed. This type is neccessary to derive
/// `Value`s from
#[derive(Copy, Clone)]
pub struct ScopedPtr<'guard> {
    ptr: FatPtr,
    value: Value<'guard>,
    _mkr: PhantomData<&'guard MutatorScope>,
}

impl<'guard> ScopedPtr<'guard> {
    pub fn new(_guard: &'guard MutatorScope, thing: FatPtr) -> ScopedPtr<'guard> {
        ScopedPtr {
            ptr: thing,
            value: unsafe { thing.as_value() },
            _mkr: PhantomData,
        }
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

/// A wrapper around `TaggedPtr` for storing pointers in data structures with interior mutability,
/// allowing pointers to be updated to point at different target objects.
pub struct CellPtr {
    inner: Cell<TaggedPtr>,
}

impl CellPtr {
    /// Construct a new Nil CellPtr instance
    pub fn new_nil() -> CellPtr {
        CellPtr {
            inner: Cell::new(TaggedPtr::nil())
        }
    }

    /// This gets the pointer as a `Value` type, providing the enclosing `Value` instance as the
    /// lifetime guard
    pub fn get<'scope>(&self, _guard: &'scope MutatorScope) -> Value<'scope> {
        let fat_ptr = FatPtr::from(self.inner.get());
        unsafe { fat_ptr.as_value() }
    }

    /// Set this pointer to point at the same object as a given `ScopedPtr` instance
    pub fn set(&self, source: &ScopedPtr) {
        self.inner.set(TaggedPtr::from(source.ptr))
    }

    /// Take the pointer of another `CellPtr` and set this instance to point at that object too
    pub fn copy_from(&self, other: &CellPtr) {
        self.inner.set(other.inner.get());
    }
}
