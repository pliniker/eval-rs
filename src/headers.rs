/// Defines an `ObjectHeader` type to immediately preceed each heap allocated
/// object, which also contains a type tag but with space for many more types.
use stickyimmix::{AllocHeader, AllocObject, AllocRaw, AllocTypeId, Mark, RawPtr, SizeClass};

use crate::memory::HeapStorage;
use crate::pointerops::{AsNonNull, Tagged};
use crate::primitives::{NumberObject, Pair, Symbol};
use crate::taggedptr::FatPtr;

/// Recognized heap-allocated types.
/// This should represent every type native to the runtime with the exception of tagged pointer inline value types.
#[repr(u16)]
pub enum TypeList {
    Pair,
    Symbol,
    NumberObject,
}

// Mark this as a Stickyimmix type-identifier type
impl AllocTypeId for TypeList {}

/// A heap-allocated object header
pub struct ObjectHeader {
    mark: Mark,
    size_class: SizeClass,
    type_id: TypeList,
    size_bytes: u32,
}

impl ObjectHeader {
    /// Convert the ObjectHeader address to a FatPtr pointing at the object itself
    pub fn get_object_fatptr(&self) -> FatPtr {
        let ptr_to_self = self.non_null_ptr();
        let object_addr = HeapStorage::get_object(ptr_to_self);

        // Only Object* types should be derived from the header.
        // Symbol, Pair and Number should have been derived from a pointer tag.
        match self.type_id {
            TypeList::NumberObject => {
                FatPtr::NumberObject(RawPtr::untag(object_addr.cast::<NumberObject>()))
            }

            _ => panic!("Invalid ObjectHeader type tag!"),
        }
    }
}

impl AsNonNull for ObjectHeader {}

impl AllocHeader for ObjectHeader {
    type TypeId = TypeList;

    fn new<O: AllocObject<Self::TypeId>>(
        size: u32,
        size_class: SizeClass,
        mark: Mark,
    ) -> ObjectHeader {
        ObjectHeader {
            mark: mark,
            size_class: size_class,
            type_id: O::TYPE_ID,
            size_bytes: size,
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

/// Apply the type ID to each native type
macro_rules! declare_allocobject {
    ($T:ty, $I:tt) => {
        impl AllocObject<TypeList> for $T {
            const TYPE_ID: TypeList = TypeList::$I;
        }
    };
}

declare_allocobject!(Symbol, Symbol);
declare_allocobject!(Pair, Pair);
declare_allocobject!(NumberObject, NumberObject);
