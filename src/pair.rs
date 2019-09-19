use std::fmt;

use crate::error::{err_eval, RuntimeError, SourcePos};
use crate::memory::MutatorView;
use crate::printer::Print;
use crate::safeptr::{CellPtr, MutatorScope, ScopedPtr};
use crate::taggedptr::Value;

/// A Pair of pointers, like a Cons cell of old
pub struct Pair {
    pub first: CellPtr,
    pub second: CellPtr,
    // Possible source code positions of the first and second values
    pub first_pos: Option<SourcePos>,
    pub second_pos: Option<SourcePos>,
}

impl Pair {
    /// Return a new empty Pair instance
    pub fn new() -> Pair {
        Pair {
            first: CellPtr::new_nil(),
            second: CellPtr::new_nil(),
            first_pos: None,
            second_pos: None,
        }
    }

    /// Set Pair.second to a new Pair with newPair.first set to the value
    pub fn append<'guard>(
        &self,
        mem: &'guard MutatorView,
        value: ScopedPtr<'guard>,
    ) -> Result<ScopedPtr<'guard>, RuntimeError> {
        let pair = Pair::new();
        pair.first.set(value);

        let pair = mem.alloc(pair)?;
        self.second.set(pair);

        Ok(pair)
    }

    /// Set Pair.second to the given value
    pub fn dot<'guard>(&self, value: ScopedPtr<'guard>) {
        self.second.set(value);
    }
}

impl Print for Pair {
    fn print<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        let mut tail = self;
        write!(f, "({}", tail.first.get(guard))?;

        while let Value::Pair(next) = *tail.second.get(guard) {
            tail = next;
            write!(f, " {}", tail.first.get(guard))?;
        }

        if let Value::Symbol(ptr) = *tail.second.get(guard) {
            write!(f, " . {}", ptr.as_str(guard))?;
        }

        write!(f, ")")
    }

    // In debug print, use dot notation
    fn debug<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        write!(
            f,
            "({:?} . {:?})",
            self.first.get(guard),
            self.second.get(guard)
        )
    }
}

/// Given a pointer to a Pair linked list, assert that the list is of length 1 and return that 1 value
pub fn get_one_from_pair_list<'guard>(
    guard: &'guard dyn MutatorScope,
    ptr: ScopedPtr<'guard>,
) -> Result<ScopedPtr<'guard>, RuntimeError> {
    match *ptr {
        Value::Pair(pair) => {
            if pair.second.is_nil() {
                Ok(pair.first.get(guard))
            } else {
                Err(err_eval("Expected no more than one parameter"))
            }
        }
        _ => Err(err_eval("Expected no less than one parameter")),
    }
}

/// Given a pointer to a Pair linked list, assert that the list is of length 2 and return the 2 values
pub fn get_two_from_pair_list<'guard>(
    guard: &'guard dyn MutatorScope,
    ptr: ScopedPtr<'guard>,
) -> Result<(ScopedPtr<'guard>, ScopedPtr<'guard>), RuntimeError> {
    match *ptr {
        Value::Pair(pair) => {
            let first_param = pair.first.get(guard);

            match *pair.second.get(guard) {
                Value::Pair(pair) => {
                    if let Value::Nil = *pair.second.get(guard) {
                        let second_param = pair.first.get(guard);
                        Ok((first_param, second_param))
                    } else {
                        Err(err_eval("Expected no more than two parameters"))
                    }
                }

                _ => Err(err_eval("Expected no less than two parameters")),
            }
        }
        _ => Err(err_eval("Expected no less than two parameters")),
    }
}
