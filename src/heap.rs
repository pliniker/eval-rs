use std::cell::Cell;
use std::marker::PhantomData;

use stickyimmix::{RawPtr, StickyImmixHeap};

use crate::headers::ObjectHeader;
use crate::safeptr::MutatorScopeGuard;
use crate::symbolmap::SymbolMap;
use crate::taggedptr::{FatPtr, TaggedPtr};

pub type Heap = StickyImmixHeap<ObjectHeader>;

// A minimal pretend GC environment
pub struct Environment {
    regs: Vec<Cell<FatPtr>>,
}

impl Environment {
    fn new() -> Environment {
        let capacity = 256;

        let mut regs = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            regs.push(Cell::new(FatPtr::Nil));
        }

        Environment {
            regs: regs
        }
    }

    pub fn get_reg(&self, reg: usize) -> FatPtr {
        self.regs[reg].get()
    }

    pub fn set_reg(&self, reg: usize, ptr: FatPtr) {
        self.regs[reg].set(ptr);
    }

    // Heap-allocate an unrooted object
    pub fn alloc<T>(&self, object: T) -> FatPtr
    where
        FatPtr: From<RawPtr<T>>
    {
        FatPtr::from(RawPtr::new(object))
    }

    // Allocate an object and store it's pointer into the specified register number
    pub fn alloc_into_reg<T>(&self, reg: usize, object: T) -> FatPtr
    where
        FatPtr: From<RawPtr<T>>
    {
        let ptr = FatPtr::from(RawPtr::new(object));
        self.regs[reg].set(ptr);
        ptr
    }

    fn mutate<F>(&self, f: F) where F: Fn(&mut MutatorScopeGuard) {
        let mut guard = MutatorScopeGuard::new(self);
        f(&mut guard);
    }
}
