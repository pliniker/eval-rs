use itertools::join;
use std::cell::Cell;
use std::fmt;

use crate::array::{Array, ArraySize};
use crate::containers::{
    Container, IndexedContainer, SliceableContainer, StackAnyContainer, StackContainer,
};
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
pub const JUMP_UNKNOWN: i16 = 0x7fff;

/// Argument count for a function call or partial application
pub type NumArgs = u8;

/// Count of call frames to look back to find a nonlocal
pub type FrameOffset = u8;

/// VM opcodes. These enum variants should be designed to fit into 32 bits. Using
/// u8 representation seems to make that happen, so long as the struct variants
/// do not add up to more than 24 bits.
/// Defining opcodes like this rather than using u32 directly comes with tradeoffs.
/// Direct u32 is more ergonomic for the compiler but enum struct variants is
/// more ergonomic for the vm and probably more performant. Lots of match repetition
/// though :(
#[repr(u8)]
#[derive(PartialEq, Debug, Clone, Copy)]
pub enum Opcode {
    NoOp,
    Return {
        reg: Register,
    },
    LoadLiteral {
        dest: Register,
        literal_id: LiteralId,
    },
    IsNil {
        dest: Register,
        test: Register,
    },
    IsAtom {
        dest: Register,
        test: Register,
    },
    FirstOfPair {
        dest: Register,
        reg: Register,
    },
    SecondOfPair {
        dest: Register,
        reg: Register,
    },
    MakePair {
        dest: Register,
        reg1: Register,
        reg2: Register,
    },
    IsIdentical {
        dest: Register,
        test1: Register,
        test2: Register,
    },
    Jump {
        offset: JumpOffset,
    },
    JumpIfTrue {
        test: Register,
        offset: JumpOffset,
    },
    JumpIfNotTrue {
        test: Register,
        offset: JumpOffset,
    },
    LoadNil {
        dest: Register,
    },
    LoadGlobal {
        dest: Register,
        name: Register,
    },
    StoreGlobal {
        src: Register,
        name: Register,
    },
    Call {
        function: Register,
        dest: Register,
        arg_count: NumArgs,
    },
    MakeClosure {
        dest: Register,
        function: Register,
    },
    LoadInteger {
        dest: Register,
        integer: LiteralInteger,
    },
    CopyRegister {
        dest: Register,
        src: Register,
    },
    LoadNonLocal {
        dest: Register,
        src: Register,
        frame_offset: FrameOffset,
    },
    Add {
        dest: Register,
        reg1: Register,
        reg2: Register,
    },
    Subtract {
        dest: Register,
        left: Register,
        right: Register,
    },
    Multiply {
        dest: Register,
        reg1: Register,
        reg2: Register,
    },
    DivideInteger {
        dest: Register,
        num: Register,
        denom: Register,
    },
    GetUpvalue {
        dest: Register,
        src: Register,
    },
    SetUpvalue {
        dest: Register,
        src: Register,
    },
    CloseUpvalues {
        reg1: Register,
        reg2: Register,
        reg3: Register,
    },
}

