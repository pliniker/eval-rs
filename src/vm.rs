use stickyimmix::ArraySize;

use crate::bytecode::{ByteCode, InstructionStream, Opcode};
use crate::containers::{Container, IndexedAnyContainer, StackAnyContainer};
use crate::error::{err_eval, RuntimeError};
use crate::memory::{Mutator, MutatorView};
use crate::pair::Pair;
use crate::primitives::ArrayAny;
use crate::safeptr::{ScopedPtr, TaggedScopedPtr};
use crate::taggedptr::Value;

/// Control flow flags
#[derive(PartialEq)]
pub enum EvalStatus<'guard> {
    Pending,
    Return(TaggedScopedPtr<'guard>),
    Halt,
}

/// Execute the next instruction and return
fn eval_next_instr<'guard>(
    mem: &'guard MutatorView,
    stack: ScopedPtr<'guard, ArrayAny>,
    instr: ScopedPtr<'guard, InstructionStream>,
) -> Result<EvalStatus<'guard>, RuntimeError> {
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
                Value::Nil => stack.set(mem, acc, mem.nil())?,
                Value::Pair(_) => stack.set(mem, acc, mem.nil())?,
                _ => stack.set(mem, acc, mem.lookup_sym("true"))?,
            }
        }

        Opcode::CAR => {
            let acc = instr.get_reg_acc() as ArraySize;
            let reg1 = instr.get_reg1() as ArraySize;

            let reg1_val = stack.get(mem, reg1)?;

            match *reg1_val {
                Value::Pair(p) => stack.set(mem, acc, p.first.get(mem))?,
                _ => return Err(err_eval("Parameter to CAR is not a list")),
            }
        }

        Opcode::CDR => {
            let acc = instr.get_reg_acc() as ArraySize;
            let reg1 = instr.get_reg1() as ArraySize;

            let reg1_val = stack.get(mem, reg1)?;

            match *reg1_val {
                Value::Pair(p) => stack.set(mem, acc, p.second.get(mem))?,
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

        Opcode::EQ => {
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

            let true_sym = mem.lookup_sym("true");  // TODO preload keyword syms

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
    }

    Ok(EvalStatus::Pending)
}

/// Given an InstructionStream, execute up to max_instr more instructions
pub fn vm_eval_stream<'guard>(
    mem: &'guard MutatorView,
    stack: ScopedPtr<'guard, ArrayAny>,
    instr: ScopedPtr<'guard, InstructionStream>,
    max_instr: ArraySize,
) -> Result<EvalStatus<'guard>, RuntimeError> {
    for _ in 0..max_instr {
        match eval_next_instr(mem, stack, instr)? {
            EvalStatus::Return(value) => return Ok(EvalStatus::Return(value)),
            EvalStatus::Halt => return Ok(EvalStatus::Halt),
            _ => (),
        }
    }
    Ok(EvalStatus::Pending)
}

/// Evaluate a whole block of byte code
pub fn quick_vm_eval<'guard>(
    mem: &'guard MutatorView,
    code: ScopedPtr<'_, ByteCode>,
) -> Result<TaggedScopedPtr<'guard>, RuntimeError> {
    let stream = mem.alloc(InstructionStream::new(code))?;

    let stack = mem.alloc(ArrayAny::with_capacity(mem, 256)?)?;
    for _ in 0..256 {
        stack.push(mem, mem.nil())?;
    }

    let mut status = EvalStatus::Pending;
    while status == EvalStatus::Pending {
        status = vm_eval_stream(mem, stack, stream, 1024)?;
        match status {
            EvalStatus::Return(value) => return Ok(value),
            EvalStatus::Halt => return Err(err_eval("Program halted")),
            _ => (),
        }
    }

    Err(err_eval("Unexpected end of evaluation"))
}

/// Mutator that instantiates a VM
struct VMFactory {}

impl Mutator for VMFactory {
    type Input = ();
    type Output = VM;

    fn run(&self, mem: &MutatorView, _: Self::Input) -> Result<VM, RuntimeError> {
        // initialize stack to 256 nil registers
        let stack = ArrayAny::with_capacity(mem, 256)?;
        for index in 0..256 {
            stack.set(mem, index, mem.nil())?;
        }

        Ok(VM { stack: stack })
    }
}

/// Mutator that implements the VM
struct VM {
    stack: ArrayAny,
}

impl Mutator for VM {
    type Input = ByteCode;
    type Output = ();

    fn run(&self, mem: &MutatorView, code: Self::Input) -> Result<Self::Output, RuntimeError> {
        Ok(())
    }
}
