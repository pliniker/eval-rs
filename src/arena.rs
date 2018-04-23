/// Implements a monotonically growing memory backing for objects that
/// will be managed externally to this store.
///
/// Implements the low level `Allocator` trait.

use std::cell::{Cell, RefCell};
use std::mem;
use std::ptr;

use memalloc::{allocate, deallocate};

use heap::{Allocator, MemError};
use rawptr::RawPtr;


const BLOCK_SIZE: usize = 4096;


/// A fixed-size block of contiguous bytes type
struct Block {
    buffer: *mut u8,
    size: usize,
    bump: Cell<usize>,
}


impl Block {
    fn new(size: usize) -> Result<Block, MemError> {
        let buffer = unsafe { allocate(size) };

        if buffer == ptr::null_mut() {
            Err(MemError::OOM)
        } else {
            Ok(
                Block {
                    buffer: buffer,
                    size: size,
                    bump: Cell::new(0),
                }
            )
        }
    }

    // Allocate a new object and return it's pointer
    fn inner_alloc<T>(&self, object: T) -> Result<*mut T, T> {
        let next_bump = self.bump.get() + mem::size_of::<T>();

        if next_bump > self.size {
            // just return the object if the block would overflow by allocating
            // the object into it
            Err(object)
        } else {
            let ptr = unsafe {
                let p = self.buffer.offset(self.bump.get() as isize) as *mut T;
                ptr::write(p, object);
                p
            };

            self.bump.set(next_bump);

            Ok(ptr)
        }
    }
}


impl Drop for Block {
    fn drop(&mut self) {
        unsafe { deallocate(self.buffer, self.size) };
    }
}


struct BlockList {
    pub current: Block,
    pub rest: Vec<Block>
}


impl BlockList {
    fn new(block_size: usize) -> BlockList {
        BlockList {
            current: Block::new(block_size).unwrap(),
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
    pub fn new(block_size: usize) -> Arena {
        Arena {
            blocks: RefCell::new(BlockList::new(block_size))
        }
    }
}


impl Allocator for Arena {
    fn alloc<T>(&self, object: T) -> Result<RawPtr<T>, MemError> {
        let mut blocks = self.blocks.borrow_mut();

        match blocks.current.inner_alloc(object) {
            Ok(ptr) => Ok(RawPtr::from_bare(ptr)),

            Err(object) => {
                let previous = mem::replace(&mut blocks.current, Block::new(BLOCK_SIZE).unwrap());
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
        Arena::new(BLOCK_SIZE)
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
        let mem = Arena::new(1024);
        if let Ok(ptr) = mem.alloc(Thing::new()) {
            assert!(unsafe { ptr.deref().check() });
        }
    }

    #[test]
    fn test_out_of_memory() {
        let mem = Block::new(mem::size_of::<Thing>() * 3).unwrap();

        for _ in 0..3 {
            if let Err(_) = mem.inner_alloc(Thing::new()) {
                assert!(false);
            }
        }

        if let Ok(_) = mem.inner_alloc(Thing::new()) {
            assert!(false)
        }
    }
}
