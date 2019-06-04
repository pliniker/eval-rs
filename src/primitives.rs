/// Native runtime types
use std::fmt;
use std::slice;
use std::str;

use crate::array::Array;
use crate::containers::IndexedContainer;
use crate::error::{RuntimeError, SourcePos};
use crate::memory::MutatorView;
use crate::printer::Print;
use crate::safeptr::{CellPtr, MutatorScope, ScopedPtr};
use crate::taggedptr::Value;

/// A Symbol is a unique object that has a unique name string. The backing storage for the
/// underlying str data must have a lifetime of at least that of the Symbol instance to
/// prevent use-after-free.
/// See `SymbolMap`
pub struct Symbol {
    name_ptr: *const u8,
    name_len: usize,
}

impl Symbol {
    /// The originating &str must be owned by a SymbolMap hash table
    pub fn new(name: &str) -> Symbol {
        Symbol {
            name_ptr: name.as_ptr(),
            name_len: name.len(),
        }
    }

    /// Unsafe because Symbol does not own the &str
    pub unsafe fn unguarded_as_str(&self) -> &str {
        let slice = slice::from_raw_parts(self.name_ptr, self.name_len);
        str::from_utf8(slice).unwrap()
    }

    pub fn as_str<'guard>(&self, _guard: &'guard MutatorScope) -> &str {
        unsafe { self.unguarded_as_str() }
    }
}

impl Print for Symbol {
    /// Safe because the lifetime of `MutatorScope` defines a safe-access window
    fn print<'scope>(&self, guard: &'scope MutatorScope, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str(guard))
    }
}

/// A Pair of pointers, like a Cons cell of old
pub struct Pair {
    pub first: CellPtr,
    pub second: CellPtr,
    // Possible source code positions of the first and second values
    pub first_pos: Option<SourcePos>,
    pub second_pos: Option<SourcePos>,
}

impl Pair {
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
    fn print<'scope>(&self, guard: &'scope MutatorScope, f: &mut fmt::Formatter) -> fmt::Result {
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
    fn debug<'scope>(&self, guard: &'scope MutatorScope, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "({:?} . {:?})",
            self.first.get(guard),
            self.second.get(guard)
        )
    }
}

/// TODO A heap-allocated number
pub struct NumberObject {
    value: isize,
}

impl Print for NumberObject {
    fn print<'scope>(&self, _guard: &'scope MutatorScope, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

/// Array type that can contain any other object
pub type ArrayAny = Array<CellPtr>;

impl Print for ArrayAny {
    fn print<'scope>(&self, _guard: &'scope MutatorScope, f: &mut fmt::Formatter) -> fmt::Result {
        // TODO
        write!(f, "[{}]", "array of things")
    }
}
