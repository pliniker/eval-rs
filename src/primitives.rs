use std::hash::{Hash, Hasher};
use std::slice;
use std::str;

use taggedptr::TaggedPtr;


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

    /// unsafe because there is no inbuilt guarantee here that the internal pointer is valid
    pub unsafe fn as_str(&self) -> &str {
        let slice = slice::from_raw_parts(self.name_ptr, self.name_len);
        str::from_utf8(slice).unwrap()
    }
}


/// TODO since as_str() is unsafe, this should be too
impl Hash for Symbol {
    fn hash<H: Hasher>(&self, state: &mut H) {
        unsafe { self.as_str() }.hash(state);
    }
}


// The following types must be have an ObjectHeader preceding them on the heap

/// Redefine Pair from types.rs
pub struct Pair {
    first: TaggedPtr,
    second: TaggedPtr
}


/// A heap-allocated number
pub struct NumberObject {
    value: isize
}


/// A heap-allocated string
pub struct StringObject {
    len: usize,
}
