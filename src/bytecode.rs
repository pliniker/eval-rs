use std::fmt;

use crate::containers::{Container, IndexedContainer, StackContainer};
use crate::error::{ErrorKind, RuntimeError};
use crate::memory::MutatorView;
use crate::primitives::{ArrayAny, ArrayU32};
use crate::printer::Print;
use crate::safeptr::{MutatorScope, ScopedPtr};

#[repr(u8)]
#[derive(FromPrimitive)]
pub enum Opcode {
    HALT,
    RETURN,
    MOV,
    ATOM,
    LOADLIT,
    CAR,
    CDR,
    CONS,
    EQ,
}

pub type Register = u8;
pub type LiteralId = u16;

pub type Code = ArrayU32;
pub type Literals = ArrayAny;

pub struct ByteCode {
    code: Code,
    literals: Literals,
}

impl ByteCode {
    pub fn new() -> ByteCode {
        ByteCode {
            code: Code::new(),
            literals: Literals::new(),
        }
    }

    pub fn push_op0<'guard>(&self, mem: &'guard MutatorView, op: Opcode) {}

    pub fn push_op1<'guard>(&self, mem: &'guard MutatorView, op: Opcode, reg: Register) {}

    pub fn push_op2<'guard>(
        &self,
        mem: &'guard MutatorView,
        op: Opcode,
        reg_acc: Register,
        reg1: Register,
    ) {
    }

    pub fn push_op3<'guard>(
        &self,
        mem: &'guard MutatorView,
        op: Opcode,
        reg_acc: Register,
        reg1: Register,
        reg2: Register,
    ) {
    }

    pub fn push_loadlit<'guard>(
        &self,
        mem: &'guard MutatorView,
        reg_acc: Register,
        lit: ScopedPtr,
    ) {
    }

    pub fn push_loadlit<'guard>(&self, mem: &'guard MutatorView, reg_acc: Register, lit: LiteralId) {}

    pub fn push_lit<'guard>(&self, mem: &'guard MutatorView, literal: ScopedPtr<'guard>) -> LiteralId {}
}

impl Print for ByteCode {
    fn print<'guard>(&self, _guard: &'guard MutatorScope, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ByteCode[...]")
    }
}
