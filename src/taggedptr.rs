use std::convert::From;

use primitives::{ObjectHeader, Pair, Symbol};


/// Wrapper around a raw pointer type
pub struct RawPtr<T> {
    raw: *mut T
}


impl<T> Clone for RawPtr<T> {
    fn clone(&self) -> RawPtr<T> {
        RawPtr {
            raw: self.raw
        }
    }
}


impl<T> Copy for RawPtr<T> {}


impl<T> RawPtr<T> {
    fn from_tagged_ptr(object: *mut T) -> RawPtr<T> {
        RawPtr {
            raw: (object as usize & TAG_MASK) as *mut T
        }
    }
}


/// An unpacked tagged Fat Pointer that carries the type information in the enum structure
#[derive(Copy, Clone)]
pub enum FatPtr {
    Nil,
    Object(RawPtr<ObjectHeader>),
    Pair(RawPtr<Pair>),
    Symbol(RawPtr<Symbol>),
    Number(isize)
}


/// An packed Tagged Pointer which carries the type information in the pointer itself
#[derive(Copy, Clone)]
pub union TaggedPtr {
    tag: usize,
    object: *mut ObjectHeader,
    pair: *mut Pair,
    symbol: *mut Symbol,
    number: isize,
}


const TAG_MASK: usize = 0x3;
const TAG_OBJECT: usize = 0x0;
const TAG_PAIR: usize = 0x1;
const TAG_SYMBOL: usize = 0x2;
const TAG_NUMBER: usize = 0x3;
const PTR_MASK: usize = !0x3;


impl TaggedPtr {
    fn nil() -> TaggedPtr {
        TaggedPtr {
            tag: 0
        }
    }

    fn object(ptr: RawPtr<ObjectHeader>) -> TaggedPtr {
        TaggedPtr {
            tag: (ptr.raw as usize) | TAG_OBJECT
        }
    }

    fn pair(ptr: RawPtr<Pair>) -> TaggedPtr {
        TaggedPtr {
            tag: (ptr.raw as usize) | TAG_PAIR
        }
    }

    fn symbol(ptr: RawPtr<Symbol>) -> TaggedPtr {
        TaggedPtr {
            tag: (ptr.raw as usize) | TAG_SYMBOL
        }
    }

    fn number(value: isize) -> TaggedPtr {
        TaggedPtr {
            tag: (value as usize << 2) | TAG_NUMBER
        }
    }

    pub fn is_nil(&self) -> bool {
        self.tag == 0
    }

    fn into_fat_ptr(&self) -> FatPtr {
        unsafe {
            if self.tag == 0 {
                FatPtr::Nil
            } else {
                match self.tag & TAG_MASK {
                    TAG_OBJECT => FatPtr::Object(RawPtr::from_tagged_ptr(self.object)),
                    TAG_PAIR => FatPtr::Pair(RawPtr::from_tagged_ptr(self.pair)),
                    TAG_SYMBOL => FatPtr::Symbol(RawPtr::from_tagged_ptr(self.symbol)),
                    TAG_NUMBER => FatPtr::Number(self.number >> 2)
                }
            }
        }
    }
}


impl From<TaggedPtr> for FatPtr {
    fn from(ptr: TaggedPtr) -> FatPtr {
        ptr.into_fat_ptr()
    }
}


impl From<FatPtr> for TaggedPtr {
    fn from(ptr: FatPtr) -> TaggedPtr {
        match ptr {
            FatPtr::Nil => TaggedPtr::nil(),
            FatPtr::Object(raw) => TaggedPtr::object(raw),
            FatPtr::Pair(raw) => TaggedPtr::pair(raw),
            FatPtr::Symbol(raw) => TaggedPtr::symbol(raw),
            FatPtr::Number(value) => TaggedPtr::number(value)
        }
    }
}


enum AllocatorError {
    OOM
}


pub trait Allocator {
    /// Allocate space and move the given object into the space, returning an instance of
    /// `AllocatorError` if allocation failed.
    fn alloc<T>(&self, object: T) -> Result<TaggedPtr, AllocatorError>;
}
