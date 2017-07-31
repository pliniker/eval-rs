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
    pub fn as_str(&self) -> &str {
        self.name.as_str()
    }
}
