/// VM-level memory abstraction
use std::rc::Rc;

use stickyimmix::{AllocObject, AllocRaw, RawPtr};

use crate::heap::Heap;
use crate::safeptr::{CellPtr, MutatorScope, ScopedPtr};

use crate::headers::TypeList;
use crate::symbolmap::SymbolMap;
use crate::taggedptr::{Value, FatPtr, TaggedPtr};

/// This type describes the mutator's view into memory - the heap and symbol name/ptr lookup.
///
/// It implements `MutatorScope` such that any `ScopedPtr` or `Value` instances must be lifetime-
/// limited to the lifetime of this instance using `&'scope MutatorScope`;
pub struct MemoryView<'env> {
    env: &'env Memory,
    regs: Vec<CellPtr>,
}

impl<'env> MemoryView<'env> {
    pub fn new(env: &'env Memory) -> MemoryView {
        let capacity = 256;

        let mut regs = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            regs.push(CellPtr::new_nil());
        }

        MemoryView {
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

impl<'env> MutatorScope for MemoryView<'env> {}

// Interpreter base memory abstraction that encapsulates allocation and symbol mapping.
pub struct Memory {
    heap: Heap,
    syms: SymbolMap,
}

impl Memory {
    pub fn new() -> Memory {
        Memory {
            heap: Heap::new(),
            syms: SymbolMap::new(),
        }
    }

    pub fn lookup_sym(&self, name: &str) -> FatPtr {
        FatPtr::from(self.syms.lookup(name))
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

    pub fn mutate<F>(&self, f: F)
    where
        F: Fn(&MemoryView),
    {
        let mut guard = MemoryView::new(self);
        f(&mut guard);
    }
}

type SharedMemory = Rc<Memory>;
