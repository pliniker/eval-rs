use std::cell::Cell;
use std::fmt;
use std::ops::Deref;

use stickyimmix::{AllocObject, RawPtr};

use crate::headers::TypeList;
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
    regs: Vec<CellPtr>,
}

impl<'env> MutatorScopeGuard<'env> {
    pub fn new(env: &'env Environment) -> MutatorScopeGuard {
        let capacity = 256;

        let mut regs = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            regs.push(CellPtr::new_nil());
        }

        MutatorScopeGuard {
            env,
            regs
        }
    }

    pub fn get_reg(&self, reg: usize) -> ScopedPtr<'_> {
        self.regs[reg].get(self)
    }

    pub fn set_reg(&self, reg: usize, ptr: ScopedPtr<'_>) {
        self.regs[reg].set(ptr);
    }

    pub fn alloc<T>(&self, object: T) -> ScopedPtr<'_>
    where
        FatPtr: From<RawPtr<T>>,
        T: AllocObject<TypeList>
    {
        ScopedPtr::new(self, self.env.alloc(object))
    }

    pub fn lookup_sym(&self, name: &str) -> Value<'_> {
        self.env.lookup_sym(name).as_value(self)
    }
}

impl<'env> MutatorScope for MutatorScopeGuard<'env> {}

/// A pointer type encapsulating `FatPtr` with scope limited by `MutatorScopeGuard` such that a
/// `Value` instance can safely be derived and accessed. This type is neccessary to derive
/// `Value`s from
#[derive(Copy, Clone)]
pub struct ScopedPtr<'guard> {
    ptr: TaggedPtr,
    value: Value<'guard>,
}

impl<'guard> ScopedPtr<'guard> {
    pub fn new(guard: &'guard MutatorScope, ptr: TaggedPtr) -> ScopedPtr<'guard> {
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

    /// Return the pointer as a `ScopedPtr` type that carries a copy of the `TaggedPtr` and
    /// a `Value` type for both copying and access convenience
    pub fn get<'scope>(&self, guard: &'scope MutatorScope) -> ScopedPtr<'scope> {
        ScopedPtr::new(guard, self.inner.get())
    }

    /// This returns the pointer as a `Value` type, given a mutator scope for safety
    pub fn get_value<'scope>(&self, guard: &'scope MutatorScope) -> Value<'scope> {
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
}
