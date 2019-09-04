use std::fmt;

use crate::containers::{Container, IndexedContainer, StackContainer};
use crate::error::{ErrorKind, RuntimeError};
use crate::memory::MutatorView;
use crate::primitives::{ArrayAny, ArrayU32};
use crate::printer::Print;
use crate::safeptr::{MutatorScope, ScopedPtr};


#[repr(u8)]
#[derive(FromPrimitive)]
enum Opcodes {
    HALT,
    ATOM,
    LOADINT,
    LOADSYM,
    CAR,
    CDR,
    CONS,
    EQ,
}


type Code = ArrayU32;
type SymbolList = ArrayAny;

pub struct ByteCode {
    code: Code,
    symbols: SymbolList,
    next_reg: u8,
}


impl ByteCode {
    pub fn new() -> ByteCode {
        ByteCode {
            code: Code::new(),
            symbols: SymbolList::new(),
            next_reg: 0
        }
    }

}

impl Print for ByteCode {
    fn print<'guard>(&self, _guard: &'guard MutatorScope, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ByteCode[...]")
    }
}
