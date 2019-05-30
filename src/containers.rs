use std::cell::Cell;
use std::ptr::{read, write};
use std::slice::from_raw_parts;

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
///
/// `T` cannot be restricted to `Copy` because of the use of `Cell` for interior mutability.
pub trait Container<T: Sized + Clone>: Sized {
    /// Create a new, empty container instance.
    fn new() -> Self;
    /// Create a new container instance with the given capacity.
    fn with_capacity<'guard>(
        mem: &'guard MutatorView,
        capacity: ArraySize,
    ) -> Result<Self, RuntimeError>;
}

/// If implemented, the container can function as a stack
pub trait StackContainer<T: Sized + Clone>: Container<T> {
    /// Push can trigger an underlying array resize, hence it requires the ability to allocate
    fn push<'guard>(&self, mem: &'guard MutatorView, item: T) -> Result<(), RuntimeError>;

    /// Pop returns None if the container is empty, otherwise moves the last item of the array
    /// out to the caller.
    fn pop<'guard>(&self, _guard: &'guard MutatorScope) -> Result<T, RuntimeError>;
}

/// If implemented, the container can function as an indexable vector
pub trait IndexedContainer<T: Sized + Clone>: Container<T> {
    /// Return a copy of the object at the given index. Bounds-checked.
    fn get<'guard>(
        &self,
        _guard: &'guard MutatorScope,
        index: ArraySize,
    ) -> Result<T, RuntimeError>;

    /// Move an object into the array at the given index. Bounds-checked.
    fn set<'guard>(
        &self,
        _guard: &'guard MutatorScope,
        index: ArraySize,
        item: T,
    ) -> Result<(), RuntimeError>;

    /// Experimental
    /// Give a closure a view of the container as a slice.
    /// Restricting to `Fn` means interior mutability rules can be maintained. The closure cannot
    /// safely escape a reference to an object inside the array.
    fn slice_apply<'guard, F>(&self, _guard: &'guard MutatorScope, op: F)
    where
        F: Fn(&[T]);
}

/// An array, like Vec
#[derive(Clone)]
pub struct Array<T: Sized + Clone> {
    length: Cell<ArraySize>,
    data: Cell<RawArray<T>>,
}

impl<T: Sized + Clone> Array<T> {
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

            let dest_ptr = unsafe { ptr.offset(index as isize) as *mut T };

            Ok(dest_ptr)
        }
    }

    /// Bounds-checked write
    fn write<'guard>(
        &self,
        _guard: &'guard MutatorScope,
        index: ArraySize,
        item: T,
    ) -> Result<&T, RuntimeError> {
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

impl<T: Sized + Clone> Container<T> for Array<T> {
    fn new() -> Array<T> {
        Array {
            length: Cell::new(0),
            data: Cell::new(RawArray::new()),
        }
    }

    fn with_capacity<'guard>(
        mem: &'guard MutatorView,
        capacity: ArraySize,
    ) -> Result<Array<T>, RuntimeError> {
        Ok(Array {
            length: Cell::new(0),
            data: Cell::new(RawArray::with_capacity(mem, capacity)?),
        })
    }
}

impl<T: Sized + Clone> StackContainer<T> for Array<T> {
    /// Push can trigger an underlying array resize, hence it requires the ability to allocate
    fn push<'guard>(&self, mem: &'guard MutatorView, item: T) -> Result<(), RuntimeError> {
        let length = self.length.get();
        let mut array = self.data.get(); // Takes a copy

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
        self.write(mem, length, item)?;
        Ok(())
    }

    /// Pop returns None if the container is empty, otherwise moves the last item of the array
    /// out to the caller.
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

impl<T: Sized + Clone> IndexedContainer<T> for Array<T> {
    /// Return a copy of the object at the given index. Bounds-checked.
    fn get<'guard>(
        &self,
        _guard: &'guard MutatorScope,
        index: ArraySize,
    ) -> Result<T, RuntimeError> {
        self.read(_guard, index)
    }

    /// Move an object into the array at the given index. Bounds-checked.
    fn set<'guard>(
        &self,
        _guard: &'guard MutatorScope,
        index: ArraySize,
        item: T,
    ) -> Result<(), RuntimeError> {
        self.write(_guard, index, item)?;
        Ok(())
    }

    /// Experimental
    /// Give a closure a view of the container as a slice.
    /// Restricting to `Fn` means interior mutability rules can be maintained. The closure cannot
    /// safely escape a reference to an object inside the array.
    fn slice_apply<'guard, F>(&self, _guard: &'guard MutatorScope, op: F)
    where
        F: Fn(&[T]),
    {
        if let Some(ptr) = self.data.get().as_ptr() {
            let as_slice = unsafe { from_raw_parts(ptr, self.length.get() as usize) };

            op(as_slice);
        }
    }
}

#[cfg(test)]
mod test {
    use super::{Array, Container, IndexedContainer, StackContainer};
    use crate::error::ErrorKind;
    use crate::memory::Memory;
    use crate::primitives::ArrayAny;
    use crate::safeptr::CellPtr;
    use crate::taggedptr::Value;

    #[test]
    fn array_push_and_pop() {
        let mem = Memory::new();

        mem.mutate(|view| {
            let array: Array<i64> = Array::new();

            // TODO StickyImmixHeap will only allocate up to 32k at time of writing
            // test some big array sizes
            for i in 0..1000 {
                array.push(view, i)?;
            }

            for i in 0..1000 {
                assert!(array.pop(view)? == 999 - i);
            }

            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn array_indexing() {
        let mem = Memory::new();

        mem.mutate(|view| {
            let array: Array<i64> = Array::new();

            for i in 0..12 {
                array.push(view, i)?;
            }

            assert!(array.get(view, 0) == Ok(0));
            assert!(array.get(view, 4) == Ok(4));

            for i in 12..1000 {
                match array.get(view, i) {
                    Ok(_) => panic!("Array index should have been out of bounds!"),
                    Err(e) => assert!(*e.error_kind() == ErrorKind::BoundsError),
                }
            }

            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn array_slice_apply() {
        let mem = Memory::new();

        mem.mutate(|view| {
            let array: Array<i64> = Array::new();

            for i in 0..12 {
                array.push(view, i)?;
            }

            array.slice_apply(view, |items| {
                for (i, value) in items.iter().enumerate() {
                    assert!(i as i64 == *value);
                }
            });

            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn allocd_array_of_tagged_pointers() {
        let mem = Memory::new();

        mem.mutate(|view| {
            let array: ArrayAny = Array::new();

            let ptr = view.alloc(array)?;

            match *ptr {
                Value::ArrayAny(array) => {
                    for _ in 0..12 {
                        array.push(view, CellPtr::new_nil())?;
                    }
                }
                _ => panic!("Expected ArrayAny type!"),
            }

            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn array_with_capacity() {
        let mem = Memory::new();

        mem.mutate(|view| {
            let array: ArrayAny = Array::with_capacity(view, 256)?;

            let ptr_before = array.data.get().as_ptr();

            // fill to capacity
            for _ in 0..256 {
                array.push(view, CellPtr::new_nil())?;
            }

            let ptr_after = array.data.get().as_ptr();

            // array storage shouldn't have been reallocated
            assert!(ptr_before == ptr_after);

            // overflow capacity, requiring reallocation
            array.push(view, CellPtr::new_nil())?;

            let ptr_realloc = array.data.get().as_ptr();

            // array storage should have been reallocated
            assert!(ptr_before != ptr_realloc);

            Ok(())
        })
        .unwrap();
    }
}
