use std::collections::HashMap;

use error::{err, err_wpos, ParseEvalError};
use memory::{Allocator, Arena, Ptr};
use symbolmap::{SymbolMap, SymbolMapper};
use types::{Symbol, Value};


type Bindings<'storage, A> = HashMap<Ptr<'storage, Symbol, A>, Value<'storage, A>>;


pub struct Environment<'storage, A: 'storage + Allocator> {
    // garbage collected heap memory
    pub heap: &'storage A,
    // keys to syms are Strings, which have pointers to them in mem.
    // The lifetime of syms must be >= the lifetime of mem
    pub syms: SymbolMap<'storage, A>,
    // mapping of Symbols to Values
    pub globals: Bindings<'storage, A>,
}


impl<'storage, A: 'storage + Allocator> Environment<'storage, A> {
    pub fn new(heap: &'storage A) -> Environment<'storage, A> {
        Environment {
            heap: heap,
            syms: SymbolMap::new(heap),
            globals: Bindings::new(),
        }
    }
}


impl<'storage, A: 'storage + Allocator> Environment<'storage, A> {
    fn add_global_bindings(&'storage mut self){
        let evalrus_true = self.syms.lookup("true");
        self.globals.insert(evalrus_true, Value::Symbol(evalrus_true));
    }
}


pub fn eval<'storage, A: 'storage + Allocator>(
    expr: Value<'storage, A>,
    env: &'storage Environment<'storage, A>)
    -> Result<Value<'storage, A>, ParseEvalError>
{
    match expr {
        Value::Symbol(ptr) => {
            match env.globals.get(&ptr) {
                Some(value) => Ok(*value),
                None => Err(err(&format!("Symbol {} is not bound to a value", ptr.as_str())))
            }
        }

        Value::Pair(ptr) => {
            apply(expr, env)
        },

        anything_else => Ok(anything_else)
    }
}


pub fn apply<'storage, A: 'storage + Allocator>(
    params: Value<'storage, A>,
    env: &'storage Environment<'storage, A>)
    -> Result<Value<'storage, A>, ParseEvalError>
{
    // TODO need to eval rest, one list item at a time

    let (function, rest) = flatten_args!(one_and_rest => params);

    if let Value::Symbol(ptr) = function {
        if let Some(&Value::Function(f)) = env.globals.get(&ptr) {
            f.call(rest, env)
        } else {
            Err(err(&format!("Symbol {} is not bound to a function", ptr.as_str())))
        }
    } else {
        Err(err("Only symbols may be bound to functions"))
    }
}


fn atom<'storage, A: 'storage + Allocator>(
    params: Value<'storage, A>,
    env: &'storage Environment<'storage, A>)
    -> Result<Value<'storage, A>, ParseEvalError>
{
    let (value,) = flatten_args!(one_only => params);

    match value {
        Value::Symbol(_) => Ok(Value::Symbol(env.syms.lookup("true"))),
        Value::Nil => Ok(Value::Symbol(env.syms.lookup("true"))),
        _ => Ok(Value::Nil)
    }
}
