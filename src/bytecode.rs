use std::fmt;

use crate::containers::{Container, StackContainer, StackAnyContainer};
use crate::error::RuntimeError;
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
    NIL,
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

    pub fn push_op0<'guard>(
        &self,
        mem: &'guard MutatorView,
        op: Opcode,
    ) -> Result<(), RuntimeError> {
        self.code.push(mem, (op as u32) << 24)
    }

    pub fn push_op1<'guard>(
        &self,
        mem: &'guard MutatorView,
        op: Opcode,
        reg: Register,
    ) -> Result<(), RuntimeError> {
        let code: u32 = (op as u32) << 24 | (reg as u32) << 16;
        self.code.push(mem, code)
    }

    pub fn push_op2<'guard>(
        &self,
        mem: &'guard MutatorView,
        op: Opcode,
        reg_acc: Register,
        reg1: Register,
    ) -> Result<(), RuntimeError> {
        let code: u32 = (op as u32) << 24 | (reg_acc as u32) << 16 | (reg1 as u32) << 8;
        self.code.push(mem, code)
    }

    pub fn push_op3<'guard>(
        &self,
        mem: &'guard MutatorView,
        op: Opcode,
        reg_acc: Register,
        reg1: Register,
        reg2: Register,
    ) -> Result<(), RuntimeError> {
        let code: u32 = (op as u32) << 24 | (reg_acc as u32) << 16 | (reg1 as u32) << 8 | (reg2 as u32);
        self.code.push(mem, code)
    }

    pub fn push_loadlit<'guard>(
        &self,
        mem: &'guard MutatorView,
        reg_acc: Register,
        literal_id: LiteralId,
    ) -> Result<(), RuntimeError> {
        let code: u32 = (Opcode::LOADLIT as u32) << 24 | (reg_acc as u32) << 16 | (literal_id as u32);
        self.code.push(mem, code)
    }

    pub fn push_lit<'guard>(
        &self,
        mem: &'guard MutatorView,
        literal: ScopedPtr<'guard>,
    ) -> Result<LiteralId, RuntimeError> {
        let lit_id = self.literals.length() as u16;
        match *literal {
            // TODO clone anything mutable
            _ => StackAnyContainer::push(&self.literals, mem, literal)?
        };
        Ok(lit_id)
    }
}

impl Print for ByteCode {
    fn print<'guard>(&self, _guard: &'guard MutatorScope, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ByteCode[...]")
    }
}
