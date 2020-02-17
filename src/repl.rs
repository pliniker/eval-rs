use crate::compiler::compile;
use crate::containers::{Container, StackAnyContainer};
use crate::dict::Dict;
use crate::error::{ErrorKind, RuntimeError};
use crate::list::List;
use crate::memory::{Mutator, MutatorView};
use crate::parser::parse;
use crate::safeptr::{CellPtr, TaggedScopedPtr};
use crate::vm::quick_vm_eval;

/// A mutator that returns a Repl instance
pub struct RepMaker {}

impl Mutator for RepMaker {
    type Input = ();
    type Output = ReadEvalPrint;

    fn run(&self, mem: &MutatorView, _input: ()) -> Result<ReadEvalPrint, RuntimeError> {
        ReadEvalPrint::new(mem)
    }
}

/// Mutator that implements the VM
pub struct ReadEvalPrint {
    value_stack: CellPtr<List>,
    //frame_stack: CellPtr<Array<CallFrame>>,
    globals: CellPtr<Dict>,
}

impl ReadEvalPrint {
    pub fn new(mem: &MutatorView) -> Result<ReadEvalPrint, RuntimeError> {
        let stack = mem.alloc(List::with_capacity(mem, 256)?)?;
        for _ in 0..256 {
            stack.push(mem, mem.nil())?;
        }

        let globals = mem.alloc(Dict::new())?;

        Ok(ReadEvalPrint {
            value_stack: CellPtr::new_with(stack),
            globals: CellPtr::new_with(globals),
        })
    }
}

impl Mutator for ReadEvalPrint {
    type Input = String;
    type Output = ();

    fn run(&self, mem: &MutatorView, line: String) -> Result<(), RuntimeError> {
        let stack = self.value_stack.get(mem);
        let globals = self.globals.get(mem);

        match (|mem, line| -> Result<TaggedScopedPtr, RuntimeError> {
            let value = parse(mem, line)?;
            let code = compile(mem, value)?;
            let value = quick_vm_eval(mem, stack, globals, code)?;
            Ok(value)
        })(mem, &line)
        {
            Ok(value) => println!("{}", value),

            Err(e) => {
                match e.error_kind() {
                    // non-fatal repl errors
                    ErrorKind::LexerError(_) => e.print_with_source(&line),
                    ErrorKind::ParseError(_) => e.print_with_source(&line),
                    ErrorKind::EvalError(_) => e.print_with_source(&line),
                    _ => return Err(e),
                }
            }
        }

        Ok(())
    }
}