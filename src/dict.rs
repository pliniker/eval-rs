/// Basic mutable dict type
/// Implemented from http://craftinginterpreters.com/hash-tables.html
use std::cell::Cell;
use std::hash::Hasher;

use fnv::FnvHasher;
use stickyimmix::ArraySize;

use crate::containers::{Container, HashIndexedAnyContainer};
use crate::error::{ErrorKind, RuntimeError};
use crate::hashable::Hashable;
use crate::memory::MutatorView;
use crate::rawarray::RawArray;
use crate::safeptr::{MutatorScope, TaggedCellPtr, TaggedScopedPtr};
use crate::taggedptr::Value;

// max load factor before resizing the table
const LOAD_FACTOR: f32 = 0.75;

/// Internal entry representation, keeping copy of hash for the key
#[derive(Clone)]
struct DictItem {
    key: TaggedCellPtr,
    value: TaggedCellPtr,
    hash: u64,
}

impl DictItem {
    fn blank() -> DictItem {
        DictItem {
            key: TaggedCellPtr::new_nil(),
            value: TaggedCellPtr::new_nil(),
            hash: 0,
        }
    }
}

/// A mutable Dict key/value associative data structure.
/// TODO: resizing, deleting with tombstone values
struct Dict {
    length: Cell<ArraySize>,
    data: Cell<RawArray<DictItem>>,
}

impl Dict {
    /// Given a key, generate the hash and search for an entry that either matches this hash
    /// or the next available blank entry.
    fn find_entry<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        key: TaggedScopedPtr
    ) -> Result<(&'guard mut DictItem, u64), RuntimeError> {
        // get raw pointer to base of array
        let data = self.data.get();
        let ptr = data
            .as_ptr()
            .ok_or(RuntimeError::new(ErrorKind::BoundsError))?;

        // hash the key
        let mut hasher = FnvHasher::default();
        match *key {
            Value::Symbol(s) => s.hash(guard, &mut hasher),
            _ => return Err(RuntimeError::new(ErrorKind::UnhashableError))
        }
        let hash = hasher.finish();

        // find the next available entry slot
        let mut index = (hash % data.capacity() as u64) as ArraySize;
        loop {
            let entry = unsafe { &mut *(ptr.offset(index as isize) as *mut DictItem) as &mut DictItem };

            if entry.hash == hash || entry.key.is_nil() {
                return Ok((entry, hash))
            }

            index = (index + 1) % data.capacity();
        }
    }

    /// Scale capacity up if needed
    fn adjust_capacity<'guard>(
        &self,
        _guard: &'guard MutatorView
    ) -> Result<(), RuntimeError> {
        unimplemented!()
    }

    /// Reset all slots to a blank entry
    fn fill_with_blank_entries<'guard>(
        &self,
        _guard: &'guard dyn MutatorScope,
    ) -> Result<(), RuntimeError> {
        let data = self.data.get();
        let ptr = data
            .as_ptr()
            .ok_or(RuntimeError::new(ErrorKind::BoundsError))?;

        let blank_entry = DictItem::blank();

        for index in 0..data.capacity() {
            let entry = unsafe { &mut *(ptr.offset(index as isize) as *mut DictItem) as &mut DictItem };
            *entry = blank_entry.clone();
        }

        Ok(())
    }
}

impl Container<DictItem> for Dict {
    fn new() -> Dict {
        Dict {
            length: Cell::new(0),
            data: Cell::new(RawArray::new()),
        }
    }

    fn with_capacity<'guard>(
        mem: &'guard MutatorView,
        capacity: ArraySize,
    ) -> Result<Self, RuntimeError> {
        let dict = Dict {
            length: Cell::new(0),
            data: Cell::new(RawArray::with_capacity(mem, capacity)?),
        };

        dict.fill_with_blank_entries(mem)?;

        Ok(dict)
    }

    fn clear<'guard>(&self, mem: &'guard MutatorView) -> Result<(), RuntimeError> {
        self.fill_with_blank_entries(mem)?;
        self.length.set(0);
        Ok(())
    }

    fn length(&self) -> ArraySize {
        self.length.get()
    }
}

/// Hashable-indexed interface. Objects used as keys must implement Hashable.
impl HashIndexedAnyContainer for Dict {
    fn lookup<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        key: TaggedScopedPtr,
    ) -> Result<TaggedScopedPtr<'guard>, RuntimeError> {
        let (entry, _) = self.find_entry(guard, key)?;

        if !entry.key.is_nil() {
            Ok(entry.value.get(guard))
        } else {
            Err(RuntimeError::new(ErrorKind::KeyError))
        }
    }

    fn assoc<'guard>(
        &self,
        mem: &'guard MutatorView,
        key: TaggedScopedPtr<'guard>,
        value: TaggedScopedPtr<'guard>,
    ) -> Result<(), RuntimeError> {
        let (entry, hash) = self.find_entry(mem, key)?;

        if entry.key.is_nil() {
            self.length.set(self.length.get() + 1);
        }

        entry.key.set(key);
        entry.value.set(value);
        entry.hash = hash;

        Ok(())
    }

    fn dissoc<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        key: TaggedScopedPtr,
    ) -> Result<TaggedScopedPtr<'guard>, RuntimeError> {
        let (entry, _) = self.find_entry(guard, key)?;

        if entry.key.is_nil() {
            return Err(RuntimeError::new(ErrorKind::KeyError))
        }

        self.length.set(self.length.get() - 1);
        entry.key.set_to_nil();

        Ok(entry.value.get(guard))
    }

    fn exists<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        key: TaggedScopedPtr,
    ) -> Result<bool, RuntimeError> {
        let (entry, _) = self.find_entry(guard, key)?;
        Ok(!entry.key.is_nil())
    }
}

