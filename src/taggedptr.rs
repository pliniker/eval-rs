/// Defines a `TaggedPtr` type where the low bits of a pointer indicate the
/// type of the object pointed to for certain types.
///
/// Defines an `ObjectHeader` type to immediately preceed each heap allocated
/// object, which also contains a type tag but with space for many more types.
///
/// Also defines a `FatPtr` type which is a safe-Rust enum version of all
/// types which can be expanded from `TaggedPtr` and `ObjectHeader` combined.

use std::mem::size_of;

use stickyimmix::{AllocHeader, AllocObject, AllocRaw, AllocTypeId, Mark, SizeClass, RawPtr};

use heap::Heap;
use primitives::{NumberObject, Pair, Symbol};


fn rawptr_from_tagged_bare<T>(object: *const T) -> RawPtr<T> {
    RawPtr::new((object as usize & TAG_MASK) as *const T)
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


/// Identity comparison
impl PartialEq for FatPtr {
    fn eq(&self, other: &FatPtr) -> bool {

        use self::FatPtr::*;

        match (*self, *other) {
            (Nil, Nil) => true,
            (Pair(p), Pair(q)) => p == q,
            (Symbol(p), Symbol(q)) => p == q,
            (Number(i), Number(j)) => i == j,
            (NumberObject(p), NumberObject(q)) => p ==q,
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
    symbol: *const Symbol,
    pair: *const Pair,
    object: *const (),
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
            tag: (ptr.get() as usize) | TAG_OBJECT
        }
    }

    fn pair(ptr: RawPtr<Pair>) -> TaggedPtr {
        TaggedPtr {
            tag: (ptr.get() as usize) | TAG_PAIR
        }
    }

    fn symbol(ptr: RawPtr<Symbol>) -> TaggedPtr {
        TaggedPtr {
            tag: (ptr.get() as usize) | TAG_SYMBOL
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
                    TAG_SYMBOL => FatPtr::Symbol(rawptr_from_tagged_bare(self.symbol)),
                    TAG_PAIR => FatPtr::Pair(rawptr_from_tagged_bare(self.pair)),

                    TAG_OBJECT => {
                        let untyped_object_ptr = rawptr_from_tagged_bare(self.object).get();
                        let header_ptr = Heap::get_header(untyped_object_ptr);

                        let header = &*header_ptr as &ObjectHeader;

                        match header.type_id {
                            // TODO
                            _ => panic!("Invalid ObjectHeader type id!")
                        }
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


/// Simple idendity equality
impl PartialEq for TaggedPtr {
    fn eq(&self, other: &TaggedPtr) -> bool {
        unsafe {
            self.tag == other.tag
        }
    }
}

// Defintions for heap allocated object header

/// Recognized heap-allocated types
#[repr(u16)]
pub enum TypeList {
    Nil,
    Pair,
    Symbol,
    NumberObject
}


// Mark this as a Stickyimmix type-identifier type
impl AllocTypeId for TypeList {}


/// A heap-allocated object header
pub struct ObjectHeader {
    mark: Mark,
    size_class: SizeClass,
    type_id: TypeList,
    size: u32
}


impl ObjectHeader {
    /// Convert the ObjectHeader address to a FatPtr pointing at the object itself
    pub fn to_object_fatptr(&self) -> FatPtr {
        unsafe {
            let object_addr = (
                self as *const ObjectHeader as *const () as usize
            ) + size_of::<Self>();

            match self.type_id {
                TypeList::Pair =>
                    FatPtr::Pair(rawptr_from_tagged_bare(object_addr as *const Pair)),

                TypeList::NumberObject =>
                    FatPtr::NumberObject(rawptr_from_tagged_bare(object_addr as *mut NumberObject)),

                // TODO

                _ => panic!("Invalid ObjectHeader type tag!")
            }
        }
    }

    // Object should be immediately after object header TODO this is up to the allocator!
    fn object_addr(&self) -> *const () {
        unsafe { (self as *const ObjectHeader as *const()).offset(size_of::<Self>() as isize) }
    }
}


impl AllocHeader for ObjectHeader {
    type TypeId = TypeList;

    fn new<O: AllocObject<Self::TypeId>>(size: u32, size_class: SizeClass, mark: Mark) -> ObjectHeader {
        ObjectHeader {
            mark: mark,
            size_class: size_class,
            type_id: O::TYPE_ID,
            size: size
        }
    }

    fn mark(&mut self) {
        self.mark = Mark::Marked;
    }

    fn is_marked(&self) -> bool {
        self.mark == Mark::Marked
    }

    fn size_class(&self) -> SizeClass {
        self.size_class
    }
}


/// Symbols are managed by the Symbol Mapper, which is backed by an Arena
impl AllocObject<TypeList> for Symbol {
    const TYPE_ID: TypeList = TypeList::Symbol;
}


impl AllocObject<TypeList> for Pair {
    const TYPE_ID: TypeList = TypeList::Pair;
}


impl AllocObject<TypeList> for NumberObject {
    const TYPE_ID: TypeList = TypeList::NumberObject;
}
