use std::cell::Cell;

use crate::array::{Array, ArraySize};
use crate::bytecode::{ByteCode, InstructionStream, Opcode};
use crate::containers::{
    Container, FillAnyContainer, HashIndexedAnyContainer, IndexedAnyContainer, IndexedContainer,
    SliceableContainer, StackContainer,
};
use crate::dict::Dict;
use crate::error::{err_eval, RuntimeError};
use crate::function::{Function, Partial};
use crate::list::List;
use crate::memory::MutatorView;
use crate::pair::Pair;
use crate::safeptr::{CellPtr, MutatorScope, ScopedPtr, TaggedCellPtr, TaggedScopedPtr};
use crate::taggedptr::{TaggedPtr, Value};

/// Control flow flags
#[derive(PartialEq)]
pub enum EvalStatus<'guard> {
    Pending,
    Return(TaggedScopedPtr<'guard>),
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
            base,
        }
    }

    fn as_string<'guard>(&self, guard: &'guard dyn MutatorScope) -> String {
        let function = self.function.get(guard);
        format!("in {}", function)
    }
}

/// A stack of CallFrame instances
pub type CallFrameList = Array<CallFrame>;

/// Closure upvalue as generally described by Lua 5.1 implementation
#[derive(Clone)]
pub struct Upvalue {
    // Upvalue location can't be a pointer because it would be a pointer into the dynamically
    // alloocated stack List - the pointer would be invalidated if the stack gets reallocated.
    closed: Option<TaggedCellPtr>,
    location: ArraySize,
    next: Option<CellPtr<Upvalue>>,
}

