use stickyimmix::ArraySize;

use crate::bytecode::{ByteCode, InstructionStream, Opcode};
use crate::containers::{Container, IndexedAnyContainer, StackAnyContainer};
use crate::error::RuntimeError;
use crate::memory::{Mutator, MutatorView};
use crate::primitives::ArrayAny;
use crate::safeptr::{ScopedPtr, TaggedScopedPtr};
use crate::taggedptr::Value;

/// Control flow flags
pub enum EvalStatus<'guard> {
    Pending,
    Return(TaggedScopedPtr<'guard>),
    Halt
}

/// Execute the next instruction and return
fn eval_instr<'guard>(
    mem: &'guard MutatorView,
    stack: ScopedPtr<'guard, ArrayAny>,
    instr: ScopedPtr<'guard, InstructionStream>
) -> Result<EvalStatus<'guard>, RuntimeError> {
    let opcode = instr.get_next_opcode(mem)?;

    match opcode {
        Opcode::HALT => return Ok(EvalStatus::Halt),

        Opcode::RETURN => {
            let reg = instr.get_reg_acc() as ArraySize;
            return Ok(EvalStatus::Return(stack.get(mem, reg)?))
        },

        Opcode::LOADLIT => unimplemented!(),

        Opcode::NIL => {
            let acc = instr.get_reg_acc() as ArraySize;
            let reg1 = instr.get_reg1() as ArraySize;

            let reg1_val = stack.get(mem, reg1)?;

            match *reg1_val {
                Value::Nil => stack.set(mem, acc, mem.lookup_sym("true"))?,
                _ => stack.set(mem, acc, mem.nil())?
            }
        },

        Opcode::ATOM => {
            let acc = instr.get_reg_acc() as ArraySize;
            let reg1 = instr.get_reg1() as ArraySize;

            let reg1_val = stack.get(mem, reg1)?;

            match *reg1_val {
                Value::Nil => stack.set(mem, acc, mem.nil())?,
                Value::Pair(_) => stack.set(mem, acc, mem.nil())?,
                _ => stack.set(mem, acc, mem.lookup_sym("true"))?
            }
        },

        Opcode::CAR => unimplemented!(),
        Opcode::CDR => unimplemented!(),
        Opcode::CONS => unimplemented!(),
        Opcode::EQ => unimplemented!(),
        Opcode::JMPT => unimplemented!(),
        Opcode::JMP => unimplemented!(),
    }

    Ok(EvalStatus::Pending)
}

/// Given an InstructionStream, execute up to max_instr more instructions
pub fn vm_eval_stream<'guard>(
    mem: &'guard MutatorView,
    stack: ScopedPtr<'guard, ArrayAny>,
    instr: ScopedPtr<'guard, InstructionStream>,
    max_instr: ArraySize,
) -> Result<EvalStatus<'guard>, RuntimeError> {
    for _ in 0..max_instr {
        match eval_instr(mem, stack, instr)? {
            EvalStatus::Return(value) => return Ok(EvalStatus::Return(value)),
            EvalStatus::Halt => return Ok(EvalStatus::Halt),
            _ => ()
        }
    }
    Ok(EvalStatus::Pending)
}

/// Evaluate a whole block of byte code
pub fn vm_eval<'guard>(
    mem: &'guard MutatorView,
    code: ScopedPtr<'_, ByteCode>,
) -> Result<(), RuntimeError> {
    Ok(())
}

/// Mutator that instantiates a VM
struct VMFactory {}

impl Mutator for VMFactory {
    type Input = ();
    type Output = VM;

    fn run(&self, mem: &MutatorView, _: Self::Input) -> Result<VM, RuntimeError> {
        // initialize stack to 256 nil registers
        let stack = ArrayAny::with_capacity(mem, 256)?;
        for index in 0..256 {
            stack.set(mem, index, mem.nil())?;
        }

        Ok(VM { stack: stack })
    }
}

/// Mutator that implements the VM
struct VM {
    stack: ArrayAny,
}

impl Mutator for VM {
    type Input = ByteCode;
    type Output = ();

    fn run(&self, mem: &MutatorView, code: Self::Input) -> Result<Self::Output, RuntimeError> {
        Ok(())
    }
}
