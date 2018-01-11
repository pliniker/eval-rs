use std::fmt;
use std::hash::{Hash, Hasher};
use std::slice;
use std::str;

use callables::Function;
use error::SourcePos;
use heap::{Heap, Ptr};


/// A fat pointer to a managed-memory object, carrying the type with it.
/// TODO: use the new union type to implement a tagged pointer?
pub enum Value<'heap, A: 'heap + Heap> {
    Nil,
    Symbol(Ptr<'heap, Symbol, A>),
    Pair(Ptr<'heap, Pair<'heap, A>, A>),
    Function(Ptr<'heap, Function<'heap, A>, A>),
}


// Type parameter A should not need to be Clone, so we can't #[derive(Copy, Clone)]
impl<'heap, A: 'heap + Heap> Clone for Value<'heap, A> {
    fn clone(&self) -> Value<'heap, A> {
        match *self {
            Value::Nil => Value::Nil,
            Value::Symbol(ptr) => Value::Symbol(ptr),
            Value::Pair(ptr) => Value::Pair(ptr),
            Value::Function(ptr) => Value::Function(ptr)
        }
    }
}


/// An enum of pointers can be copied
impl<'heap, A: 'heap + Heap> Copy for Value<'heap, A> {}


impl<'heap, A: 'heap + Heap> PartialEq for Value<'heap, A> {
    fn eq(&self, other: &Value<'heap, A>) -> bool {
        match *self {
            Value::Nil => if let &Value::Nil = other { true } else { false },

            // A Symbol is equal if it's pointers are equal
            Value::Symbol(lptr) => {
                if let &Value::Symbol(rptr) = other {
                    lptr.is(rptr)
                } else {
                    false
                }
            }

            // A pair is equal if it's contents have the same structure
            Value::Pair(lptr) => {
                if let &Value::Pair(rptr) = other {
                    lptr.eq(&rptr)
                } else {
                    false
                }
            }

            // A Function is equal if it's pointers are equal
            Value::Function(lptr) => {
                if let &Value::Function(rptr) = other {
                    lptr.is(rptr)
                } else {
                    false
                }
            }
        }
    }
}


/// Standard Display output should print out S-expressions.
impl<'heap, A: 'heap + Heap> fmt::Display for Value<'heap, A> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Value::Nil => write!(f, "()"),

            Value::Symbol(ptr) => write!(f, "{}", ptr.as_str()),

            Value::Pair(ptr) => {
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
            },

            Value::Function(ptr) => write!(f, "{}", ptr.name()),
        }
    }
}


/// Debug printing will print Pairs as literally as possible, using dot notation everywhere.
impl<'heap, A: 'heap + Heap> fmt::Debug for Value<'heap, A> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Value::Nil => write!(f, "nil"),
            Value::Symbol(ptr) => write!(f, "{}", ptr.as_str()),
            Value::Pair(ptr) => write!(f, "({:?} . {:?})", ptr.first, ptr.second),
            Value::Function(ptr) => write!(f, "{}", ptr.name()),
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


impl Hash for Symbol {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}


/// A basic Cons type cell
pub struct Pair<'heap, A: 'heap + Heap> {
    pub first: Value<'heap, A>,
    pub second: Value<'heap, A>,
    // Possible source code positions of the first and second values
    pub first_pos: Option<SourcePos>,
    pub second_pos: Option<SourcePos>
}


impl<'heap, A: 'heap + Heap> Pair<'heap, A> {
    pub fn new() -> Pair<'heap, A> {
        Pair {
            first: Value::Nil,
            second: Value::Nil,
            first_pos: None,
            second_pos: None
        }
    }

    /// Set the first value in the Pair
    pub fn set(&mut self, value: Value<'heap, A>) {
        self.first = value
    }

    /// Set the second value in the Pair directly
    pub fn dot(&mut self, value: Value<'heap, A>) {
        self.second = value
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
    pub fn eq(&self, other: Ptr<'heap, Pair<'heap, A>, A>) -> bool {
        self.first == other.first && self.second == other.second
    }

    /// Set Pair.second to a new Pair with newPair.first set to the value
    pub fn append(&mut self, allocator: &'heap A, value: Value<'heap, A>) -> Ptr<'heap, Pair<'heap, A>, A> {
        let mut pair = allocator.alloc(Pair::new());
        self.second = Value::Pair(pair);
        pair.first = value;
        pair
    }
}
