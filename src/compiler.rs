use std::cell::Cell;
use std::collections::HashMap;

use crate::array::{Array, ArraySize};
use crate::bytecode::{ByteCode, JumpOffset, Opcode, Register, JUMP_UNKNOWN};
use crate::containers::AnyContainerFromSlice;
use crate::error::{err_eval, RuntimeError};
use crate::function::Function;
use crate::list::List;
use crate::memory::MutatorView;
use crate::pair::{value_from_1_pair, values_from_2_pairs, vec_from_pairs};
use crate::safeptr::{CellPtr, ScopedPtr, TaggedScopedPtr};
use crate::taggedptr::Value;

/// A variable can be of three different kinds depending on how a closure refers to it.
#[derive(Copy, Clone, PartialEq)]
enum VariableKind {
    /// An Unclosed variable is one never refered to by a closure
    Unclosed,
    /// An Upvalue variable is one refered to by a closure and must be converted to an Upvalue
    /// when it goes out of scope
    Upvalue,
    /// To construct a closure, a lambda is lifted so that all free variables are converted into
    /// function parameters. An UpvalueParam is a free variable converted to a parameter. Such a
    /// variable must be treated as an invisible binding by further nested scopes. As an example,
    /// suppose function F binds variable x and contains a nested function G that refers to x. G
    /// then gets it's own UpvalueParam reference to x. Suppose then that function G contains a
    /// nested function H that also refers to x. H should refer to F's binding of x, not G's.
    UpvalueParam,
}

/// A variable is a named register. It has compile time metadata about how it is used by closures.
struct Variable {
    register: Cell<Register>,
    kind: Cell<VariableKind>,
}

impl Variable {
    /// A variable can be initialised as Unclosed
    fn new_unclosed(register: Register) -> Variable {
        Variable {
            register: Cell::new(register),
            kind: Cell::new(VariableKind::Unclosed),
        }
    }

    /// A variable can be initialized as an UpvalueParam
    fn new_upvalue_param(register: Register) -> Variable {
        Variable {
            register: Cell::new(register),
            kind: Cell::new(VariableKind::UpvalueParam),
        }
    }

    fn register(&self) -> Register {
        self.register.get()
    }

    fn kind(&self) -> VariableKind {
        self.kind.get()
    }

    /// Convert the variable to an Upvalue. This is not a valid operation for a variable that is an
    /// UpvalueParam.
    fn make_upvalue(&self) {
        if self.kind.get() == VariableKind::UpvalueParam {
            panic!("Cannot convert an UpvalueParam variable to an Upvalue!");
        }
        self.kind.set(VariableKind::Upvalue);
    }

    /// Pushing an UpvalueParam into the parameter scope of a function forces all other registers
    /// to increment by 1
    fn increment_register(&self) {
        self.register.set(self.register.get() + 1);
    }
}

/// A Scope contains a set of local variable to register bindings
struct Scope {
    /// symbol -> variable mapping
    bindings: HashMap<String, Variable>,
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
            _ => return Err(err_eval("A binding name must be a symbol")),
        };

        self.bindings
            .insert(name_string, Variable::new_unclosed(reg));

        Ok(())
    }

    // Push a block of bindings into this scope, returning the next register available
    // after these bound registers. All these variables will be Unclosed by default.
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
    ) -> Result<Option<&Variable>, RuntimeError> {
        let name_string = match *name {
            Value::Symbol(s) => String::from(s.as_str(&name)),
            _ => {
                return Err(err_eval(
                    "Cannot lookup a variable bound to a non-symbol type",
                ))
            }
        };

        match self.bindings.get(&name_string) {
            Some(var) => Ok(Some(var)),
            None => Ok(None),
        }
    }

    /// To construct a closure, a lambda is lifted so that all free variables are converted into
    /// function parameters. All existing parameter and evaluation registers must be punted
    /// further back into the register window to make space. This function iterates over all
    /// instructions, incrementing each register by 1, to make space for 1 free variable param
    /// at the head of the register window.
    fn increment_all_registers(&mut self) {
        for entry in self.bindings.iter_mut() {
            entry.1.increment_register();
        }
    }
}

