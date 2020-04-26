use std::cell::Cell;
use std::fmt;

use crate::array::{ArraySize, ArrayU32};
use crate::containers::{
    Container, IndexedAnyContainer, IndexedContainer, StackAnyContainer, StackContainer,
};
use crate::error::{err_eval, RuntimeError};
use crate::list::List;
use crate::memory::MutatorView;
use crate::printer::Print;
use crate::safeptr::{CellPtr, MutatorScope, ScopedPtr, TaggedScopedPtr};
use crate::taggedptr::TaggedPtr;

/// VM opcodes
#[repr(u8)]
#[derive(FromPrimitive, PartialEq, Debug)]
pub enum Opcode {
    HALT = 0x00,
    RETURN = 0x01,
    LOADLIT = 0x02,
    NIL = 0x03,
    ATOM = 0x04,
    CAR = 0x05,
    CDR = 0x06,
    CONS = 0x07,
    IS = 0x08,
    JMP = 0x09,
    JMPT = 0x0A,
    JMPNT = 0x0B,
    LOADNIL = 0x0C,
    LOADGLOBAL = 0x0D,
    STOREGLOBAL = 0x0E,
    CALL = 0x0F,
    LOADINT = 0x10,
    COPYREG = 0x11,
    LOADNONLOCAL = 0x12,
    ADD = 0x13,
    SUB = 0x14,
    MUL = 0x15,
    DIVINTEGER = 0x16,
}

/// A register can be in the range 0..255
pub type Register = u8;

/// A register in a previous stack frame can be in the range 0..65535, as a value subtracted from
/// the current stack base pointer
pub type NonLocalRegister = u16;

/// A literal integer that can be baked into an opcode can be in the range -32768..32767
pub type LiteralInteger = i16;

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

/// Encode a literal load operation
fn encode_load_integer(reg_acc: Register, integer: LiteralInteger) -> u32 {
    (Opcode::LOADINT as u32) << 24 | (reg_acc as u32) << 16 | ((integer as u32) & 0xFFFF)
}

/// Encode a literal load operation
fn encode_load_nonlocal(reg_acc: Register, nonlocal_reg: NonLocalRegister) -> u32 {
    (Opcode::LOADINT as u32) << 24 | (reg_acc as u32) << 16 | (nonlocal_reg as u32)
}

/// Encode a jump operation
fn encode_jump(op: Opcode, reg: Register, offset: ArraySize) -> u32 {
    (op as u32) << 24 | (reg as u32) << 16 | offset as u32
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

/// Decode the literal id operand in an instruction
fn decode_literal_integer(instr: u32) -> LiteralInteger {
    (instr & 0xFFFF) as i16
}

/// Decode the nonlocal register offset in an instruction
fn decode_nonlocal_reg(instr: u32) -> NonLocalRegister {
    (instr & 0xFFFF) as u16
}

/// Decode the jump offset in an instruction
fn decode_jump_offset(instr: u32) -> i16 {
    (instr & 0xFFFF) as i16
}

/// Bytecode is stored as fixed-width 32-bit operator+operand values.
/// This is not the most efficient format but it is easy to work with.
pub type Code = ArrayU32;

/// Literals are stored in a separate list of machine-word-width pointers.
/// This is also not the most efficient scheme but it is easy to work with.
pub type Literals = List;

/// Byte code consists of the code and any literals used.
#[derive(Clone)]
pub struct ByteCode {
    code: Code,
    literals: Literals,
}

impl ByteCode {
    /// Instantiate a blank ByteCode instance
    pub fn alloc<'guard>(
        mem: &'guard MutatorView,
    ) -> Result<ScopedPtr<'guard, ByteCode>, RuntimeError> {
        mem.alloc(ByteCode {
            code: Code::new(),
            literals: Literals::new(),
        })
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

    /// Push an unconditionl jump instruction to the back of the sequence
    pub fn push_jump<'guard>(&self, mem: &'guard MutatorView) -> Result<(), RuntimeError> {
        self.code.push(mem, encode_jump(Opcode::JMP, 0, 0xFFFF))
    }

    /// Push an unconditionl jump instruction to the back of the sequence
    pub fn push_cond_jump<'guard>(
        &self,
        mem: &'guard MutatorView,
        op: Opcode,
        reg: Register,
    ) -> Result<(), RuntimeError> {
        self.code.push(mem, encode_jump(op, reg, 0xFFFF))
    }

    /// Set the jump offset of a jump instruction to a new value
    pub fn write_jump_offset<'guard>(
        &self,
        mem: &'guard MutatorView,
        instruction: ArraySize,
        offset: ArraySize,
    ) -> Result<(), RuntimeError> {
        let bytecode = self.code.get(mem, instruction)? & 0xFFFF0000;
        self.code.set(mem, instruction, bytecode | offset as u32)?;
        Ok(())
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
        literal: TaggedScopedPtr<'guard>,
    ) -> Result<LiteralId, RuntimeError> {
        let lit_id = self.literals.length() as u16;
        StackAnyContainer::push(&self.literals, mem, literal)?;
        Ok(lit_id)
    }

    pub fn push_load_integer<'guard>(
        &self,
        mem: &'guard MutatorView,
        reg_acc: Register,
        literal: LiteralInteger,
    ) -> Result<(), RuntimeError> {
        self.code.push(mem, encode_load_integer(reg_acc, literal))
    }

    /// Get the index into the bytecode array of the last instruction
    pub fn last_instruction(&self) -> ArraySize {
        self.code.length() - 1
    }

    /// Get the index into the bytecode array of the next instruction that will be pushed
    pub fn next_instruction(&self) -> ArraySize {
        self.code.length()
    }
}

