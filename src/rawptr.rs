use std::convert::From;
use std::ptr;

use types::{Symbol};

struct Pair {
    first: TaggedPtr,
    second: TaggedPtr
}


#[derive(Copy, Clone)]
struct ObjectHeader {
    flags: usize
}


pub const TAG_OBJECT: usize = 0x0;
pub const TAG_PAIR: usize = 0x1;
pub const TAG_SYMBOL: usize = 0x2;
pub const TAG_NUMBER: usize = 0x3;

pub enum Tag {
    Object = TAG_OBJECT,
    Pair = TAG_PAIR,
    Symbol = TAG_SYMBOL,
    Number = TAG_NUMBER
}


#[derive(Copy, Clone)]
union TaggedPtr {
    tag: usize,
    object: *mut ObjectHeader,
    pair: *mut Pair,
    symbol: *mut Symbol,
    number: usize,
}


impl TaggedPtr {
    pub fn null() -> TaggedPtr {
        TaggedPtr {
            tag: 0
        }
    }

    pub fn type_id(&self) -> Tag {
        unsafe {
            match self.tag && 0x3 {
                TAG_OBJECT => Tag::Object,
                TAG_PAIR => Tag::Pair,
                TAG_SYMBOL => Tag::Symbol,
                TAG_NUMBER => Tag::Number
            }
        }
    }
}


enum AllocatorError {
    OOM
}


/// A `RawAllocator`
pub trait RawAllocator {
    /// Allocate space and move the given object into the space, returning an instance of
    /// `RawMemError` if allocation failed.
    fn alloc<T>(&self, object: T) -> Result<TaggedPtr<T>, AllocatorError>;
}
