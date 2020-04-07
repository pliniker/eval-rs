use std::cell::Cell;

use crate::array::{Array, ArraySize};
use crate::bytecode::{ByteCode, InstructionStream, Opcode};
use crate::containers::{
    Container, FillAnyContainer, HashIndexedAnyContainer, IndexedAnyContainer, IndexedContainer,
    SliceableContainer, StackAnyContainer, StackContainer,
};
use crate::dict::Dict;
use crate::error::{err_eval, RuntimeError};
use crate::function::Function;
use crate::list::List;
use crate::memory::MutatorView;
use crate::pair::Pair;
use crate::safeptr::{CellPtr, MutatorScope, ScopedPtr, TaggedScopedPtr};
use crate::taggedptr::Value;

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
        let ip = self.ip.get();
        format!("{}: {}", function, ip)
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
                    window[0].set(window[reg].get(mem));

                    // remove this function's stack frame
                    frames.pop(mem)?;

                    // if we just returned from the last stack frame, program evaluation is complete
                    if frames.length() == 0 {
                        return Ok(EvalStatus::Return(window[0].get(mem)));
                    } else {
                        // otherwise restore the previous stack frame settings
                        let frame = frames.top(mem)?;
                        self.stack_base.set(frame.base);
                        instr.switch_frame(frame.function.get(mem).code.get(mem), frame.ip.get());
                    }
                }

                Opcode::LOADLIT => {
                    let acc = instr.get_reg_acc() as usize;
                    let literal = instr.get_literal(mem)?;
                    window[acc].set(literal);
                }

                Opcode::NIL => {
                    let acc = instr.get_reg_acc() as usize;
                    let reg1 = instr.get_reg1() as usize;

                    let reg1_val = window[reg1].get(mem);

                    match *reg1_val {
                        Value::Nil => window[acc].set(mem.lookup_sym("true")),
                        _ => window[acc].set(mem.nil()),
                    }
                }

                Opcode::ATOM => {
                    let acc = instr.get_reg_acc() as usize;
                    let reg1 = instr.get_reg1() as usize;

                    let reg1_val = window[reg1].get(mem);

                    match *reg1_val {
                        Value::Pair(_) => window[acc].set(mem.nil()),
                        Value::Nil => window[acc].set(mem.nil()),
                        _ => window[acc].set(mem.lookup_sym("true")),
                    }
                }

                Opcode::CAR => {
                    let acc = instr.get_reg_acc() as usize;
                    let reg1 = instr.get_reg1() as usize;

                    let reg1_val = window[reg1].get(mem);

                    match *reg1_val {
                        Value::Pair(p) => window[acc].set(p.first.get(mem)),
                        Value::Nil => window[acc].set(mem.nil()),
                        _ => return Err(err_eval("Parameter to CAR is not a list")),
                    }
                }

                Opcode::CDR => {
                    let acc = instr.get_reg_acc() as usize;
                    let reg1 = instr.get_reg1() as usize;

                    let reg1_val = window[reg1].get(mem);

                    match *reg1_val {
                        Value::Pair(p) => window[acc].set(p.second.get(mem)),
                        Value::Nil => window[acc].set(mem.nil()),
                        _ => return Err(err_eval("Parameter to CDR is not a list")),
                    }
                }

                Opcode::CONS => {
                    let acc = instr.get_reg_acc() as usize;
                    let reg1 = instr.get_reg1() as usize;
                    let reg2 = instr.get_reg2() as usize;

                    let reg1_val = window[reg1].get(mem);
                    let reg2_val = window[reg2].get(mem);

                    let new_pair = Pair::new();
                    new_pair.first.set(reg1_val);
                    new_pair.second.set(reg2_val);

                    window[acc].set(mem.alloc_tagged(new_pair)?);
                }

                Opcode::IS => {
                    let acc = instr.get_reg_acc() as usize;
                    let reg1 = instr.get_reg1() as usize;
                    let reg2 = instr.get_reg2() as usize;

                    let reg1_val = window[reg1].get(mem);
                    let reg2_val = window[reg2].get(mem);

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
                    let reg = instr.get_reg1() as usize;
                    let reg_val = window[reg].get(mem);

                    let true_sym = mem.lookup_sym("true"); // TODO preload keyword syms

                    if reg_val == true_sym {
                        instr.jump()
                    }
                }

                Opcode::JMPNT => {
                    let reg = instr.get_reg1() as usize;
                    let reg_val = window[reg].get(mem);

                    let true_sym = mem.lookup_sym("true");

                    if reg_val != true_sym {
                        instr.jump()
                    }
                }

                Opcode::LOADNIL => {
                    let reg = instr.get_reg1() as usize;
                    window[reg].set(mem.nil());
                }

                Opcode::LOADGLOBAL => {
                    let assign_reg = instr.get_reg_acc() as usize;
                    let reg1 = instr.get_reg1() as usize;

                    let reg1_val = window[reg1].get(mem);

                    if let Value::Symbol(_) = *reg1_val {
                        let lookup_result = globals.lookup(mem, reg1_val);

                        match lookup_result {
                            Ok(binding) => window[assign_reg].set(binding),
                            Err(_) => return Err(err_eval("Symbol not bound to value")),
                        }
                    } else {
                        return Err(err_eval("Cannot lookup value for non-symbol type"));
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
                        return Err(err_eval("Cannot bind value to non-symbol type"));
                    }
                }

                Opcode::CALL => {
                    // TODO params

                    let result_reg = instr.get_reg_acc() as usize;
                    let function_reg = instr.get_reg1() as usize;

                    let binding = window[function_reg].get(mem);

                    match *binding {
                        Value::Function(function) => {
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
                            let code = function.code.get(mem);
                            self.stack_base.set(new_stack_base);
                            instr.switch_frame(code, 0);

                            // Ensure the stack has 256 registers allocated
                            stack.fill(mem, new_stack_base + 256, mem.nil())?;
                        }

                        Value::Partial(_partial) => unimplemented!(),

                        _ => return Err(err_eval("Type is not callable")),
                    }
                }
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

                    // Print a stack trace
                    frames.access_slice(mem, |window| {
                        for item in &window[1..] {
                            println!("    {}", item.as_string(mem));
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

        let code = function.code.get(mem);

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
