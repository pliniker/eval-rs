/// Basic mutable array type:
///
///  Array<T>
///  ArrayAny = Array<TaggedCellPtr> (see primitives)
use std::cell::Cell;
use std::fmt;
use std::ptr::{read, write};

use stickyimmix::ArraySize;

use crate::containers::{
    Container, ContainerFromPairList, IndexedAnyContainer, IndexedContainer, StackAnyContainer,
    StackContainer,
};
use crate::error::{ErrorKind, RuntimeError};
use crate::memory::MutatorView;
use crate::printer::Print;
use crate::rawarray::{default_array_growth, RawArray, DEFAULT_ARRAY_SIZE};
use crate::safeptr::{MutatorScope, TaggedCellPtr, TaggedScopedPtr};
use crate::taggedptr::Value;

/// An array, like Vec
#[derive(Clone)]
pub struct Array<T: Sized + Clone> {
    length: Cell<ArraySize>,
    data: Cell<RawArray<T>>,
}

/// Internal implementation
impl<T: Sized + Clone> Array<T> {
    /// Return a bounds-checked pointer to the object at the given index
    fn get_offset(&self, index: ArraySize) -> Result<*mut T, RuntimeError> {
        if index >= self.length.get() {
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
        _guard: &'guard dyn MutatorScope,
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
        _guard: &'guard dyn MutatorScope,
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
        _guard: &'guard dyn MutatorScope,
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

    fn clear<'guard>(&self, _guard: &'guard MutatorView) -> Result<(), RuntimeError> {
        self.length.set(0);
        Ok(())
    }

    fn length(&self) -> ArraySize {
        self.length.get()
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
    fn pop<'guard>(&self, _guard: &'guard dyn MutatorScope) -> Result<T, RuntimeError> {
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

impl StackAnyContainer for ArrayAny {
    /// Push can trigger an underlying array resize, hence it requires the ability to allocate
    fn push<'guard>(
        &self,
        mem: &'guard MutatorView,
        item: TaggedScopedPtr<'guard>,
    ) -> Result<(), RuntimeError> {
        Ok(StackContainer::<TaggedCellPtr>::push(
            self,
            mem,
            TaggedCellPtr::new_with(item),
        )?)
    }

    /// Pop returns None if the container is empty, otherwise moves the last item of the array
    /// out to the caller.
    fn pop<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
    ) -> Result<TaggedScopedPtr<'guard>, RuntimeError> {
        Ok(StackContainer::<TaggedCellPtr>::pop(self, guard)?.get(guard))
    }
}

impl<T: Sized + Clone> IndexedContainer<T> for Array<T> {
    /// Return a copy of the object at the given index. Bounds-checked.
    fn get<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        index: ArraySize,
    ) -> Result<T, RuntimeError> {
        self.read(guard, index)
    }

    /// Move an object into the array at the given index. Bounds-checked.
    fn set<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        index: ArraySize,
        item: T,
    ) -> Result<(), RuntimeError> {
        self.write(guard, index, item)?;
        Ok(())
    }
}

impl IndexedAnyContainer for ArrayAny {
    /// Return a pointer to the object at the given index. Bounds-checked.
    fn get<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        index: ArraySize,
    ) -> Result<TaggedScopedPtr<'guard>, RuntimeError> {
        Ok(self.read_ref(guard, index)?.get(guard))
    }

    /// Set the object pointer at the given index. Bounds-checked.
    fn set<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        index: ArraySize,
        item: TaggedScopedPtr<'guard>,
    ) -> Result<(), RuntimeError> {
        self.read_ref(guard, index)?.set(item);
        Ok(())
    }
}

impl ContainerFromPairList for ArrayAny {
    fn from_pair_list<'guard>(
        &self,
        mem: &'guard MutatorView,
        pair_list: TaggedScopedPtr<'guard>,
    ) -> Result<(), RuntimeError> {
        self.length.set(0);

        let mut head = pair_list;
        while let Value::Pair(p) = *head {
            StackAnyContainer::push(self, mem, p.first.get(mem))?;
            head = p.second.get(mem);
        }

        Ok(())
    }
}

/// Array type that can contain any other object
pub type ArrayAny = Array<TaggedCellPtr>;

impl Print for ArrayAny {
    fn print<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        write!(f, "[")?;

        for i in 0..self.length() {
            if i > 1 {
                write!(f, ", ")?;
            }

            let ptr =
                IndexedAnyContainer::get(self, guard, i).expect("Failed to read ptr from array");

            fmt::Display::fmt(&ptr.value(), f)?;
        }

        write!(f, "]")
    }
}

/// Array of u8
pub type ArrayU8 = Array<u8>;

