use crate::array::Array;
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
        Ok(VM {
            stack: ArrayAny::with_capacity(mem, 256)?,
        })
    }
}

/// Mutator that implements the VM
struct VM {
    stack: ArrayAny,
}

impl Mutator for VM {
    type Input = ();
    type Output = ();

    fn run(&self, mem: &MutatorView, _: ()) -> Result<(), RuntimeError> {
        Ok(())
    }
}
