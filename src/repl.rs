/// Mutator that implements the VM
struct ReadEvalPrint {
    value_stack: List,
    //frame_stack: Array<CallFrame>,
    globals: Dict,
}

impl ReadEvalPrint {
    pub fn new() -> ReadEvalPrint {
        ReadEvalPrint {
            value_stack: List::new(),
            //frame_stack: Array::new(),
            globals: Dict::new(),
        }
    }
}

impl Mutator for ReadEvalPrint {
    type Input = ByteCode;
    type Output = ();

    fn run(&self, mem: &MutatorView, code: Self::Input) -> Result<Self::Output, RuntimeError> {
        Ok(())
    }
}

impl Mutator for ReadEvalPrint {
    type Input = String;
    type Output = ();

    fn run(&self, mem: &MutatorView, line: String) -> Result<(), RuntimeError> {
        match parse(mem, &line) {
            Ok(value) => {
                match compile(mem, value) {
                    Ok(result) => {
                        // println!("{}", result);  // prints bytecode
                        let value = quick_vm_eval(mem, result)?;
                        println!("{}", value);
                    }
                    Err(e) => e.print_with_source(&line),
                }

                // println!("{}", printer::print(*value));

                Ok(())
            }

            Err(e) => {
                match e.error_kind() {
                    // non-fatal repl errors
                    ErrorKind::LexerError(_) => e.print_with_source(&line),
                    ErrorKind::ParseError(_) => e.print_with_source(&line),
                    ErrorKind::EvalError(_) => e.print_with_source(&line),
                    _ => return Err(e),
                }
                Ok(())
            }
        }
    }
}
