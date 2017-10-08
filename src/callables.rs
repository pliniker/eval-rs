use environment::Environment;
use error::ParseEvalError;
use memory::{Allocator, Ptr};
use types::{Symbol, Value};


type FunctionPtr<'a, A> = fn(Value<'a, A>, &'a Environment<'a, A>) -> Result<Value<'a, A>, ParseEvalError>;


/// A function pointer enabling first class functions
pub struct Function<'a, A: 'a + Allocator> {
    name: Ptr<'a, Symbol, A>,
    func: FunctionPtr<'a, A>,
}


impl<'a, A: 'a + Allocator> Function<'a, A> {
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn call(&self, params: Value<'a, A>, env: &'a Environment<'a, A>) -> Result<Value<'a, A>, ParseEvalError> {
        (self.func)(params, env)
    }
}


/// Take a Value. If it's a Pair, or list of Pairs, extract the desired number of values.
/// Assert argument count expectations.
#[macro_export]
macro_rules! flatten_args {
    (zero => $e:expr) => {
        match $e {
            Value::Nil => (),
            _ => return Err(err("Function received arguments, expected none"))
        }
    };

    // (first,)
    (one_only => $e:expr) => {
        match $e {
            Value::Pair(pair) => {
                match pair.second {
                    Value::Nil => (pair.first,),
                    _ => return Err(err("Function given too many arguments"))
                }
            },
            _ => return Err(err("Function expected 1 more argument"))
        }
    };

    // (first, (second,))
    (two_only => $e:expr) => {
        match $e {
            Value::Pair(pair) => (pair.first, flatten_args!(one_only => pair.second)),
            _ => return Err(err("Function expected 2 more arguments"))
        }
    };

    // (first, (second, (third,)))
    (three_only => $e:expr) => {
        match $e {
            Value::Pair(pair) => (pair.first, flatten_args!(two_only => pair.second)),
            _ => return Err(err("Function expected 3 more arguments"))
        }
    };

    // (first, rest)
    (one_and_rest => $e:expr) => {
        match $e {
            Value::Pair(pair) => (pair.first, pair.second),
            _ => return Err(err("Function expected 1 more argument"))
        }
    };

    // (first, (second, rest))
    (two_and_rest => $e:expr) => {
        match $e {
            Value::Pair(pair) => (pair.first, flatten_args!(one_and_rest => pair.second)),
            _ => return Err(err("Function expected 2 more arguments"))
        }
    };

    // (first, (second, (third, rest)))
    (three_and_rest => $e:expr) => {
        match $e {
            Value::Pair(pair) => (pair.first, flatten_args!(two_and_rest => pair.second)),
            _ => return Err(err("Function expected 3 more arguments"))
        }
    };
}
