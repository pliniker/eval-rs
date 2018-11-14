
use std::fmt;

use crate::taggedptr::FatPtr;


pub fn print(value: FatPtr) -> String {
    format!("{}", value)
}


pub fn debug(value: FatPtr) -> String {
    format!("{:?}", value)
}


/// TODO unsafe inside
/// Standard Display output should print out S-expressions.
impl fmt::Display for FatPtr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            FatPtr::Nil => write!(f, "()"),

            FatPtr::Symbol(sym) => write!(f, "{}", unsafe { sym.as_ref().as_str() }),

            FatPtr::Pair(pair) => {
                let mut tail = pair;

                let mut first = unsafe { tail.as_ref().first };
                let mut second = unsafe { tail.as_ref().second };

                write!(f, "({}", FatPtr::from(first))?;

                while let FatPtr::Pair(next) = FatPtr::from(second) {
                    tail = next;

                    first = unsafe { tail.as_ref().first };
                    second = unsafe { tail.as_ref().second };

                    write!(f, " {}", FatPtr::from(unsafe { tail.as_ref().first }))?;
                }

                if let FatPtr::Symbol(sym) = FatPtr::from(second) {
                    write!(f, " . {}", unsafe { sym.as_ref().as_str() })?;
                }

                write!(f, ")")
            },

            _ => write!(f, "<UNKNOWN-TYPE!>"),
        }
    }

}


/// Debug printing will print Pairs as literally as possible, using dot notation everywhere.
impl fmt::Debug for FatPtr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            FatPtr::Nil => write!(f, "nil"),
            FatPtr::Symbol(ptr) => write!(f, "{}", unsafe { ptr.as_ref().as_str() } ),
            FatPtr::Pair(ptr) => {
                let pair = unsafe { ptr.as_ref() };
                write!(f, "({:?} . {:?})", FatPtr::from(pair.first), FatPtr::from(pair.second))
            },
            _ => write!(f, "<UNKNOWN-TYPE!>")
        }
    }
}
