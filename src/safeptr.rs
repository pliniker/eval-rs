use std::cell::Cell;
use std::fmt;
use std::marker::PhantomData;
use std::ops::Deref;

use stickyimmix::RawPtr;

use crate::taggedptr::{Value, FatPtr, TaggedPtr};
use crate::heap::Environment;

// A thing to limit moveability and lifetime of ScopedPtr pointers; also the mutator's view into
/// an allocation API
pub struct MutatorScopeGuard<'env> {
    env: &'env Environment
}

impl<'env> MutatorScopeGuard<'env> {
    fn new(env: &'env Environment) -> MutatorScopeGuard {
        MutatorScopeGuard { env }
    }

    fn get_reg(&self, reg: usize) -> ScopedPtr<'_, 'env> {
        ScopedPtr::new(self, self.env.get_reg(reg))
    }

    fn set_reg(&self, reg: usize, ptr: ScopedPtr<'_, 'env>) {
        self.env.set_reg(reg, ptr.ptr);
    }

    fn alloc<T>(&self, object: T) -> ScopedPtr<'_, 'env>
    where
        FatPtr: From<RawPtr<T>>
    {
        ScopedPtr::new(self, self.env.alloc(object))
    }

    fn alloc_into_reg<T>(&self, reg: usize, object: T) -> ScopedPtr<'_, 'env>
    where
        FatPtr: From<RawPtr<T>>
    {
        ScopedPtr::new(self, self.env.alloc_into_reg(reg, object))
    }
}

/// A pointer type encapsulating `FatPtr` with scope limited by `MutatorScopeGuard` such that a
/// `Value` instance can safely be derived and accessed.
#[derive(Copy, Clone)]
pub struct ScopedPtr<'guard, 'env: 'guard> {
    ptr: FatPtr,
    value: Value<'guard>,
    _mkr: PhantomData<&'guard MutatorScopeGuard<'env>>,
}

impl<'guard, 'env> ScopedPtr<'guard, 'env> {
    pub fn new(_guard: &'guard MutatorScopeGuard<'env>, thing: FatPtr) -> ScopedPtr<'guard, 'env> {
        ScopedPtr {
            ptr: thing,
            value: unsafe { thing.as_value() },
            _mkr: PhantomData,
        }
    }
}

impl<'guard, 'env> Deref for ScopedPtr<'guard, 'env> {
    type Target = Value<'guard>;

    fn deref(&self) -> &Value<'guard> {
        &self.value
    }
}

impl<'guard, 'env> fmt::Display for ScopedPtr<'guard, 'env> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.value.fmt(f)
    }
}

/// A wrapper around `TaggedPtr` for storing pointers in data structures with interior mutability,
/// allowing pointers to be updated to point at different target objects.
pub struct CellPtr {
    inner: Cell<TaggedPtr>
}

impl CellPtr {
    /// Read the pointer into a `ScopedPtr` for safe access
    pub fn get<'guard, 'env>(&self, _guard: &'guard MutatorScopeGuard<'env>) -> ScopedPtr<'guard, 'env> {
        let fat_ptr = FatPtr::from(self.inner.get());

        ScopedPtr {
            ptr: fat_ptr,
            value: unsafe { fat_ptr.as_value() },
            _mkr: PhantomData,
        }
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