impl Upvalue {
    fn alloc<'guard>(
        mem: &'guard MutatorView,
        location: ArraySize,
    ) -> Result<ScopedPtr<'guard, Upvalue>, RuntimeError> {
        mem.alloc(Upvalue {
            closed: None,
            location,
            next: None,
        })
    }

    // Dereference the upvalue
    fn get<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        stack: ScopedPtr<'guard, List>,
    ) -> Result<TaggedScopedPtr<'guard>, RuntimeError> {
        match &self.closed {
            Some(value) => Ok(value.get(guard)),
            None => IndexedAnyContainer::get(&*stack, guard, self.location),
        }
    }

    // Write a new value to the Upvalue, placing it here or on the stack depending on the
    // closedness of it.
    fn set<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        stack: ScopedPtr<'guard, List>,
        ptr: TaggedPtr,
    ) -> Result<(), RuntimeError> {
        match &self.closed {
            Some(value) => value.set_to_ptr(ptr),
            None => {
                IndexedContainer::set(&*stack, guard, self.location, TaggedCellPtr::new_ptr(ptr))?
            }
        };
        Ok(())
    }

    // Close the upvalue, copying the stack variable's value into the Upvalue
    fn close<'guard>(
        &mut self,
        guard: &'guard dyn MutatorScope,
        stack: ScopedPtr<'guard, List>,
    ) -> Result<(), RuntimeError> {
        let ptr = IndexedContainer::get(&*stack, guard, self.location)?;
        self.closed = Some(ptr.clone());
        Ok(())
    }
}

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
                Opcode::NoOp => return Ok(EvalStatus::Pending),

                Opcode::Return { reg } => {
                    // write the return value to register 0
                    let result = window[reg as usize].get_ptr();
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

                Opcode::LoadLiteral { dest, literal_id } => {
                    let literal_ptr = instr.get_literal(mem, literal_id)?;
                    window[dest as usize].set_to_ptr(literal_ptr);
                }

                Opcode::IsNil { dest, test } => {
                    let test_val = window[test as usize].get(mem);

                    match *test_val {
                        Value::Nil => window[dest as usize].set(mem.lookup_sym("true")),
                        _ => window[dest as usize].set_to_nil(),
                    }
                }

                Opcode::IsAtom { dest, test } => {
                    let test_val = window[test as usize].get(mem);

                    match *test_val {
                        Value::Pair(_) => window[dest as usize].set_to_nil(),
                        Value::Nil => window[dest as usize].set_to_nil(),
                        // TODO what other types?
                        _ => window[dest as usize].set(mem.lookup_sym("true")),
                    }
                }

                Opcode::FirstOfPair { dest, reg } => {
                    let reg_val = window[reg as usize].get(mem);

                    match *reg_val {
                        Value::Pair(p) => window[dest as usize].set_to_ptr(p.first.get_ptr()),
                        Value::Nil => window[dest as usize].set_to_nil(),
                        _ => return Err(err_eval("Parameter to FirstOfPair is not a list")),
                    }
                }

                Opcode::SecondOfPair { dest, reg } => {
                    let reg_val = window[reg as usize].get(mem);

                    match *reg_val {
                        Value::Pair(p) => window[dest as usize].set_to_ptr(p.second.get_ptr()),
                        Value::Nil => window[dest as usize].set_to_nil(),
                        _ => return Err(err_eval("Parameter to SecondOfPair is not a list")),
                    }
                }

                Opcode::MakePair { dest, reg1, reg2 } => {
                    let reg1_val = window[reg1 as usize].get_ptr();
                    let reg2_val = window[reg2 as usize].get_ptr();

                    let new_pair = Pair::new();
                    new_pair.first.set_to_ptr(reg1_val);
                    new_pair.second.set_to_ptr(reg2_val);

                    window[dest as usize].set(mem.alloc_tagged(new_pair)?);
                }

                Opcode::IsIdentical { dest, test1, test2 } => {
                    // compare raw pointers - identity comparison
                    let test1_val = window[test1 as usize].get_ptr();
                    let test2_val = window[test2 as usize].get_ptr();

                    if test1_val == test2_val {
                        window[dest as usize].set(mem.lookup_sym("true"));
                    } else {
                        window[dest as usize].set(mem.nil());
                    }
                }

                Opcode::Jump { offset } => {
                    instr.jump(offset);
                }

                Opcode::JumpIfTrue { test, offset } => {
                    let test_val = window[test as usize].get(mem);

                    let true_sym = mem.lookup_sym("true"); // TODO preload keyword syms

                    if test_val == true_sym {
                        instr.jump(offset)
                    }
                }

                Opcode::JumpIfNotTrue { test, offset } => {
                    let test_val = window[test as usize].get(mem);

                    let true_sym = mem.lookup_sym("true");

                    if test_val != true_sym {
                        instr.jump(offset)
                    }
                }

                Opcode::LoadNil { dest } => {
                    window[dest as usize].set_to_nil();
                }

                Opcode::LoadInteger { dest, integer } => {
                    let tagged_ptr = TaggedPtr::literal_integer(integer);
                    window[dest as usize].set_to_ptr(tagged_ptr);
                }

                Opcode::LoadGlobal { dest, name } => {
                    let name_val = window[name as usize].get(mem);

                    if let Value::Symbol(_) = *name_val {
                        let lookup_result = globals.lookup(mem, name_val);

                        match lookup_result {
                            Ok(binding) => window[dest as usize].set(binding),
                            Err(_) => {
                                return Err(err_eval(&format!(
                                    "Symbol {} is not bound to a value",
                                    name_val
                                )))
                            }
                        }
                    } else {
                        return Err(err_eval("Cannot lookup global for non-symbol type"));
                    }
                }

                Opcode::StoreGlobal { src, name } => {
                    let name_val = window[name as usize].get(mem);
                    if let Value::Symbol(_) = *name_val {
                        let src_val = window[src as usize].get(mem);
                        globals.assoc(mem, name_val, src_val)?;
                    } else {
                        return Err(err_eval("Cannot bind global to non-symbol type"));
                    }
                }

                Opcode::Call {
                    function,
                    dest,
                    arg_count,
                } => {
                    let binding = window[function as usize].get(mem);

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
                        let new_stack_base = self.stack_base.get() + dest as ArraySize;
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
                                let args_start = dest as usize + 1;
                                let args_end = args_start + arg_count as usize;

                                let partial =
                                    Partial::alloc(mem, function, &window[args_start..args_end])?;

                                window[dest as usize].set(partial.as_tagged(mem));

                                return Ok(EvalStatus::Pending);
                            } else if arg_count > arity {
                                // Too many args, we haven't got a continuations stack (yet)
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
                                window[dest as usize].set(partial.as_tagged(mem));
                                return Ok(EvalStatus::Pending);
                            } else if arg_count < arity {
                                // Too few args, bake a new Partial from the existing one, adding the new
                                // arguments
                                let args_start = dest as usize + 1;
                                let args_end = args_start + arg_count as usize;

                                let new_partial = Partial::alloc_clone(
                                    mem,
                                    partial,
                                    &window[args_start..args_end],
                                )?;

                                window[dest as usize].set(new_partial.as_tagged(mem));

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
                            let from_reg = dest as usize + 1;
                            let to_reg = from_reg + push_dist as usize;
                            for index in (0..arg_count as usize).rev() {
                                window[to_reg + index] = window[from_reg + index].clone();
                            }

                            // copy args from Partial to the register window
                            let args = partial.args(mem);
                            let start_reg = dest as usize + 1;
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

                Opcode::MakeClosure {
                    dest,
                    function,
                    function_scope,
                } => {
                    // TODO
                    // 1. create new Partial
                    // 2. iter over function nonlocals, offset by function_scope, copying to new Partial
                    // 3. set dest to Partial
                    unimplemented!()
                }

                Opcode::CopyRegister { dest, src } => {
                    window[dest as usize] = window[src as usize].clone();
                }

                Opcode::LoadNonLocal {
                    dest,
                    src,
                    frame_offset,
                } => {
                    let full_dest = stack_base + dest as usize;

                    let frame = frames.get(mem, frames.length() - 1 - frame_offset as ArraySize)?;
                    let frame_base = frame.base;
                    let full_src = frame_base + src as ArraySize;

                    let value = &full_stack[full_src as usize];
                    full_stack[full_dest] = value.clone();
                }

                Opcode::Add { dest, reg1, reg2 } => unimplemented!(),

                Opcode::Subtract { dest, left, right } => unimplemented!(),

                Opcode::Multiply { dest, reg1, reg2 } => unimplemented!(),

                Opcode::DivideInteger { dest, num, denom } => unimplemented!(),
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
                _ => (),
            }
        }

        Err(err_eval("Unexpected end of evaluation"))
    }
}
