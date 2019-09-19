use crate::bytecode::ByteCode;
use crate::containers::{Container, IndexedAnyContainer, StackAnyContainer};
use crate::error::RuntimeError;
use crate::memory::{Mutator, MutatorView};
use crate::primitives::ArrayAny;

/// Mutator that instantiates a VM
struct VMFactory {}

impl Mutator for VMFactory {
    type Input = ();
    type Output = VM;

    fn run(&self, mem: &MutatorView, _: ()) -> Result<VM, RuntimeError> {
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

    fn run(&self, mem: &MutatorView, code: ByteCode) -> Result<(), RuntimeError> {
        Ok(())
    }
}
