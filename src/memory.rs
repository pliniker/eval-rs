

use memalloc::{allocate, deallocate};
use std::mem;
use std::ops::{Deref, DerefMut};
use std::ptr;


pub struct Ptr<T> {
    ptr: *mut T,
}


impl<T> Deref for Ptr<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.ptr }
    }
}


impl<T> DerefMut for Ptr<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.ptr }
    }
}


impl<T> Copy for Ptr<T> {}


impl<T> Clone for Ptr<T> {
    fn clone(&self) -> Ptr<T> {
        Ptr { ptr: self.ptr }
    }
}


pub struct Arena {
    buffer: *mut u8,
    size: isize,
    bump: isize,
}


impl Arena {
    pub fn new(size: isize) -> Arena {
        let buffer = unsafe { allocate(size as usize) };

        if buffer == ptr::null_mut() {
            panic!("could not allocate memory!");
        }

        Arena {
            buffer: buffer,
            size: size,
            bump: 0,
        }
    }

    pub fn allocate<T>(&mut self, object: T) -> Ptr<T> {
        let next_bump = self.bump + (mem::size_of::<T>() as isize);
        if next_bump > self.size {
            panic!("out of memory!");
        }

        let p = unsafe {
            let p = self.buffer.offset(self.bump) as *mut T;
            ptr::write(p, object);
            p
        };

        self.bump = next_bump;

        Ptr { ptr: p }
    }
}


impl Drop for Arena {
    fn drop(&mut self) {
        unsafe { deallocate(self.buffer, self.size as usize) };
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
        let mut mem = Arena::new(1024);
        let ptr = mem.allocate(Thing::new());
        assert!(ptr.check());
    }

    #[test]
    #[should_panic]
    fn test_out_of_memory() {
        let mut mem = Arena::new(1024);
        loop {
            let _ptr = mem.allocate(Thing::new());
        }
    }
}
