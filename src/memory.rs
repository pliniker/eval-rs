/// VM-level memory abstraction
///
/// Defines Stack, Heap and Memory types, and a MemoryView type that gives a mutator a safe
/// view into the stack and heap.
use std::ops::Deref;
use std::rc::Rc;

use stickyimmix::{AllocObject, AllocRaw, RawPtr, StickyImmixHeap};

use crate::safeptr::{CellPtr, MutatorScope, ScopedPtr};

use crate::headers::{ObjectHeader, TypeList};
use crate::symbolmap::SymbolMap;
use crate::taggedptr::{FatPtr, TaggedPtr};

/// The heap implementation
pub type HeapStorage = StickyImmixHeap<ObjectHeader>;

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

// Heap memory types.
struct Heap {
    heap: HeapStorage,
    syms: SymbolMap,
}

impl Heap {
    fn new() -> Heap {
        Heap {
            heap: HeapStorage::new(),
            syms: SymbolMap::new(),
        }
    }

    /// Get a Symbol pointer from its name
    fn lookup_sym(&self, name: &str) -> TaggedPtr {
        TaggedPtr::symbol(self.syms.lookup(name))
    }

    /// Write an object into the heapn and return the pointer to it
    /// TODO implement error handling
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
    heap: &'guard Heap,
    stack: &'guard Stack,
}

impl<'guard> MutatorView<'guard> {
    fn new(foo: &'guard Memory) -> MutatorView<'guard> {
        MutatorView {
            heap: &foo.heap,
            stack: &foo.stack,
        }
    }

    /// Get the copy of the pointer for the given register
    pub fn get_reg(&self, reg: usize) -> ScopedPtr<'_> {
        self.stack.get_reg(reg, self)
    }

    /// Write a pointer into the specified register
    pub fn set_reg(&self, reg: usize, ptr: ScopedPtr<'_>) {
        self.stack.set_reg(reg, ptr);
    }

    /// Get a Symbol pointer from its name
    pub fn lookup_sym(&self, name: &str) -> ScopedPtr<'_> {
        ScopedPtr::new(self, self.heap.lookup_sym(name))
    }

    /// Write an object into the heap and return the pointer to it
    pub fn alloc<T>(&self, object: T) -> ScopedPtr<'_>
    where
        FatPtr: From<RawPtr<T>>,
        T: AllocObject<TypeList>
    {
        ScopedPtr::new(self, self.heap.alloc(object))
    }
}

impl<'guard> MutatorScope for MutatorView<'guard> {}

// Composed of a Heap and a Stack instance
pub struct Memory {
    heap: Heap,
    stack: Stack
}

impl Memory {
    /// Run a mutator process
    pub fn mutate<F>(&self, f: F)
    where
        F: Fn(&MutatorView),
    {
        let mut guard = MutatorView::new(self);
        f(&mut guard);
    }

    // TODO pub fn collect()
}
