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
    pub name: TaggedPtr,
    pub arity: u8,
    pub code: CellPtr<ByteCode>,
}

impl Function {
    pub fn new<'guard>(
        mem: &'guard MutatorView,
        name: TaggedPtr,
        arity: u8,
        code: ScopedPtr<'guard, ByteCode>,
    ) -> Result<ScopedPtr<'guard, Function>, RuntimeError> {
        mem.alloc(Function {
            name,
            arity,
            code: CellPtr::new_with(code),
        })
    }
}

impl Print for Function {
    fn print<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        let name = if let Value::Symbol(s) = TaggedScopedPtr::new(guard, self.name).value() {
            s.as_str(guard)
        } else {
            "anonymous_function"
        };

        write!(f, "Function({}, {})", name, self.arity)
    }
}

/// A partial function application object type
#[derive(Clone)]
pub struct PartialApplication {
    pub arity: u8,
    pub used: u8,
    pub args: CellPtr<List>,
    pub func: CellPtr<Function>,
}

impl Print for PartialApplication {
    fn print<'guard>(
        &self,
        _guard: &'guard dyn MutatorScope,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        write!(f, "PartialApplication({}/{})", self.used, self.arity)
    }
}