/// Bytecode is stored as fixed-width 32-bit values.
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
        offset: JumpOffset,
    ) -> Result<(), RuntimeError> {
        let code = self.code.get(mem, instruction)?;
        let new_code = match code {
            Opcode::Jump { offset: _ } => Opcode::Jump { offset },
            Opcode::JumpIfTrue { test, offset: _ } => Opcode::JumpIfTrue { test, offset },
            Opcode::JumpIfNotTrue { test, offset: _ } => Opcode::JumpIfNotTrue { test, offset },
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
        self.code
            .push(mem, Opcode::LoadLiteral { dest, literal_id })
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

    /// To construct a closure, a lambda is lifted so that all free variables are converted into
    /// function parameters. All existing parameter and evaluation registers must be punted
    /// further back into the register window to make space. This function iterates over all
    /// instructions, incrementing each register by 1, to make space for 1 free variable param
    /// at the head of the register window.
    pub fn increment_all_registers<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
    ) -> Result<(), RuntimeError> {
        use Opcode::*;
        self.code.access_slice(guard, |code| {
            for opcode in code {
                *opcode = match *opcode {
                    NoOp => NoOp,
                    Return { reg } => Return { reg: reg + 1 },
                    LoadLiteral { dest, literal_id } => LoadLiteral {
                        dest: dest + 1,
                        literal_id,
                    },
                    IsNil { dest, test } => IsNil {
                        dest: dest + 1,
                        test: test + 1,
                    },
                    IsAtom { dest, test } => IsAtom {
                        dest: dest + 1,
                        test: test + 1,
                    },
                    FirstOfPair { dest, reg } => FirstOfPair {
                        dest: dest + 1,
                        reg: reg + 1,
                    },
                    SecondOfPair { dest, reg } => SecondOfPair {
                        dest: dest + 1,
                        reg: reg + 1,
                    },
                    MakePair { dest, reg1, reg2 } => MakePair {
                        dest: dest + 1,
                        reg1: reg1 + 1,
                        reg2: reg2 + 1,
                    },
                    IsIdentical { dest, test1, test2 } => IsIdentical {
                        dest: dest + 1,
                        test1: test1 + 1,
                        test2: test2 + 1,
                    },
                    Jump { offset } => Jump { offset },
                    JumpIfTrue { test, offset } => JumpIfTrue {
                        test: test + 1,
                        offset,
                    },
                    JumpIfNotTrue { test, offset } => JumpIfNotTrue {
                        test: test + 1,
                        offset,
                    },
                    LoadNil { dest } => LoadNil { dest: dest + 1 },
                    LoadGlobal { dest, name } => LoadGlobal {
                        dest: dest + 1,
                        name: name + 1,
                    },
                    StoreGlobal { src, name } => StoreGlobal {
                        src: src + 1,
                        name: name + 1,
                    },
                    Call {
                        function,
                        dest,
                        arg_count,
                    } => Call {
                        function: function + 1,
                        dest: dest + 1,
                        arg_count,
                    },
                    MakeClosure { dest, function } => MakeClosure {
                        dest: dest + 1,
                        function: function + 1,
                    },
                    LoadInteger { dest, integer } => LoadInteger {
                        dest: dest + 1,
                        integer,
                    },
                    CopyRegister { dest, src } => CopyRegister {
                        dest: dest + 1,
                        src: src + 1,
                    },
                    LoadNonLocal {
                        dest,
                        src,
                        frame_offset,
                    } => LoadNonLocal {
                        dest: dest + 1,
                        src: src + 1,
                        frame_offset,
                    },
                    Add { dest, reg1, reg2 } => Add {
                        dest: dest + 1,
                        reg1: reg1 + 1,
                        reg2: reg2 + 1,
                    },
                    Subtract { dest, left, right } => Subtract {
                        dest: dest + 1,
                        left: left + 1,
                        right: right + 1,
                    },
                    Multiply { dest, reg1, reg2 } => Multiply {
                        dest: dest + 1,
                        reg1: reg1 + 1,
                        reg2: reg2 + 1,
                    },
                    DivideInteger { dest, num, denom } => DivideInteger {
                        dest: dest + 1,
                        num: num + 1,
                        denom: denom + 1,
                    },
                    GetUpvalue { dest, src } => GetUpvalue {
                        dest: dest + 1,
                        src: src + 1,
                    },
                    SetUpvalue { dest, src } => SetUpvalue {
                        dest: dest + 1,
                        src: src + 1,
                    },
                    // if an upvalue register is 0, it doesn't point to an upvalue
                    CloseUpvalues { reg1, reg2, reg3 } => CloseUpvalues {
                        reg1: if reg1 == 0 { 0 } else { reg1 + 1 },
                        reg2: if reg2 == 0 { 0 } else { reg2 + 1 },
                        reg3: if reg3 == 0 { 0 } else { reg3 + 1 },
                    },
                }
            }
        });

        Ok(())
    }
}

