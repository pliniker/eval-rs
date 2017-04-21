use std::slice;
use std::fmt;
use std::str;

use error::SourcePos;
use memory::{Allocator, Ptr};


/// This type is not optimally stored. It could be implemented as a tagged pointer.
#[derive(Copy, Clone)]
pub enum Value {
    Nil,
    Symbol(Ptr<Symbol>, SourcePos),
    Pair(Ptr<Pair>),
}


impl PartialEq for Value {
    fn eq(&self, other: &Value) -> bool {
        match self {
            &Value::Nil => if let &Value::Nil = other { true } else { false },

            // A Symbol is equal if it's pointers are equal
            &Value::Symbol(lptr, _) => {
                if let &Value::Symbol(rptr, _) = other {
                    lptr.is(rptr)
                } else {
                    false
                }
            }

            // A pair is equal if it's contents have the same structure
            &Value::Pair(lptr) => {
                if let &Value::Pair(rptr) = other {
                    lptr.eq(rptr)
                } else {
                    false
                }
            }
        }
    }
}


/// Standard Display output should print out S-expressions.
impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Value::Nil => write!(f, "()"),
            &Value::Symbol(ptr, _) => write!(f, "{}", unsafe { ptr.as_str() }),

            &Value::Pair(ptr) => {
                let mut tail = ptr;
                write!(f, "({}", tail.first)?;

                while let Value::Pair(next) = tail.second {
                    tail = next;
                    write!(f, " {}", tail.first)?;
                }

                if let Value::Symbol(ptr, _) = tail.second {
                    write!(f, " . {}", unsafe { ptr.as_str() })?;
                }

                write!(f, ")")
            }
        }
    }
}


/// Debug printing will print Pairs as literally as possible, using dot notation everywhere.
impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Value::Nil => write!(f, "nil"),
            &Value::Symbol(ptr, _) => write!(f, "{}", unsafe { ptr.as_str() }),
            &Value::Pair(ptr) => write!(f, "({:?} . {:?})", ptr.first, ptr.second),
        }
    }
}


/// A Symbol is a unique object that has a name string. See SymbolMap also - there should
/// never be two Symbol instances with the same name.
pub struct Symbol {
    // the String object is be owned by a SymbolMap hash table
    name_ptr: *const u8,
    name_len: usize,
}


impl Symbol {
    pub fn new<M>(name: &str, mem: &mut M) -> Ptr<Symbol> where M: Allocator {
        mem.alloc(Symbol {
            name_ptr: name.as_ptr(),
            name_len: name.len(),
        })
    }

    /// Unsafe because the &str pointer is owned elsewhere and can't be known
    /// here if it still exists
    pub unsafe fn as_str(&self) -> &str {
        let slice = slice::from_raw_parts(self.name_ptr, self.name_len);
        str::from_utf8(slice).unwrap()
    }
}


// A basic Cons cell type
pub struct Pair {
    pub first: Value,
    pub second: Value,
}


impl Pair {
    pub fn new<M>(mem: &mut M) -> Ptr<Pair> where M: Allocator {
        mem.alloc(Pair {
            first: Value::Nil,
            second: Value::Nil,
        })
    }

    /// Set the first value in the Pair
    pub fn set(&mut self, value: Value) {
        self.first = value
    }

    /// Set the second value in the Pair directly
    pub fn dot(&mut self, value: Value) {
        self.second = value
    }

    /// Set Pair.second to a new Pair with newPair.first set to the value
    pub fn append<M>(&mut self, value: Value, mem: &mut M) -> Ptr<Pair> where M: Allocator {
        let mut pair = Pair::new(mem);
        self.second = Value::Pair(pair);
        pair.first = value;
        pair

    }

    /// Compare contents of one Pair to another
    pub fn eq(&self, other: Ptr<Pair>) -> bool {
        self.first == other.first && self.second == other.second
    }
}