impl Print for ArrayU8 {
    fn print<'guard>(
        &self,
        _guard: &'guard dyn MutatorScope,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        write!(f, "ArrayU8[...]")
    }
}

/// Array of u32
pub type ArrayU32 = Array<u32>;

impl Print for ArrayU32 {
    fn print<'guard>(
        &self,
        _guard: &'guard dyn MutatorScope,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        write!(f, "ArrayU32[...]")
    }
}

#[cfg(test)]
mod test {
    use super::{
        Array, Container, ContainerFromPairList, IndexedAnyContainer, IndexedContainer,
        StackAnyContainer, StackContainer,
    };
    use crate::error::{ErrorKind, RuntimeError};
    use crate::memory::{Memory, Mutator, MutatorView};
    use crate::pair::Pair;
    use crate::primitives::ArrayAny;
    use crate::taggedptr::Value;

    #[test]
    fn array_generic_push_and_pop() {
        let mem = Memory::new();

        struct Test {}
        impl Mutator for Test {
            type Input = ();
            type Output = ();

            fn run(
                &self,
                view: &MutatorView,
                _input: Self::Input,
            ) -> Result<Self::Output, RuntimeError> {
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
            }
        }

        let test = Test {};
        mem.mutate(&test, ()).unwrap();
    }

    #[test]
    fn array_generic_indexing() {
        let mem = Memory::new();

        struct Test {}
        impl Mutator for Test {
            type Input = ();
            type Output = ();

            fn run(
                &self,
                view: &MutatorView,
                _input: Self::Input,
            ) -> Result<Self::Output, RuntimeError> {
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
            }
        }

        let test = Test {};
        mem.mutate(&test, ()).unwrap();
    }

    #[test]
    fn arrayany_tagged_pointers() {
        let mem = Memory::new();

        struct Test {}
        impl Mutator for Test {
            type Input = ();
            type Output = ();

            fn run(
                &self,
                view: &MutatorView,
                _input: Self::Input,
            ) -> Result<Self::Output, RuntimeError> {
                let array: ArrayAny = Array::new();
                let array = view.alloc(array)?;

                for _ in 0..12 {
                    StackAnyContainer::push(&*array, view, view.nil())?;
                }

                // or by copy/clone
                let pair = view.alloc_tagged(Pair::new())?;

                IndexedAnyContainer::set(&*array, view, 3, pair)?;

                Ok(())
            }
        }

        let test = Test {};
        mem.mutate(&test, ()).unwrap();
    }

    #[test]
    fn array_with_capacity_and_realloc() {
        let mem = Memory::new();

        struct Test {}
        impl Mutator for Test {
            type Input = ();
            type Output = ();

            fn run(
                &self,
                view: &MutatorView,
                _input: Self::Input,
            ) -> Result<Self::Output, RuntimeError> {
                let array: ArrayAny = Array::with_capacity(view, 256)?;

                let ptr_before = array.data.get().as_ptr();

                // fill to capacity
                for _ in 0..256 {
                    StackAnyContainer::push(&array, view, view.nil())?;
                }

                let ptr_after = array.data.get().as_ptr();

                // array storage shouldn't have been reallocated
                assert!(ptr_before == ptr_after);

                // overflow capacity, requiring reallocation
                StackAnyContainer::push(&array, view, view.nil())?;

                let ptr_realloc = array.data.get().as_ptr();

                // array storage should have been reallocated
                assert!(ptr_before != ptr_realloc);

                Ok(())
            }
        }

        let test = Test {};
        mem.mutate(&test, ()).unwrap();
    }

    #[test]
    fn arrayany_from_pair_list() {
        let mem = Memory::new();

        struct Test {}
        impl Mutator for Test {
            type Input = ();
            type Output = ();

            fn run(
                &self,
                view: &MutatorView,
                _input: Self::Input,
            ) -> Result<Self::Output, RuntimeError> {
                let array: ArrayAny = Array::new();
                let array = view.alloc(array)?;

                let pair = Pair::new();
                pair.first.set(view.lookup_sym("thing0"));

                let head = view.alloc_tagged(pair)?;
                let mut tail = head;

                for n in 1..12 {
                    if let Value::Pair(pair) = *tail {
                        tail = pair.append(view, view.lookup_sym(&format!("thing{}", n)))?;
                    } else {
                        panic!("expected pair!")
                    }
                }

                array.from_pair_list(view, head)?;

                for n in 0..12 {
                    let thing = IndexedAnyContainer::get(&*array, view, n)?;

                    match *thing {
                        Value::Symbol(s) => assert!(s.as_str(view) == format!("thing{}", n)),
                        _ => panic!("expected symbol!"),
                    }
                }

                Ok(())
            }
        }

        let test = Test {};
        mem.mutate(&test, ()).unwrap();
    }
}
