use std::fmt;

use stickyimmix::ArraySize;

use crate::containers::{
    Container, IndexedAnyContainer, IndexedContainer, StackAnyContainer, StackContainer,
};
use crate::error::{err_eval, RuntimeError};
use crate::memory::MutatorView;
use crate::primitives::{ArrayAny, ArrayU32};
use crate::printer::Print;
use crate::safeptr::{MutatorScope, ScopedPtr};

/// VM opcodes
#[repr(u8)]
#[derive(FromPrimitive, PartialEq)]
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
    JMPT = 0x09,
    JMP = 0x0A,
}

/// A register can be in the range 0..255
pub type Register = u8;

/// Literals are stored in a list, a LiteralId describes the index of the value in the list
pub type LiteralId = u16;

/// Encode an opcode with no operands
fn encode_0(op: Opcode) -> u32 {
    (op as u32) << 24
}

/// Encode an opcode with one register operand
fn encode_1(op: Opcode, reg: Register) -> u32 {
    (op as u32) << 24 | (reg as u32) << 16
}

/// Encode an opcode with two register operands, the result being stored in the first
fn encode_2(op: Opcode, reg_acc: Register, reg1: Register) -> u32 {
    (op as u32) << 24 | (reg_acc as u32) << 16 | (reg1 as u32) << 8
}

/// Encode an opcode with three register operands, the result being stored in the first
fn encode_3(op: Opcode, reg_acc: Register, reg1: Register, reg2: Register) -> u32 {
    (op as u32) << 24 | (reg_acc as u32) << 16 | (reg1 as u32) << 8 | (reg2 as u32)
}

/// Encode a literal load operation
fn encode_load_lit(reg_acc: Register, literal_id: LiteralId) -> u32 {
    (Opcode::LOADLIT as u32) << 24 | (reg_acc as u32) << 16 | (literal_id as u32)
}

/// Decode an instruction and return the opcode
fn decode_op(instr: u32) -> Result<Opcode, RuntimeError> {
    let opcode = (instr >> 24) as u8;
    if let Some(opcode) = num::FromPrimitive::from_u8(opcode) {
        Ok(opcode)
    } else {
        Err(err_eval("Invalid opcode in bytecode"))
    }
}

/// Decode the first register operand in an instruction
fn decode_reg_acc(instr: u32) -> Register {
    ((instr >> 16) & 0xFF) as u8
}

/// Decode the second register operand in an instruction
fn decode_reg1(instr: u32) -> Register {
    ((instr >> 8) & 0xFF) as u8
}

/// Decode the third register operand in an instruction
fn decode_reg2(instr: u32) -> Register {
    (instr & 0xFF) as u8
}

/// Decode the literal id operand in an instruction
fn decode_literal_id(instr: u32) -> LiteralId {
    (instr & 0xFFFF) as u16
}

/// Bytecode is stored as fixed-width 32-bit operator+operand values as in Lua 5
pub type Code = ArrayU32;

/// Literals are stored in a separate list of machine-word-width pointers
pub type Literals = ArrayAny;

/// Byte code consists of the code and any literals used.
pub struct ByteCode {
    code: Code,
    literals: Literals,
}

impl ByteCode {
    /// Instantiate a blank ByteCode instance
    pub fn new() -> ByteCode {
        ByteCode {
            code: Code::new(),
            literals: Literals::new(),
        }
    }

