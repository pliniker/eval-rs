use std::collections::HashMap;
use error::{err, err_wpos, ParseEvalError};
use memory::{Allocator, Arena, Ptr};
use symbolmap::{SymbolMap, SymbolMapper};
use types::{Symbol, Value};


type Bindings<'a, A> = HashMap<Ptr<'a, Symbol, A>, Value<'a, A>>;


pub struct Environment<'a, A: 'a + Allocator> {
    pub mem: A,
    // keys to syms are Strings, which have pointers to them in mem.
    // The lifetime of syms must be >= the lifetime of mem
    pub syms: SymbolMap<'a, A>,
    // mapping of Symbols to Values
    pub bindings: Bindings<'a, A>,
}


impl<'a> Environment<'a, Arena> {
    pub fn new(block_size: usize) -> Environment<'a, Arena> {
        Environment {
            mem: Arena::new(block_size),
            syms: SymbolMap::new(block_size),
            bindings: Bindings::new(),
        }
    }
}


pub fn eval<'a, A: 'a + Allocator>(expr: Value<'a, A>,
                                   env: &'a Environment<'a, A>)
                                   -> Result<Value<'a, A>, ParseEvalError> {
    match expr {
        Value::Symbol(ptr) => {
            match env.bindings.get(&ptr) {
                Some(value) => Ok(*value),
                None => Err(err("No value associated with that symbol"))
            }
        }

        Value::Pair(ptr) => {
            apply(ptr.first, ptr.second, env)
        },

        anything_else => Ok(anything_else)
    }
}


pub fn apply<'a, A: 'a + Allocator>(function: Value<'a, A>,
                                    params: Value<'a, A>,
                                    env: &'a Environment<'a, A>)
                                    -> Result<Value<'a, A>, ParseEvalError> {
    if let Value::Pair(pair) = params {

        match function {
            Value::Symbol(ptr) =>{
                if ptr == env.syms.lookup("atom") {
                    atom(params, env)
                } else {
                    Err(err("Not a function"))
                }
            },
            _ => Err(err("Not a function"))
        }

    } else {
        Err(err("Parameter(s) must be in a list"))
    }
}


fn atom<'a, A: 'a + Allocator>(expr: Value<'a, A>,
                               env: &'a Environment<'a, A>)
                               -> Result<Value<'a, A>, ParseEvalError> {
    match expr {
        Value::Symbol(_) => Ok(Value::Symbol(env.syms.lookup("true"))),
        _ => Ok(Value::Nil)
    }
}
