use memory::{Allocator, Arena};
use symbolmap::SymbolMap;


/// I'm not sure what an environment should hold just yet, so for now it contains
/// a way to allocate memory and a mapping of symbol names to symbol addresses
pub struct Environment<'a, A: 'a + Allocator> {
    pub mem: A,
    // keys to syms are Strings, which have pointers to them in mem.
    // The lifetime of syms must be >= the lifetime of mem
    pub syms: SymbolMap<'a, A>,
}


impl<'a> Environment<'a, Arena> {
    pub fn new(block_size: usize) -> Environment<'a, Arena> {
        Environment {
            mem: Arena::new(block_size),
            syms: SymbolMap::new(block_size)
        }
    }
}
