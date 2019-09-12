use crate::bytecode::{ByteCode, Opcode, Register};
use crate::containers::{Container, IndexedContainer, StackContainer};
use crate::error::{err_compile, RuntimeError};
use crate::memory::MutatorView;
use crate::primitives::{ArrayAny, ArrayU32};
use crate::safeptr::{MutatorScope, ScopedPtr};
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
                self.push_literal(mem, literal)
            }

            _ => self.push_literal(mem, ast_node),
        }
    }

    fn compile_apply<'guard>(
        &mut self,
        mem: &'guard MutatorView,
        function: ScopedPtr<'guard>,
        params: ScopedPtr<'guard>,
    ) -> Result<Register, RuntimeError> {
        match *function {
            Value::Symbol(s) => match s.as_str(mem) {
                "quote" => self.push_literal(mem, self.get_first_of_undotted_pair(mem, params)?),
                _ => unimplemented!(),
            },

            _ => Err(err_compile("Non symbol in function-call position")),
        }
    }

    // Push a literal onto the literals list and a load instruction onto the bytecode list
    fn push_literal<'guard>(
        &mut self,
        mem: &'guard MutatorView,
        literal: ScopedPtr<'guard>,
    ) -> Result<Register, RuntimeError> {
        let reg = self.acquire_reg();
        let lit_id = self.bytecode.push_lit(mem, literal)?;
        self.bytecode.push_loadlit(mem, reg, lit_id)?;
        Ok(reg)
    }

    // Assert that pointer is to a Pair, that only the first is non-nil, and return the first
    fn get_first_of_undotted_pair<'guard>(
        &self,
        guard: &'guard MutatorScope,
        ptr: ScopedPtr<'guard>,
    ) -> Result<ScopedPtr<'guard>, RuntimeError> {
        match *ptr {
            Value::Pair(pair) => {
                if pair.second.is_nil() {
                    Ok(pair.first.get(guard))
                } else {
                    Err(err_compile("Pair must not be dotted"))
                }
            }
            _ => Err(err_compile("Expected Pair type")),
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
