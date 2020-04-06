use std::collections::HashMap;

use crate::array::ArraySize;
use crate::bytecode::{ByteCode, Opcode, Register};
use crate::error::{err_eval, RuntimeError};
use crate::function::Function;
use crate::memory::MutatorView;
use crate::pair::{value_from_1_pair, values_from_2_pairs, vec_from_pairs};
use crate::safeptr::{CellPtr, ScopedPtr, TaggedScopedPtr};
use crate::taggedptr::Value;

/// A compile-time intermediate data structure, hence can be discarded immediately after use and
/// be built on std
struct Scope {
    // symbol -> register mapping
    pub bindings: HashMap<String, u8>,
}

struct Compiler {
    bytecode: CellPtr<ByteCode>,
    next_reg: Register,
    // TODO:
    // optional function name
    name: Option<String>,
    // optional link to parent scope
    //  * parent: Option<&Compiler>
    // function-local nested scopes bindings list (including parameters at outer level)
    locals: Vec<Scope>,
}

impl Compiler {
    fn new<'guard>(mem: &'guard MutatorView) -> Result<Compiler, RuntimeError> {
        Ok(Compiler {
            bytecode: CellPtr::new_with(ByteCode::new(mem)?),
            next_reg: 0,
            name: None,
            locals: Vec::new(),
        })
    }

    /// Compile an outermost-level expression, at the 'main' function level
    fn compile<'guard>(
        &mut self,
        mem: &'guard MutatorView,
        ast: TaggedScopedPtr<'guard>,
    ) -> Result<(), RuntimeError> {
        let result_reg = self.compile_eval(mem, ast)?;
        self.bytecode
            .get(mem)
            .push_op1(mem, Opcode::RETURN, result_reg)?;
        Ok(())
    }

    /// Compile an expression that has parameters and possibly a name
    fn compile_function<'guard>(
        &mut self,
        mem: &'guard MutatorView,
        params: &[TaggedScopedPtr<'guard>], // TODO make use of params in scopes
        exprs: &[TaggedScopedPtr<'guard>],
    ) -> Result<(), RuntimeError> {
        let mut result_reg = 0;

        for expr in exprs.iter() {
            result_reg = self.compile_eval(mem, *expr)?;
        }

        self.bytecode
            .get(mem)
            .push_op1(mem, Opcode::RETURN, result_reg)?;
        Ok(())
    }

    fn compile_eval<'guard>(
        &mut self,
        mem: &'guard MutatorView,
        ast_node: TaggedScopedPtr<'guard>,
    ) -> Result<Register, RuntimeError> {
        match *ast_node {
            Value::Pair(p) => self.compile_apply(mem, p.first.get(mem), p.second.get(mem)),

            Value::Symbol(s) => {
                match s.as_str(mem) {
                    "nil" => self.push_load_literal(mem, mem.nil()),

                    "true" => self.push_load_literal(mem, mem.lookup_sym("true")),

                    // TODO: check for local variable and return register number first
                    _ => {
                        // load sym, then replace sym with global
                        let reg1 = self.push_load_literal(mem, ast_node)?;
                        self.bytecode
                            .get(mem)
                            .push_op2(mem, Opcode::LOADGLOBAL, reg1, reg1)?;
                        Ok(reg1)
                    }
                }
            }

            _ => self.push_load_literal(mem, ast_node),
        }
    }

    fn compile_apply<'guard>(
        &mut self,
        mem: &'guard MutatorView,
        function: TaggedScopedPtr<'guard>,
        params: TaggedScopedPtr<'guard>,
    ) -> Result<Register, RuntimeError> {
        match *function {
            Value::Symbol(s) => match s.as_str(mem) {
                "quote" => self.push_load_literal(mem, value_from_1_pair(mem, params)?),
                "atom?" => self.push_op2(mem, Opcode::ATOM, params),
                "nil?" => self.push_op2(mem, Opcode::NIL, params),
                "car" => self.push_op2(mem, Opcode::CAR, params),
                "cdr" => self.push_op2(mem, Opcode::CDR, params),
                "cons" => self.push_op3(mem, Opcode::CONS, params),
                "cond" => self.compile_apply_cond(mem, params),
                "is?" => self.push_op3(mem, Opcode::IS, params),
                "set" => self.compile_apply_assign(mem, params),
                "def" => self.compile_named_function(mem, params),

                _ => {
                    // TODO params - use register to pass in parameter count?
                    let result = self.acquire_reg();
                    let fn_name_reg = self.compile_eval(mem, function)?;
                    self.bytecode
                        .get(mem)
                        .push_op2(mem, Opcode::CALL, result, fn_name_reg)?;
                    Ok(result)
                }
            },

            _ => Err(err_eval("Non symbol in function-call position")),
        }
    }

    fn compile_apply_cond<'guard>(
        &mut self,
        mem: &'guard MutatorView,
        params: TaggedScopedPtr<'guard>,
    ) -> Result<Register, RuntimeError> {
        //
        //   for each param:
        //     eval cond
        //     if false then jmp -> next
        //     else eval expr
        //     jmp -> end
        //
        let bytecode = self.bytecode.get(mem);

        let mut end_jumps: Vec<ArraySize> = Vec::new();
        let mut last_cond_jump: Option<ArraySize> = None;

        let result = self.next_reg;

        let mut head = params;
        while let Value::Pair(p) = *head {
            let cond = p.first.get(mem);
            head = p.second.get(mem);
            match *head {
                Value::Pair(p) => {
                    let expr = p.first.get(mem);
                    head = p.second.get(mem);

                    // if this is not the first condition, set the offset of the last
                    // condition-not-true jump to the beginning of this condition
                    if let Some(address) = last_cond_jump {
                        let offset = bytecode.next_instruction() - address - 1;
                        bytecode.write_jump_offset(mem, address, offset)?;
                    }

                    // We have a condition to evaluate. If the resut is Not True, jump to the
                    // next condition.
                    self.reset_reg(result); // reuse this register for condition and result
                    let cond_result = self.compile_eval(mem, cond)?;
                    self.bytecode
                        .get(mem)
                        .push_cond_jump(mem, Opcode::JMPNT, cond_result)?;
                    last_cond_jump = Some(bytecode.last_instruction());

                    // Compile the expression and jump to the end of the entire cond
                    self.reset_reg(result); // reuse this register for condition and result
                    let _expr_result = self.compile_eval(mem, expr)?;
                    bytecode.push_jump(mem)?;
                    end_jumps.push(bytecode.last_instruction());
                }

                _ => return Err(err_eval("Unexpected end of cond list")),
            }
        }

        // Close out with a default NIL result if none of the conditions passed
        if let Some(address) = last_cond_jump {
            self.reset_reg(result);
            self.push_op1(mem, Opcode::LOADNIL)?;
            let offset = bytecode.next_instruction() - address - 1;
            bytecode.write_jump_offset(mem, address, offset)?;
        }

        // Update all the post-expr jumps to point at the next instruction after the entire cond
        for address in end_jumps.iter() {
            let offset = bytecode.next_instruction() - address - 1;
            bytecode.write_jump_offset(mem, *address, offset)?;
        }

        Ok(result)
    }

    /// Assignment expression - evaluate the two expressions, binding the result of the first
    /// to the (hopefully) symbol provided by the second
    fn compile_apply_assign<'guard>(
        &mut self,
        mem: &'guard MutatorView,
        params: TaggedScopedPtr<'guard>,
    ) -> Result<Register, RuntimeError> {
        let (first, second) = values_from_2_pairs(mem, params)?;
        let expr = self.compile_eval(mem, second)?;
        let assign_to = self.compile_eval(mem, first)?;
        self.bytecode
            .get(mem)
            .push_op2(mem, Opcode::STOREGLOBAL, assign_to, expr)?;
        Ok(expr)
    }

    fn compile_named_function<'guard>(
        &mut self,
        mem: &'guard MutatorView,
        params: TaggedScopedPtr<'guard>,
    ) -> Result<Register, RuntimeError> {
        let items = vec_from_pairs(mem, params)?;

        if items.len() < 3 {
            return Err(err_eval(
                "A function definition must have at least (name params expr)",
            ));
        }

        // a function consists of (name (params) expr1 .. exprn)
        let fn_name = items[0];
        let fn_params = vec_from_pairs(mem, items[1])?;
        let fn_exprs = &items[2..];

        // compile the function to a Function object
        let fn_object = compile_function(mem, fn_name, &fn_params, fn_exprs)?;

        // load the function object as a literal and associate it with a global name
        let name_reg = self.push_load_literal(mem, fn_name)?;
        let function_reg = self.push_load_literal(mem, fn_object)?;
        self.bytecode
            .get(mem)
            .push_op2(mem, Opcode::STOREGLOBAL, name_reg, function_reg)?;

        Ok(function_reg)
    }

    fn push_op0<'guard>(
        &mut self,
        mem: &'guard MutatorView,
        op: Opcode,
    ) -> Result<(), RuntimeError> {
        self.bytecode.get(mem).push_op0(mem, op)?;
        Ok(())
    }

    fn push_op1<'guard>(
        &mut self,
        mem: &'guard MutatorView,
        op: Opcode,
    ) -> Result<Register, RuntimeError> {
        let result = self.acquire_reg();
        self.bytecode.get(mem).push_op1(mem, op, result)?;
        Ok(result)
    }

    fn push_op2<'guard>(
        &mut self,
        mem: &'guard MutatorView,
        op: Opcode,
        params: TaggedScopedPtr<'guard>,
    ) -> Result<Register, RuntimeError> {
        let result = self.acquire_reg();
        let reg1 = self.compile_eval(mem, value_from_1_pair(mem, params)?)?;
        self.bytecode.get(mem).push_op2(mem, op, result, reg1)?;
        Ok(result)
    }

    fn push_op3<'guard>(
        &mut self,
        mem: &'guard MutatorView,
        op: Opcode,
        params: TaggedScopedPtr<'guard>,
    ) -> Result<Register, RuntimeError> {
        let result = self.acquire_reg();
        let (first, second) = values_from_2_pairs(mem, params)?;
        let reg1 = self.compile_eval(mem, first)?;
        let reg2 = self.compile_eval(mem, second)?;
        self.bytecode
            .get(mem)
            .push_op3(mem, op, result, reg1, reg2)?;
        Ok(result)
    }

    // Push a literal onto the literals list and a load instruction onto the bytecode list
    fn push_load_literal<'guard>(
        &mut self,
        mem: &'guard MutatorView,
        literal: TaggedScopedPtr<'guard>,
    ) -> Result<Register, RuntimeError> {
        let reg = self.acquire_reg();
        let lit_id = self.bytecode.get(mem).push_lit(mem, literal)?;
        self.bytecode.get(mem).push_loadlit(mem, reg, lit_id)?;
        Ok(reg)
    }

    // this is a naive way of allocating registers - every result gets it's own register
    fn acquire_reg(&mut self) -> Register {
        // TODO check overflow
        let reg = self.next_reg;
        self.next_reg += 1;
        reg
    }

    // reset the next register back to the given one so that it is reused
    fn reset_reg(&mut self, reg: Register) {
        self.next_reg = reg
    }
}

