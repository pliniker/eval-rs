use std::collections::HashMap;

use error::{err, err_wpos, ParseEvalError};
use memory::{Allocator, Arena, Ptr};
use symbolmap::{SymbolMap, SymbolMapper};
use types::{Symbol, Value};


type Bindings<'a, A> = HashMap<Ptr<'a, Symbol, A>, Value<'a, A>>;


pub struct Environment<'a, A: 'a + Allocator> {
    // garbage collected heap memory
    pub heap: &'a A,
    // keys to syms are Strings, which have pointers to them in mem.
    // The lifetime of syms must be >= the lifetime of mem
    pub syms: SymbolMap<'a, A>,
    // mapping of Symbols to Values
    pub globals: Bindings<'a, A>,
}


impl<'a, A: 'a + Allocator> Environment<'a, A> {
    pub fn new(sym_heap: &'a A, heap: &'a A) -> Environment<'a, A> {
        Environment {
            heap: heap,
            syms: SymbolMap::new(sym_heap),
            globals: Bindings::new(),
        }
    }
}


impl<'a, A: 'a + Allocator> Environment<'a, A> {
    fn add_global_bindings(&'a mut self){
        let evalrus_true = self.syms.lookup("true");
        self.globals.insert(evalrus_true, Value::Symbol(evalrus_true));
    }
}


pub fn eval<'a, A: 'a + Allocator>(expr: Value<'a, A>,
                                   env: &'a Environment<'a, A>)
                                   -> Result<Value<'a, A>, ParseEvalError> {
    match expr {
        Value::Symbol(ptr) => {
            match env.globals.get(&ptr) {
                Some(value) => Ok(*value),
                None => Err(err(&format!("Symbol {} is not bound to a value", ptr.as_str())))
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
    // TODO need to eval params
    let params = match params {
        Value::Pair(_) => eval(params, env)?,
        Value::Nil => params,
        _ => return Err(err("Parameter(s) must be in list form"))
    };

    if let Value::Symbol(ptr) = function {
        if ptr == env.syms.lookup("atom") {
            atom(params, env)
        } else {
            Err(err(&format!("Symbol {} is not bound to a function", ptr.as_str())))
        }
    } else {
        Err(err("Object in function position is not a symbol"))
    }
}


fn next_param<'a, A: 'a + Allocator>(param_list: Value<'a, A>)
                                     -> Result<(Value<'a, A>, Value<'a, A>), ParseEvalError> {
    match param_list {
        Value::Pair(pair) => Ok((pair.first, pair.second)),
        Value::Nil => Ok((Value::Nil, Value::Nil)),
        _ => Err(err("Expected a parameter list"))
    }
}


fn atom<'a, A: 'a + Allocator>(params: Value<'a, A>,
                               env: &'a Environment<'a, A>)
                               -> Result<Value<'a, A>, ParseEvalError> {
    let (value, rest) = next_param(params)?;

    if let Value::Nil = rest {
        match value {
            Value::Symbol(_) => Ok(Value::Symbol(env.syms.lookup("true"))),
            Value::Nil => Ok(Value::Symbol(env.syms.lookup("true"))),
            _ => Ok(Value::Nil)
        }
    } else {
        Err(err("One parameter expected"))
    }
}
