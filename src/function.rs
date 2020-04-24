use itertools::join;
use std::fmt;

use crate::bytecode::ByteCode;
use crate::containers::{Container, IndexedAnyContainer, SliceableContainer};
use crate::error::RuntimeError;
use crate::list::List;
use crate::memory::MutatorView;
use crate::printer::Print;
use crate::safeptr::{CellPtr, MutatorScope, ScopedPtr, TaggedCellPtr, TaggedScopedPtr};
use crate::taggedptr::Value;

/// A function object type
#[derive(Clone)]
pub struct Function {
    // name could be a Symbol, or nil if it is an anonymous fn
    name: TaggedCellPtr,
    arity: u8,
    code: CellPtr<ByteCode>,
    param_names: CellPtr<List>,
    // TODO - list of negative indexes into stack where free variable values should be copied from
    // free_variables: CellPtr<ArrayU32> <- but signed integers
}

impl Function {
    pub fn alloc<'guard>(
        mem: &'guard MutatorView,
        name: TaggedScopedPtr<'guard>,
        param_names: ScopedPtr<'guard, List>,
        code: ScopedPtr<'guard, ByteCode>,
        //free_variables
    ) -> Result<ScopedPtr<'guard, Function>, RuntimeError> {
        mem.alloc(Function {
            name: TaggedCellPtr::new_with(name),
            arity: param_names.length() as u8,
            code: CellPtr::new_with(code),
            param_names: CellPtr::new_with(param_names),
        })
    }

    pub fn name<'guard>(&self, guard: &'guard dyn MutatorScope) -> &'guard str {
        let name = self.name.get(guard);
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
        let name = self.name.get(guard);
        let params = self.param_names.get(guard);

        let mut param_string = String::new();
        params.access_slice(guard, |items| {
            param_string = join(items.iter().map(|item| item.get(guard)), " ")
        });

        match *name {
            Value::Symbol(s) => write!(f, "(def {} ({}) ...)", s.as_str(guard), param_string),
            _ => write!(f, "(lambda ({}) ...)", param_string),
        }
    }
}

/// A partial function application object type
#[derive(Clone)]
pub struct Partial {
    arity: u8,
    used: u8,
    pub args: CellPtr<List>,
    pub func: CellPtr<Function>,
}

impl Partial {
    pub fn alloc<'guard>(
        mem: &'guard MutatorView,
        function: ScopedPtr<'guard, Function>,
        args: ScopedPtr<'guard, List>,
    ) -> Result<ScopedPtr<'guard, Partial>, RuntimeError> {
        let used = args.length() as u8;
        let arity = function.arity() - used;
        mem.alloc(Partial {
            arity: arity,
            used: used,
            args: CellPtr::new_with(args),
            func: CellPtr::new_with(function),
        })
    }

    pub fn arity(&self) -> u8 {
        self.arity
    }
}

impl Print for Partial {
    fn print<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        let function = self.func.get(guard);
        let name = function.name.get(guard);
        let params = function.param_names.get(guard);

        let mut param_string = String::new();
        params.access_slice(guard, |items| {
            let start = self.used as usize;
            param_string = join(items[start..].iter().map(|item| item.get(guard)), " ")
        });

        match *name {
            Value::Symbol(s) => write!(f, "(partial {} ({}) ...)", s.as_str(guard), param_string),
            _ => write!(f, "(partial ({}) ...)", param_string),
        }
    }
}

/// A list of arguments to apply to functions
pub struct CurriedArguments {
    // TODO
// not sure of the mechanics of this.
// The ghc runtime would push all these to the stack and then consume the stack with
// function continuations
}
