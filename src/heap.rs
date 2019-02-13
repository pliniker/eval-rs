use std::cell::Cell;
use std::marker::PhantomData;

use stickyimmix::{RawPtr, StickyImmixHeap};

use crate::headers::ObjectHeader;
use crate::safeptr::{CellPtr, MutatorScopeGuard};
use crate::symbolmap::SymbolMap;
use crate::taggedptr::{FatPtr, TaggedPtr};

/// The heap implementation
pub type Heap = StickyImmixHeap<ObjectHeader>;

/// Describes the memory interface to the mutator
pub trait Memory {
    type Ptr;

    fn get_reg(&self, reg: usize) -> Self::Ptr;
    fn set_reg(&self, reg: usize, ptr: Self::Ptr);
    fn alloc<T>(&self, object: T) -> Self::Ptr;
    fn lookup_sym(&self, name: &str) -> Self::Ptr;
}

// A minimal pretend GC environment
pub struct Environment {
    heap: Heap,
    syms: SymbolMap,
    regs: Vec<CellPtr>,
}

impl Environment {
    pub fn new() -> Environment {
        let capacity = 256;

        let mut regs = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            regs.push(Cell::new(FatPtr::Nil));
        }

        Environment {
            heap: Heap::new(),
            syms: SymbolMap::new(),
            regs: regs
        }
    }

    pub fn mutate<F>(&self, f: F)
    where
        F: Fn(&mut MutatorScopeGuard),
    {
        let mut guard = MutatorScopeGuard::new(self);
        f(&mut guard);
    }
}

impl Memory for Environment {
    type Ptr = FatPtr;

    fn get_reg(&self, reg: usize) -> FatPtr {
        self.regs[reg].get()
    }

    fn set_reg(&self, reg: usize, ptr: FatPtr) {
        self.regs[reg].set(ptr);
    }

    fn alloc<T>(&self, object: T) -> FatPtr
//    where
//        FatPtr: From<RawPtr<T>>,
    {
        FatPtr::Nil //from(RawPtr::new(object))
    }

    fn lookup_sym(&self, name: &str) -> FatPtr {
        FatPtr::from(self.syms.lookup(name))
    }
}
