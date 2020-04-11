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
    arity: u8,
    code: CellPtr<ByteCode>,
    param_names: CellPtr<List>,
    // TODO - list of negative indexes into stack where free variable values should be copied from
}

impl Function {
    pub fn alloc<'guard>(
        mem: &'guard MutatorView,
        name: TaggedScopedPtr<'guard>,
        arity: u8,
        code: ScopedPtr<'guard, ByteCode>,
        //param_names: ScopedPtr<'guard, List>,
    ) -> Result<ScopedPtr<'guard, Function>, RuntimeError> {
        mem.alloc(Function {
            name: name.as_unscoped(),
            arity,
            code: CellPtr::new_with(code),
            param_names: CellPtr::new_with(List::alloc(mem)?), //CellPtr::new_with(param_names),
        })
    }

    pub fn name<'guard>(&self, guard: &'guard dyn MutatorScope) -> &'guard str {
        let name = TaggedScopedPtr::new(guard, self.name);
        match *name {
            Value::Symbol(s) => s.as_str(guard),
            _ => "<lambda>",
        }
    }

    pub fn arity(&self) -> u8 {
        self.arity
    }

    pub fn code<'guard>(&self, guard: &'guard dyn MutatorScope) -> ScopedPtr<'guard, ByteCode> {
        self.code.get(guard)
    }

    pub fn param_names<'guard>(&self, guard: &'guard dyn MutatorScope) -> ScopedPtr<'guard, List> {
        self.param_names.get(guard)
    }
}

impl Print for Function {
    fn print<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        let name = TaggedScopedPtr::new(guard, self.name);
        match *name {
            Value::Symbol(s) => write!(f, "(def {} ({}) ...)", s.as_str(guard), self.arity),
            _ => write!(f, "(lambda ({}) ...)", self.arity),
        }
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

/// A list of arguments to apply to functions
pub struct Arguments {
    // TODO
// not sure of the mechanics of this.
// The ghc runtime would push all these to the stack and then consume the stack with
// function applications
}
