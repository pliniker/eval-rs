use std::hash::{Hash, Hasher};
use std::slice;
use std::mem::size_of;
use std::str;

use taggedptr::{FatPtr, RawPtr, TaggedPtr};


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


/// A heap-allocated object header
pub struct ObjectHeader {
    flags: u32,
    size: u32
}


impl ObjectHeader {
    pub fn object_rawptr(&self) -> FatPtr {
        unsafe {
            let object_pos = (
                self as *const ObjectHeader as *const () as usize) + size_of::<Self>();

            match self.flags & HEADER_TAG_MASK {
                HEADER_TAG_PAIR => FatPtr::Pair(RawPtr::from_raw(object_pos as *mut Pair)),
                _ => panic!("Corrupt ObjectHeader type tag!")
            }
        }
    }
}


const HEADER_MARK_BIT: u32 = 0x1;

const HEADER_TAG_MASK: u32 = !(0xf << 1);
const HEADER_TAG_PAIR: u32 = 0x00 << 1;
const HEADER_TAG_NUMBER: u32 = 0x01 << 1;
const HEADER_TAG_STRING: u32 = 0x02 << 1;
const HEADER_TAG_REDIRECT: u32 = 0x3 << 1;


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


/// A pointer redirection when an object has been moved
pub struct Redirect {
    new_location: *mut ()
}
