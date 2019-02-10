/// Native runtime types

use std::fmt;
use std::hash::{Hash, Hasher};
use std::slice;
use std::str;

use crate::error::SourcePos;
use crate::taggedptr::{TaggedPtr, Value};


impl<'scope> fmt::Display for Value<'scope> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Nil => write!(f, "nil"),
            Value::Pair(p) => write!(f, "{}", p),
            Value::Symbol(s) => write!(f, "{}", s),
            Value::Number(n) => write!(f, "{}", *n),
            Value::NumberObject(n) => write!(f, "{}", n),
        }
    }
}


/// A Symbol is a unique object that has a unique name string. The backing storage for the
/// underlying str data must have a lifetime of at least that of the Symbol instance to
/// prevent use-after-free.
/// See `SymbolMap`
/// TODO is there a way to formalize this relationship?
pub struct Symbol {
    name_ptr: *const u8,
    name_len: usize,
}


impl Symbol {
    /// The originating &str must be owned by a SymbolMap hash table
    pub fn new(name: &str) -> Symbol {
        Symbol {
            name_ptr: name.as_ptr(),
            name_len: name.len(),
        }
    }

    /// Unsafe because Symbol does not own the &str
    pub unsafe fn as_str(&self) -> &str {
        let slice = slice::from_raw_parts(self.name_ptr, self.name_len);
        str::from_utf8(slice).unwrap()
    }
}


/// TODO since as_str() is unsafe, this should be too, but how can this be made to make sense?
impl Hash for Symbol {
    fn hash<H: Hasher>(&self, state: &mut H) {
        unsafe { self.as_str() }.hash(state);
    }
}


/// Redefine Pair from types.rs
pub struct Pair {
    pub first: TaggedPtr,
    pub second: TaggedPtr,
    // Possible source code positions of the first and second values
    pub first_pos: Option<SourcePos>,
    pub second_pos: Option<SourcePos>
}


impl Pair {
    pub fn new() -> Pair {
        Pair {
            first: TaggedPtr::nil(),
            second: TaggedPtr::nil(),
            first_pos: None,
            second_pos: None
        }
    }
/*
    /// Compare contents of one Pair to another
    pub fn eq(&self, other: RawPtr<Pair>) -> bool {
        self.first == other.first && self.second == other.second
    }

    /// Set Pair.second to a new Pair with newPair.first set to the value
    pub fn append(&mut self, allocator: &'heap A, value: Value<'heap, A>) -> Ptr<'heap, Pair<'heap, A>, A> {
        let mut pair = allocator.alloc(Pair::new());
        self.second = Value::Pair(pair);
        pair.first = value;
        pair
    }
*/
}


/// TODO A heap-allocated number
pub struct NumberObject {
    value: isize
}
