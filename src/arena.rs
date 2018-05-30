/// An Arena type for backing interned objects.
///
/// Implements the low level `Allocator` trait.


use std::cell::UnsafeCell;
use std::mem::{replace, size_of};
use std::ptr;

use blockalloc::{Block, BlockError};

use heap::{Allocator, MemError};
use rawptr::RawPtr;


/// Global fixed sized blocks
const BLOCK_SIZE: usize = 4096 * 2;


/// Any BlockError we'll just convert to Out Of Memory - in any case it's
/// a terminating error.
impl From<BlockError> for MemError {
    fn from(_error: BlockError) -> Self {
        MemError::OOM
    }
}


/// A wrapper around a Block, adding a bump-allocation offset
struct SimpleBumpBlock {
    block: Block,
    bump: usize,
}


impl SimpleBumpBlock {
    fn new() -> Result<SimpleBumpBlock, MemError> {
        let block = Block::new(BLOCK_SIZE)?;

        Ok(
            SimpleBumpBlock {
                block: block,
                bump: 0,
            }
        )
    }

    // Allocate a new object and return it's pointer
    fn inner_alloc<T>(&mut self, object: T) -> Result<*mut T, T> {

        // word align everything
        let align = size_of::<usize>();
        let size = (size_of::<T>() & !(align - 1)) + align;

        let next_bump = self.bump + size;

        if next_bump > BLOCK_SIZE {
            // just return the object if the block would overflow by allocating
            // the object into it
            Err(object)
        } else {
            let ptr = unsafe {
                let p = self.block.as_ptr().offset(self.bump as isize) as *mut T;
                ptr::write(p, object);
                p
            };

            self.bump = next_bump;

            Ok(ptr)
        }
    }
}


struct BlockList {
    current: SimpleBumpBlock,
    rest: Vec<SimpleBumpBlock>
}


impl BlockList {
    fn new() -> Result<BlockList, MemError> {
        Ok(BlockList {
            current: SimpleBumpBlock::new()?,
            rest: Vec::new()
        })
    }
}


/// An arena of any object type. Allocation returns `RawPtr<T>` types.
///
pub struct Arena {
    /// Use UnsafeCell to avoid RefCell overhead. This member will only be
    /// accessed in `alloc<T>()`.
    blocks: UnsafeCell<BlockList>
}


impl Arena {
    pub fn new() -> Result<Arena, MemError> {
        Ok(Arena {
            blocks: UnsafeCell::new(BlockList::new()?)
        })
    }
}


impl Allocator for Arena {
    fn alloc<T>(&self, object: T) -> Result<RawPtr<T>, MemError> {
        let blocks: &mut BlockList = unsafe { &mut *self.blocks.get() };

        match blocks.current.inner_alloc(object) {
            Ok(ptr) => Ok(RawPtr::from_bare(ptr)),

            Err(object) => {
                let previous = replace(&mut blocks.current, SimpleBumpBlock::new()?);
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
        Arena::new().expect("failed to allocate an initial Arena block")
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
        let mem = Arena::new().unwrap();
        if let Ok(ptr) = mem.alloc(Thing::new()) {
            assert!(unsafe { ptr.deref().check() });
        }
    }

    #[test]
    fn test_bump() {
        // test expected block capacity and overflow handling

        let mut mem = SimpleBumpBlock::new().unwrap();

        let align = size_of::<usize>();
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
