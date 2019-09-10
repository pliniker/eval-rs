use crate::bytecode::{ByteCode, Opcode, Register};
use crate::containers::{Container, IndexedContainer, StackContainer};
use crate::error::{ErrorKind, RuntimeError};
use crate::memory::MutatorView;
use crate::primitives::{ArrayAny, ArrayU32};
use crate::safeptr::ScopedPtr;
use crate::taggedptr::Value;

struct Compiler {
    bytecode: ByteCode,
    next_reg: Register,
}

impl Compiler {
    fn new() -> Compiler {
        Compiler {
            bytecode: ByteCode::new(),
            next_reg: 0,
        }
    }

    fn compile<'guard>(
        &mut self,
        mem: &'guard MutatorView,
        ast: ScopedPtr<'guard>,
    ) -> Result<(), RuntimeError> {
        let result_reg = self.compile_eval(mem, ast)?;
        self.bytecode.push_op1(mem, Opcode::RETURN, result_reg)
    }

    fn compile_eval<'guard>(
        &mut self,
        mem: &'guard MutatorView,
        ast_node: ScopedPtr<'guard>,
    ) -> Result<Register, RuntimeError> {
        match *ast_node {
            Value::Pair(p) => self.compile_apply(mem, p.first.get(mem), p.second.get(mem)),

            Value::Symbol(s) => {
                let literal = match s.as_str(mem) {
                    "nil" => mem.nil(),
                    _ => ast_node,
                };

                let reg = self.acquire_reg();
                let lit_id = self.bytecode.push_lit(mem, literal)?;
                self.bytecode.push_loadlit(mem, reg, lit_id)?;
                Ok(reg)
            }

            _ => {
                let reg = self.acquire_reg();
                let lit_id = self.bytecode.push_lit(mem, ast_node)?;
                self.bytecode.push_loadlit(mem, reg, lit_id)?;
                Ok(reg)
            }
        }
    }

    fn compile_apply<'guard>(
        &mut self,
        mem: &'guard MutatorView,
        function: ScopedPtr<'guard>,
        params: ScopedPtr<'guard>,
    ) -> Result<Register, RuntimeError> {
        match *function {
            Value::Symbol(s) => {
                match s.as_str(mem) {
                    "quote" => {
                        let reg = self.acquire_reg();
                        let lit_id = self.bytecode.push_lit(mem, params)?;
                        self.bytecode.push_loadlit(mem, reg, lit_id)?;
                        Ok(reg)
                    },

                    _ => unimplemented!()
                }
            },

            _ => Err(RuntimeError::new(ErrorKind::CompileError(String::from(
                "Non-symbol in function-call position",
            )))),
        }
    }

    // this is a naive way of allocating registers - every result gets it's own register
    fn acquire_reg(&mut self) -> Register {
        let reg = self.next_reg;
        self.next_reg += 1;
        reg
    }
}

/// Compile the given AST and return a bytecode structure
pub fn compile<'guard>(
    mem: &'guard MutatorView,
    ast: ScopedPtr<'guard>,
) -> Result<ScopedPtr<'guard>, RuntimeError> {
    // depth-first tree traversal, flattening the output

    let mut compiler = Compiler::new();
    compiler.compile(mem, ast)?;

    let bytecode = compiler.bytecode;
    mem.alloc(bytecode)
}