/// A Scope instance represents a set of nested local binding scopes for a single function
/// definition.
struct Locals<'parent> {
    scopes: Vec<Scope>,
    parent: Option<&'parent Locals<'parent>>,
}

impl<'parent> Locals<'parent> {
    fn new(parent: Option<&'parent Locals<'parent>>) -> Locals<'parent> {
        Locals {
            scopes: Vec::new(),
            parent,
        }
    }

    /// Search for a binding, following parent scopes.
    fn lookup_binding<'guard>(
        &self,
        name: TaggedScopedPtr<'guard>,
    ) -> Result<Option<(u8, &Variable)>, RuntimeError> {
        //  return value should be (count-of-parent-functions-followed, Variable)

        // The depth is the number of parent nesting functions searched for a variable
        let mut depth: u8 = 0;

        let mut locals = Some(self);
        while let Some(l) = locals {
            for scope in l.scopes.iter().rev() {
                if let Some(var) = scope.lookup_binding(name)? {
                    // If the matched variable is an UpvalueParam and the call frame depth is
                    // nonzero, discard this result and keep looking for the original Unclosed or
                    // Upvalue variable
                    if depth == 0 || var.kind() != VariableKind::UpvalueParam {
                        return Ok(Some((depth, var)));
                    }
                }
            }
            locals = l.parent;
            depth += 1;
        }

        Ok(None)
    }

    /// To construct a closure, a lambda is lifted so that all free variables are converted into
    /// function parameters. All existing parameter and evaluation registers must be punted
    /// further back into the register window to make space. This function iterates over all
    /// instructions, incrementing each register by 1, to make space for 1 free variable param
    /// at the head of the register window.
    fn increment_all_registers(&mut self) {
        for scope in self.scopes.iter_mut() {
            scope.increment_all_registers();
        }
    }
}

/// This is a simple, naive compiler of a nested s-expression Pair (Cons cell) data structure.
/// It compiles for the VM in vm.rs, a sliding-window register machine.  Register allocation
/// follows the expression nesting structure, essentially pushing and popping register locations
/// from the evaluation tree as scopes are entered and exited. This is super simple but not
/// the most efficient scheme possible.
struct Compiler<'parent> {
    bytecode: CellPtr<ByteCode>,
    next_reg: Register,
    // optional function name
    name: Option<String>,
    // function-local nested scopes bindings list (including parameters at outer level)
    locals: Locals<'parent>,
}

