/// This file defines pointer abstractions.
/// From high level to low, safest to unsafest:
///  * Value > FatPtr > TaggedPtr
///
/// Defines a `Value` type which is a safe-Rust enum of references to object
/// types.
///
/// Defines a `FatPtr` type which is a Rust tagged-union enum version of all
/// types which can be expanded from `TaggedPtr` and `ObjectHeader` combined.
///
/// Defines a `TaggedPtr` type where the low bits of a pointer indicate the
/// type of the object pointed to for certain types, but the object header is
/// required to provide most object type ids.
use std::fmt;
use std::ptr::NonNull;

use stickyimmix::{AllocRaw, RawPtr};

use crate::memory::HeapStorage;
use crate::pointerops::{get_tag, ScopedRef, Tagged, TAG_NUMBER, TAG_OBJECT, TAG_PAIR, TAG_SYMBOL};
use crate::primitives::{ArrayAny, NumberObject, Pair, Symbol};
use crate::printer::Print;
use crate::safeptr::MutatorScope;

/// A safe interface to GC-heap managed objects. The `'scope` lifetime must be a safe lifetime for
/// the GC not to move or collect the referenced object.
/// This should represent every type native to the runtime.
#[derive(Copy, Clone)]
pub enum Value<'scope> {
    Nil,
    Pair(&'scope Pair),
    Symbol(&'scope Symbol),
    Number(isize),
    NumberObject(&'scope NumberObject),
    ArrayAny(&'scope ArrayAny),
}

/// `Value` can have a safe `Display` implementation
impl<'scope> fmt::Display for Value<'scope> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Nil => write!(f, "()"),
            Value::Pair(p) => p.print(self, f),
            Value::Symbol(s) => s.print(self, f),
            Value::Number(n) => write!(f, "{}", *n),
            Value::ArrayAny(a) => a.print(self, f),
            _ => write!(f, "<unidentified-object-type>"),
        }
    }
}

impl<'scope> fmt::Debug for Value<'scope> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Nil => write!(f, "()"),
            Value::Pair(p) => p.debug(self, f),
            Value::Symbol(s) => s.debug(self, f),
            Value::Number(n) => write!(f, "{}", *n),
            Value::ArrayAny(a) => a.debug(self, f),
            _ => write!(f, "<unidentified-object-type>"),
        }
    }
}

impl<'scope> MutatorScope for Value<'scope> {}

/// An unpacked tagged Fat Pointer that carries the type information in the enum structure.
/// This should represent every type native to the runtime.
#[derive(Copy, Clone)]
pub enum FatPtr {
    Nil,
    Pair(RawPtr<Pair>),
    Symbol(RawPtr<Symbol>),
    Number(isize),
    NumberObject(RawPtr<NumberObject>),
    ArrayAny(RawPtr<ArrayAny>),
}

impl FatPtr {
    /// Given a lifetime, convert to a `Value` type. Unsafe because anything can provide a lifetime
    /// without any safety guarantee that it's valid.
    pub fn as_value<'scope>(&self, guard: &'scope MutatorScope) -> Value<'scope> {
        match self {
            FatPtr::Nil => Value::Nil,
            FatPtr::Pair(raw_ptr) => Value::Pair(raw_ptr.scoped_ref(guard)),
            FatPtr::Symbol(raw_ptr) => Value::Symbol(raw_ptr.scoped_ref(guard)),
            FatPtr::Number(num) => Value::Number(*num),
            FatPtr::NumberObject(raw_ptr) => Value::NumberObject(raw_ptr.scoped_ref(guard)),
            FatPtr::ArrayAny(raw_ptr) => Value::ArrayAny(raw_ptr.scoped_ref(guard)),
        }
    }
}

/// Implement `From<RawPtr<T>> for FatPtr` for the given FatPtr discriminant and the given `T`
macro_rules! fatptr_from_primitive {
    ($F:tt, $T:ty) => {
        impl From<RawPtr<$T>> for FatPtr {
            fn from(ptr: RawPtr<$T>) -> FatPtr {
                FatPtr::$F(ptr)
            }
        }
    };
}

fatptr_from_primitive!(Pair, Pair);
fatptr_from_primitive!(Symbol, Symbol);
fatptr_from_primitive!(NumberObject, NumberObject);
fatptr_from_primitive!(ArrayAny, ArrayAny);

/// Conversion from a TaggedPtr type
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
            (NumberObject(p), NumberObject(q)) => p == q,
            _ => false,
        }
    }
}

/// An packed Tagged Pointer which carries type information in the pointers low 2 bits
#[derive(Copy, Clone)]
pub union TaggedPtr {
    tag: usize,
    number: isize,
    symbol: NonNull<Symbol>,
    pair: NonNull<Pair>,
    object: NonNull<()>,
}

impl TaggedPtr {
    /// Construct a nil TaggedPtr
    pub fn nil() -> TaggedPtr {
        TaggedPtr { tag: 0 }
    }

    /// Construct a generic object TaggedPtr
    fn object<T>(ptr: RawPtr<T>) -> TaggedPtr {
        TaggedPtr {
            object: ptr.tag(TAG_OBJECT).cast::<()>(),
        }
    }

    /// Construct a Pair TaggedPtr
    fn pair(ptr: RawPtr<Pair>) -> TaggedPtr {
        TaggedPtr {
            pair: ptr.tag(TAG_PAIR),
        }
    }

    /// Construct a Symbol TaggedPtr
    pub fn symbol(ptr: RawPtr<Symbol>) -> TaggedPtr {
        TaggedPtr {
            symbol: ptr.tag(TAG_SYMBOL),
        }
    }

    /// Construct an inline integer TaggedPtr
    // TODO deal with big numbers later
    fn number(value: isize) -> TaggedPtr {
        TaggedPtr {
            number: (((value as usize) << 2) | TAG_NUMBER) as isize,
        }
    }

    fn into_fat_ptr(&self) -> FatPtr {
        unsafe {
            if self.tag == 0 {
                FatPtr::Nil
            } else {
                match get_tag(self.tag) {
                    TAG_NUMBER => FatPtr::Number(self.number >> 2),
                    TAG_SYMBOL => FatPtr::Symbol(RawPtr::untag(self.symbol)),
                    TAG_PAIR => FatPtr::Pair(RawPtr::untag(self.pair)),

                    TAG_OBJECT => {
                        let untyped_object_ptr = RawPtr::untag(self.object).as_untyped();
                        let header_ptr = HeapStorage::get_header(untyped_object_ptr);

                        header_ptr.as_ref().get_object_fatptr()
                    }

                    _ => panic!("Invalid TaggedPtr type tag!"),
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
            FatPtr::ArrayAny(raw) => TaggedPtr::object(raw),
        }
    }
}

/// Simple identity equality
impl PartialEq for TaggedPtr {
    fn eq(&self, other: &TaggedPtr) -> bool {
        unsafe { self.tag == other.tag }
    }
}
