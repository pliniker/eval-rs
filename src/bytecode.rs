use std::cell::Cell;
use std::fmt;

use crate::array::{Array, ArraySize};
use crate::containers::{Container, IndexedContainer, StackAnyContainer, StackContainer};
use crate::error::{err_eval, RuntimeError};
use crate::list::List;
use crate::memory::MutatorView;
use crate::printer::Print;
use crate::safeptr::{CellPtr, MutatorScope, ScopedPtr, TaggedScopedPtr};
use crate::taggedptr::TaggedPtr;

/// A register can be in the range 0..255
pub type Register = u8;

/// A literal integer that can be baked into an opcode can be in the range -32768..32767
pub type LiteralInteger = i16;

/// Literals are stored in a list, a LiteralId describes the index of the value in the list
pub type LiteralId = u16;

/// An instruction jump target is a signed integer, relative to the jump instruction
pub type JumpOffset = i16;
/// Jump offset when the target is still unknown.
pub const JumpUnknown: i16 = 0x7fff;

/// Argument count for a function call or partial application
pub type NumArgs = u8;

/// Count of call frames to look back to find a nonlocal
pub type FrameOffset = u8;

/// VM opcodes. These enum variants should be designed to fit into 32 bits. Using
/// u8 representation seems to make that happen, so long as the struct variants
/// do not add up to more than 24 bits
#[repr(u8)]
#[derive(PartialEq, Debug, Clone, Copy)]
pub enum Opcode {
    NOP,
    RETURN {
        reg: Register,
    },
    LOADLIT {
        dest: Register,
        literal_id: LiteralId,
    },
    NIL {
        dest: Register,
        test: Register,
    },
    ATOM {
        dest: Register,
        test: Register,
    },
    CAR {
        dest: Register,
        reg: Register,
    },
    CDR {
        dest: Register,
        reg: Register,
    },
    CONS {
        dest: Register,
        reg1: Register,
        reg2: Register,
    },
    IS {
        dest: Register,
        test: Register,
    },
    JMP {
        offset: JumpOffset,
    },
    JMPT {
        test: Register,
        offset: JumpOffset,
    },
    JMPNT {
        test: Register,
        offset: JumpOffset,
    },
    LOADNIL {
        dest: Register,
    },
    LOADGLOBAL {
        dest: Register,
        name: Register,
    },
    STOREGLOBAL {
        src: Register,
        name: Register,
    },
    CALL {
        function: Register,
        return_reg: Register,
        nargs: NumArgs,
    },
    LOADINT {
        dest: Register,
        integer: LiteralInteger,
    },
    COPYREG {
        dest: Register,
        src: Register,
    },
    LOADNONLOCAL {
        dest: Register,
        src: Register,
        frame_offset: FrameOffset,
    },
    ADD {
        dest: Register,
        reg1: Register,
        reg2: Register,
    },
    SUB {
        dest: Register,
        left: Register,
        right: Register,
    },
    MUL {
        dest: Register,
        reg1: Register,
        reg2: Register,
    },
    DIVINTEGER {
        dest: Register,
        num: Register,
        denom: Register,
    },
}

/// Bytecode is stored as fixed-width 32-bit operator+operand values.
/// This is not the most efficient format but it is easy to work with.
pub type ArrayOpcode = Array<Opcode>;

/// Literals are stored in a separate list of machine-word-width pointers.
/// This is also not the most efficient scheme but it is easy to work with.
pub type Literals = List;

/// Byte code consists of the code and any literals used.
#[derive(Clone)]
pub struct ByteCode {
    code: ArrayOpcode,
    literals: Literals,
}

impl ByteCode {
    /// Instantiate a blank ByteCode instance
    pub fn alloc<'guard>(
        mem: &'guard MutatorView,
    ) -> Result<ScopedPtr<'guard, ByteCode>, RuntimeError> {
        mem.alloc(ByteCode {
            code: ArrayOpcode::new(),
            literals: Literals::new(),
        })
    }

    /// Push an instuction to the back of the sequence
    pub fn push<'guard>(&self, mem: &'guard MutatorView, op: Opcode) -> Result<(), RuntimeError> {
        self.code.push(mem, op)
    }

    /// Set the jump offset of a jump instruction to a new value
    pub fn update_jump_offset<'guard>(
        &self,
        mem: &'guard MutatorView,
        instruction: ArraySize,
        new_offset: JumpOffset,
    ) -> Result<(), RuntimeError> {
        let code = self.code.get(mem, instruction)?;
        let new_code = match code {
            Opcode::JMP { offset } => Opcode::JMP { offset: new_offset },
            Opcode::JMPT { test, offset } => Opcode::JMPT {
                test,
                offset: new_offset,
            },
            Opcode::JMPNT { test, offset } => Opcode::JMPNT {
                test,
                offset: new_offset,
            },
            _ => {
                return Err(err_eval(
                    "Cannot modify jump offset for non-jump instruction",
                ))
            }
        };
        self.code.set(mem, instruction, new_code)?;
        Ok(())
    }

    /// Push a literal-load operation to the back of the sequence
    pub fn push_loadlit<'guard>(
        &self,
        mem: &'guard MutatorView,
        dest: Register,
        literal_id: LiteralId,
    ) -> Result<(), RuntimeError> {
        // TODO clone anything mutable
        self.code.push(mem, Opcode::LOADLIT { dest, literal_id })
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
            match instr {
                _ => (), // TODO
            }
        }
        Ok(())
    }
}

/// Interpret a ByteCode as a stream of instructions, handling an instruction-pointer abstraction.
pub struct InstructionStream {
    instructions: CellPtr<ByteCode>,
    ip: Cell<ArraySize>,
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
        Ok(instr)
    }

    /// Retrieve the literal pointer from the current instruction
    pub fn get_literal<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        lit_id: LiteralId,
    ) -> Result<TaggedPtr, RuntimeError> {
        Ok(IndexedContainer::get(
            &self.instructions.get(guard).literals,
            guard,
            lit_id as ArraySize,
        )?
        .get_ptr())
    }

    /// Return the next instruction pointer
    pub fn get_next_ip(&self) -> ArraySize {
        self.ip.get()
    }

    /// Adjust the instruction pointer by the given signed offset from the current ip
    pub fn jump(&self, offset: JumpOffset) {
        let mut ip = self.ip.get() as i32;
        ip += offset as i32;
        self.ip.set(ip as ArraySize);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn test_opcode_is_32_bits() {
        // An Opcode should be 32 bits; anything bigger and we've mis-defined some
        // discriminant
        assert!(size_of::<Opcode>() == 4);
    }
}
