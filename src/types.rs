use std::fmt;

use error::SourcePos;
use memory::{Arena, Ptr};


#[derive(Copy, Clone)]
pub enum Value {
    //  Symbol(String, SourcePos),  // TODO do something about this!
    Symbol(SourcePos),
    Pair(Ptr<Pair>),
    Nil,
}


impl PartialEq for Value {
    fn eq(&self, other: &Value) -> bool {
        match self {
            &Value::Nil => if let &Value::Nil = other { true } else { false },
            &Value::Symbol(_) => {
                if let &Value::Symbol(_) = other {
                    true
                } else {
                    false
                }
            }
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


impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Value::Nil => write!(f, "()"),
            &Value::Symbol(_) => write!(f, "X"),

            &Value::Pair(ptr) => {
                let mut tail = ptr;
                write!(f, "({}", tail.first)?;

                while let Value::Pair(next) = tail.second {
                    tail = next;
                    write!(f, " {}", tail.first)?;
                }

                if let Value::Symbol(_) = tail.second {
                    write!(f, " . X")?;
                }

                write!(f, ")")
            }
        }
    }
}


impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Value::Nil => write!(f, "nil"),
            &Value::Symbol(_) => write!(f, "X"),
            &Value::Pair(ptr) => write!(f, "({:?} . {:?})", ptr.first, ptr.second),
        }
    }
}


// A basic cons cell type
pub struct Pair {
    pub first: Value,
    pub second: Value,
}


impl Pair {
    pub fn alloc(mem: &mut Arena) -> Ptr<Pair> {
        mem.allocate(Pair {
            first: Value::Nil,
            second: Value::Nil,
        })
    }

    pub fn set(&mut self, value: Value) {
        self.first = value
    }

    pub fn dot(&mut self, value: Value) {
        self.second = value
    }

    pub fn append(&mut self, mem: &mut Arena, value: Value) -> Ptr<Pair> {
        let mut pair = Pair::alloc(mem);
        self.second = Value::Pair(pair);
        pair.first = value;
        pair

    }

    pub fn eq(&self, other: Ptr<Pair>) -> bool {
        self.first == other.first && self.second == other.second
    }
}
