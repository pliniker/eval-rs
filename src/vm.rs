use std::cell::Cell;

use crate::array::{Array, ArraySize};
use crate::bytecode::{ByteCode, InstructionStream, Opcode};
use crate::containers::{
    Container, FillAnyContainer, HashIndexedAnyContainer, IndexedContainer, SliceableContainer,
    StackContainer,
};
use crate::dict::Dict;
use crate::error::{err_eval, RuntimeError};
use crate::function::{Function, Partial};
use crate::list::List;
use crate::memory::MutatorView;
use crate::pair::Pair;
use crate::safeptr::{CellPtr, MutatorScope, ScopedPtr, TaggedScopedPtr};
use crate::taggedptr::{TaggedPtr, Value};

/// Control flow flags
#[derive(PartialEq)]
pub enum EvalStatus<'guard> {
    Pending,
    Return(TaggedScopedPtr<'guard>),
    Halt,
}

/// A call frame, separate from the register stack
#[derive(Clone)]
pub struct CallFrame {
    function: CellPtr<Function>,
    ip: Cell<ArraySize>,
    base: ArraySize,
}

impl CallFrame {
    pub fn new_main<'guard>(main_fn: ScopedPtr<'guard, Function>) -> CallFrame {
        CallFrame {
            function: CellPtr::new_with(main_fn),
            ip: Cell::new(0),
            base: 0,
        }
    }

    fn new<'guard>(
        function: ScopedPtr<'guard, Function>,
        ip: ArraySize,
        base: ArraySize,
    ) -> CallFrame {
        CallFrame {
            function: CellPtr::new_with(function),
            ip: Cell::new(ip),
            base: base,
        }
    }

    fn as_string<'guard>(&self, guard: &'guard dyn MutatorScope) -> String {
        let function = self.function.get(guard);
        format!("in {}", function)
    }
}

/// A stack of CallFrame instances
pub type CallFrameList = Array<CallFrame>;

/// The set of data structures comprising an execution thread
pub struct Thread {
    frames: CellPtr<CallFrameList>,
    stack: CellPtr<List>,
    globals: CellPtr<Dict>,
    instr: CellPtr<InstructionStream>,
    stack_base: Cell<ArraySize>,
}

