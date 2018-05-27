/// Implements a monotonically growing memory backing for objects that
/// will be managed externally to this store.
///
/// Implements the low level `Allocator` trait.

use std::cell::{Cell, RefCell};
use std::mem::{replace, size_of};
use std::ptr;

use blockalloc::{Block, BlockError};

use heap::{Allocator, MemError};
use rawptr::RawPtr;


/// Global fixed sized blocks
const BLOCK_SIZE: usize = 4096 * 2;


/// Any BlockError we'll just convert to Out Of Memory
impl From<BlockError> for MemError {
    fn from(_error: BlockError) -> Self {
        MemError::OOM
    }
}


/// A wrapper around a Block, adding a bump-allocation offset
struct BumpBlock {
    block: Block,
    bump: Cell<usize>,
}


impl BumpBlock {
    fn new() -> Result<BumpBlock, MemError> {
        let block = Block::new(BLOCK_SIZE)?;

        Ok(
            BumpBlock {
                block: block,
                bump: Cell::new(0),
            }
        )
    }

    // Allocate a new object and return it's pointer
    fn inner_alloc<T>(&self, object: T) -> Result<*mut T, T> {

        // double-word alignment
        let align = size_of::<usize>() * 2;
        let size = (size_of::<T>() & !(align - 1)) + align;

        let next_bump = self.bump.get() + size;

        if next_bump > BLOCK_SIZE {
            // just return the object if the block would overflow by allocating
            // the object into it
            Err(object)
        } else {
            let ptr = unsafe {
                let p = self.block.as_ptr().offset(self.bump.get() as isize) as *mut T;
                ptr::write(p, object);
                p
            };

            self.bump.set(next_bump);

            Ok(ptr)
        }
    }
}


struct BlockList {
    pub current: BumpBlock,
    pub rest: Vec<BumpBlock>
}


impl BlockList {
    fn new() -> BlockList {
        BlockList {
            current: BumpBlock::new().unwrap(),
            rest: Vec::new()
        }
    }
}


/// An arena of any object type. Allocation returns `RawPtr<T>` types
/// which must be separately lifetime-managed.
pub struct Arena {
    blocks: RefCell<BlockList>
}


impl Arena {
    pub fn new() -> Arena {
        Arena {
            blocks: RefCell::new(BlockList::new())
        }
    }
}


impl Allocator for Arena {
    fn alloc<T>(&self, object: T) -> Result<RawPtr<T>, MemError> {
        let mut blocks = self.blocks.borrow_mut();

        match blocks.current.inner_alloc(object) {
            Ok(ptr) => Ok(RawPtr::from_bare(ptr)),

            Err(object) => {
                let previous = replace(&mut blocks.current, BumpBlock::new()?);
                blocks.rest.push(previous);

                match blocks.current.inner_alloc(object) {
                    Ok(ptr) => Ok(RawPtr::from_bare(ptr)),
                    Err(_) => Err(MemError::OOM)
                }
            }
        }
    }
}


impl Default for Arena {
    fn default() -> Arena {
        Arena::new()
    }
}


#[cfg(test)]
mod test {

    use super::*;

    struct Thing {
        a: u8,
        b: u16,
        c: u32,
        d: u64,
    }

    impl Thing {
        fn new() -> Thing {
            Thing {
                a: 1,
                b: 2,
                c: 3,
                d: 4,
            }
        }

        fn check(&self) -> bool {
            self.a == 1 && self.b == 2 && self.c == 3 && self.d == 4
        }
    }


    #[test]
    fn test_alloc_struct() {
        let mem = Arena::new();
        if let Ok(ptr) = mem.alloc(Thing::new()) {
            assert!(unsafe { ptr.deref().check() });
        }
    }

    #[test]
    fn test_bump() {
        let mem = BumpBlock::new().unwrap();

        let align = size_of::<usize>() * 2;
        let size = (size_of::<Thing>() & !(align - 1)) + align;

        for i in 0..(BLOCK_SIZE / size) {
            if let Err(_) = mem.inner_alloc(Thing::new()) {
                assert!(false, format!("no {} failed to allocate", i));
            }
        }

        if let Ok(_) = mem.inner_alloc(Thing::new()) {
            assert!(false, "last failed to fail to allocate")
        }
    }
}
