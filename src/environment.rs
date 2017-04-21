use memory::{Allocator, Arena, Ptr};
use symbolmap::{SymbolMap, SymbolMapper};
use types::Symbol;


/// I'm not sure what an environment should hold just yet, so for now it contains
/// a way to allocate memory and a mapping of symbol names to symbol addresses
pub struct Environment {
    mem: Arena,
    // keys to syms are Strings, which have pointers to them in mem.
    // The lifetime of syms must be >= the lifetime of mem
    syms: SymbolMap,
}


impl Environment {
    pub fn new(block_size: usize) -> Environment {
        Environment {
            mem: Arena::new(block_size),
            syms: SymbolMap::new()
        }
    }
}


impl Allocator for Environment {
    fn alloc<T>(&mut self, object: T) -> Ptr<T> {
        self.mem.alloc(object)
    }
}


impl SymbolMapper for Environment {
    fn lookup(&mut self, name: &String) -> Ptr<Symbol> {
        self.syms.lookup(name, &mut self.mem)
    }
}
