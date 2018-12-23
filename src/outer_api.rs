/// The collector and mutator should be thought of as coroutines sharing
/// mutable access to the heap and roots data structures.

use std::cell::Cell;
use std::rc::{Rc, Weak};

use crate::heap::Heap;
use crate::symbolmap::SymbolMap;
use crate::taggedptr::{FatPtr, TaggedPtr};

/// Wrap up a heap, a symbol table and a reference to the stack roots in a
/// data structure. The heap and stack roots are the minimum necessary for
/// garbage collection.
struct Memory {
    heap: Heap,
    syms: SymbolMap,
    roots: Rc<Stack>,
}

impl Memory {
    fn new(stack: Rc<Stack>) -> Memory {
        Memory {
            heap: Heap::new(),
            syms: SymbolMap::new(),
            roots: stack,
        }
    }
}

/// The only roots allowed must be retained in the Stack structure.
/// Any pointers outside of this are not guaranteed to be safely dereferencable
/// or traced.
struct Stack {
    registers: Vec<Cell<TaggedPtr>>,
    memory: Option<NonNull<Memory>>,
}

impl Stack {
    fn new() -> Stack {
        Stack {
            registers: Vec::with_capacity(256),
            memory: None,
        }
    }
}

/// A safe interface to access the managed memory via the Stack
struct Mutator {
    stack: Rc<Stack>,
}

impl Mutator {
    fn new(stack: Rc<Stack>) -> Mutator {
        Mutator {
            stack
        }
    }

    fn read_eval_print(&self, input: &str) -> String {
        unimplemented!()
    }
}

/// Create a managed memory environment and a mutator context
fn new_interpreter() -> (Rc<Memory>, Mutator) {
    let mut mem = Rc::new(Memory::new());

    let stack = Rc::new(Stack::new(mem.clone()));

    Rc::get_mut(&mut mem).unwrap().roots = Rc::downgrade(&stack);

    let mutator = Mutator::new(stack);

    (mem, mutator)
}
