use std::fmt;

use crate::error::{err_eval, RuntimeError, SourcePos};
use crate::memory::MutatorView;
use crate::printer::Print;
use crate::safeptr::{MutatorScope, TaggedCellPtr, TaggedScopedPtr};
use crate::taggedptr::Value;

/// A Pair of pointers, like a Cons cell of old
pub struct Pair {
    pub first: TaggedCellPtr,
    pub second: TaggedCellPtr,
    // Possible source code positions of the first and second values
    pub first_pos: Option<SourcePos>,
    pub second_pos: Option<SourcePos>,
}

impl Pair {
    /// Return a new empty Pair instance
    pub fn new() -> Pair {
        Pair {
            first: TaggedCellPtr::new_nil(),
            second: TaggedCellPtr::new_nil(),
            first_pos: None,
            second_pos: None,
        }
    }

    /// Set Pair.second to a new Pair with newPair.first set to the value
    pub fn append<'guard>(
        &self,
        mem: &'guard MutatorView,
        value: TaggedScopedPtr<'guard>,
    ) -> Result<TaggedScopedPtr<'guard>, RuntimeError> {
        let pair = Pair::new();
        pair.first.set(value);

        let pair = mem.alloc_tagged(pair)?;
        self.second.set(pair);

        Ok(pair)
    }

    /// Set Pair.second to the given value
    pub fn dot<'guard>(&self, value: TaggedScopedPtr<'guard>) {
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

        // clunky way to print anything but nil
        let second = *tail.second.get(guard);
        match second {
            Value::Nil => (),
            _ => write!(f, " . {}", second)?
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
    ptr: TaggedScopedPtr<'guard>,
) -> Result<TaggedScopedPtr<'guard>, RuntimeError> {
    match *ptr {
        Value::Pair(pair) => {
            if pair.second.is_nil() {
                Ok(pair.first.get(guard))
            } else {
                Err(err_eval("Expected no more than one value in Pair list"))
            }
        }
        _ => Err(err_eval("Expected a Pair list")),
    }
}

/// Given a pointer to a Pair linked list, assert that the list is of length 2 and return the 2 values
pub fn get_two_from_pair_list<'guard>(
    guard: &'guard dyn MutatorScope,
    ptr: TaggedScopedPtr<'guard>,
) -> Result<(TaggedScopedPtr<'guard>, TaggedScopedPtr<'guard>), RuntimeError> {
    match *ptr {
        Value::Pair(pair) => {
            let first_param = pair.first.get(guard);

            match *pair.second.get(guard) {
                Value::Pair(pair) => {
                    if let Value::Nil = *pair.second.get(guard) {
                        let second_param = pair.first.get(guard);
                        Ok((first_param, second_param))
                    } else {
                        Err(err_eval("Expected no more than two values in Pair list"))
                    }
                }
                _ => Err(err_eval("Expected no less than two values in Pair list")),
            }
        }
        _ => Err(err_eval("Expected a Pair list")),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::error::{ErrorKind, RuntimeError};
    use crate::memory::{Memory, Mutator, MutatorView};
    use crate::pair::Pair;

    fn test_helper(test_fn: fn(&MutatorView) -> Result<(), RuntimeError>) {
        let mem = Memory::new();

        struct Test {}
        impl Mutator for Test {
            type Input = fn(&MutatorView) -> Result<(), RuntimeError>;
            type Output = ();

            fn run(
                &self,
                mem: &MutatorView,
                test_fn: Self::Input,
            ) -> Result<Self::Output, RuntimeError> {
                test_fn(mem)
            }
        }

        let test = Test {};
        mem.mutate(&test, test_fn).unwrap();
    }

    #[test]
    fn pair_list_length_1() {
        fn test_inner(mem: &MutatorView) -> Result<(), RuntimeError> {
            let thing = mem.lookup_sym("thing");

            let head = Pair::new();
            head.first.set(thing);

            let head = mem.alloc_tagged(head)?;

            assert!(get_one_from_pair_list(mem, head).unwrap() == thing);

            Ok(())
        }

        test_helper(test_inner)
    }

    #[test]
    fn pair_list_length_0_should_be_1() {
        fn test_inner(mem: &MutatorView) -> Result<(), RuntimeError> {
            let thing = mem.nil();

            match get_one_from_pair_list(mem, thing) {
                Ok(_) => panic!("No list given, no value should have been found!"),
                Err(e) => assert!(
                    *e.error_kind() == ErrorKind::EvalError(String::from("Expected a Pair list"))
                ),
            }

            Ok(())
        }

        test_helper(test_inner)
    }

    #[test]
    fn pair_list_length_2_should_be_1() {
        fn test_inner(mem: &MutatorView) -> Result<(), RuntimeError> {
            let head = Pair::new();
            head.append(mem, mem.nil())?;

            let head = mem.alloc_tagged(head)?;

            match get_one_from_pair_list(mem, head) {
                Ok(_) => panic!("Too-long list given, no value should have been returned!"),
                Err(e) => assert!(
                    *e.error_kind()
                        == ErrorKind::EvalError(String::from(
                            "Expected no more than one value in Pair list"
                        ))
                ),
            }

            Ok(())
        }

        test_helper(test_inner)
    }

    #[test]
    fn pair_list_length_2() {
        fn test_inner(mem: &MutatorView) -> Result<(), RuntimeError> {
            let thing1 = mem.lookup_sym("thing1");
            let thing2 = mem.lookup_sym("thing2");

            let head = Pair::new();
            head.first.set(thing1);
            head.append(mem, thing2)?;

            let head = mem.alloc_tagged(head)?;

            assert!(get_two_from_pair_list(mem, head).unwrap() == (thing1, thing2));

            Ok(())
        }

        test_helper(test_inner)
    }

    #[test]
    fn pair_list_length_0_should_be_2() {
        fn test_inner(mem: &MutatorView) -> Result<(), RuntimeError> {
            let thing = mem.nil();

            match get_two_from_pair_list(mem, thing) {
                Ok(_) => panic!("No list given, no values could have been found!"),
                Err(e) => assert!(
                    *e.error_kind() == ErrorKind::EvalError(String::from("Expected a Pair list"))
                ),
            }

            Ok(())
        }

        test_helper(test_inner)
    }

    #[test]
    fn pair_list_length_1_should_be_2() {
        fn test_inner(mem: &MutatorView) -> Result<(), RuntimeError> {
            let head = mem.alloc_tagged(Pair::new())?;

            match get_two_from_pair_list(mem, head) {
                Ok(_) => panic!("Too-short list given, no values should have been found!"),
                Err(e) => assert!(
                    *e.error_kind()
                        == ErrorKind::EvalError(String::from(
                            "Expected no less than two values in Pair list"
                        ))
                ),
            }

            Ok(())
        }

        test_helper(test_inner)
    }

    #[test]
    fn pair_list_length_3_should_be_2() {
        fn test_inner(mem: &MutatorView) -> Result<(), RuntimeError> {
            let pair = Pair::new();
            pair.append(mem, mem.nil())?;
            let head = Pair::new();
            head.dot(mem.alloc_tagged(pair)?);
            let head = mem.alloc_tagged(head)?;

            match get_two_from_pair_list(mem, head) {
                Ok(_) => panic!("Too-long list given, no values should have been returned!"),
                Err(e) => assert!(
                    *e.error_kind()
                        == ErrorKind::EvalError(String::from(
                            "Expected no more than two values in Pair list"
                        ))
                ),
            }

            Ok(())
        }

        test_helper(test_inner)
    }
}
