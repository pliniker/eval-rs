

use std::collections::HashMap;

use memory::{Allocator, Ptr};
use types::Symbol;


/// A trait that describes the ability to look up a Symbol by it's name in a String
pub trait SymbolMapper {
    fn lookup(&mut self, name: &String) -> Ptr<Symbol>;
}


/// A mapping of symbol names (Strings) to Symbol pointers. Only one copy of the symbol
/// name String is kept; a Symbol resides in managed memory with a raw pointer to the
/// String. Thus the lifetime of the SymbolMap must be at least the lifetime of the
/// managed memory.
///
/// No Symbol is ever deleted. Symbol name strings must be immutable.
///
/// As the internal HashMap is not integrated with managed memory, Symbols cannot be
/// relocated in managed memory.
pub struct SymbolMap {
    map: HashMap<String, Ptr<Symbol>>
}


impl SymbolMap {
    pub fn new() -> SymbolMap {
        SymbolMap {
            map: HashMap::new()
        }
    }

    pub fn lookup<M>(&mut self, name: &str, mem: &mut M) -> Ptr<Symbol>
        where M: Allocator
    {
        // Can't take a map.entry(name) without providing an owned String, i.e. cloning 'name'
        // Can't insert a new entry with just a reference without hashing twice, and cloning 'name'
        // Which is the lesser weevil? Perhaps making lookups fast and inserts slower.

        { // appease le borrow chequer inside this block
            if let Some(ptr) = self.map.get(name) {
                return ptr.clone();
            }
        }

        let name = String::from(name);
        let ptr = Symbol::new(&name, mem);
        self.map.insert(name, ptr);
        ptr
    }
}