impl Thread {
    pub fn alloc<'guard>(
        mem: &'guard MutatorView,
    ) -> Result<ScopedPtr<'guard, Thread>, RuntimeError> {
        // create an empty stack frame array
        let frames = CallFrameList::alloc_with_capacity(mem, 32)?;

        // create a minimal value stack
        let stack = List::alloc_with_capacity(mem, 256)?;
        stack.fill(mem, 256, mem.nil())?;

        // create an empty globals dict
        let globals = Dict::alloc(mem)?;

        // create an empty instruction stream
        let blank_code = ByteCode::alloc(mem)?;
        let instr = InstructionStream::alloc(mem, blank_code)?;

        mem.alloc(Thread {
            frames: CellPtr::new_with(frames),
            stack: CellPtr::new_with(stack),
            globals: CellPtr::new_with(globals),
            instr: CellPtr::new_with(instr),
            stack_base: Cell::new(0),
        })
    }

    fn eval_next_instr<'guard>(
        &self,
        mem: &'guard MutatorView,
    ) -> Result<EvalStatus<'guard>, RuntimeError> {
        let frames = self.frames.get(mem);
        let stack = self.stack.get(mem);
        let globals = self.globals.get(mem);
        let instr = self.instr.get(mem);

        // create a 256-register window into the stack from the stack base
        stack.access_slice(mem, |full_stack| {
            let stack_base = self.stack_base.get() as usize;
            let window = &mut full_stack[stack_base..stack_base + 256];

            let opcode = instr.get_next_opcode(mem)?;

            match opcode {
                Opcode::HALT => return Ok(EvalStatus::Halt),

                Opcode::RETURN => {
                    // write the return value to register 0
                    let reg = instr.get_reg_acc() as usize;
                    let result = window[reg].get_ptr();
                    window[0].set_to_ptr(result);

                    // remove this function's stack frame
                    frames.pop(mem)?;

                    // if we just returned from the last stack frame, program evaluation is complete
                    if frames.length() == 0 {
                        return Ok(EvalStatus::Return(window[0].get(mem)));
                    } else {
                        // otherwise restore the previous stack frame settings
                        let frame = frames.top(mem)?;
                        self.stack_base.set(frame.base);
                        instr.switch_frame(frame.function.get(mem).code(mem), frame.ip.get());
                    }
                }

                Opcode::LOADLIT => {
                    let acc = instr.get_reg_acc() as usize;
                    let literal_ptr = instr.get_literal(mem)?;
                    window[acc].set_to_ptr(literal_ptr);
                }

                Opcode::NIL => {
                    let acc = instr.get_reg_acc() as usize;
                    let reg1 = instr.get_reg1() as usize;

                    let reg1_val = window[reg1].get(mem);

                    match *reg1_val {
                        Value::Nil => window[acc].set(mem.lookup_sym("true")),
                        _ => window[acc].set_to_nil(),
                    }
                }

                Opcode::ATOM => {
                    let acc = instr.get_reg_acc() as usize;
                    let reg1 = instr.get_reg1() as usize;

                    let reg1_val = window[reg1].get(mem);

                    match *reg1_val {
                        Value::Pair(_) => window[acc].set_to_nil(),
                        Value::Nil => window[acc].set_to_nil(),
                        _ => window[acc].set(mem.lookup_sym("true")),
                    }
                }

                Opcode::CAR => {
                    let acc = instr.get_reg_acc() as usize;
                    let reg1 = instr.get_reg1() as usize;

                    let reg1_val = window[reg1].get(mem);

                    match *reg1_val {
                        Value::Pair(p) => window[acc].set_to_ptr(p.first.get_ptr()),
                        Value::Nil => window[acc].set_to_nil(),
                        _ => return Err(err_eval("Parameter to CAR is not a list")),
                    }
                }

                Opcode::CDR => {
                    let acc = instr.get_reg_acc() as usize;
                    let reg1 = instr.get_reg1() as usize;

                    let reg1_val = window[reg1].get(mem);

                    match *reg1_val {
                        Value::Pair(p) => window[acc].set_to_ptr(p.second.get_ptr()),
                        Value::Nil => window[acc].set_to_nil(),
                        _ => return Err(err_eval("Parameter to CDR is not a list")),
                    }
                }

                Opcode::CONS => {
                    let acc = instr.get_reg_acc() as usize;
                    let reg1 = instr.get_reg1() as usize;
                    let reg2 = instr.get_reg2() as usize;

                    let reg1_val = window[reg1].get_ptr();
                    let reg2_val = window[reg2].get_ptr();

                    let new_pair = Pair::new();
                    new_pair.first.set_to_ptr(reg1_val);
                    new_pair.second.set_to_ptr(reg2_val);

                    window[acc].set(mem.alloc_tagged(new_pair)?);
                }

                Opcode::IS => {
                    let acc = instr.get_reg_acc() as usize;
                    let reg1 = instr.get_reg1() as usize;
                    let reg2 = instr.get_reg2() as usize;

                    // compare raw pointers - identity comparison
                    let reg1_val = window[reg1].get_ptr();
                    let reg2_val = window[reg2].get_ptr();

                    if reg1_val == reg2_val {
                        window[acc].set(mem.lookup_sym("true"));
                    } else {
                        window[acc].set(mem.nil());
                    }
                }

                Opcode::JMP => {
                    instr.jump();
                }

                Opcode::JMPT => {
                    let reg = instr.get_reg_acc() as usize;
                    let reg_val = window[reg].get(mem);

                    let true_sym = mem.lookup_sym("true"); // TODO preload keyword syms

                    if reg_val == true_sym {
                        instr.jump()
                    }
                }

                Opcode::JMPNT => {
                    let reg = instr.get_reg_acc() as usize;
                    let reg_val = window[reg].get(mem);

                    let true_sym = mem.lookup_sym("true");

                    if reg_val != true_sym {
                        instr.jump()
                    }
                }

                Opcode::LOADNIL => {
                    let reg = instr.get_reg_acc() as usize;
                    window[reg].set_to_nil();
                }

                Opcode::LOADINT => {
                    let reg = instr.get_reg_acc() as usize;
                    let integer = instr.get_literal_integer();
                    let tagged_ptr = TaggedPtr::literal_integer(integer);
                    window[reg].set_to_ptr(tagged_ptr);
                }

                Opcode::LOADGLOBAL => {
                    let assign_reg = instr.get_reg_acc() as usize;
                    let reg1 = instr.get_reg1() as usize;

                    let reg1_val = window[reg1].get(mem);

                    if let Value::Symbol(_) = *reg1_val {
                        let lookup_result = globals.lookup(mem, reg1_val);

                        match lookup_result {
                            Ok(binding) => window[assign_reg].set(binding),
                            Err(_) => {
                                return Err(err_eval(&format!(
                                    "Symbol {} is not bound to a value",
                                    reg1_val
                                )))
                            }
                        }
                    } else {
                        return Err(err_eval("Cannot lookup global for non-symbol type"));
                    }
                }

                Opcode::STOREGLOBAL => {
                    let assign_reg = instr.get_reg_acc() as usize;
                    let reg1 = instr.get_reg1() as usize;

                    let assign_reg_val = window[assign_reg].get(mem);
                    if let Value::Symbol(_) = *assign_reg_val {
                        let reg1_val = window[reg1].get(mem);
                        globals.assoc(mem, assign_reg_val, reg1_val)?;
                    } else {
                        return Err(err_eval("Cannot bind global to non-symbol type"));
                    }
                }

                Opcode::CALL => {
                    let function_reg = instr.get_reg1() as usize;
                    let result_reg = instr.get_reg_acc() as usize;
                    let arg_count = instr.get_reg2();

                    let binding = window[function_reg].get(mem);

                    // To avoid duplicating code in function and partial application cases,
                    // this is declared as a closure so it can access local variables
                    let new_call_frame = |function| -> Result<(), RuntimeError> {
                        // Modify the current call frame, saving the return ip
                        let current_frame_ip = instr.get_next_ip();
                        frames.access_slice(mem, |f| {
                            f.last()
                                .expect("No CallFrames in slice!")
                                .ip
                                .set(current_frame_ip)
                        });

                        // Create a new call frame, pushing it to the frame stack
                        let new_stack_base = self.stack_base.get() + result_reg as ArraySize;
                        let frame = CallFrame::new(function, 0, new_stack_base);
                        frames.push(mem, frame)?;

                        // Update the instruction stream to point to the new function
                        let code = function.code(mem);
                        self.stack_base.set(new_stack_base);
                        instr.switch_frame(code, 0);

                        // Ensure the stack has 256 registers allocated
                        // TODO reset to nil to avoid accidental leakage of previous call values
                        // TODO Ruh-roh we shouldn't be able to modify the stack size from
                        // within an access_slice() call :grimace:
                        stack.fill(mem, new_stack_base + 256, mem.nil())?;

                        Ok(())
                    };

                    // Handle the two similar-but-different cases: this might be a Function object
                    // or a Partial application object
                    match *binding {
                        Value::Function(function) => {
                            let arity = function.arity();

                            if arg_count < arity {
                                // Too few args, return a Partial object
                                let args_start = result_reg + 1;
                                let args_end = args_start + arg_count as usize;

                                let partial =
                                    Partial::alloc(mem, function, &window[args_start..args_end])?;

                                window[result_reg].set(partial.as_tagged(mem));

                                return Ok(EvalStatus::Pending);
                            } else if arg_count > arity {
                                // Too many args, we haven't got a continuations stack
                                return Err(err_eval(&format!(
                                    "Function {} expected {} arguments, got {}",
                                    binding,
                                    function.arity(),
                                    arg_count
                                )));
                            }

                            new_call_frame(function)?;
                        }

                        Value::Partial(partial) => {
                            let arity = partial.arity();

                            if arg_count == 0 {
                                // Partial is unchanged, no args added
                                window[result_reg].set(partial.as_tagged(mem));
                                return Ok(EvalStatus::Pending);
                            } else if arg_count < arity {
                                // Too few args, bake a new Partial from the existing one, adding the new
                                // arguments
                                let args_start = result_reg + 1;
                                let args_end = args_start + arg_count as usize;

                                let new_partial = Partial::alloc_clone(
                                    mem,
                                    partial,
                                    &window[args_start..args_end],
                                )?;

                                window[result_reg].set(new_partial.as_tagged(mem));

                                return Ok(EvalStatus::Pending);
                            } else if arg_count > arity {
                                // Too many args, we haven't got a continuations stack
                                return Err(err_eval(&format!(
                                    "Partial {} expected {} arguments, got {}",
                                    binding,
                                    partial.arity(),
                                    arg_count
                                )));
                            }

                            // Shunt _call_ args back into the window to make space for the
                            // partially applied args
                            let push_dist = partial.used();
                            let from_reg = result_reg as usize + 1;
                            let to_reg = from_reg + push_dist as usize;
                            for index in (0..arg_count as usize).rev() {
                                window[to_reg + index] = window[from_reg + index].clone();
                            }

                            // copy args from Partial to the register window
                            let args = partial.args(mem);
                            let start_reg = result_reg + 1;
                            args.access_slice(mem, |items| {
                                for (index, item) in items.iter().enumerate() {
                                    window[start_reg + index] = item.clone();
                                }
                            });

                            new_call_frame(partial.function(mem))?;
                        }

                        _ => return Err(err_eval("Type is not callable")),
                    }
                }

                Opcode::COPYREG => {
                    let reg_acc = instr.get_reg_acc() as usize;
                    let reg1 = instr.get_reg1() as usize;
                    window[reg_acc] = window[reg1].clone();
                }

                Opcode::LOADNONLOCAL => {
                    let reg_acc = stack_base + instr.get_reg_acc() as usize;

                    let reg = instr.get_reg1() as ArraySize;
                    let frame_offset = instr.get_reg2() as ArraySize;

                    let frame = frames.get(mem, frames.length() - 1 - frame_offset)?;
                    let frame_base = frame.base;
                    let nonlocal_reg = frame_base + reg;

                    let value = &full_stack[nonlocal_reg as usize];
                    full_stack[reg_acc] = value.clone();
                }

                Opcode::ADD => unimplemented!(),

                Opcode::SUB => unimplemented!(),

                Opcode::MUL => unimplemented!(),

                Opcode::DIVINTEGER => unimplemented!(),
            }

            Ok(EvalStatus::Pending)
        })
    }

    /// Given an InstructionStream, execute up to max_instr more instructions
    fn vm_eval_stream<'guard>(
        &self,
        mem: &'guard MutatorView,
        code: ScopedPtr<'guard, ByteCode>,
        max_instr: ArraySize,
    ) -> Result<EvalStatus<'guard>, RuntimeError> {
        let instr = self.instr.get(mem);
        instr.switch_frame(code, 0);

        for _ in 0..max_instr {
            match self.eval_next_instr(mem) {
                // Evaluation paused or completed without error
                Ok(exit_cond) => match exit_cond {
                    EvalStatus::Return(value) => return Ok(EvalStatus::Return(value)),
                    EvalStatus::Halt => return Ok(EvalStatus::Halt),
                    _ => (),
                },

                // Evaluation hit an error
                Err(rt_error) => {
                    // unwind the stack, printing a trace
                    let frames = self.frames.get(mem);

                    // Print a stack trace if the error is multiple call frames deep
                    frames.access_slice(mem, |window| {
                        if window.len() > 1 {
                            println!("Error traceback:");
                        }

                        for frame in &window[1..] {
                            println!("  {}", frame.as_string(mem));
                        }
                    });

                    // Unwind by clearing all frames from the stack
                    frames.clear(mem)?;
                    self.stack_base.set(0);

                    return Err(rt_error);
                }
            }
        }

        Ok(EvalStatus::Pending)
    }

    /// Evaluate a whole block of byte code
    pub fn quick_vm_eval<'guard>(
        &self,
        mem: &'guard MutatorView,
        function: ScopedPtr<'guard, Function>,
    ) -> Result<TaggedScopedPtr<'guard>, RuntimeError> {
        let mut status = EvalStatus::Pending;

        let frames = self.frames.get(mem);
        frames.push(mem, CallFrame::new_main(function))?;

        let code = function.code(mem);

        while status == EvalStatus::Pending {
            status = self.vm_eval_stream(mem, code, 1024)?;
            match status {
                EvalStatus::Return(value) => return Ok(value),
                EvalStatus::Halt => return Err(err_eval("Program halted")),
                _ => (),
            }
        }

        Err(err_eval("Unexpected end of evaluation"))
    }
}
