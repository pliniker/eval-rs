/// Defines a `TaggedPtr` type where the low bits of a pointer indicate the
/// type of the object pointed to for certain types.
///
/// Defines an `ObjectHeader` type to immediately preceed each heap allocated
/// objects which also contains a type tag but with space for many more types.
///
/// Also defines a `FatPtr` type which is a safe-Rust enum version of all
/// types which can be expanded from `TaggedPtr` and `ObjectHeader` combined.

use std::convert::From;
use std::mem::size_of;

use primitives::{NumberObject, Pair, Symbol};
use stickyimmix::RawPtr;


impl<T> RawPtr<T> {
    /// Zero out the tag bits and keep the pointer
    fn from_tagged_bare(object: *mut T) -> RawPtr<T> {
        Self::from_bare((object as usize & TAG_MASK) as *mut T)
    }

    /// Get a pointer to an ObjectHeader (that may or may not exist) for the
    /// object pointed at
    unsafe fn get_header_ptr(&self) -> RawPtr<ObjectHeader> {
        let header_pos = (self.to_bare() as usize) - size_of::<ObjectHeader>();

        RawPtr::from_bare(header_pos as *mut ObjectHeader)
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
}


impl From<TaggedPtr> for FatPtr {
    fn from(ptr: TaggedPtr) -> FatPtr {
        ptr.into_fat_ptr()
    }
}


/// Value comparison, not identity comparison
impl PartialEq for FatPtr {
    fn eq(&self, other: &FatPtr) -> bool {

        use self::FatPtr::*;

        match (*self, *other) {
            (Nil, Nil) => true,
            (Number(i), Number(j)) => i == j,
            // TODO
            _ => false
        }
    }
}


/// An packed Tagged Pointer which carries type information in the pointers
/// low bits
#[derive(Copy, Clone)]
pub union TaggedPtr {
    tag: usize,
    number: isize,
    symbol: *mut Symbol,
    pair: *mut Pair,
    object: *mut (),
}


const TAG_MASK: usize = 0x3;
const TAG_NUMBER: usize = 0x0;
const TAG_SYMBOL: usize = 0x1;
const TAG_PAIR: usize = 0x2;
const TAG_OBJECT: usize = 0x3;
const PTR_MASK: usize = !0x3;


impl TaggedPtr {
    pub fn nil() -> TaggedPtr {
        TaggedPtr {
            tag: 0
        }
    }

    fn object<T>(ptr: RawPtr<T>) -> TaggedPtr {
        TaggedPtr {
            tag: (ptr.to_bare() as usize) | TAG_OBJECT
        }
    }

    fn pair(ptr: RawPtr<Pair>) -> TaggedPtr {
        TaggedPtr {
            tag: (ptr.to_bare() as usize) | TAG_PAIR
        }
    }

    fn symbol(ptr: RawPtr<Symbol>) -> TaggedPtr {
        TaggedPtr {
            tag: (ptr.to_bare() as usize) | TAG_SYMBOL
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
                    TAG_NUMBER => FatPtr::Number(self.number >> 2),
                    TAG_SYMBOL => FatPtr::Symbol(RawPtr::from_tagged_bare(self.symbol)),
                    TAG_PAIR => FatPtr::Pair(RawPtr::from_tagged_bare(self.pair)),
                    TAG_OBJECT => {
                        let object_ptr = RawPtr::from_tagged_bare(self.object);
                        let header_ptr = object_ptr.get_header_ptr();

                        header_ptr.deref().to_object_fatptr()
                    },
                    _ => panic!("Invalid TaggedPtr type tag!")
                }
            }
        }
    }
}


impl From<FatPtr> for TaggedPtr {
    fn from(ptr: FatPtr) -> TaggedPtr {
        match ptr {
            FatPtr::Nil => TaggedPtr::nil(),
            FatPtr::Number(value) => TaggedPtr::number(value),
            FatPtr::Symbol(raw) => TaggedPtr::symbol(raw),
            FatPtr::Pair(raw) => TaggedPtr::pair(raw),
            FatPtr::NumberObject(raw) => TaggedPtr::object(raw),
        }
    }
}


/// The pointer values must be identical for two TaggedPtr instances
/// to be equal
impl PartialEq for TaggedPtr {
    fn eq(&self, other: &TaggedPtr) -> bool {
        unsafe {
            self.tag == other.tag
        }
    }
}

// Defintions for heap allocated object header

const HEADER_MARK_BIT: u32 = 0x1;
const HEADER_TAG_MASK: u32 = !(0x0f << 1);
const HEADER_TAG_PAIR: u32 = 0x00 << 1;
const HEADER_TAG_NUMBER: u32 = 0x01 << 1;


/// A heap-allocated object header
pub struct ObjectHeader {
    flags: u32,
    size: u32
}


impl ObjectHeader {
    /// Convert the ObjectHeader address to a FatPtr pointing at the object itself
    pub fn to_object_fatptr(&self) -> FatPtr {
        unsafe {
            let object_addr = (
                self as *const ObjectHeader as *const () as usize
            ) + size_of::<Self>();

            match self.flags & HEADER_TAG_MASK {
                HEADER_TAG_PAIR =>
                    FatPtr::Pair(RawPtr::from_bare(object_addr as *mut Pair)),

                HEADER_TAG_NUMBER =>
                    FatPtr::NumberObject(RawPtr::from_bare(object_addr as *mut NumberObject)),

                _ => panic!("Invalid ObjectHeader type tag!")
            }
        }
    }
}
