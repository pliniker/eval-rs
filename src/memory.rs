/// VM-level memory abstraction
use std::ops::Deref;
use std::rc::Rc;

use stickyimmix::{AllocObject, AllocRaw, RawPtr};

use crate::heap::Heap;
use crate::safeptr::{CellPtr, MutatorScope, ScopedPtr};

use crate::headers::TypeList;
use crate::symbolmap::SymbolMap;
use crate::taggedptr::{FatPtr, TaggedPtr};

/// A stack (well, a Vec of registers for now). Each register is a `CellPtr` with getters and
/// setters that require a MutatorScope lifetime'd guard.
struct Stack {
    regs: Vec<CellPtr>,
}

impl Stack {
    /// Return a Stack instance with all registers initialized to nil
    fn new() -> Stack {
        let capacity = 256;

        let mut regs = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            regs.push(CellPtr::new_nil());
        }

        Stack {
            regs
        }
    }

    /// Get the copy of the pointer for the given register as a ScopedPtr
    fn get_reg<'guard>(&self, reg: usize, guard: &'guard MutatorScope) -> ScopedPtr<'guard> {
        self.regs[reg].get(guard)
    }

    /// Write a pointer into the specified register
    fn set_reg(&self, reg: usize, ptr: ScopedPtr<'_>) {
        self.regs[reg].set(ptr);
    }
}

// Heap memory types. Needs a better name.
struct Memory {
    heap: Heap,
    syms: SymbolMap,
}

impl Memory {
    fn new() -> Memory {
        Memory {
            heap: Heap::new(),
            syms: SymbolMap::new(),
        }
    }

    fn lookup_sym(&self, name: &str) -> TaggedPtr {
        TaggedPtr::symbol(self.syms.lookup(name))
    }

    fn alloc<T>(&self, object: T) -> TaggedPtr
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
}

/// This type describes the mutator's view into memory - the heap and symbol name/ptr lookup.
///
/// It implements `MutatorScope` such that any `ScopedPtr` or `Value` instances must be lifetime-
/// limited to the lifetime of this instance using `&'scope MutatorScope`;
pub struct MutatorView<'guard> {
    mem: &'guard Memory,
    stack: &'guard Stack,
}

impl<'guard> MutatorView<'guard> {
    fn new(foo: &'guard System) -> MutatorView<'guard> {
        MutatorView {
            mem: &foo.mem,
            stack: &foo.stack,
        }
    }

    pub fn get_reg(&self, reg: usize) -> ScopedPtr<'_> {
        self.stack.get_reg(reg, self)
    }

    pub fn set_reg(&self, reg: usize, ptr: ScopedPtr<'_>) {
        self.stack.set_reg(reg, ptr);
    }

    pub fn alloc<T>(&self, object: T) -> ScopedPtr<'_>
    where
        FatPtr: From<RawPtr<T>>,
        T: AllocObject<TypeList>
    {
        ScopedPtr::new(self, self.mem.alloc(object))
    }

    pub fn lookup_sym(&self, name: &str) -> ScopedPtr<'_> {
        ScopedPtr::new(self, self.mem.lookup_sym(name))
    }
}

impl<'guard> MutatorScope for MutatorView<'guard> {}

// Heam and stack. Needs a better name.
pub struct System {
    mem: Memory,
    stack: Stack
}

impl System {
    /// Run a mutator process
    pub fn mutate<F>(&self, f: F)
    where
        F: Fn(&MutatorView),
    {
        let mut guard = MutatorView::new(self);
        f(&mut guard);
    }

    // pub fn collect()
}
