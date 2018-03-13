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

    /// unsafe because there is no inbuilt guarntee here that the internal pointer is valid
    pub unsafe fn as_str(&self) -> &str {
        let slice = slice::from_raw_parts(self.name_ptr, self.name_len);
        str::from_utf8(slice).unwrap()
    }
}


/// TODO if as_str() is unsafe, this really should be too
impl Hash for Symbol {
    fn hash<H: Hasher>(&self, state: &mut H) {
        unsafe { self.as_str() }.hash(state);
    }
}


/// Redefine Pair from types.rs
pub struct Pair {
    first: TaggedPtr,
    second: TaggedPtr
}


/// A heap-allocated object header
pub union ObjectHeader {
    flags: usize,
    tobj_vtable: *mut ()
}


const HEADER_TAG_TOBJ: usize = 0x00;
const HEADER_TAG_NUMBER: usize = 0x01;
const HEADER_TAG_STRING: usize = 0x02;


impl ObjectHeader {

}


/// A heap-allocated number
pub struct NumberObject {
    value: isize
}


/// A heap-allocated string
pub struct StringObject {
    length: usize,
}
