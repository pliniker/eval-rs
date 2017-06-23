use std::slice;
use std::fmt;
use std::str;

use error::SourcePos;
use memory::{Allocator, Ptr};


/// This type is not optimally stored. It could be implemented as a tagged pointer.
#[derive(Copy, Clone)]
pub enum Value<'a, A: 'a + Allocator> {
    Nil,
    Symbol(Ptr<'a, Symbol, A>),
    Pair(Ptr<'a, Pair<'a, A>, A>),
}


impl<'a, A: 'a + Allocator> PartialEq for Value<'a, A> {
    fn eq(&self, other: &Value<'a, A>) -> bool {
        match self {
            &Value::Nil => if let &Value::Nil = other { true } else { false },

            // A Symbol is equal if it's pointers are equal
            &Value::Symbol(lptr) => {
                if let &Value::Symbol(rptr) = other {
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
impl<'a, A: 'a + Allocator> fmt::Display for Value<'a, A> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Value::Nil => write!(f, "()"),
            &Value::Symbol(ptr) => write!(f, "{}", ptr.as_str()),

            &Value::Pair(ptr) => {
                let mut tail = ptr;
                write!(f, "({}", tail.first)?;

                while let Value::Pair(next) = tail.second {
                    tail = next;
                    write!(f, " {}", tail.first)?;
                }

                if let Value::Symbol(ptr) = tail.second {
                    write!(f, " . {}", ptr.as_str())?;
                }

                write!(f, ")")
            }
        }
    }
}


/// Debug printing will print Pairs as literally as possible, using dot notation everywhere.
impl<'a, A: 'a + Allocator> fmt::Debug for Value<'a, A> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Value::Nil => write!(f, "nil"),
            &Value::Symbol(ptr) => write!(f, "{}", ptr.as_str()),
            &Value::Pair(ptr) => write!(f, "({:?} . {:?})", ptr.first, ptr.second),
        }
    }
}


/// A Symbol is a unique object that has a name string. See SymbolMap also - there should
/// never be two Symbol instances with the same name.
pub struct Symbol {
    // the String object must be owned by a SymbolMap hash table
    name_ptr: *const u8,
    name_len: usize,
}


impl Symbol {
    pub fn new(name: &str) -> Symbol {
        Symbol {
            name_ptr: name.as_ptr(),
            name_len: name.len(),
        }
    }

    // As Symbols are owned by a SymbolMap, the name String lifetime is guaranteed
    // to be at least that of the Symbol
    pub fn as_str(&self) -> &str {
        unsafe {
            let slice = slice::from_raw_parts(self.name_ptr, self.name_len);
            str::from_utf8(slice).unwrap()
        }
    }
}


// A basic Cons cell type
pub struct Pair<'a, A: 'a + Allocator> {
    pub first: Value<'a, A>,
    pub second: Value<'a, A>,
    pub first_pos: Option<SourcePos>,
    pub second_pos: Option<SourcePos>
}


impl<'a, A: 'a + Allocator> Pair<'a, A> {
    pub fn new() -> Pair<'a, A> {
        Pair {
            first: Value::Nil,
            second: Value::Nil,
            first_pos: None,
            second_pos: None
        }
    }

    /// Set the first value in the Pair
    pub fn set(&mut self, value: Value<'a, A>) {
        self.first = value
    }

    /// Set the second value in the Pair directly
    pub fn dot(&mut self, value: Value<'a, A>) {
        self.second = value
    }

    /// Set Pair.second to a new Pair with newPair.first set to the value
    pub fn append(&mut self, value: Value<'a, A>, mem: &'a A) -> Ptr<'a, Pair<'a, A>, A> {
        let mut pair = mem.alloc(Pair::new());
        self.second = Value::Pair(pair);
        pair.first = value;
        pair
    }

    /// Set the source code position of the lhs of the pair
    pub fn set_first_source_pos(&mut self, pos: SourcePos) {
        self.first_pos = Some(pos);
    }

    /// Set the source code position of the rhs of the pair
    pub fn set_second_source_pos(&mut self, pos: SourcePos) {
        self.second_pos = Some(pos)
    }

    /// Compare contents of one Pair to another
    pub fn eq(&self, other: Ptr<'a, Pair<'a, A>, A>) -> bool {
        self.first == other.first && self.second == other.second
    }
}
