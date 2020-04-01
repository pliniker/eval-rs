use crate::compiler::compile;
use crate::error::{ErrorKind, RuntimeError};
use crate::memory::{Mutator, MutatorView};
use crate::parser::parse;
use crate::safeptr::{CellPtr, TaggedScopedPtr};
use crate::vm::Thread;

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
    main_thread: CellPtr<Thread>
}

impl ReadEvalPrint {
    pub fn new(mem: &MutatorView) -> Result<ReadEvalPrint, RuntimeError> {
        Ok(ReadEvalPrint {
            main_thread: CellPtr::new_with(Thread::new(mem)?)
        })
    }
}

impl Mutator for ReadEvalPrint {
    type Input = String;
    type Output = ();

    fn run(&self, mem: &MutatorView, line: String) -> Result<(), RuntimeError> {
        let thread = self.main_thread.get(mem);

        match (|mem, line| -> Result<TaggedScopedPtr, RuntimeError> {
            let value = parse(mem, line)?;
            let code = compile(mem, value)?;
            let value = thread.quick_vm_eval(mem, code)?;
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
