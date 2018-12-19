/// Defines a `TaggedPtr` type where the low bits of a pointer indicate the
/// type of the object pointed to for certain types.
///
/// Defines an `ObjectHeader` type to immediately preceed each heap allocated
/// object, which also contains a type tag but with space for many more types.
///
/// Also defines a `FatPtr` type which is a safe-Rust enum version of all
/// types which can be expanded from `TaggedPtr` and `ObjectHeader` combined.

use std::ptr::NonNull;

use stickyimmix::{AllocHeader, AllocObject, AllocRaw, AllocTypeId, Mark, SizeClass, RawPtr};

use crate::heap::Heap;
use crate::primitives::{NumberObject, Pair, Symbol};


/// For conversion of a reference to a NonNull<T>
trait AsNonNull {
    fn non_null_ptr(&self) -> NonNull<Self> {
        unsafe { NonNull::new_unchecked(self as *const Self as *mut Self) }
    }
}


/// Type tag ops on RawPtr<T>
trait Tagged<T> {
    fn tag(self, tag: usize) -> NonNull<T>;
    fn untag(from: NonNull<T>) -> RawPtr<T>;
}

impl<T> Tagged<T> for RawPtr<T> {
    fn tag(self, tag: usize) -> NonNull<T> {
        unsafe { NonNull::new_unchecked((self.as_word() | tag) as *mut T) }
    }

    fn untag(from: NonNull<T>) -> RawPtr<T> {
        RawPtr::new((from.as_ptr() as usize & TAG_MASK) as *const T)
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
    symbol: NonNull<Symbol>,
    pair: NonNull<Pair>,
    object: NonNull<()>,
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
            object: ptr.tag(TAG_OBJECT).cast::<()>()
        }
    }

    fn pair(ptr: RawPtr<Pair>) -> TaggedPtr {
        TaggedPtr {
            pair: ptr.tag(TAG_PAIR)
        }
    }

    fn symbol(ptr: RawPtr<Symbol>) -> TaggedPtr {
        TaggedPtr {
            symbol: ptr.tag(TAG_SYMBOL)
        }
    }

    // TODO deal with big numbers later
    fn number(value: isize) -> TaggedPtr {
        TaggedPtr {
            number: (((value as usize) << 2) | TAG_NUMBER) as isize
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
                    TAG_SYMBOL => FatPtr::Symbol(RawPtr::untag(self.symbol)),
                    TAG_PAIR => FatPtr::Pair(RawPtr::untag(self.pair)),

                    TAG_OBJECT => {
                        let untyped_object_ptr = RawPtr::untag(self.object).as_untyped();
                        let header_ptr = Heap::get_header(untyped_object_ptr);

                        header_ptr.as_ref().get_object_fatptr()
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


/// Simple identity equality
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
    size_bytes: u32
}


impl AsNonNull for ObjectHeader {}


impl ObjectHeader {
    /// Convert the ObjectHeader address to a FatPtr pointing at the object itself
    pub fn get_object_fatptr(&self) -> FatPtr {
        let self_as_nonnull = self.non_null_ptr();
        let object_addr = Heap::get_object(self_as_nonnull);

        // Only Object* types should be derived from the header.
        // Symbol, Pair and Number should have been derived from a pointer tag.
        match self.type_id {
            TypeList::NumberObject =>
                FatPtr::NumberObject(RawPtr::untag(object_addr.cast::<NumberObject>())),

            _ => panic!("Invalid ObjectHeader type tag!")
        }
    }
}


impl AllocHeader for ObjectHeader {
    type TypeId = TypeList;

    fn new<O: AllocObject<Self::TypeId>>(size: u32, size_class: SizeClass, mark: Mark) -> ObjectHeader {
        ObjectHeader {
            mark: mark,
            size_class: size_class,
            type_id: O::TYPE_ID,
            size_bytes: size
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

    fn size(&self) -> u32 {
        self.size_bytes
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