impl Print for ByteCode {
    fn print<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        for index in 0..self.code.length() {
            let instr = self.code.get(guard, index)?;
            let op = decode_op(instr)?;
            let acc = decode_reg_acc(instr);
            let reg1 = decode_reg1(instr);
            let reg2 = decode_reg2(instr);
            write!(
                f,
                "BC: {:02} {:8} acc={} reg1={} reg2={}\n",
                index,
                format!("{:?}", op),
                acc,
                reg1,
                reg2
            )?;
        }
        Ok(())
    }
}

/// Interpret a ByteCode as a stream of instructions, handling an instruction-pointer abstraction.
pub struct InstructionStream {
    instructions: CellPtr<ByteCode>,
    ip: Cell<ArraySize>,
    current: Cell<u32>,
}

impl InstructionStream {
    /// Create an InstructionStream instance with the given ByteCode instance that will be iterated over
    pub fn alloc<'guard>(
        mem: &'guard MutatorView,
        code: ScopedPtr<'_, ByteCode>,
    ) -> Result<ScopedPtr<'guard, InstructionStream>, RuntimeError> {
        mem.alloc(InstructionStream {
            instructions: CellPtr::new_with(code),
            ip: Cell::new(0),
            current: Cell::new(0),
        })
    }

    /// Change to a different stack frame, either as a function call or a return
    pub fn switch_frame(&self, code: ScopedPtr<'_, ByteCode>, ip: ArraySize) {
        self.instructions.set(code);
        self.ip.set(ip);
    }

    /// Retrieve the next instruction and return the Opcode, if it correctly decodes
    pub fn get_next_opcode<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
    ) -> Result<Opcode, RuntimeError> {
        let instr = self
            .instructions
            .get(guard)
            .code
            .get(guard, self.ip.get())?;
        self.ip.set(self.ip.get() + 1);
        self.current.set(instr);
        decode_op(instr)
    }

    /// Retrieve the accumulator register operand from the current instruction
    pub fn get_reg_acc(&self) -> Register {
        decode_reg_acc(self.current.get())
    }

    /// Retrieve the first argument register operand from the current instruction
    pub fn get_reg1(&self) -> Register {
        decode_reg1(self.current.get())
    }

    /// Retrieve the second argument register operand from the current instruction
    pub fn get_reg2(&self) -> Register {
        decode_reg2(self.current.get())
    }

    /// Retrieve the literal pointer from the current instruction
    pub fn get_literal<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
    ) -> Result<TaggedPtr, RuntimeError> {
        let lit_id = decode_literal_id(self.current.get());

        Ok(IndexedContainer::get(
            &self.instructions.get(guard).literals,
            guard,
            lit_id as ArraySize,
        )?
        .get_ptr())
    }

    /// Extract the signed 16bit integer literal from the opcode
    pub fn get_literal_integer(&self) -> LiteralInteger {
        decode_literal_integer(self.current.get())
    }

    pub fn get_nonlocal_reg(&self) -> NonLocalRegister {
        decode_nonlocal_reg(self.current.get())
    }

    /// Return the next instruction pointer
    pub fn get_next_ip(&self) -> ArraySize {
        self.ip.get()
    }

    /// Adjust the instruction pointer by the given signed offset from the current ip
    pub fn jump(&self) {
        let offset = decode_jump_offset(self.current.get());
        let mut ip = self.ip.get() as i32;
        ip += offset as i32;
        self.ip.set(ip as ArraySize);
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
        let code = encode_3(Opcode::IS, 0x10, 0x11, 0x12);
        assert!(code == 0x08101112);
    }

    #[test]
    fn code_encode_load_lit() {
        let code = encode_load_lit(0x23, 0x1234);
        assert!(code == 0x02231234);
    }

    #[test]
    fn code_encode_load_int() {
        let code = encode_load_integer(0x23, -1);
        assert!(code == 0x1023FFFF);
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
    fn code_decode_int() {
        let code = 0x1023FFFF;
        let op = decode_op(code).unwrap();
        assert!(op == Opcode::LOADINT);
        let integer = decode_literal_integer(code);
        assert!(integer == -1);
    }

    #[test]
    #[should_panic]
    fn code_decode_invalid_op() {
        let code = 0xff000000;
        let _op = decode_op(code).unwrap();
    }
}
