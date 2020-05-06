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
        test1: Register,
        test2: Register,
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
        dest: Register,
        arg_count: NumArgs,
    },
    MAKE_CLOSURE {
        dest: Register,
        function: Register,
        function_scope: FrameOffset,
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
            Opcode::JMP { offset: _ } => Opcode::JMP { offset },
            Opcode::JMPT { test, offset: _ } => Opcode::JMPT { test, offset },
            Opcode::JMPNT { test, offset: _ } => Opcode::JMPNT { test, offset },
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
                    RETURN { reg } => RETURN { reg: reg + 1 },
                    LOADLIT { dest, literal_id } => LOADLIT {
                        dest: dest + 1,
                        literal_id,
                    },
                    NIL { dest, test } => NIL {
                        dest: dest + 1,
                        test: test + 1,
                    },
                    ATOM { dest, test } => ATOM {
                        dest: dest + 1,
                        test: test + 1,
                    },
                    CAR { dest, reg } => CAR {
                        dest: dest + 1,
                        reg: reg + 1,
                    },
                    CDR { dest, reg } => CDR {
                        dest: dest + 1,
                        reg: reg + 1,
                    },
                    CONS { dest, reg1, reg2 } => CONS {
                        dest: dest + 1,
                        reg1: reg1 + 1,
                        reg2: reg2 + 1,
                    },
                    IS { dest, test1, test2 } => IS {
                        dest: dest + 1,
                        test1: test1 + 1,
                        test2: test2 + 1,
                    },
                    JMP { offset } => JMP { offset },
                    JMPT { test, offset } => JMPT {
                        test: test + 1,
                        offset,
                    },
                    JMPNT { test, offset } => JMPNT {
                        test: test + 1,
                        offset,
                    },
                    LOADNIL { dest } => LOADNIL { dest: dest + 1 },
                    LOADGLOBAL { dest, name } => LOADGLOBAL {
                        dest: dest + 1,
                        name: name + 1,
                    },
                    STOREGLOBAL { src, name } => STOREGLOBAL {
                        src: src + 1,
                        name: name + 1,
                    },
                    CALL {
                        function,
                        dest,
                        arg_count,
                    } => CALL {
                        function: function + 1,
                        dest: dest + 1,
                        arg_count,
                    },
                    MAKE_CLOSURE {
                        dest,
                        function,
                        function_scope,
                    } => MAKE_CLOSURE {
                        dest: dest + 1,
                        function: function + 1,
                        function_scope,
                    },
                    LOADINT { dest, integer } => LOADINT {
                        dest: dest + 1,
                        integer,
                    },
                    COPYREG { dest, src } => COPYREG {
                        dest: dest + 1,
                        src: src + 1,
                    },
                    LOADNONLOCAL {
                        dest,
                        src,
                        frame_offset,
                    } => LOADNONLOCAL {
                        dest: dest + 1,
                        src: src + 1,
                        frame_offset,
                    },
                    ADD { dest, reg1, reg2 } => ADD {
                        dest: dest + 1,
                        reg1: reg1 + 1,
                        reg2: reg2 + 1,
                    },
                    SUB { dest, left, right } => SUB {
                        dest: dest + 1,
                        left: left + 1,
                        right: right + 1,
                    },
                    MUL { dest, reg1, reg2 } => MUL {
                        dest: dest + 1,
                        reg1: reg1 + 1,
                        reg2: reg2 + 1,
                    },
                    DIVINTEGER { dest, num, denom } => DIVINTEGER {
                        dest: dest + 1,
                        num: num + 1,
                        denom: denom + 1,
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
            code.push(mem, RETURN { reg: 1 })?;
            code.push(
                mem,
                LOADLIT {
                    dest: 1,
                    literal_id: 0,
                },
            )?;
            code.push(mem, NIL { dest: 1, test: 1 })?;
            code.push(mem, ATOM { dest: 1, test: 1 })?;
            code.push(mem, CAR { dest: 1, reg: 1 })?;
            code.push(mem, CDR { dest: 1, reg: 1 })?;
            code.push(
                mem,
                CONS {
                    dest: 1,
                    reg1: 1,
                    reg2: 1,
                },
            )?;
            code.push(
                mem,
                IS {
                    dest: 1,
                    test1: 1,
                    test2: 1,
                },
            )?;
            code.push(mem, JMP { offset: 0 })?;
            code.push(mem, JMPT { test: 1, offset: 0 })?;
            code.push(mem, JMPNT { test: 1, offset: 0 })?;
            code.push(mem, LOADNIL { dest: 1 })?;
            code.push(mem, LOADGLOBAL { dest: 1, name: 1 })?;
            code.push(mem, STOREGLOBAL { src: 1, name: 1 })?;
            code.push(
                mem,
                CALL {
                    function: 1,
                    dest: 1,
                    arg_count: 0,
                },
            )?;
            code.push(
                mem,
                MAKE_CLOSURE {
                    dest: 1,
                    function: 1,
                    function_scope: 0,
                },
            )?;
            code.push(
                mem,
                LOADINT {
                    dest: 1,
                    integer: 0,
                },
            )?;
            code.push(mem, COPYREG { dest: 1, src: 1 })?;
            code.push(
                mem,
                LOADNONLOCAL {
                    dest: 1,
                    src: 1,
                    frame_offset: 0,
                },
            )?;
            code.push(
                mem,
                ADD {
                    dest: 1,
                    reg1: 1,
                    reg2: 1,
                },
            )?;
            code.push(
                mem,
                SUB {
                    dest: 1,
                    left: 1,
                    right: 1,
                },
            )?;
            code.push(
                mem,
                MUL {
                    dest: 1,
                    reg1: 1,
                    reg2: 1,
                },
            )?;
            code.push(
                mem,
                DIVINTEGER {
                    dest: 1,
                    num: 1,
                    denom: 1,
                },
            )?;

            code.increment_all_registers(mem)?;

            code.code.access_slice(mem, |code| {
                for opcode in code {
                    match *opcode {
                        NoOp => (),
                        RETURN { reg } => assert!(reg == 2),
                        LOADLIT { dest, literal_id } => {
                            assert!(dest == 2);
                            assert!(literal_id == 0);
                        }
                        NIL { dest, test } => {
                            assert!(dest == 2);
                            assert!(test == 2);
                        }
                        ATOM { dest, test } => {
                            assert!(dest == 2);
                            assert!(test == 2);
                        }
                        CAR { dest, reg } => {
                            assert!(dest == 2);
                            assert!(reg == 2);
                        }
                        CDR { dest, reg } => {
                            assert!(dest == 2);
                            assert!(reg == 2);
                        }
                        CONS { dest, reg1, reg2 } => {
                            assert!(dest == 2);
                            assert!(reg1 == 2);
                            assert!(reg2 == 2);
                        }
                        IS { dest, test1, test2 } => {
                            assert!(dest == 2);
                            assert!(test1 == 2);
                            assert!(test2 == 2);
                        }
                        JMP { offset } => assert!(offset == 0),
                        JMPT { test, offset } => {
                            assert!(test == 2);
                            assert!(offset == 0);
                        }
                        JMPNT { test, offset } => {
                            assert!(test == 2);
                            assert!(offset == 0);
                        }
                        LOADNIL { dest } => assert!(dest == 2),
                        LOADGLOBAL { dest, name } => {
                            assert!(dest == 2);
                            assert!(name == 2);
                        }
                        STOREGLOBAL { src, name } => {
                            assert!(src == 2);
                            assert!(name == 2);
                        }
                        CALL {
                            function,
                            dest,
                            arg_count,
                        } => {
                            assert!(function == 2);
                            assert!(dest == 2);
                            assert!(arg_count == 0);
                        }
                        MAKE_CLOSURE {
                            dest,
                            function,
                            function_scope,
                        } => {
                            assert!(dest == 2);
                            assert!(function == 2);
                            assert!(function_scope == 0);
                        }
                        LOADINT { dest, integer } => {
                            assert!(dest == 2);
                            assert!(integer == 0);
                        }
                        COPYREG { dest, src } => {
                            assert!(dest == 2);
                            assert!(src == 2);
                        }
                        LOADNONLOCAL {
                            dest,
                            src,
                            frame_offset,
                        } => {
                            assert!(dest == 2);
                            assert!(src == 2);
                            assert!(frame_offset == 0);
                        }
                        ADD { dest, reg1, reg2 } => {
                            assert!(dest == 2);
                            assert!(reg1 == 2);
                            assert!(reg2 == 2);
                        }
                        SUB { dest, left, right } => {
                            assert!(dest == 2);
                            assert!(left == 2);
                            assert!(right == 2);
                        }
                        MUL { dest, reg1, reg2 } => {
                            assert!(dest == 2);
                            assert!(reg1 == 2);
                            assert!(reg2 == 2);
                        }
                        DIVINTEGER { dest, num, denom } => {
                            assert!(dest == 2);
                            assert!(num == 2);
                            assert!(denom == 2);
                        }
                    }
                }
            });

            Ok(())
        }

        test_helper(test_inner);
    }
}