#[cfg(test)]
mod test {
    use super::{Container, Dict, HashIndexedAnyContainer};
    use crate::error::{ErrorKind, RuntimeError};
    use crate::memory::{Memory, Mutator, MutatorView};
    use crate::pair::Pair;
    use crate::taggedptr::Value;

    #[test]
    fn dict_assoc_lookup() {
        let mem = Memory::new();

        struct Test {}
        impl Mutator for Test {
            type Input = ();
            type Output = ();

            fn run(
                &self,
                mem: &MutatorView,
                _input: Self::Input,
            ) -> Result<Self::Output, RuntimeError> {
                let dict = Dict::with_capacity(mem, 256)?;

                let key = mem.lookup_sym("foo");
                let val = mem.lookup_sym("bar");

                dict.assoc(mem, key, val)?;

                let lookup = dict.lookup(mem, key)?;

                assert!(lookup == val);

                Ok(())
            }
        }

        let test = Test {};
        mem.mutate(&test, ()).unwrap();
    }

    #[test]
    fn dict_lookup_fail() {
        let mem = Memory::new();

        struct Test {}
        impl Mutator for Test {
            type Input = ();
            type Output = ();

            fn run(
                &self,
                mem: &MutatorView,
                _input: Self::Input,
            ) -> Result<Self::Output, RuntimeError> {
                let dict = Dict::with_capacity(mem, 256)?;

                let key = mem.lookup_sym("foo");

                let lookup = dict.lookup(mem, key);

                match lookup {
                    Ok(_) => panic!("Key should not have been found!"),
                    Err(e) => assert!(*e.error_kind() == ErrorKind::KeyError)
                }

                Ok(())
            }
        }

        let test = Test {};
        mem.mutate(&test, ()).unwrap();
    }

    #[test]
    fn dict_dissoc_lookup() {
        let mem = Memory::new();

        struct Test {}
        impl Mutator for Test {
            type Input = ();
            type Output = ();

            fn run(
                &self,
                mem: &MutatorView,
                _input: Self::Input,
            ) -> Result<Self::Output, RuntimeError> {
                let dict = Dict::with_capacity(mem, 256)?;

                let key = mem.lookup_sym("foo");
                let val = mem.lookup_sym("bar");

                dict.assoc(mem, key, val)?;

                let value = dict.lookup(mem, key)?;
                assert!(value == val);

                let value = dict.dissoc(mem, key)?;
                assert!(value == val);

                let result = dict.lookup(mem,key);
                match result {
                    Ok(_) => panic!("Key should not have been found!"),
                    Err(e) => assert!(*e.error_kind() == ErrorKind::KeyError)
                }

                Ok(())
            }
        }

        let test = Test {};
        mem.mutate(&test, ()).unwrap();
    }

    #[test]
    fn dict_assoc_lookup_256_in_capacity_256() {
        let mem = Memory::new();

        struct Test {}
        impl Mutator for Test {
            type Input = ();
            type Output = ();

            fn run(
                &self,
                mem: &MutatorView,
                _input: Self::Input,
            ) -> Result<Self::Output, RuntimeError> {
                let dict = Dict::with_capacity(mem, 256)?;

                for num in 0..256 {
                    let key_name = format!("foo_{}", num);
                    let key = mem.lookup_sym(&key_name);

                    let val_name = format!("val_{}", num);
                    let val = mem.lookup_sym(&val_name);

                    dict.assoc(mem, key, val)?;
                }

                for num in 0..100 {
                    let key_name = format!("foo_{}", num);
                    let key = mem.lookup_sym(&key_name);

                    let val_name = format!("val_{}", num);
                    let val = mem.lookup_sym(&val_name);

                    assert!(dict.exists(mem, key)?);

                    let lookup = dict.lookup(mem, key)?;

                    assert!(lookup == val);
                }

                Ok(())
            }
        }

        let test = Test {};
        mem.mutate(&test, ()).unwrap();
    }

    #[test]
    fn dict_unhashable() {
        let mem = Memory::new();

        struct Test {}
        impl Mutator for Test {
            type Input = ();
            type Output = ();

            fn run(
                &self,
                mem: &MutatorView,
                _input: Self::Input,
            ) -> Result<Self::Output, RuntimeError> {
                let dict = Dict::with_capacity(mem, 256)?;

                // a Pair type does not implement Hashable
                let key = mem.alloc_tagged(Pair::new())?;
                let val = mem.lookup_sym("bar");

                let result = dict.assoc(mem, key, val);

                match result {
                    Ok(_) => panic!("Key should not have been found!"),
                    Err(e) => assert!(*e.error_kind() == ErrorKind::UnhashableError)
                }

                Ok(())
            }
        }

        let test = Test {};
        mem.mutate(&test, ()).unwrap();
    }
}
