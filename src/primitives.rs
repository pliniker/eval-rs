/// Native runtime types
use std::fmt;
use std::hash::{Hash, Hasher};
use std::slice;
use std::str;

use crate::error::SourcePos;
use crate::printer::Print;
use crate::safeptr::{CellPtr, MutatorScope};
use crate::taggedptr::{TaggedPtr, Value};

/// `Value` can have a safe `Display` implementation
impl<'scope> fmt::Display for Value<'scope> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Nil => write!(f, "nil"),
//            Value::Pair(p) => write!(f, "{}", p),
            Value::Symbol(s) => s.print(self, f),
            Value::Number(n) => write!(f, "{}", *n),
//            Value::NumberObject(n) => write!(f, "{}", n),
            _ => write!(f, "unimplemented")
        }
    }
}

impl<'scope> MutatorScope for Value<'scope> {}

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

impl Print for Symbol {
    fn print<'scope>(&self, _guard: &'scope MutatorScope, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", unsafe { self.as_str() })
    }
}

/// TODO since as_str() is unsafe, this should be too, but how can this be made to make sense?
//impl Hash for Symbol {
//    fn hash<H: Hasher>(&self, state: &mut H) {
//        unsafe { self.as_str() }.hash(state);
//    }
//}

/// A Pair of pointers, like a Cons cell of old
pub struct Pair {
    pub first: CellPtr,
    pub second: CellPtr,
    // Possible source code positions of the first and second values
    pub first_pos: Option<SourcePos>,
    pub second_pos: Option<SourcePos>,
}

impl Pair {
    pub fn new() -> Pair {
        Pair {
            first: CellPtr::new_nil(),
            second: CellPtr::new_nil(),
            first_pos: None,
            second_pos: None,
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

impl Print for Pair {
    fn print<'scope>(&self, guard: &'scope MutatorScope, f: &mut fmt::Formatter) -> fmt::Result {
        let second = self.second.get(guard);

        match second {
            Value::Nil => write!(f, "({})", self.first.get(guard)),
            _ => write!(f, "({} {})", self.first.get(guard), second),
        }
    }
}

/// TODO A heap-allocated number
pub struct NumberObject {
    value: isize,
}
