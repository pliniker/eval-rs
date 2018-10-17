
use primitives::Symbol;
use stickyimmix::RawPtr;


/// A memory error type, encompassing all memory related errors at this time.
#[derive(Debug)]
pub enum MemError {
    OOM,
}
