use std::fmt;

use crate::bytecode::ByteCode;
use crate::printer::Print;
use crate::safeptr::{CellPtr, MutatorScope};
use crate::symbol::Symbol;

#[derive(Clone)]
pub struct Function {
    pub name: CellPtr<Symbol>,
    pub arity: u8,
    pub code: CellPtr<ByteCode>,
}

impl Print for Function {
    /// Safe because the lifetime of `MutatorScope` defines a safe-access window
    fn print<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        write!(f, "Function({})", self.name.get(guard).as_str(guard))
    }
}

// TODO
// pub struct Closure
// pub struct Partial
// pub struct Coroutine
