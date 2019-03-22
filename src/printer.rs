use std::fmt;

use crate::safeptr::MutatorScope;
use crate::taggedptr::Value;

/// Trait for using a `Value` lifted pointer in the `Display` trait
pub trait Print {
    fn print<'guard>(&self, _guard: &'guard MutatorScope, f: &mut fmt::Formatter) -> fmt::Result;

    fn debug<'guard>(&self, _guard: &'guard MutatorScope, f: &mut fmt::Formatter) -> fmt::Result {
        self.print(_guard, f)
    }
}

pub fn print(value: Value) -> String {
    format!("{}", value)
}

pub fn debug(value: Value) -> String {
    format!("{:?}", value)
}