impl Print for ByteCode {
    fn print<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        let mut instr_str = String::new();

        self.code.access_slice(guard, |code| {
            instr_str = join(code.iter().map(|opcode| format!("{:?}", opcode)), "\n")
        });

        write!(f, "{}", instr_str)
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
    use crate::memory::{Memory, Mutator};
    use std::mem::size_of;

    // TODO - create common module for test utilities
    fn test_helper(test_fn: fn(&MutatorView) -> Result<(), RuntimeError>) {
        let mem = Memory::new();

        struct Test {}
        impl Mutator for Test {
            type Input = fn(&MutatorView) -> Result<(), RuntimeError>;
            type Output = ();

            fn run(
                &self,
                mem: &MutatorView,
                test_fn: Self::Input,
            ) -> Result<Self::Output, RuntimeError> {
                test_fn(mem)
            }
        }

        let test = Test {};
        mem.mutate(&test, test_fn).unwrap();
    }

    #[test]
    fn test_opcode_is_32_bits() {
        // An Opcode should be 32 bits; anything bigger and we've mis-defined some
        // discriminant
        assert!(size_of::<Opcode>() == 4);
    }

    #[test]
    fn test_increment_all_registers() {
        fn test_inner(mem: &MutatorView) -> Result<(), RuntimeError> {
            use Opcode::*;

            let code = ByteCode::alloc(mem)?;
            code.push(mem, NoOp)?;
            code.push(mem, Return { reg: 1 })?;
            code.push(
                mem,
                LoadLiteral {
                    dest: 1,
                    literal_id: 0,
                },
            )?;
            code.push(mem, IsNil { dest: 1, test: 1 })?;
            code.push(mem, IsAtom { dest: 1, test: 1 })?;
            code.push(mem, FirstOfPair { dest: 1, reg: 1 })?;
            code.push(mem, SecondOfPair { dest: 1, reg: 1 })?;
            code.push(
                mem,
                MakePair {
                    dest: 1,
                    reg1: 1,
                    reg2: 1,
                },
            )?;
            code.push(
                mem,
                IsIdentical {
                    dest: 1,
                    test1: 1,
                    test2: 1,
                },
            )?;
            code.push(mem, Jump { offset: 0 })?;
            code.push(mem, JumpIfTrue { test: 1, offset: 0 })?;
            code.push(mem, JumpIfNotTrue { test: 1, offset: 0 })?;
            code.push(mem, LoadNil { dest: 1 })?;
            code.push(mem, LoadGlobal { dest: 1, name: 1 })?;
            code.push(mem, StoreGlobal { src: 1, name: 1 })?;
            code.push(
                mem,
                Call {
                    function: 1,
                    dest: 1,
                    arg_count: 0,
                },
            )?;
            code.push(
                mem,
                MakeClosure {
                    dest: 1,
                    function: 1,
                },
            )?;
            code.push(
                mem,
                LoadInteger {
                    dest: 1,
                    integer: 0,
                },
            )?;
            code.push(mem, CopyRegister { dest: 1, src: 1 })?;
            code.push(
                mem,
                LoadNonLocal {
                    dest: 1,
                    src: 1,
                    frame_offset: 0,
                },
            )?;
            code.push(
                mem,
                Add {
                    dest: 1,
                    reg1: 1,
                    reg2: 1,
                },
            )?;
            code.push(
                mem,
                Subtract {
                    dest: 1,
                    left: 1,
                    right: 1,
                },
            )?;
            code.push(
                mem,
                Multiply {
                    dest: 1,
                    reg1: 1,
                    reg2: 1,
                },
            )?;
            code.push(
                mem,
                DivideInteger {
                    dest: 1,
                    num: 1,
                    denom: 1,
                },
            )?;
            code.push(mem, GetUpvalue { dest: 1, src: 1 })?;
            code.push(mem, SetUpvalue { dest: 1, src: 1 })?;
            code.push(
                mem,
                CloseUpvalues {
                    reg1: 1,
                    reg2: 1,
                    reg3: 0,
                },
            )?;

            code.increment_all_registers(mem)?;

            code.code.access_slice(mem, |code| {
                for opcode in code {
                    match *opcode {
                        NoOp => (),
                        Return { reg } => assert!(reg == 2),
                        LoadLiteral { dest, literal_id } => {
                            assert!(dest == 2);
                            assert!(literal_id == 0);
                        }
                        IsNil { dest, test } => {
                            assert!(dest == 2);
                            assert!(test == 2);
                        }
                        IsAtom { dest, test } => {
                            assert!(dest == 2);
                            assert!(test == 2);
                        }
                        FirstOfPair { dest, reg } => {
                            assert!(dest == 2);
                            assert!(reg == 2);
                        }
                        SecondOfPair { dest, reg } => {
                            assert!(dest == 2);
                            assert!(reg == 2);
                        }
                        MakePair { dest, reg1, reg2 } => {
                            assert!(dest == 2);
                            assert!(reg1 == 2);
                            assert!(reg2 == 2);
                        }
                        IsIdentical { dest, test1, test2 } => {
                            assert!(dest == 2);
                            assert!(test1 == 2);
                            assert!(test2 == 2);
                        }
                        Jump { offset } => assert!(offset == 0),
                        JumpIfTrue { test, offset } => {
                            assert!(test == 2);
                            assert!(offset == 0);
                        }
                        JumpIfNotTrue { test, offset } => {
                            assert!(test == 2);
                            assert!(offset == 0);
                        }
                        LoadNil { dest } => assert!(dest == 2),
                        LoadGlobal { dest, name } => {
                            assert!(dest == 2);
                            assert!(name == 2);
                        }
                        StoreGlobal { src, name } => {
                            assert!(src == 2);
                            assert!(name == 2);
                        }
                        Call {
                            function,
                            dest,
                            arg_count,
                        } => {
                            assert!(function == 2);
                            assert!(dest == 2);
                            assert!(arg_count == 0);
                        }
                        MakeClosure { dest, function } => {
                            assert!(dest == 2);
                            assert!(function == 2);
                        }
                        LoadInteger { dest, integer } => {
                            assert!(dest == 2);
                            assert!(integer == 0);
                        }
                        CopyRegister { dest, src } => {
                            assert!(dest == 2);
                            assert!(src == 2);
                        }
                        // TODO deprecate
                        LoadNonLocal {
                            dest,
                            src,
                            frame_offset,
                        } => {
                            assert!(dest == 2);
                            assert!(src == 2);
                            assert!(frame_offset == 0);
                        }
                        Add { dest, reg1, reg2 } => {
                            assert!(dest == 2);
                            assert!(reg1 == 2);
                            assert!(reg2 == 2);
                        }
                        Subtract { dest, left, right } => {
                            assert!(dest == 2);
                            assert!(left == 2);
                            assert!(right == 2);
                        }
                        Multiply { dest, reg1, reg2 } => {
                            assert!(dest == 2);
                            assert!(reg1 == 2);
                            assert!(reg2 == 2);
                        }
                        DivideInteger { dest, num, denom } => {
                            assert!(dest == 2);
                            assert!(num == 2);
                            assert!(denom == 2);
                        }
                        GetUpvalue { dest, src } => {
                            assert!(dest == 2);
                            assert!(src == 2);
                        }
                        SetUpvalue { dest, src } => {
                            assert!(dest == 2);
                            assert!(src == 2);
                        }
                        CloseUpvalues { reg1, reg2, reg3 } => {
                            assert!(reg1 == 2);
                            assert!(reg2 == 2);
                            assert!(reg3 == 0);
                        }
                    }
                }
            });

            Ok(())
        }

        test_helper(test_inner);
    }
}
