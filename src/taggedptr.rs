/// This file defines internal pointer abstractions for runtime tag-typed pointers.
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
/// required to provide all other object type ids.
use std::fmt;
use std::ptr::NonNull;

use stickyimmix::{AllocRaw, RawPtr};

use crate::array::{ArrayU32, ArrayU8};
use crate::dict::Dict;
use crate::function::{Function, Partial};
use crate::list::List;
use crate::memory::HeapStorage;
use crate::number::NumberObject;
use crate::pair::Pair;
use crate::pointerops::{get_tag, ScopedRef, Tagged, TAG_NUMBER, TAG_OBJECT, TAG_PAIR, TAG_SYMBOL};
use crate::printer::Print;
use crate::safeptr::MutatorScope;
use crate::symbol::Symbol;
use crate::text::Text;

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
    Text(&'scope Text),
    List(&'scope List),
    ArrayU8(&'scope ArrayU8),
    ArrayU32(&'scope ArrayU32),
    Dict(&'scope Dict),
    Function(&'scope Function),
    Partial(&'scope Partial),
}

/// `Value` can have a safe `Display` implementation
impl<'scope> fmt::Display for Value<'scope> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Nil => write!(f, "nil"),
            Value::Pair(p) => p.print(self, f),
            Value::Symbol(s) => s.print(self, f),
            Value::Number(n) => write!(f, "{}", *n),
            Value::Text(t) => t.print(self, f),
            Value::List(a) => a.print(self, f),
            Value::ArrayU8(a) => a.print(self, f),
            Value::ArrayU32(a) => a.print(self, f),
            Value::Dict(d) => d.print(self, f),
            Value::Function(n) => n.print(self, f),
            Value::Partial(p) => p.print(self, f),
            _ => write!(f, "<unidentified-object-type>"),
        }
    }
}

impl<'scope> fmt::Debug for Value<'scope> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Nil => write!(f, "nil"),
            Value::Pair(p) => p.debug(self, f),
            Value::Symbol(s) => s.debug(self, f),
            Value::Number(n) => write!(f, "{}", *n),
            Value::Text(t) => t.debug(self, f),
            Value::List(a) => a.debug(self, f),
            Value::ArrayU8(a) => a.debug(self, f),
            Value::ArrayU32(a) => a.debug(self, f),
            Value::Dict(d) => d.debug(self, f),
            Value::Function(n) => n.debug(self, f),
            Value::Partial(p) => p.debug(self, f),
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
    Text(RawPtr<Text>),
    List(RawPtr<List>),
    ArrayU8(RawPtr<ArrayU8>),
    ArrayU32(RawPtr<ArrayU32>),
    Dict(RawPtr<Dict>),
    Function(RawPtr<Function>),
    Partial(RawPtr<Partial>),
}

impl FatPtr {
    /// Given a lifetime, convert to a `Value` type. Unsafe because anything can provide a lifetime
    /// without any safety guarantee that it's valid.
    pub fn as_value<'scope>(&self, guard: &'scope dyn MutatorScope) -> Value<'scope> {
        match self {
            FatPtr::Nil => Value::Nil,
            FatPtr::Pair(raw_ptr) => Value::Pair(raw_ptr.scoped_ref(guard)),
            FatPtr::Symbol(raw_ptr) => Value::Symbol(raw_ptr.scoped_ref(guard)),
            FatPtr::Number(num) => Value::Number(*num),
            FatPtr::NumberObject(raw_ptr) => Value::NumberObject(raw_ptr.scoped_ref(guard)),
            FatPtr::Text(raw_ptr) => Value::Text(raw_ptr.scoped_ref(guard)),
            FatPtr::List(raw_ptr) => Value::List(raw_ptr.scoped_ref(guard)),
            FatPtr::ArrayU8(raw_ptr) => Value::ArrayU8(raw_ptr.scoped_ref(guard)),
            FatPtr::ArrayU32(raw_ptr) => Value::ArrayU32(raw_ptr.scoped_ref(guard)),
            FatPtr::Dict(raw_ptr) => Value::Dict(raw_ptr.scoped_ref(guard)),
            FatPtr::Function(raw_ptr) => Value::Function(raw_ptr.scoped_ref(guard)),
            FatPtr::Partial(raw_ptr) => {Value::Partial(raw_ptr.scoped_ref(guard))
            }
        }
    }
}

/// Implement `From<RawPtr<T>> for FatPtr` for the given FatPtr discriminant and the given `T`
macro_rules! fatptr_from_rawptr {
    ($F:tt, $T:ty) => {
        impl From<RawPtr<$T>> for FatPtr {
            fn from(ptr: RawPtr<$T>) -> FatPtr {
                FatPtr::$F(ptr)
            }
        }
    };
}

fatptr_from_rawptr!(Pair, Pair);
fatptr_from_rawptr!(Symbol, Symbol);
fatptr_from_rawptr!(NumberObject, NumberObject);
fatptr_from_rawptr!(Text, Text);
fatptr_from_rawptr!(List, List);
fatptr_from_rawptr!(ArrayU8, ArrayU8);
fatptr_from_rawptr!(ArrayU32, ArrayU32);
fatptr_from_rawptr!(Dict, Dict);
fatptr_from_rawptr!(Function, Function);
fatptr_from_rawptr!(Partial, Partial);

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

    /// Return true if the pointer is nil
    pub fn is_nil(&self) -> bool {
        unsafe { self.tag == 0 }
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
            FatPtr::Text(raw) => TaggedPtr::object(raw),
            FatPtr::List(raw) => TaggedPtr::object(raw),
            FatPtr::ArrayU8(raw) => TaggedPtr::object(raw),
            FatPtr::ArrayU32(raw) => TaggedPtr::object(raw),
            FatPtr::Dict(raw) => TaggedPtr::object(raw),
            FatPtr::Function(raw) => TaggedPtr::object(raw),
            FatPtr::Partial(raw) => TaggedPtr::object(raw),
        }
    }
}

/// Simple identity equality
impl PartialEq for TaggedPtr {
    fn eq(&self, other: &TaggedPtr) -> bool {
        unsafe { self.tag == other.tag }
    }
}
