use itertools::join;
use std::fmt;

use crate::array::ArrayU8;
use crate::bytecode::ByteCode;
use crate::containers::{Container, ContainerFromSlice, SliceableContainer, StackContainer};
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
    // Param names are stored for examination of a function signature
    param_names: CellPtr<List>,
    // TODO - list of negative indexes into stack where free variable values should be copied from
    nonlocal_refs: CellPtr<ArrayU8>,
}

impl Function {
    /// Allocate a Function object on the heap
    pub fn alloc<'guard>(
        mem: &'guard MutatorView,
        name: TaggedScopedPtr<'guard>,
        param_names: ScopedPtr<'guard, List>,
        code: ScopedPtr<'guard, ByteCode>,
        nonlocal_refs: ScopedPtr<'guard, ArrayU8>,
    ) -> Result<ScopedPtr<'guard, Function>, RuntimeError> {
        mem.alloc(Function {
            name: TaggedCellPtr::new_with(name),
            arity: param_names.length() as u8,
            code: CellPtr::new_with(code),
            param_names: CellPtr::new_with(param_names),
            nonlocal_refs: CellPtr::new_with(nonlocal_refs),
        })
    }

    /// Return the Function's name as a string slice
    pub fn name<'guard>(&self, guard: &'guard dyn MutatorScope) -> &'guard str {
        let name = self.name.get(guard);
        match *name {
            Value::Symbol(s) => s.as_str(guard),
            _ => "<lambda>",
        }
    }

    /// Return the number of arguments the Function can take
    pub fn arity(&self) -> u8 {
        self.arity
    }

    /// Return the names of the parameters that the Function takes
    pub fn param_names<'guard>(&self, guard: &'guard dyn MutatorScope) -> ScopedPtr<'guard, List> {
        self.param_names.get(guard)
    }

    /// Return the ByteCode object associated with the Function
    pub fn code<'guard>(&self, guard: &'guard dyn MutatorScope) -> ScopedPtr<'guard, ByteCode> {
        self.code.get(guard)
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
            Value::Symbol(s) => write!(f, "(Function {} ({}))", s.as_str(guard), param_string),
            _ => write!(f, "(Function ({}))", param_string),
        }
    }

    fn debug<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        self.code.get(guard).debug(guard, f)
    }
}

/// A partial function application object type
#[derive(Clone)]
pub struct Partial {
    arity: u8,
    used: u8,
    args: CellPtr<List>,
    func: CellPtr<Function>,
}

impl Partial {
    /// Allocate a Partial application of a Function on the heap
    pub fn alloc<'guard>(
        mem: &'guard MutatorView,
        function: ScopedPtr<'guard, Function>,
        args: &[TaggedCellPtr],
    ) -> Result<ScopedPtr<'guard, Partial>, RuntimeError> {
        let used = args.len() as u8;
        let arity = function.arity() - used;

        let args_list: ScopedPtr<'guard, List> = ContainerFromSlice::from_slice(mem, &args)?;

        mem.alloc(Partial {
            arity,
            used,
            args: CellPtr::new_with(args_list),
            func: CellPtr::new_with(function),
        })
    }

    /// Allocate a clone of an existing Partial application, adding the given arguments to the
    /// list of existing args.
    pub fn alloc_clone<'guard>(
        mem: &'guard MutatorView,
        partial: ScopedPtr<'guard, Partial>,
        new_args: &[TaggedCellPtr],
    ) -> Result<ScopedPtr<'guard, Partial>, RuntimeError> {
        let used = partial.used() + new_args.len() as u8;
        let arity = partial.arity() - new_args.len() as u8;

        let arg_list = List::alloc_clone(mem, partial.args(mem))?;
        for arg in new_args {
            arg_list.push(mem, arg.clone())?
        }

        mem.alloc(Partial {
            arity,
            used,
            args: CellPtr::new_with(arg_list),
            func: CellPtr::new_with(partial.function(mem)),
        })
    }

    /// Return the number of arguments this Partial needs before the function can be called
    pub fn arity(&self) -> u8 {
        self.arity
    }

    /// Return the count of arguments already applied
    pub fn used(&self) -> u8 {
        self.used
    }

    /// Return the arguments already supplied to the Partial
    pub fn args<'guard>(&self, guard: &'guard dyn MutatorScope) -> ScopedPtr<'guard, List> {
        self.args.get(guard)
    }

    /// Return the Function object that the Partial will call
    pub fn function<'guard>(&self, guard: &'guard dyn MutatorScope) -> ScopedPtr<'guard, Function> {
        self.func.get(guard)
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
            Value::Symbol(s) => write!(f, "(Partial {} ({}))", s.as_str(guard), param_string),
            _ => write!(f, "(Partial ({}))", param_string),
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
