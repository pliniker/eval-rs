use std::cell::Cell;
use std::ptr::{read, write};

use stickyimmix::ArraySize;

use crate::error::{ErrorKind, RuntimeError};
use crate::memory::MutatorView;
use crate::safeptr::MutatorScope;
use crate::rawarray::{DEFAULT_ARRAY_SIZE, default_array_growth, RawArray};

/// Base container-type trait. All container types are subtypes of `Container`
pub trait Container<T: Sized> {
    /// Create a new, empty container instance.
    fn new() -> Self;
}

/// If implemented, the container can function as a stack
pub trait StackContainer<T: Sized>: Container<T> {
    /// Push can trigger an underlying array resize, hence it requires the ability to allocate:w
    fn push<'guard>(&self, mem: &'guard MutatorView, item: T) -> Result<&T, RuntimeError>;

    /// Pop returns None if the container is empty
    fn pop(&self) -> Option<T>;
}

/// If implemented, the container can function as an indexable vector
pub trait IndexedContainer<T: Sized>: Container<T> {
    /// TODO needs correct lifetime
    fn get(&self, index: ArraySize) -> &T;

    /// Write an object into the array at the given index. Bounds-checked.
    fn set(&self, index: ArraySize, item: T) -> &T;

    /// Return a mutator-lifetime-limited slice
    fn as_slice<'guard>(&self, _guard: &'guard MutatorScope) -> &[T];
}

/// An array, like Vec
pub struct Array<T: Sized> {
    length: Cell<ArraySize>,
    data: Cell<RawArray<T>>,
}

impl<T: Sized> Array<T> {
    /// Bounds-checked write
    fn write(&self, index: ArraySize, item: T) -> Result<&T, RuntimeError> {
        if index >= self.length.get() {
            Err(RuntimeError::new(ErrorKind::BoundsError))
        } else {
            let ptr = self.data.get().as_ptr().ok_or(RuntimeError::new(ErrorKind::BoundsError))?;

            let dest = ptr.offset(index as isize) as *mut T;
            write(dest, item);

            let dest_ref = unsafe { &*dest as &T };

            Ok(dest_ref)
        }
    }

    /// Bounds-checked read
    fn read(&self, index: ArraySize) -> Result<T, RuntimeError> {
        unimplemented!()
    }

    /// Bounds-checked reference-read
    fn read_ref(&self, index: ArraySize) -> Result<&T, RuntimeError> {
        unimplemented!()
    }
}

impl<T: Sized> Container<T> for Array<T> {
    fn new() -> Array<T> {
        Array {
            length: Cell::new(0),
            data: Cell::new(RawArray::new()),
        }
    }
}

impl<T: Sized> StackContainer<T> for Array<T> {
    fn push<'guard>(&self, mem: &'guard MutatorView, item: T) -> Result<&T, RuntimeError> {
        let length = self.length.get();
        let mut array = self.data.get();
        let capacity = array.capacity();

        if length == capacity {
            if capacity == 0 {
                array.resize(mem, DEFAULT_ARRAY_SIZE)?;
            } else {
                // grow by 50%
                array.resize(mem, default_array_growth(capacity)?)?;
            }
        }

        let itemref = self.write(length, item);
        self.length.set(length + 1);

        Ok(itemref)
    }

    fn pop(&self) -> Option<T> {
        let head = self.length.get();

        if head == 0 {
            None
        } else {
            self.length.set(head - 1);
            Some(self.read(head))
        }
    }
}
