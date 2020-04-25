use std::collections::HashMap;

use crate::array::ArraySize;
use crate::bytecode::{ByteCode, Opcode, Register};
use crate::containers::AnyContainerFromSlice;
use crate::error::{err_eval, RuntimeError};
use crate::function::Function;
use crate::list::List;
use crate::memory::MutatorView;
use crate::pair::{value_from_1_pair, values_from_2_pairs, vec_from_pairs};
use crate::safeptr::{CellPtr, ScopedPtr, TaggedScopedPtr};
use crate::taggedptr::Value;

/// A compile-time intermediate data structure, hence can be discarded immediately after use and
/// be built on std
struct Scope {
    // symbol -> register mapping
    bindings: HashMap<String, Register>,
}

impl Scope {
    fn new() -> Scope {
        Scope {
            bindings: HashMap::new(),
        }
    }

    // Add a Symbol->Register binding to this scope
    fn push_binding<'guard>(
        &mut self,
        name: TaggedScopedPtr<'guard>,
        reg: Register,
    ) -> Result<(), RuntimeError> {
        let name_string = match *name {
            Value::Symbol(s) => String::from(s.as_str(&name)),
            _ => return Err(err_eval("A parameter name must be a symbol")),
        };

        self.bindings.insert(name_string, reg);

        Ok(())
    }

    // Push a block of bindings into this scope, returning the next register available
    // after these bound registers
    fn push_bindings<'guard>(
        &mut self,
        names: &[TaggedScopedPtr<'guard>],
        start_reg: Register,
    ) -> Result<Register, RuntimeError> {
        let mut reg = start_reg;
        for name in names {
            self.push_binding(*name, reg)?;
            reg += 1;
        }
        Ok(reg)
    }

    // Find a Symbol->Register binding in this scope
    fn lookup_binding<'guard>(
        &self,
        name: TaggedScopedPtr<'guard>,
    ) -> Result<Option<Register>, RuntimeError> {
        let name_string = match *name {
            Value::Symbol(s) => String::from(s.as_str(&name)),
            _ => return Err(err_eval("A variable must be represented by a symbol")),
        };

        match self.bindings.get(&name_string) {
            Some(reg) => Ok(Some(*reg)),
            None => Ok(None),
        }
    }
}

struct Compiler {
    bytecode: CellPtr<ByteCode>,
    next_reg: Register,
    // TODO:
    // optional function name
    name: Option<String>,
    // function-local nested scopes bindings list (including parameters at outer level)
    locals: Vec<Scope>,
    // optional link to parent scope for finding free variable register indexes
    //parent: Option<&'outer_scope Compiler>
    // call site return register, needed to know relative base of parent nested function calls
    // when compiling a partial function
    //call_return_reg: Option<Register>
}

