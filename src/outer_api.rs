/// The collector and mutator should be thought of as coroutines sharing
/// mutable access to the heap and roots data structures.

use std::cell::Cell;
use std::ptr::NonNull;
use std::rc::{Rc, Weak};

use crate::heap::Heap;
use crate::symbolmap::SymbolMap;
use crate::taggedptr::{FatPtr, TaggedPtr};

/*
enum TraceStatus {
    MoreTracingNeeded,
    Finished,
}

enum CollectStatus {
    MoreCollectionNeeded,
    Finished,
}

trait Collector {
    fn trace(&self) -> Result<TraceStatus, ()>;
    fn collect(&mut self) -> Result<CollectStatus, ()>;
}

type IterRoots = Iterator<Item = NonNull<()>>;

trait Memory {
    fn lookup_sym(&self, sym: &str) -> FatPtr;
    fn alloc<T>(&self, object: T) -> FatPtr;
}

trait Mutator {
    fn apply<F, M, S>(&self, step: F) -> Result<(), ()>
    where M: Memory,
          S: Iterator<Item = NonNull<()>>,
          F: Fn(&M, &mut S);
}
*/

struct Collector {
    mem: *const Memory,
    roots: *const Stack,
}

struct Memory {
    heap: Heap,
    syms: SymbolMap,
}

/// Interface to Memory
struct Stack {
    mem: *const Memory,
    roots: Vec<Cell<TaggedPtr>>,
}

struct Mutator {
    stack: *mut Stack
}