impl<'parent> Compiler<'parent> {
    /// Instantiate a new nested function-level compiler
    fn new<'guard>(
        mem: &'guard MutatorView,
        parent: Option<&'parent Locals<'parent>>,
    ) -> Result<Compiler<'parent>, RuntimeError> {
        Ok(Compiler {
            bytecode: CellPtr::new_with(ByteCode::alloc(mem)?),
            // register 0 is reserved for the return value
            next_reg: 1,
            name: None,
            locals: Locals::new(parent),
        })
    }

    /// Compile an expression that has parameters and possibly a name
    fn compile_function<'guard>(
        mut self,
        mem: &'guard MutatorView,
        name: TaggedScopedPtr<'guard>,
        params: &[TaggedScopedPtr<'guard>],
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
        self.locals.scopes.push(param_scope);

        // validate expression list
        if exprs.len() == 0 {
            return Err(err_eval("A function must have at least one expression"));
        }

        // compile expressions
        let mut result_reg = 0;
        for expr in exprs.iter() {
            result_reg = self.compile_eval(mem, *expr)?;
        }

        // TODO Question: for every Return instruction in the bytecode, if the preceding
        // instruction is a Call, can the Call be converted to a TAILCall?

        let fn_bytecode = self.bytecode.get(mem);
        fn_bytecode.push(mem, Opcode::Return { reg: result_reg })?;

        Ok(Function::alloc(
            mem,
            fn_name,
            fn_params,
            fn_bytecode,
            Array::alloc(mem)?,
        )?)
    }

    /// Compile an expression - this can be an 'atomic' value or a nested function application
    fn compile_eval<'guard>(
        &mut self,
        mem: &'guard MutatorView,
        ast_node: TaggedScopedPtr<'guard>,
    ) -> Result<Register, RuntimeError> {
        match *ast_node {
            Value::Pair(p) => self.compile_apply(mem, p.first.get(mem), p.second.get(mem)),

            Value::Symbol(s) => {
                match s.as_str(mem) {
                    "nil" => {
                        let dest = self.acquire_reg();
                        self.push(mem, Opcode::LoadNil { dest })?;
                        Ok(dest)
                    }

                    "true" => self.push_load_literal(mem, mem.lookup_sym("true")),

                    // Search scopes for a binding; if none do a global lookup
                    _ => {
                        // First search local and nonlocal bindings from inner to outermost scope
                        if let Some((frame_offset, src)) = self.locals.lookup_binding(ast_node)? {
                            if frame_offset > 0 {
                                // nonlocal nonglobal binding
                                // TODO instead of issuing a LoadNonLocal, add this variable to the
                                // parameter scope as an Upvalue and issue a GetUpvalue instruction
                                let dest = self.acquire_reg();
                                self.push(
                                    mem,
                                    Opcode::LoadNonLocal {
                                        dest,
                                        src,
                                        frame_offset,
                                    },
                                )?;
                                return Ok(dest);
                            } else {
                                // local call-frame register
                                return Ok(src);
                            }
                        }

                        // Otherwise do a late-binding global lookup
                        let name = self.push_load_literal(mem, ast_node)?;
                        let dest = name; // reuse the register
                        self.push(mem, Opcode::LoadGlobal { dest, name })?;
                        Ok(dest)
                    }
                }
            }

            _ => self.push_load_literal(mem, ast_node),
        }
    }

    /// Compile a function or special-form application
    fn compile_apply<'guard>(
        &mut self,
        mem: &'guard MutatorView,
        function: TaggedScopedPtr<'guard>,
        args: TaggedScopedPtr<'guard>,
    ) -> Result<Register, RuntimeError> {
        match *function {
            Value::Symbol(s) => match s.as_str(mem) {
                "quote" => self.push_load_literal(mem, value_from_1_pair(mem, args)?),
                "atom?" => self.push_op2(mem, args, |dest, test| Opcode::IsAtom { dest, test }),
                "nil?" => self.push_op2(mem, args, |dest, test| Opcode::IsNil { dest, test }),
                "car" => self.push_op2(mem, args, |dest, reg| Opcode::FirstOfPair { dest, reg }),
                "cdr" => self.push_op2(mem, args, |dest, reg| Opcode::SecondOfPair { dest, reg }),
                "cons" => self.push_op3(mem, args, |dest, reg1, reg2| Opcode::MakePair {
                    dest,
                    reg1,
                    reg2,
                }),
                "cond" => self.compile_apply_cond(mem, args),
                "is?" => self.push_op3(mem, args, |dest, test1, test2| Opcode::IsIdentical {
                    dest,
                    test1,
                    test2,
                }),
                "set" => self.compile_apply_assign(mem, args),
                "def" => self.compile_named_function(mem, args),
                "lambda" => self.compile_anonymous_function(mem, args),
                "\\" => self.compile_anonymous_function(mem, args),
                "let" => self.compile_apply_let(mem, args),
                _ => self.compile_apply_call(mem, function, args),
            },

            // Here we allow the value in the function position to be evaluated dynamically
            _ => self.compile_apply_call(mem, function, args),
        }
    }

    /// Compile a 'cond' application
    /// (cond
    ///   (<if-expr-is-true?>) (<then-expr>)
    ///   (<or-expr-is-true?) (<then-expr>)
    /// )
    /// result is nil if no expression evaluates to true
    fn compile_apply_cond<'guard>(
        &mut self,
        mem: &'guard MutatorView,
        args: TaggedScopedPtr<'guard>,
    ) -> Result<Register, RuntimeError> {
        //
        //   for each arg:
        //     eval cond
        //     if false then jmp -> next
        //     else eval expr
        //     jmp -> end
        //
        let bytecode = self.bytecode.get(mem);

        let mut end_jumps: Vec<ArraySize> = Vec::new();
        let mut last_cond_jump: Option<ArraySize> = None;

        let dest = self.next_reg;

        let mut head = args;
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
                        bytecode.update_jump_offset(mem, address, offset as JumpOffset)?;
                    }

                    // We have a condition to evaluate. If the resut is Not True, jump to the
                    // next condition.
                    self.reset_reg(dest); // reuse this register for condition and dest
                    let test = self.compile_eval(mem, cond)?;
                    let offset = JUMP_UNKNOWN;
                    self.push(mem, Opcode::JumpIfNotTrue { test, offset })?;
                    last_cond_jump = Some(bytecode.last_instruction());

                    // Compile the expression and jump to the end of the entire cond
                    self.reset_reg(dest); // reuse this register for condition and dest
                    let _expr_result = self.compile_eval(mem, expr)?;
                    let offset = JUMP_UNKNOWN;
                    bytecode.push(mem, Opcode::Jump { offset })?;
                    end_jumps.push(bytecode.last_instruction());
                }

                _ => return Err(err_eval("Unexpected end of cond list")),
            }
        }

        // Close out with a default nil result if none of the conditions passed
        if let Some(address) = last_cond_jump {
            self.reset_reg(dest);
            self.push(mem, Opcode::LoadNil { dest })?;
            let offset = bytecode.next_instruction() - address - 1;
            bytecode.update_jump_offset(mem, address, offset as JumpOffset)?;
        }

        // Update all the post-expr jumps to point at the next instruction after the entire cond
        for address in end_jumps.iter() {
            let offset = bytecode.next_instruction() - address - 1;
            bytecode.update_jump_offset(mem, *address, offset as JumpOffset)?;
        }

        Ok(dest)
    }

    /// Assignment expression - evaluate the two expressions, binding the result of the first
    /// to the (hopefully) symbol provided by the second
    /// (set <identifier-expr> <expr>)
    fn compile_apply_assign<'guard>(
        &mut self,
        mem: &'guard MutatorView,
        params: TaggedScopedPtr<'guard>,
    ) -> Result<Register, RuntimeError> {
        let (first, second) = values_from_2_pairs(mem, params)?;
        let src = self.compile_eval(mem, second)?;
        let name = self.compile_eval(mem, first)?;
        self.push(mem, Opcode::StoreGlobal { src, name })?;
        Ok(src)
    }

    /// (lambda (args) (exprs))
    /// OR
    /// (\ (args) (exprs))
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
        let fn_object = compile_function(mem, Some(&self.locals), mem.nil(), &fn_params, fn_exprs)?;

        // load the function object as a literal
        self.push_load_literal(mem, fn_object)

        // TODO if fn_object has nonlocal refs, compile a MakeClosure instruction in addition
    }

    /// (def name (args) (expr))
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
        let fn_object = compile_function(mem, Some(&self.locals), fn_name, &fn_params, fn_exprs)?;

        // load the function object as a literal and associate it with a global name
        // TODO store in local scope if we're nested in an expression
        let name = self.push_load_literal(mem, fn_name)?;
        let src = self.push_load_literal(mem, fn_object)?;
        self.push(mem, Opcode::StoreGlobal { src, name })?;

        Ok(src)

        // TODO if fn_object has nonlocal refs, compile a MakeClosure instruction in addition
    }

    /// (name <arg-expr-1> <arg-expr-n>)
    fn compile_apply_call<'guard>(
        &mut self,
        mem: &'guard MutatorView,
        function_expr: TaggedScopedPtr<'guard>,
        args: TaggedScopedPtr<'guard>,
    ) -> Result<Register, RuntimeError> {
        // allocate a register for the return value
        let dest = self.acquire_reg();

        // evaluate arguments first
        let arg_list = vec_from_pairs(mem, args)?;
        let arg_count = arg_list.len() as u8;

        for arg in arg_list {
            let src = self.compile_eval(mem, arg)?;
            // if a local variable register was returned, we need to copy the register to the arg
            // list. Bound registers are necessarily lower indexes than where the function call is
            // situated because expression scope and register acquisition progresses the register
            // index in use.
            if src <= dest {
                let dest = self.acquire_reg();
                self.push(mem, Opcode::CopyRegister { dest, src })?;
            }
        }

        // put the function pointer in the last register of the call so it'll be discarded
        let function = self.compile_eval(mem, function_expr)?;
        self.push(
            mem,
            Opcode::Call {
                function,
                dest,
                arg_count,
            },
        )?;

        // ignore use of any registers beyond the result once the call is complete
        self.reset_reg(dest + 1);
        Ok(dest)
    }

    /// Basic non-recursive let expressions
    /// (let
    ///   ((<name> <expr>)
    ///    (<name> <expr>))
    ///   (<expr>)
    /// )
    fn compile_apply_let<'guard>(
        &mut self,
        mem: &'guard MutatorView,
        args: TaggedScopedPtr<'guard>,
    ) -> Result<Register, RuntimeError> {
        let let_expr = vec_from_pairs(mem, args)?;
        if let_expr.len() < 2 {
            return Err(err_eval("A let expression must have at least 2 arguments"));
        }

        // the binding expressions should be a pair-list itself, and each expression another
        // pair list of length 2.  Convert it to a Vec<(name, expr)> structure for convenience.
        let let_exprs: Vec<(TaggedScopedPtr<'guard>, TaggedScopedPtr<'guard>)> = {
            let vec_of_pairs = vec_from_pairs(mem, let_expr[0])?;
            let mut vec_of_tuples = Vec::new();
            for pairs in &vec_of_pairs {
                vec_of_tuples.push(values_from_2_pairs(mem, *pairs)?);
            }
            vec_of_tuples
        };

        // acquire a let expression dest reg
        let dest = self.acquire_reg();

        // get the names of each binding to push a scope, assigning registers post-result for
        // each binding
        let names: Vec<TaggedScopedPtr<'guard>> = let_exprs.iter().map(|tup| tup.0).collect();

        let mut let_scope = Scope::new();
        self.next_reg = let_scope.push_bindings(&names, self.next_reg)?;
        self.locals.scopes.push(let_scope);

        // compile each binding expression
        for (name, expr) in let_exprs {
            let src = self.compile_eval(mem, expr)?;
            let dest = self.compile_eval(mem, name)?;
            // TODO - more efficient to be able to write the result directly to the let binding reg
            self.push(mem, Opcode::CopyRegister { dest, src })?;
        }

        // compile the expressions after the bindings
        let result_exprs = &let_expr[1..];

        for expr in result_exprs {
            let src = self.compile_eval(mem, *expr)?;
            // TODO - more efficient to be able to write the result directly to the let binding reg
            self.push(mem, Opcode::CopyRegister { dest, src })?;
        }

        // finish up - pop the scope, de-scope all registers except the result, return the result
        self.locals.scopes.pop();
        self.reset_reg(dest + 1);
        Ok(dest)
    }

    /// Push an instruction to the function bytecode list
    fn push<'guard>(&mut self, mem: &'guard MutatorView, op: Opcode) -> Result<(), RuntimeError> {
        self.bytecode.get(mem).push(mem, op)
    }

    /// Push an instruction with a result and a single argument to the function bytecode list
    fn push_op2<'guard, F>(
        &mut self,
        mem: &'guard MutatorView,
        params: TaggedScopedPtr<'guard>,
        f: F,
    ) -> Result<Register, RuntimeError>
    where
        F: Fn(Register, Register) -> Opcode,
    {
        let result = self.acquire_reg();
        let reg1 = self.compile_eval(mem, value_from_1_pair(mem, params)?)?;
        self.bytecode.get(mem).push(mem, f(result, reg1))?;
        Ok(result)
    }

    /// Push an instruction with a result and two arguments to the function bytecode list
    fn push_op3<'guard, F>(
        &mut self,
        mem: &'guard MutatorView,
        params: TaggedScopedPtr<'guard>,
        f: F,
    ) -> Result<Register, RuntimeError>
    where
        F: Fn(Register, Register, Register) -> Opcode,
    {
        let result = self.acquire_reg();
        let (first, second) = values_from_2_pairs(mem, params)?;
        let reg1 = self.compile_eval(mem, first)?;
        let reg2 = self.compile_eval(mem, second)?;
        self.bytecode.get(mem).push(mem, f(result, reg1, reg2))?;
        Ok(result)
    }

    // Push a literal onto the literals list and a load instruction onto the bytecode list
    fn push_load_literal<'guard>(
        &mut self,
        mem: &'guard MutatorView,
        literal: TaggedScopedPtr<'guard>,
    ) -> Result<Register, RuntimeError> {
        let result = self.acquire_reg();
        let lit_id = self.bytecode.get(mem).push_lit(mem, literal)?;
        self.bytecode.get(mem).push_loadlit(mem, result, lit_id)?;
        Ok(result)
    }

    // this is a naive way of allocating registers - every result gets it's own register
    fn acquire_reg(&mut self) -> Register {
        // TODO check overflow
        let reg = self.next_reg;
        self.next_reg += 1;
        reg
    }

    // this is a naive way of allocating registers - every result gets it's own register
    fn acquire_dest_reg(&mut self, push_dest: Option<Register>) -> Result<Register, RuntimeError> {
        if let Some(dest) = push_dest {
            Ok(dest)
        } else {
            let dest = self.next_reg;
            // check for 8 bit overflow. A function cannot allocate more than 255 registers for
            // itself.
            if dest == 255 {
                return Err(err_eval(
                    "Compiler ran out of registers for this function, consider reducing complexity",
                ));
            }
            self.next_reg += 1;
            Ok(dest)
        }
    }

    // reset the next register back to the given one so that it is reused
    fn reset_reg(&mut self, reg: Register) {
        self.next_reg = reg
    }
}

