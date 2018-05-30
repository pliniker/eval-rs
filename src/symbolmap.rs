/// Implements str interning for mapping Symbol names to unique pointers

use std::cell::RefCell;
use std::collections::HashMap;

use heap::{Allocator, SymbolMapper};
use primitives::Symbol;
use rawptr::RawPtr;



/// A mapping of symbol names (Strings) to Symbol pointers. Only one copy of the symbol
/// name String is kept; a Symbol resides in managed memory with a raw pointer to the
/// String. Thus the lifetime of the SymbolMap must be at least the lifetime of the
/// managed memory. This is arranged here by maintaining Symbol memory alongside the
/// mapping HashMap.
///
/// No Symbol is ever deleted. Symbol name strings must be immutable.
pub struct SymbolMap<A: Allocator> {
    map: RefCell<HashMap<String, RawPtr<Symbol>>>,
    arena: A,
}


impl<A: Allocator + Default> SymbolMap<A> {
    pub fn new() -> SymbolMap<A> {
        SymbolMap {
            map: RefCell::new(HashMap::new()),
            arena: Default::default(),
        }
    }
}


impl<A: Allocator> SymbolMapper for SymbolMap<A> {
    fn lookup(&self, name: &str) -> RawPtr<Symbol> {
        // Can't take a map.entry(name) without providing an owned String, i.e. cloning 'name'
        // Can't insert a new entry with just a reference without hashing twice, and cloning 'name'
        // The common case, lookups, should be fast, inserts can be slower.

        {
            if let Some(ptr) = self.map.borrow().get(name) {
                return *ptr
            }
        }

        let name = String::from(name);
        let ptr = self.arena.alloc(Symbol::new(&name)).unwrap();
        self.map.borrow_mut().insert(name, ptr);
        ptr
    }
}
