
use blockalloc::{Block, BlockError};

use error::AllocError;


struct BlockList {
    cursor: usize,
    head: Option<Block>,
    rest: Vec<Block>,
}


impl BlockList {
    pub fn new() -> BlockList {
        BlockList {
            cursor: 0,
            head: None,
            rest: Vec::new()
        }
    }
}


/// A non-garbage-collected arena for interned types
struct Arena {
    blocks: UnsafeCell<BlockList>
}


impl Arena {

    pub fn alloc<T>
}