/// Compile a function - parameters and expression, returning a tagged Function object
fn compile_function<'guard, 'scope>(
    mem: &'guard MutatorView,
    parent: Option<&'scope Locals<'scope>>,
    name: TaggedScopedPtr<'guard>,
    params: &[TaggedScopedPtr<'guard>],
    exprs: &[TaggedScopedPtr<'guard>],
) -> Result<TaggedScopedPtr<'guard>, RuntimeError> {
    let compiler = Compiler::new(mem, parent)?;
    Ok(compiler
        .compile_function(mem, name, params, exprs)?
        .as_tagged(mem))
}

/// Compile the given AST and return an anonymous Function object
pub fn compile<'guard>(
    mem: &'guard MutatorView,
    ast: TaggedScopedPtr<'guard>,
) -> Result<ScopedPtr<'guard, Function>, RuntimeError> {
    let compiler = Compiler::new(mem, None)?;
    compiler.compile_function(mem, mem.nil(), &[], &[ast])
}

/// INTEGRATION TESTS
/// TODO - move to a separate module
#[cfg(test)]
mod integration {
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
            let expr = "(let ((x 'y)) x)";

            let t = Thread::alloc(mem)?;

            let result = eval_helper(mem, t, expr)?;
            assert!(result == mem.lookup_sym("y"));

