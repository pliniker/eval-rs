/// Native runtime types
use std::fmt;
use std::slice;
use std::str;

use crate::array::Array;
use crate::containers::{Container, IndexedAnyContainer};
use crate::printer::Print;
use crate::safeptr::{MutatorScope, TaggedCellPtr};

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

    pub fn as_str<'guard>(&self, _guard: &'guard dyn MutatorScope) -> &str {
        unsafe { self.unguarded_as_str() }
    }
}

impl Print for Symbol {
    /// Safe because the lifetime of `MutatorScope` defines a safe-access window
    fn print<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        write!(f, "{}", self.as_str(guard))
    }
}

/// TODO A heap-allocated number
pub struct NumberObject {
    value: Array<u64>,
}

impl Print for NumberObject {
    fn print<'guard>(
        &self,
        _guard: &'guard dyn MutatorScope,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        // TODO
        write!(f, "NumberObject(nan)")
    }
}

/// Array type that can contain any other object
pub type ArrayAny = Array<TaggedCellPtr>;

impl Print for ArrayAny {
    fn print<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        write!(f, "[")?;

        for i in 0..self.length() {
            if i > 1 {
                write!(f, ", ")?;
            }

            let ptr =
                IndexedAnyContainer::get(self, guard, i).expect("Failed to read ptr from array");

            fmt::Display::fmt(&ptr.value(), f)?;
        }

        write!(f, "]")
    }
}

/// Array of u8
pub type ArrayU8 = Array<u8>;

impl Print for ArrayU8 {
    fn print<'guard>(
        &self,
        _guard: &'guard dyn MutatorScope,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        write!(f, "ArrayU8[...]")
    }
}

/// Array of u32
pub type ArrayU32 = Array<u32>;

impl Print for ArrayU32 {
    fn print<'guard>(
        &self,
        _guard: &'guard dyn MutatorScope,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        write!(f, "ArrayU32[...]")
    }
}
