use std::collections::HashMap;

use error::{err, ParseEvalError};
use heap::{Heap, Ptr};
use symbolmap::{SymbolMap, SymbolMapper};
use types::{Symbol, Value};

/*
Memory ties together all heap and stack abstractions
 */

// TODO this should be similar to SymbolMap?
type Bindings<'storage, A> = HashMap<Ptr<'storage, Symbol, A>, Value<'storage, A>>;


pub struct Memory<'storage, A: 'storage + Heap> {
    // garbage collected heap memory
    heap: &'storage A,
    // keys to syms are Strings, which have pointers to them in heap
    syms: SymbolMap<'storage, A>,
    // mapping of Symbols to Values
    globals: Bindings<'storage, A>,
}


impl<'storage, A: 'storage + Heap> Memory<'storage, A> {
    /// Instantiate a Memory abstraction
    pub fn with_heap(heap: &'storage A) -> Memory<'storage, A> {
        Memory {
            heap: heap,
            syms: SymbolMap::new(heap),
            globals: Bindings::new(),
        }
    }

    /// Run a mutator closure
    pub fn mutate_with<F>(&mut self, callable: F)
        where F: FnMut(&A, &SymbolMap<'storage, A>)
    {
        callable(&self.heap, &self.syms)
    }
}


pub fn eval<'storage, A: 'storage + Heap>(
    expr: Value<'storage, A>,
    mem: &'storage Memory<'storage, A>)
    -> Result<Value<'storage, A>, ParseEvalError>
{
    match expr {
        Value::Symbol(ptr) => {
            match mem.globals.get(&ptr) {
                Some(value) => Ok(*value),
                None => Err(err(&format!("Symbol {} is not bound to a value", ptr.as_str())))
            }
        }

        Value::Pair(ptr) => {
            apply(expr, mem)
        },

        anything_else => Ok(anything_else)
    }
}


pub fn apply<'storage, A: 'storage + Heap>(
    params: Value<'storage, A>,
    mem: &'storage Memory<'storage, A>)
    -> Result<Value<'storage, A>, ParseEvalError>
{
    // TODO need to eval rest, one list item at a time

    let (function, rest) = flatten_args!(one_and_rest => params);

    if let Value::Symbol(ptr) = function {
        if let Some(&Value::Function(f)) = mem.globals.get(&ptr) {
            f.call(rest, mem)
        } else {
            Err(err(&format!("Symbol {} is not bound to a function", ptr.as_str())))
        }
    } else {
        Err(err("Only symbols may be bound to functions"))
    }
}


fn atom<'storage, A: 'storage + Heap>(
    params: Value<'storage, A>,
    mem: &'storage Memory<'storage, A>)
    -> Result<Value<'storage, A>, ParseEvalError>
{
    let (value,) = flatten_args!(one_only => params);

    match value {
        Value::Symbol(_) => Ok(Value::Symbol(mem.syms.lookup("true"))),
        Value::Nil => Ok(Value::Symbol(mem.syms.lookup("true"))),
        _ => Ok(Value::Nil)
    }
}