    /// Push a 0-operand instruction to the back of the sequence
    pub fn push_op0<'guard>(
        &self,
        mem: &'guard MutatorView,
        op: Opcode,
    ) -> Result<(), RuntimeError> {
        self.code.push(mem, encode_0(op))
    }

    /// Push a 1-operand instuction to the back of the sequence
    pub fn push_op1<'guard>(
        &self,
        mem: &'guard MutatorView,
        op: Opcode,
        reg: Register,
    ) -> Result<(), RuntimeError> {
        self.code.push(mem, encode_1(op, reg))
    }

    /// Push a 2-operand instruction to the back of the sequence
    pub fn push_op2<'guard>(
        &self,
        mem: &'guard MutatorView,
        op: Opcode,
        reg_acc: Register,
        reg1: Register,
    ) -> Result<(), RuntimeError> {
        self.code.push(mem, encode_2(op, reg_acc, reg1))
    }

    /// Push a 3-operand instruction to the back of the sequence
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

    /// Push a literal-load operation to the back of the sequence
    pub fn push_loadlit<'guard>(
        &self,
        mem: &'guard MutatorView,
        reg_acc: Register,
        literal_id: LiteralId,
    ) -> Result<(), RuntimeError> {
        // TODO clone anything mutable
        self.code.push(mem, encode_load_lit(reg_acc, literal_id))
    }

    /// Push a literal pointer/value to the back of the literals list and return it's index
    pub fn push_lit<'guard>(
        &self,
        mem: &'guard MutatorView,
        literal: ScopedPtr<'guard>,
    ) -> Result<LiteralId, RuntimeError> {
        let lit_id = self.literals.length() as u16;
        StackAnyContainer::push(&self.literals, mem, literal)?;
        Ok(lit_id)
    }
}

impl Print for ByteCode {
    fn print<'guard>(
        &self,
        _guard: &'guard dyn MutatorScope,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        write!(f, "ByteCode[...]")
    }
}

/// Interpret a ByteCode as a stream of instructions, handling an instruction-pointer abstraction.
pub struct InstructionStream {
    instructions: ByteCode,
    ip: ArraySize,
    current: u32,
}

impl InstructionStream {
    /// Create an InstructionStream instance with the given ByteCode instance that will be iterated over
    pub fn new(code: ByteCode) -> InstructionStream {
        InstructionStream {
            instructions: code,
            ip: 0,
            current: 0,
        }
    }

    /// Retrieve the next instruction and return the Opcode, if it correctly decodes
    pub fn get_next_opcode<'guard>(
        &mut self,
        guard: &'guard dyn MutatorScope,
    ) -> Result<Opcode, RuntimeError> {
        let instr = self.instructions.code.get(guard, self.ip)?;
        self.ip += 1;
        decode_op(instr)
    }

    /// Retrieve the accumulator register operand from the current instruction
    pub fn get_reg_acc(&self) -> Register {
        decode_reg_acc(self.current)
    }

    /// Retrieve the first argument register operand from the current instruction
    pub fn get_reg1(&self) -> Register {
        decode_reg1(self.current)
    }

    /// Retrieve the second argument register operand from the current instruction
    pub fn get_reg2(&self) -> Register {
        decode_reg2(self.current)
    }

    /// Retrieve the literal pointer from the current instruction
    pub fn get_literal<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
    ) -> Result<ScopedPtr<'guard>, RuntimeError> {
        let lit_id = decode_literal_id(self.current);
        IndexedAnyContainer::get(&self.instructions.literals, guard, lit_id as ArraySize)
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

    #[test]
    fn code_decode_op() {
        let code = 0x04010203;
        let op = decode_op(code).unwrap();
        assert!(op == Opcode::ATOM);
    }

    #[test]
    fn code_decode_reg_acc() {
        let code = 0x08101112;
        let reg_acc = decode_reg_acc(code);
        assert!(reg_acc == 0x10);
    }

    #[test]
    fn code_decode_reg1() {
        let code = 0x08101112;
        let reg1 = decode_reg1(code);
        assert!(reg1 == 0x11);
    }

    #[test]
    fn code_decode_reg2() {
        let code = 0x08101112;
        let reg2 = decode_reg2(code);
        assert!(reg2 == 0x12);
    }

    #[test]
    #[should_panic]
    fn code_decode_invalid_op() {
        let code = 0xff000000;
        let _op = decode_op(code).unwrap();
    }
}
