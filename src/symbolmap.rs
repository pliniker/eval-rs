use std::cell::RefCell;
use std::collections::HashMap;

use memory::{StaticAllocator, Ptr};
use types::Symbol;


/// A trait that describes the ability to look up a Symbol by it's name in a String
pub trait SymbolMapper<'heap, A: 'heap + StaticAllocator> {
    fn lookup(&self, name: &str) -> Ptr<'heap, Symbol, A>;
}


/// A mapping of symbol names (Strings) to Symbol pointers. Only one copy of the symbol
/// name String is kept; a Symbol resides in managed memory with a raw pointer to the
/// String. Thus the lifetime of the SymbolMap must be at least the lifetime of the
/// managed memory. This is arranged here by maintaining Symbol memory alongside the
/// mapping HashMap.
///
/// No Symbol is ever deleted. Symbol name strings must be immutable.
pub struct SymbolMap<'heap, A: 'heap + StaticAllocator> {
    map: RefCell<HashMap<String, Ptr<'heap, Symbol, A>>>,
    heap: &'heap A,
}


impl<'heap, A: 'heap + StaticAllocator> SymbolMap<'heap, A> {
    pub fn new(heap: &'heap A) -> SymbolMap<'heap, A> {
        SymbolMap {
            map: RefCell::new(HashMap::new()),
            heap: heap,
        }
    }
}


impl<'heap, A: 'heap + StaticAllocator> SymbolMapper<'heap, A> for SymbolMap<'heap, A> {
    fn lookup(&self, name: &str) -> Ptr<'heap, Symbol, A> {
        // Can't take a map.entry(name) without providing an owned String, i.e. cloning 'name'
        // Can't insert a new entry with just a reference without hashing twice, and cloning 'name'
        // The common case, lookups, should be fast, inserts can be slower.

        {
            if let Some(ptr) = self.map.borrow().get(name) {
                return ptr.clone();
            }
        }

        let name = String::from(name);
        let ptr = self.heap.alloc_static(Symbol::new(&name));
        self.map.borrow_mut().insert(name, ptr);
        ptr
    }
}
