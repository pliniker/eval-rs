use crate::containers::{Container, IndexedContainer, StackContainer};
use crate::error::{ErrorKind, RuntimeError};
use crate::memory::MutatorView;
use crate::primitives::ArrayU8;
use crate::safeptr::ScopedPtr;

type ByteCode = ArrayU8;

/// Compile the given AST and return a bytecode structure
pub fn compile<'guard>(
    mem: &'guard MutatorView,
    ast: ScopedPtr<'guard>,
) -> Result<ScopedPtr<'guard>, RuntimeError> {
    // depth-first tree traversal, flattening the output

    let bytecode = ByteCode::with_capacity(mem, 256)?;

    mem.alloc(bytecode)
}
