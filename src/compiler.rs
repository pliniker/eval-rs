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
        let result_reg = self.compile_inner(mem, ast)?;
        self.bytecode.push_op1(mem, Opcode::RETURN, result_reg);
        Ok(())
    }

    fn compile_inner<'guard>(
        &mut self,
        mem: &'guard MutatorView,
        ast_node: ScopedPtr<'guard>,
    ) -> Result<Register, RuntimeError> {
        match *ast {
            Value::Symbol(s) => {
                let reg = self.acquire_reg();
                let lit_id = self.bytecode.push_lit(mem, ast_node);
                self.bytecode.push_loadlit(mem, reg, lit_id);
                Ok(reg)
            },
            Value::Pair(p) => {
                Ok(1)
            },
            _ => Err(RuntimeError::new(ErrorKind::CompileError(String::from("Unexpected type in AST"))))
        }
    }

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

    let compiler = Compiler::new();

    let bytecode = compiler.bytecode;
    mem.alloc(bytecode)
}
