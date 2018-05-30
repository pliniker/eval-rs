
use primitives::Symbol;
use rawptr::RawPtr;


/// A memory error type, encompassing all memory related errors at this time.
#[derive(Debug)]
pub enum MemError {
    OOM,
}


pub trait Allocator {
    fn alloc<T>(&self, object: T) -> Result<RawPtr<T>, MemError>;
}


/// A trait that describes the ability to look up a Symbol by it's name in a str
pub trait SymbolMapper {
    fn lookup(&self, name: &str) -> RawPtr<Symbol>;
}


/// A heap trait
pub trait Heap : Allocator + SymbolMapper {
}