impl Compiler {
    /// Instantiate a new nested function-level compiler
    fn new<'guard>(mem: &'guard MutatorView) -> Result<Compiler, RuntimeError> {
        Ok(Compiler {
            bytecode: CellPtr::new_with(ByteCode::alloc(mem)?),
            // register 0 is reserved for the return value
            next_reg: 1,
            name: None,
            locals: Vec::new(),
            //parent: None,
            //call_return_reg: None,
        })
    }

    /// Compile an expression that has parameters and possibly a name
    fn compile_function<'guard>(
        mut self,
        mem: &'guard MutatorView,
        name: TaggedScopedPtr<'guard>,
        params: &[TaggedScopedPtr<'guard>], // TODO make use of params in scopes
        exprs: &[TaggedScopedPtr<'guard>],
    ) -> Result<ScopedPtr<'guard, Function>, RuntimeError> {
        // validate function name
        self.name = match *name {
            Value::Symbol(s) => Some(String::from(s.as_str(mem))),
            Value::Nil => None,
            _ => {
                return Err(err_eval(
                    "A function name may be nil (anonymous) or a symbol (named)",
                ))
            }
        };
        let fn_name = name;

        // validate arity
        if params.len() > 254 {
            return Err(err_eval("A function cannot have more than 254 parameters"));
        }
        // put params into a list for the Function object
        let fn_params = List::from_slice(mem, params)?;

        // also assign params to the first level function scope and give each one a register
        let mut param_scope = Scope::new();
        self.next_reg = param_scope.push_bindings(params, self.next_reg)?;
        self.locals.push(param_scope);

        // validate expression list
        if exprs.len() == 0 {
            return Err(err_eval("A function must have at least one expression"));
        }

        // compile expressions
        let mut result_reg = 0;
        for expr in exprs.iter() {
            result_reg = self.compile_eval(mem, *expr)?;
        }

        // TODO Question: for every RETURN instruction in the bytecode, if the preceding
        // instruction is a CALL, can the CALL be converted to a TAILCALL?

        let fn_bytecode = self.bytecode.get(mem);
        fn_bytecode.push_op1(mem, Opcode::RETURN, result_reg)?;

        Ok(Function::alloc(mem, fn_name, fn_params, fn_bytecode)?)
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
                    "nil" => self.push_op1(mem, Opcode::LOADNIL),

                    "true" => self.push_load_literal(mem, mem.lookup_sym("true")),

                    // Search scopes for a binding; if none do a global lookup
                    _ => {
                        // First search local bindings from inner to outer
                        for scope in self.locals.iter().rev() {
                            if let Some(reg) = scope.lookup_binding(ast_node)? {
                                return Ok(reg);
                            }
                        }

                        // Otherwise do a late-binding global lookup
                        let reg_acc = self.push_load_literal(mem, ast_node)?;
                        self.bytecode.get(mem).push_op2(
                            mem,
                            Opcode::LOADGLOBAL,
                            reg_acc,
                            reg_acc,
                        )?;
                        Ok(reg_acc)
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
                "lambda" => self.compile_anonymous_function(mem, params),
                "let" => self.compile_apply_let(mem, params),
                _ => self.compile_apply_call(mem, function, params),
            },

            // Here we allow the value in the function position to be evaluated dynamically
            _ => self.compile_apply_call(mem, function, params),
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

    fn compile_anonymous_function<'guard>(
        &mut self,
        mem: &'guard MutatorView,
        params: TaggedScopedPtr<'guard>,
    ) -> Result<Register, RuntimeError> {
        let items = vec_from_pairs(mem, params)?;

        if items.len() < 2 {
            return Err(err_eval(
                "An anonymous function definition must have at least (lambda (params) expr)",
            ));
        }

        // a function consists of (name (params) expr1 .. exprn)
        let fn_params = vec_from_pairs(mem, items[0])?;
        let fn_exprs = &items[1..];

        // compile the function to a Function object
        let fn_object = compile_function(mem, mem.nil(), &fn_params, fn_exprs)?;

        // load the function object as a literal
        self.push_load_literal(mem, fn_object)
    }

    fn compile_named_function<'guard>(
        &mut self,
        mem: &'guard MutatorView,
        params: TaggedScopedPtr<'guard>,
    ) -> Result<Register, RuntimeError> {
        let items = vec_from_pairs(mem, params)?;

        if items.len() < 3 {
            return Err(err_eval(
                "A function definition must have at least (def name (params) expr)",
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

    fn compile_apply_call<'guard>(
        &mut self,
        mem: &'guard MutatorView,
        function: TaggedScopedPtr<'guard>,
        args: TaggedScopedPtr<'guard>,
    ) -> Result<Register, RuntimeError> {
        let bytecode = self.bytecode.get(mem);

        // allocate a register for the return value
        let result = self.acquire_reg();

        // evaluate arguments first
        let arg_list = vec_from_pairs(mem, args)?;
        let arg_count = arg_list.len() as u8;

        for arg in arg_list {
            let reg = self.compile_eval(mem, arg)?;
            // if a bound variable was returned, we have a direct reference to its register;
            // we need to copy the register to the arg list. Bound registers are necessarily
            // lower indexes than where the function call is situated.
            if reg <= result {
                let arg_reg = self.acquire_reg();
                bytecode.push_op2(mem, Opcode::COPYREG, arg_reg, reg)?;
            }
        }

        // put the function pointer in the last register of the call so it'll be discarded
        let fn_reg = self.compile_eval(mem, function)?;

        bytecode.push_op3(mem, Opcode::CALL, result, fn_reg, arg_count)?;

        // ignore use of any registers beyond the result once the call is complete
        self.reset_reg(result + 1);
        Ok(result)
    }

    fn compile_apply_let<'guard>(
        &mut self,
        mem: &'guard MutatorView,
        params: TaggedScopedPtr<'guard>,
    ) -> Result<Register, RuntimeError> {
        unimplemented!()
    }

    fn _push_op0<'guard>(
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

/// Compile a function - parameters and expression, returning a tagged Function object
fn compile_function<'guard>(
    mem: &'guard MutatorView,
    name: TaggedScopedPtr<'guard>,
    params: &[TaggedScopedPtr<'guard>],
    exprs: &[TaggedScopedPtr<'guard>],
) -> Result<TaggedScopedPtr<'guard>, RuntimeError> {
    let compiler = Compiler::new(mem)?;
    Ok(compiler
        .compile_function(mem, name, params, exprs)?
        .as_tagged(mem))
}

/// Compile the given AST and return an anonymous Function object
pub fn compile<'guard>(
    mem: &'guard MutatorView,
    ast: TaggedScopedPtr<'guard>,
) -> Result<ScopedPtr<'guard, Function>, RuntimeError> {
    let compiler = Compiler::new(mem)?;
    compiler.compile_function(mem, mem.nil(), &[], &[ast])
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::memory::{Memory, Mutator};
    use crate::parser::parse;
    use crate::vm::Thread;

    fn eval_helper<'guard>(
        mem: &'guard MutatorView,
        thread: ScopedPtr<'guard, Thread>,
        code: &str,
    ) -> Result<TaggedScopedPtr<'guard>, RuntimeError> {
        let compiled_code = compile(mem, parse(mem, code)?)?;
        println!("RUN CODE {}", code);
        let result = thread.quick_vm_eval(mem, compiled_code)?;
        println!("RUN RESULT {}", result);
        Ok(result)
    }

    fn test_helper(test_fn: fn(&MutatorView) -> Result<(), RuntimeError>) {
        let mem = Memory::new();

        struct Test {}
        impl Mutator for Test {
            type Input = fn(&MutatorView) -> Result<(), RuntimeError>;
            type Output = ();

            fn run(
                &self,
                mem: &MutatorView,
                test_fn: Self::Input,
            ) -> Result<Self::Output, RuntimeError> {
                test_fn(mem)
            }
        }

        let test = Test {};
        mem.mutate(&test, test_fn).unwrap();
    }

    #[test]
    fn compile_cond_first_is_true() {
        fn test_inner(mem: &MutatorView) -> Result<(), RuntimeError> {
            // testing 'cond'
            // (nil? nil) == true, so result should be x
            let code = "(cond (nil? nil) 'x (nil? 'a) 'y)";

            let t = Thread::alloc(mem)?;

            let result = eval_helper(mem, t, code)?;

            assert!(result == mem.lookup_sym("x"));

            Ok(())
        }

        test_helper(test_inner);
    }

    #[test]
    fn compile_cond_second_is_true() {
        fn test_inner(mem: &MutatorView) -> Result<(), RuntimeError> {
            // testing 'cond'
            // (nil? 'a) == nil, (nil? nil) == true, so result should be y
            let code = "(cond (nil? 'a) 'x (nil? nil) 'y)";

            let t = Thread::alloc(mem)?;

            let result = eval_helper(mem, t, code)?;

            assert!(result == mem.lookup_sym("y"));

            Ok(())
        }

        test_helper(test_inner);
    }

    #[test]
    fn compile_cond_none_is_true() {
        fn test_inner(mem: &MutatorView) -> Result<(), RuntimeError> {
            // testing 'cond'
            // (nil? 'a) == nil, (nil? 'b) == nil, result should be nil
            let code = "(cond (nil? 'a) 'x (nil? 'b) 'y)";

            let t = Thread::alloc(mem)?;

            let result = eval_helper(mem, t, code)?;

            assert!(result == mem.nil());

            Ok(())
        }

        test_helper(test_inner);
    }

    #[test]
    fn compile_call_functions() {
        fn test_inner(mem: &MutatorView) -> Result<(), RuntimeError> {
            // this test calls a function from another function
            let compare_fn = "(def is_it (ask expect) (is? ask expect))";
            let curried_fn = "(def is_it_a (ask) (is_it ask 'a))";
            let query1 = "(is_it_a nil)";
            let query2 = "(is_it_a 'a)";

            let t = Thread::alloc(mem)?;

            eval_helper(mem, t, compare_fn)?;
            eval_helper(mem, t, curried_fn)?;

            let result1 = eval_helper(mem, t, query1)?;
            assert!(result1 == mem.nil());

            let result2 = eval_helper(mem, t, query2)?;
            assert!(result2 == mem.lookup_sym("true"));

            Ok(())
        }

        test_helper(test_inner);
    }

    #[test]
    fn compile_map_function_over_list() {
        fn test_inner(mem: &MutatorView) -> Result<(), RuntimeError> {
            // this test passes a function as a parameter through recursive function calls
            let compare_fn = "(def is_y (ask) (is? ask 'y))";
            let map_fn =
                "(def map (f l) (cond (nil? l) nil true (cons (f (car l)) (map f (cdr l)))))";

            let query = "(map is_y '(x y z z y))";

            let t = Thread::alloc(mem)?;

            eval_helper(mem, t, compare_fn)?;
            eval_helper(mem, t, map_fn)?;

            let result = eval_helper(mem, t, query)?;

            let result = vec_from_pairs(mem, result)?;
            let sym_nil = mem.nil();
            let sym_true = mem.lookup_sym("true");
            assert!(result == &[sym_nil, sym_true, sym_nil, sym_nil, sym_true]);

            Ok(())
        }

        test_helper(test_inner);
    }

    #[test]
    fn compile_eval_nested_partials() {
        fn test_inner(mem: &MutatorView) -> Result<(), RuntimeError> {
            // this test evaluates nested Partial applications in function position
            let a_fn = "(def isit (a b) (is? a b))";

            let query1 = "((isit 'x) 'x)";
            let query2 = "((isit 'x) 'y)";

            let t = Thread::alloc(mem)?;

            eval_helper(mem, t, a_fn)?;

            let result = eval_helper(mem, t, query1)?;
            assert!(result == mem.lookup_sym("true"));

            let result = eval_helper(mem, t, query2)?;
            assert!(result == mem.nil());

            Ok(())
        }

        test_helper(test_inner);
    }

    #[test]
    fn compile_pass_partial_as_param() {
        fn test_inner(mem: &MutatorView) -> Result<(), RuntimeError> {
            // this test passes a Partial as an argument of another function that will call it
            // with it's last argument.
            let isit_fn = "(def isit (a b) (is? a b))";
            let map_fn = "(def map (f v) (f v))";

            let query1 = "(map (isit 'x) 'x)";
            let query2 = "(map (isit 'x) 'y)";

            let t = Thread::alloc(mem)?;

            eval_helper(mem, t, isit_fn)?;
            eval_helper(mem, t, map_fn)?;

            let result = eval_helper(mem, t, query1)?;
            assert!(result == mem.lookup_sym("true"));

            let result = eval_helper(mem, t, query2)?;
            assert!(result == mem.nil());

            Ok(())
        }

        test_helper(test_inner);
    }

    #[test]
    fn compile_simple_let() {
        fn test_inner(mem: &MutatorView) -> Result<(), RuntimeError> {
            // this test compiles a basic let expression
            let expr = "(let (x 'y) x)";

            let t = Thread::alloc(mem)?;

            let result = eval_helper(mem, t, expr)?;
            assert!(result == mem.lookup_sym("y"));

            Ok(())
        }

        test_helper(test_inner);
    }
}
