use std::cell::Cell;

use crate::array::{Array, ArraySize};
use crate::bytecode::{ByteCode, InstructionStream, Opcode};
use crate::containers::{
    Container, HashIndexedAnyContainer, IndexedAnyContainer, SliceableContainer, StackAnyContainer,
    StackContainer,
};
use crate::dict::Dict;
use crate::error::{err_eval, RuntimeError};
use crate::function::Function;
use crate::list::List;
use crate::memory::MutatorView;
use crate::pair::Pair;
use crate::safeptr::{CellPtr, ScopedPtr, TaggedCellPtr, TaggedScopedPtr};
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
    pub fn new<'guard>(
        mem: &'guard MutatorView,
    ) -> Result<ScopedPtr<'guard, Thread>, RuntimeError> {
        // create a stack frame list with a 'main' entry
        let frames = mem.alloc(CallFrameList::with_capacity(mem, 32)?)?;

        // create a minimal value stack
        let stack = mem.alloc(List::with_capacity(mem, 256)?)?;
        for _ in 0..256 {
            StackAnyContainer::push(&*stack, mem, mem.nil())?;
        }

        // create an empty globals dict
        let globals = mem.alloc(Dict::new())?;

        // create an empty instruction stream
        let blank_code = ByteCode::new(mem)?;
        let main_fn = Function::new(mem, mem.nil(), 0, blank_code)?;

        let instr = mem.alloc(InstructionStream::new(blank_code))?;

        frames.push(mem, CallFrame::new_main(main_fn))?;

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

        stack.access_slice(mem, |full_stack| {
            let stack_base = self.stack_base.get() as usize;
            let window = &mut full_stack[stack_base..stack_base + 256];

            let opcode = instr.get_next_opcode(mem)?;

            match opcode {
                Opcode::HALT => return Ok(EvalStatus::Halt),

                Opcode::RETURN => {
                    let reg = instr.get_reg_acc() as ArraySize;
                    return Ok(EvalStatus::Return(stack.get(mem, reg)?));
                }

                Opcode::LOADLIT => {
                    let acc = instr.get_reg_acc() as ArraySize;
                    let literal = instr.get_literal(mem)?;
                    stack.set(mem, acc, literal)?;
                }

                Opcode::NIL => {
                    let acc = instr.get_reg_acc() as ArraySize;
                    let reg1 = instr.get_reg1() as ArraySize;

                    let reg1_val = stack.get(mem, reg1)?;

                    match *reg1_val {
                        Value::Nil => stack.set(mem, acc, mem.lookup_sym("true"))?,
                        _ => stack.set(mem, acc, mem.nil())?,
                    }
                }

                Opcode::ATOM => {
                    let acc = instr.get_reg_acc() as ArraySize;
                    let reg1 = instr.get_reg1() as ArraySize;

                    let reg1_val = stack.get(mem, reg1)?;

                    match *reg1_val {
                        Value::Pair(_) => stack.set(mem, acc, mem.nil())?,
                        Value::Nil => stack.set(mem, acc, mem.nil())?,
                        _ => stack.set(mem, acc, mem.lookup_sym("true"))?,
                    }
                }

                Opcode::CAR => {
                    let acc = instr.get_reg_acc() as ArraySize;
                    let reg1 = instr.get_reg1() as ArraySize;

                    let reg1_val = stack.get(mem, reg1)?;

                    match *reg1_val {
                        Value::Pair(p) => stack.set(mem, acc, p.first.get(mem))?,
                        Value::Nil => stack.set(mem, acc, mem.nil())?,
                        _ => return Err(err_eval("Parameter to CAR is not a list")),
                    }
                }

                Opcode::CDR => {
                    let acc = instr.get_reg_acc() as ArraySize;
                    let reg1 = instr.get_reg1() as ArraySize;

                    let reg1_val = stack.get(mem, reg1)?;

                    match *reg1_val {
                        Value::Pair(p) => stack.set(mem, acc, p.second.get(mem))?,
                        Value::Nil => stack.set(mem, acc, mem.nil())?,
                        _ => return Err(err_eval("Parameter to CDR is not a list")),
                    }
                }

                Opcode::CONS => {
                    let acc = instr.get_reg_acc() as ArraySize;
                    let reg1 = instr.get_reg1() as ArraySize;
                    let reg2 = instr.get_reg2() as ArraySize;

                    let reg1_val = stack.get(mem, reg1)?;
                    let reg2_val = stack.get(mem, reg2)?;

                    let new_pair = Pair::new();
                    new_pair.first.set(reg1_val);
                    new_pair.second.set(reg2_val);

                    stack.set(mem, acc, mem.alloc_tagged(new_pair)?)?
                }

                Opcode::IS => {
                    let acc = instr.get_reg_acc() as ArraySize;
                    let reg1 = instr.get_reg1() as ArraySize;
                    let reg2 = instr.get_reg2() as ArraySize;

                    let reg1_val = stack.get(mem, reg1)?;
                    let reg2_val = stack.get(mem, reg2)?;

                    if reg1_val == reg2_val {
                        stack.set(mem, acc, mem.lookup_sym("true"))?;
                    } else {
                        stack.set(mem, acc, mem.nil())?;
                    }
                }

                Opcode::JMP => {
                    instr.jump();
                }

                Opcode::JMPT => {
                    let reg = instr.get_reg1() as ArraySize;
                    let reg_val = stack.get(mem, reg)?;

                    let true_sym = mem.lookup_sym("true"); // TODO preload keyword syms

                    if reg_val == true_sym {
                        instr.jump()
                    }
                }

                Opcode::JMPNT => {
                    let reg = instr.get_reg1() as ArraySize;
                    let reg_val = stack.get(mem, reg)?;

                    let true_sym = mem.lookup_sym("true");

                    if reg_val != true_sym {
                        instr.jump()
                    }
                }

                Opcode::LOADNIL => {
                    let reg = instr.get_reg1() as ArraySize;
                    stack.set(mem, reg, mem.nil())?;
                }

                Opcode::LOADGLOBAL => {
                    let reg1 = instr.get_reg1() as ArraySize;
                    let reg1_val = stack.get(mem, reg1)?;

                    if let Value::Symbol(s) = *reg1_val {
                        let lookup_result = globals.lookup(mem, reg1_val);

                        match lookup_result {
                            Ok(binding) => stack.set(mem, reg1, binding)?,
                            Err(_) => return Err(err_eval("Symbol not bound to value")),
                        }
                    } else {
                        return Err(err_eval("Cannot lookup value for non-symbol type"));
                    }
                }

                Opcode::STOREGLOBAL => {
                    let assign_reg = instr.get_reg_acc() as ArraySize;
                    let reg1 = instr.get_reg1() as ArraySize;

                    let assign_reg_val = stack.get(mem, assign_reg)?;
                    if let Value::Symbol(_) = *assign_reg_val {
                        let reg1_val = stack.get(mem, reg1)?;
                        globals.assoc(mem, assign_reg_val, reg1_val)?;
                    } else {
                        return Err(err_eval("Cannot bind value to non-symbol type"));
                    }
                }

                Opcode::CALL => {
                    // TODO params

                    let result_reg = instr.get_reg_acc() as ArraySize;
                    let function_reg = instr.get_reg1() as ArraySize;

                    let binding = stack.get(mem, function_reg)?;

                    match *binding {
                        Value::Function(function) => {
                            // create a new call frame:
                            //   function being called
                            //   base = current base + index of result register
                            //
                            // get current ip
                            //
                            // modify current call frame:
                            //   set ip to current ip
                            //
                            // represent current register window as a slice, run this
                            // function as an SliceableContainer::access_slice() function

                            // Modify the current call frame, saving the return ip
                            let current_frame_ip = instr.get_next_ip();
                            frames.access_slice(mem, |f| {
                                f.last()
                                    .expect("No CallFrames in slice!")
                                    .ip
                                    .set(current_frame_ip)
                            });

                            // Create a new call frame, pushing it to the frame stack
                            let new_stack_base = self.stack_base.get() + result_reg;
                            let frame = CallFrame::new(function, 0, new_stack_base);
                            frames.push(mem, frame);

                            // Update the instruction stream
                            let code = function.code.get(mem);
                            instr.switch_frame(code, 0);

                            unimplemented!() // TODO represent registers as slice
                        }

                        Value::Partial(_p) => unimplemented!(),

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
            match self.eval_next_instr(mem)? {
                EvalStatus::Return(value) => return Ok(EvalStatus::Return(value)),
                EvalStatus::Halt => return Ok(EvalStatus::Halt),
                _ => (),
            }
        }
        Ok(EvalStatus::Pending)
    }

    /// Evaluate a whole block of byte code
    pub fn quick_vm_eval<'guard>(
        &self,
        mem: &'guard MutatorView,
        code: ScopedPtr<'guard, ByteCode>,
    ) -> Result<TaggedScopedPtr<'guard>, RuntimeError> {
        let stream = mem.alloc(InstructionStream::new(code))?;

        let mut status = EvalStatus::Pending;
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
