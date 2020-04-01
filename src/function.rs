use std::fmt;

use crate::bytecode::ByteCode;
use crate::error::RuntimeError;
use crate::list::List;
use crate::memory::MutatorView;
use crate::printer::Print;
use crate::safeptr::{CellPtr, MutatorScope, ScopedPtr, TaggedScopedPtr};
use crate::taggedptr::{TaggedPtr, Value};

/// A function object type
#[derive(Clone)]
pub struct Function {
    // name could be a Symbol, or nil if it is an anonymous fn
    name: TaggedPtr,
    pub arity: u8,
    pub code: CellPtr<ByteCode>,
}

impl Function {
    pub fn new<'guard>(
        mem: &'guard MutatorView,
        name: TaggedScopedPtr<'guard>,
        arity: u8,
        code: ScopedPtr<'guard, ByteCode>,
    ) -> Result<ScopedPtr<'guard, Function>, RuntimeError> {
        mem.alloc(Function {
            name: name.as_unscoped(),
            arity,
            code: CellPtr::new_with(code),
        })
    }

    pub fn name<'guard>(&self, guard: &'guard dyn MutatorScope) -> &'guard str {
        let name = TaggedScopedPtr::new(guard, self.name);
        match *name {
            Value::Symbol(s) => s.as_str(guard),
            _ => "<lambda>",
        }
    }
}

impl Print for Function {
    fn print<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        write!(f, "Function({}, {})", self.name(guard), self.arity)
    }
}

/// A partial function application object type
#[derive(Clone)]
pub struct Partial {
    pub arity: u8,
    pub used: u8,
    pub args: CellPtr<List>,
    pub func: CellPtr<Function>,
}

impl Print for Partial {
    fn print<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        let function = self.func.get(guard);
        write!(
            f,
            "Partial({}, {}/{})",
            function.name(guard),
            self.used,
            self.arity
        )
    }
}
