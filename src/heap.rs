
use rawptr::RawPtr;


/// A memory error type, encompassing all memory related errors at this time.
#[derive(Debug)]
pub enum MemError {
    OOM,
}


pub trait Allocator {
    fn alloc<T>(&self, object: T) -> Result<RawPtr<T>, MemError>;
}


/// A heap trait
pub trait Heap : Allocator {
}