            Ok(())
        }

        test_helper(test_inner);
    }

    #[test]
    fn compile_function_with_simple_let() {
        fn test_inner(mem: &MutatorView) -> Result<(), RuntimeError> {
            // this test compiles a let expression that deconstructs and reconstructs a pair list
            let a_fn = "(def deconrecon (list) (let ((a (car list)) (b (cdr list))) (cons a b)))";
            let query = "(deconrecon '(x y z z y))";

            let t = Thread::alloc(mem)?;

            eval_helper(mem, t, a_fn)?;

            let result = eval_helper(mem, t, query)?;

            let result = vec_from_pairs(mem, result)?;
            let sym_x = mem.lookup_sym("x");
            let sym_y = mem.lookup_sym("y");
            let sym_z = mem.lookup_sym("z");
            assert!(result == &[sym_x, sym_y, sym_z, sym_z, sym_y]);

            Ok(())
        }

        test_helper(test_inner);
    }

    #[test]
    fn compile_function_with_lambda_with_nonlocal_ref() {
        fn test_inner(mem: &MutatorView) -> Result<(), RuntimeError> {
            // this test compiles a function containing a lambda that references a nonlocal
            let head_fn = "(def head (a) (let ((inner (\\ () (car a)))) (inner)))";
            let query = "(head '(x y z z y))";

            let t = Thread::alloc(mem)?;

            eval_helper(mem, t, head_fn)?;

            let result = eval_helper(mem, t, query)?;
            assert!(result == mem.lookup_sym("x"));

            Ok(())
        }

        test_helper(test_inner);
    }
}
