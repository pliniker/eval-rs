use crate::bytecode::{ByteCode, Opcode, Register};
use crate::error::{err_compile, RuntimeError};
use crate::memory::MutatorView;
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
                self.push_load_literal(mem, literal)
            }

            _ => self.push_load_literal(mem, ast_node),
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
                "quote" => {
                    self.push_load_literal(mem, self.get_first_of_undotted_pair(mem, params)?)
                }
                "atom" => self.push_op2(mem, Opcode::ATOM, params),
                _ => unimplemented!(),
            },

            _ => Err(err_compile("Non symbol in function-call position")),
        }
    }

    fn push_op0<'guard>(
        &mut self,
        mem: &'guard MutatorView,
        op: Opcode,
    ) -> Result<(), RuntimeError> {
        self.bytecode.push_op0(mem, op)?;
        Ok(())
    }

    fn push_op2<'guard>(
        &mut self,
        mem: &'guard MutatorView,
        op: Opcode,
        params: ScopedPtr<'guard>,
    ) -> Result<Register, RuntimeError> {
        let result = self.acquire_reg();
        let reg1 = self.compile_eval(mem, self.get_first_of_undotted_pair(mem, params)?)?;
        self.bytecode.push_op2(mem, op, result, reg1)?;
        Ok(result)
    }

    fn push_op3<'guard>(
        &mut self,
        mem: &'guard MutatorView,
        op: Opcode,
        params: ScopedPtr<'guard>,
    ) -> Result<Register, RuntimeError> {
        let result = self.acquire_reg();
        let (first, second) = self.get_two_from_pairs(mem, params)?;
        let reg1 = self.compile_eval(mem, first)?;
        let reg2 = self.compile_eval(mem, second)?;
        self.bytecode.push_op3(mem, op, result, reg1, reg2)?;
        Ok(result)
    }

    // Push a literal onto the literals list and a load instruction onto the bytecode list
    fn push_load_literal<'guard>(
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
        guard: &'guard dyn MutatorScope,
        ptr: ScopedPtr<'guard>,
    ) -> Result<ScopedPtr<'guard>, RuntimeError> {
        match *ptr {
            Value::Pair(pair) => {
                if pair.second.is_nil() {
                    Ok(pair.first.get(guard))
                } else {
                    Err(err_compile("Expected no more than one parameter"))
                }
            }
            _ => Err(err_compile("Expected no less than one parameter")),
        }
    }

    // Assert that pointer is to a Pair, that the second is also a Pair and return both Pair.first values
    fn get_two_from_pairs<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        ptr: ScopedPtr<'guard>,
    ) -> Result<(ScopedPtr<'guard>, ScopedPtr<'guard>), RuntimeError> {
        match *ptr {
            Value::Pair(pair) => {
                let first_param = pair.first.get(guard);

                match *pair.second.get(guard) {
                    Value::Pair(pair) => {

                        if let Value::Nil = *pair.second.get(guard) {
                            let second_param = pair.first.get(guard);
                            Ok((first_param, second_param))
                        } else {
                            Err(err_compile("Expected no more than two parameters"))
                        }

                    },

                    _ => Err(err_compile("Expected no less than two parameters")),
                }
            }
            _ => Err(err_compile("Expected no less than two parameters")),
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