/// Compile a function - parameters and expression, returning a Function object
fn compile_function<'guard>(
    mem: &'guard MutatorView,
    name: TaggedScopedPtr<'guard>,
    params: &[TaggedScopedPtr<'guard>],
    exprs: &[TaggedScopedPtr<'guard>],
) -> Result<TaggedScopedPtr<'guard>, RuntimeError> {
    // validate function name
    match *name {
        Value::Symbol(_) => (),
        Value::Nil => (),
        _ => {
            return Err(err_eval(
                "A function name may be nil (anonymous) or a symbol (named)",
            ))
        }
    };
    let fn_name = name;

    // validate arity
    let fn_arity = if params.len() > 250 {
        return Err(err_eval("A function cannot have more than 250 parameters"));
    } else {
        params.len() as u8
    };

    // validate expression list
    if exprs.len() == 0 {
        return Err(err_eval("A function must have at least one expression"));
    }

    // compile the expresssions
    let mut compiler = Compiler::new(mem)?;
    compiler.compile_function(mem, params, exprs)?;
    let fn_bytecode = compiler.bytecode.get(mem);

    Ok(Function::new(mem, fn_name, fn_arity, fn_bytecode)?.as_tagged(mem))
}

/// Compile the given AST and return a bytecode structure
pub fn compile<'guard>(
    mem: &'guard MutatorView,
    ast: TaggedScopedPtr<'guard>,
) -> Result<ScopedPtr<'guard, Function>, RuntimeError> {
    let mut compiler = Compiler::new(mem)?;
    compiler.compile(mem, ast)?;

    Ok(Function::new(mem, mem.nil(), 0, compiler.bytecode.get(mem))?)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn compile_cond() {
        // TODO
    }
}
