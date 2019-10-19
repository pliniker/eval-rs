use stickyimmix::ArraySize;

use crate::bytecode::{ByteCode, InstructionStream, Opcode};
use crate::containers::{Container, IndexedAnyContainer, StackAnyContainer};
use crate::error::RuntimeError;
use crate::memory::{Mutator, MutatorView};
use crate::primitives::ArrayAny;
use crate::safeptr::ScopedPtr;


fn eval_instr<'guard>(
    mem: &'guard MutatorView,
    opcode: Opcode,
    instr: ScopedPtr<'guard, InstructionStream>
) -> Result<(), RuntimeError> {
    match opcode {
        Opcode::HALT => unimplemented!(),
        Opcode::RETURN => unimplemented!(),
        Opcode::LOADLIT => unimplemented!(),
        Opcode::NIL => unimplemented!(),
        Opcode::ATOM => unimplemented!(),
        Opcode::CAR => unimplemented!(),
        Opcode::CDR => unimplemented!(),
        Opcode::CONS => unimplemented!(),
        Opcode::EQ => unimplemented!(),
        Opcode::JMPT => unimplemented!(),
        Opcode::JMP => unimplemented!(),
    };
    Ok(())
}

/// Given an InstructionStream, execute up to max_instr more instructions
pub fn vm_eval_stream<'guard>(
    mem: &'guard MutatorView,
    instr: ScopedPtr<'guard, InstructionStream>,
    max_instr: ArraySize,
) -> Result<(), RuntimeError> {
    for _ in 0..max_instr {
        let opcode = instr.get_next_opcode(mem)?;
        eval_instr(mem, opcode, instr)?;
    }
    Ok(())
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
