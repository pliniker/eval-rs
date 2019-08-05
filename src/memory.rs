/// VM-level memory abstraction
///
/// Defines Stack, Heap and Memory types, and a MemoryView type that gives a mutator a safe
/// view into the stack and heap.
use stickyimmix::{AllocObject, AllocRaw, ArraySize, RawPtr, StickyImmixHeap};

use crate::containers::{Container, IndexedAnyContainer, StackAnyContainer};
use crate::error::RuntimeError;
use crate::headers::{ObjectHeader, TypeList};
use crate::primitives::ArrayAny;
use crate::safeptr::{CellPtr, MutatorScope, ScopedPtr};
use crate::symbolmap::SymbolMap;
use crate::taggedptr::{FatPtr, TaggedPtr};

/// This type describes the mutator's view into memory - the heap and symbol name/ptr lookup.
///
/// It implements `MutatorScope` such that any `ScopedPtr` or `Value` instances must be lifetime-
/// limited to the lifetime of this instance using `&'scope MutatorScope`;
pub struct MutatorView<'memory> {
    heap: &'memory Heap,
    stack: &'memory Stack,
}

impl<'memory> MutatorView<'memory> {
    fn new(foo: &'memory Memory) -> MutatorView<'memory> {
        MutatorView {
            heap: &foo.heap,
            stack: &foo.stack,
        }
    }

    /// Get the copy of the pointer for the given register
    pub fn get_reg(&self, reg: ArraySize) -> ScopedPtr<'_> {
        self.stack.get_reg(self, reg)
    }

    /// Write a pointer into the specified register
    pub fn set_reg(&self, reg: ArraySize, ptr: ScopedPtr<'_>) {
        self.stack.set_reg(reg, ptr);
    }

    /// Get a Symbol pointer from its name
    pub fn lookup_sym(&self, name: &str) -> ScopedPtr<'_> {
        ScopedPtr::new(self, self.heap.lookup_sym(name))
    }

    /// Write an object into the heap and return the pointer to it
    pub fn alloc<T>(&self, object: T) -> Result<ScopedPtr<'_>, RuntimeError>
    where
        FatPtr: From<RawPtr<T>>,
        T: AllocObject<TypeList>,
    {
        Ok(ScopedPtr::new(self, self.heap.alloc(object)?))
    }

    /// Make space for an array of bytes
    pub fn alloc_array(&self, capacity: ArraySize) -> Result<RawPtr<u8>, RuntimeError> {
        self.heap.alloc_array(capacity)
    }

    pub fn nil(&self) -> ScopedPtr<'_> {
        ScopedPtr::new(self, TaggedPtr::nil())
    }
}

impl<'guard> MutatorScope for MutatorView<'guard> {}

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

        Stack { regs }
    }

    /// Get the copy of the pointer for the given register as a ScopedPtr
    fn get_reg<'guard>(&self, guard: &'guard MutatorScope, reg: ArraySize) -> ScopedPtr<'guard> {
        self.regs[reg as usize].get(guard)
    }

    /// Write a pointer into the specified register
    fn set_reg(&self, reg: ArraySize, ptr: ScopedPtr<'_>) {
        self.regs[reg as usize].set(ptr);
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

    /// Write an object into the heap and return the pointer to it
    fn alloc<T>(&self, object: T) -> Result<TaggedPtr, RuntimeError>
    where
        FatPtr: From<RawPtr<T>>,
        T: AllocObject<TypeList>,
    {
        Ok(TaggedPtr::from(FatPtr::from(self.heap.alloc(object)?)))
    }

    fn alloc_array(&self, capacity: ArraySize) -> Result<RawPtr<u8>, RuntimeError> {
        Ok(self.heap.alloc_array(capacity)?)
    }
}

// Composed of a Heap and a Stack instance
pub struct Memory {
    heap: Heap,
    stack: Stack,
}

impl Memory {
    /// Instantiate a new memory environment
    pub fn new() -> Memory {
        Memory {
            heap: Heap::new(),
            stack: Stack::new(),
        }
    }

    /// Run a mutator process
    pub fn mutate<F>(&self, f: F) -> Result<(), RuntimeError>
    where
        F: Fn(&MutatorView) -> Result<(), RuntimeError>,
    {
        let mut guard = MutatorView::new(self);
        f(&mut guard)
    }

    // TODO pub fn collect()
}

/// This represents a pointer to a window of registers on the stack
struct ActivationFramePtr {
    regs: [CellPtr; 256]
}

struct RegisterStack {
    regs: ArrayAny,
    base: ArraySize,
}

impl RegisterStack {
    fn new() -> RegisterStack {
        RegisterStack {
            regs: ArrayAny::new(),
            base: 0
        }
    }

    // fn current_frame(&self) -> ArraySlice {}

    // Pad the stack out to 256 entries from the current top of the stack
    fn pad<'guard>(
        &self,
        view: &'guard MutatorView,
        pad_base: ArraySize,
        pad_length: ArraySize
    ) -> Result<(), RuntimeError> {
        let pad = self.regs.length() - pad_base;
        if pad < pad_length {
            for _ in pad..pad_length {
                self.regs.push(view, view.nil())?;
            }
        }
        Ok(())
    }

    // TODO fn push_activation_record(&self, callee, param_count);
    // TODO fn alloc_activation_record()

    fn push_main<'guard>(&self, view: &'guard MutatorView) -> Result<(), RuntimeError> {
        self.pad(view, 0, 256)
    }
}
