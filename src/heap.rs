use std::cell::Cell;

use stickyimmix::{AllocObject, AllocRaw, RawPtr, StickyImmixHeap};

use crate::headers::{ObjectHeader, TypeList};
use crate::safeptr::MutatorScopeGuard;
use crate::symbolmap::SymbolMap;
use crate::taggedptr::{FatPtr, TaggedPtr};

/// The heap implementation
pub type Heap = StickyImmixHeap<ObjectHeader>;

// A minimal pretend GC environment
pub struct Environment {
    heap: Heap,
    syms: SymbolMap,
}

impl Environment {
    pub fn new() -> Environment {
        Environment {
            heap: Heap::new(),
            syms: SymbolMap::new(),
        }
    }

    pub fn mutate<F>(&self, f: F)
    where
        F: Fn(&mut MutatorScopeGuard),
    {
        let mut guard = MutatorScopeGuard::new(self);
        f(&mut guard);
    }

    pub fn alloc<T>(&self, object: T) -> TaggedPtr
    where
        FatPtr: From<RawPtr<T>>,
        T: AllocObject<TypeList>
    {
        if let Ok(rawptr) = self.heap.alloc(object) {
            TaggedPtr::from(FatPtr::from(rawptr))
        } else {
            TaggedPtr::nil()
        }
    }

    pub fn lookup_sym(&self, name: &str) -> FatPtr {
        FatPtr::from(self.syms.lookup(name))
    }
}
