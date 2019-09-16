use std::fmt;

use crate::containers::{Container, StackAnyContainer, StackContainer};
use crate::error::RuntimeError;
use crate::memory::MutatorView;
use crate::primitives::{ArrayAny, ArrayU32};
use crate::printer::Print;
use crate::safeptr::{MutatorScope, ScopedPtr};

#[repr(u8)]
#[derive(FromPrimitive)]
pub enum Opcode {
    HALT = 0x00,
    RETURN = 0x01,
    LOADLIT = 0x02,
    NIL = 0x03,
    ATOM = 0x04,
    CAR = 0x05,
    CDR = 0x06,
    CONS = 0x07,
    EQ = 0x08,
    COND = 0x09,
}

pub type Register = u8;
pub type LiteralId = u16;

fn encode_0(op: Opcode) -> u32 {
    (op as u32) << 24
}

fn encode_1(op: Opcode, reg: Register) -> u32 {
    (op as u32) << 24 | (reg as u32) << 16
}

fn encode_2(op: Opcode, reg_acc: Register, reg1: Register) -> u32 {
    (op as u32) << 24 | (reg_acc as u32) << 16 | (reg1 as u32) << 8
}

fn encode_3(op: Opcode, reg_acc: Register, reg1: Register, reg2: Register) -> u32 {
    (op as u32) << 24 | (reg_acc as u32) << 16 | (reg1 as u32) << 8 | (reg2 as u32)
}

fn encode_load_lit(reg_acc: Register, literal_id: LiteralId) -> u32 {
    (Opcode::LOADLIT as u32) << 24 | (reg_acc as u32) << 16 | (literal_id as u32)
}

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
        self.code.push(mem, encode_0(op))
    }

    pub fn push_op1<'guard>(
        &self,
        mem: &'guard MutatorView,
        op: Opcode,
        reg: Register,
    ) -> Result<(), RuntimeError> {
        self.code.push(mem, encode_1(op, reg))
    }

    pub fn push_op2<'guard>(
        &self,
        mem: &'guard MutatorView,
        op: Opcode,
        reg_acc: Register,
        reg1: Register,
    ) -> Result<(), RuntimeError> {
        self.code.push(mem, encode_2(op, reg_acc, reg1))
    }

    pub fn push_op3<'guard>(
        &self,
        mem: &'guard MutatorView,
        op: Opcode,
        reg_acc: Register,
        reg1: Register,
        reg2: Register,
    ) -> Result<(), RuntimeError> {
        self.code.push(mem, encode_3(op, reg_acc, reg1, reg2))
    }

    pub fn push_loadlit<'guard>(
        &self,
        mem: &'guard MutatorView,
        reg_acc: Register,
        literal_id: LiteralId,
    ) -> Result<(), RuntimeError> {
        self.code.push(mem, encode_load_lit(reg_acc, literal_id))
    }

    pub fn push_lit<'guard>(
        &self,
        mem: &'guard MutatorView,
        literal: ScopedPtr<'guard>,
    ) -> Result<LiteralId, RuntimeError> {
        let lit_id = self.literals.length() as u16;
        match *literal {
            // TODO clone anything mutable
            _ => StackAnyContainer::push(&self.literals, mem, literal)?,
        };
        Ok(lit_id)
    }
}

impl Print for ByteCode {
    fn print<'guard>(&self, _guard: &'guard dyn MutatorScope, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ByteCode[...]")
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn code_encode_0() {
        let code = encode_0(Opcode::HALT);
        assert!(code == 0x0);
    }

    #[test]
    fn code_encode_1() {
        let code = encode_1(Opcode::ATOM, 0x05);
        assert!(code == 0x04050000)
    }

    #[test]
    fn code_encode_2() {
        let code = encode_2(Opcode::CAR, 0x06, 0x07);
        assert!(code == 0x05060700);
    }

    #[test]
    fn code_encode_3() {
        let code = encode_3(Opcode::EQ, 0x10, 0x11, 0x12);
        assert!(code == 0x08101112);
    }

    #[test]
    fn code_encode_load_lit() {
        let code = encode_load_lit(0x23, 0x1234);
        assert!(code == 0x02231234);
    }
}
