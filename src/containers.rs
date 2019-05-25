use std::cell::Cell;
use std::ptr::{read, write};

use stickyimmix::ArraySize;

use crate::error::{ErrorKind, RuntimeError};
use crate::memory::MutatorView;
use crate::rawarray::{default_array_growth, RawArray, DEFAULT_ARRAY_SIZE};
use crate::safeptr::MutatorScope;

/// Base container-type trait. All container types are subtypes of `Container`.
///
/// All container operations _must_ follow interior mutability only rules.
/// Because there are no compile-time mutable aliasing guarantees, there can be no references
/// into arrays at all, unless there can be a guarantee that the array memory will not be
/// reallocated.
pub trait Container<T: Sized> {
    /// Create a new, empty container instance.
    fn new() -> Self;
}

/// If implemented, the container can function as a stack
pub trait StackContainer<T: Sized>: Container<T> {
    /// Push can trigger an underlying array resize, hence it requires the ability to allocate
    fn push<'guard>(&self, mem: &'guard MutatorView, item: T) -> Result<&'guard T, RuntimeError>;

    /// Pop returns None if the container is empty, otherwise moves the last item of the array
    /// out to the caller.
    fn pop<'guard>(&self, _guard: &'guard MutatorScope) -> Result<T, RuntimeError>;
}

/// If implemented, the container can function as an indexable vector
pub trait IndexedContainer<T: Sized>: Container<T> {
    /// Return a reference to the object at the given index. Bounds-checked.
    fn get<'guard>(&self, _guard: &'guard MutatorScope, index: ArraySize) -> &'guard T;

    /// Write an object into the array at the given index. Bounds-checked.
    fn set<'guard>(&self, _guard: &'guard MutatorScope, index: ArraySize, item: T) -> &'guard T;

    // Return a mutator-lifetime-limited slice
    // probably not possible because interior mutability
    //fn as_slice<'guard>(&self, _guard: &'guard MutatorScope) -> &'guard [T];
}

/// An array, like Vec
pub struct Array<T: Sized> {
    length: Cell<ArraySize>,
    data: Cell<RawArray<T>>,
}

impl<T: Sized> Array<T> {
    /// Return a bounds-checked pointer to the object at the given index
    fn get_offset(&self, index: ArraySize) -> Result<*mut T, RuntimeError> {
        if index < 0 || index >= self.length.get() {
            Err(RuntimeError::new(ErrorKind::BoundsError))
        } else {
            let ptr = self
                .data
                .get()
                .as_ptr()
                .ok_or(RuntimeError::new(ErrorKind::BoundsError))?;

            let dest_ref = unsafe { ptr.offset(index as isize) as *mut T };

            Ok(dest_ref)
        }
    }

    /// Bounds-checked write
    fn write<'guard>(
        &self,
        _guard: &'guard MutatorScope,
        index: ArraySize,
        item: T,
    ) -> Result<&'guard T, RuntimeError> {
        unsafe {
            let dest = self.get_offset(index)?;
            write(dest, item);
            Ok(&*dest as &T)
        }
    }

    /// Bounds-checked read
    fn read<'guard>(
        &self,
        _guard: &'guard MutatorScope,
        index: ArraySize,
    ) -> Result<T, RuntimeError> {
        unsafe {
            let dest = self.get_offset(index)?;
            Ok(read(dest))
        }
    }

    /// Bounds-checked reference-read
    fn read_ref<'guard>(
        &self,
        _guard: &'guard MutatorScope,
        index: ArraySize,
    ) -> Result<&T, RuntimeError> {
        unsafe {
            let dest = self.get_offset(index)?;
            Ok(&*dest as &T)
        }
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
    fn push<'guard>(&self, mem: &'guard MutatorView, item: T) -> Result<&'guard T, RuntimeError> {
        let length = self.length.get();
        let mut array = self.data.get();  // Takes a copy

        let capacity = array.capacity();

        if length == capacity {
            if capacity == 0 {
                array.resize(mem, DEFAULT_ARRAY_SIZE)?;
            } else {
                array.resize(mem, default_array_growth(capacity)?)?;
            }
            // Replace the struct's copy with the resized RawArray object
            self.data.set(array);
        }

        self.length.set(length + 1);
        Ok(self.write(mem, length, item)?)
    }

    fn pop<'guard>(&self, _guard: &'guard MutatorScope) -> Result<T, RuntimeError> {
        let length = self.length.get();

        if length == 0 {
            Err(RuntimeError::new(ErrorKind::BoundsError))
        } else {
            let last = length - 1;
            let item = self.read(_guard, last)?;
            self.length.set(last);
            Ok(item)
        }
    }
}

#[cfg(test)]
mod test {
    use super::{Array, Container, StackContainer};
    use crate::memory::Memory;
    use crate::printer::print;

    #[test]
    fn array_push_and_pop() {
        let mem = Memory::new();

        mem.mutate(|view| {
            let array: Array<i64> = Array::new();

            // XXX StickyImmixHeap will only allocate up to 32k at time of writing
            for i in 0..1000 {
                array.push(view, i)?;
            }

            for i in 0..1000 {
                assert!(array.pop(view)? == 999 - i);
            }

            Ok(())
        }).unwrap();
    }
}
