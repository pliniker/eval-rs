use std::convert::From;
use std::mem::{size_of, transmute};

use primitives::{ObjectHeader, Pair, Symbol,
                 StringObject, NumberObject,
                 Redirect};


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
    /// From raw-raw pointer
    pub fn from_raw(object: *mut T) -> RawPtr<T> {
        RawPtr {
            raw: object
        }
    }

    /// Zero out the tag bits and keep the pointer
    fn from_tagged_ptr(object: *mut T) -> RawPtr<T> {
        RawPtr {
            raw: (object as usize & TAG_MASK) as *mut T
        }
    }

    /// Get a pointer to an ObjectHeader (that may or may not exist) for the
    /// object pointed at
    unsafe fn header(&self) -> RawPtr<ObjectHeader> {
        let header_pos = (self.raw as usize) - size_of::<ObjectHeader>();

        RawPtr {
            raw: header_pos as *mut ObjectHeader
        }
    }

    unsafe fn deref(&self) -> &T {
        &*self.raw
    }

    unsafe fn deref_mut(&self) -> &mut T {
        &mut *self.raw
    }
}


/// An unpacked tagged Fat Pointer that carries the type information in the enum structure
#[derive(Copy, Clone)]
pub enum FatPtr {
    Nil,
    Pair(RawPtr<Pair>),
    Symbol(RawPtr<Symbol>),
    Number(isize),
    NumberObject(RawPtr<NumberObject>),
    StringObject(RawPtr<StringObject>),
}


/// An packed Tagged Pointer which carries the type information in the pointer itself
#[derive(Copy, Clone)]
pub union TaggedPtr {
    tag: usize,
    pair: *mut Pair,
    symbol: *mut Symbol,
    number: isize,
    object: *mut (),
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

    fn object(ptr: RawPtr<()>) -> TaggedPtr {
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
            tag: ((value as usize) << 2) | TAG_NUMBER
        }
    }

    pub fn is_nil(&self) -> bool {
        unsafe { self.tag == 0 }
    }

    fn into_fat_ptr(&self) -> FatPtr {
        unsafe {
            if self.tag == 0 {
                FatPtr::Nil
            } else {
                match self.tag & TAG_MASK {
                    TAG_OBJECT => {
                        let raw_object = RawPtr::from_tagged_ptr(self.object);
                        let header = raw_object.header();

                        header.deref().object_rawptr()
                    },
                    TAG_PAIR => FatPtr::Pair(RawPtr::from_tagged_ptr(self.pair)),
                    TAG_SYMBOL => FatPtr::Symbol(RawPtr::from_tagged_ptr(self.symbol)),
                    TAG_NUMBER => FatPtr::Number(self.number >> 2),
                    _ => panic!("Corrupt pointer tag!")
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
            //FatPtr::ObjectHeader(raw) => TaggedPtr::object(raw),
            FatPtr::Pair(raw) => TaggedPtr::pair(raw),
            FatPtr::Symbol(raw) => TaggedPtr::symbol(raw),
            FatPtr::Number(value) => TaggedPtr::number(value),
        }
    }
}
