/// Basic mutable dict type:
use std::cell::Cell;
use std::hash::Hasher;

use fnv::FnvHasher;
use stickyimmix::ArraySize;

use crate::containers::{Container, ContainerFromPairList, HashIndexedAnyContainer};
use crate::error::{ErrorKind, RuntimeError};
use crate::hashable::Hashable;
use crate::memory::MutatorView;
use crate::rawarray::RawArray;
use crate::safeptr::{MutatorScope, TaggedCellPtr, TaggedScopedPtr};
use crate::taggedptr::Value;

// max load factor before resizing the table
const LOAD_FACTOR: f32 = 0.75;

#[derive(Clone)]
struct DictItem {
    key: TaggedCellPtr,
    value: TaggedCellPtr,
    hash: u64,
}

/// Internal entry representation, keeping copy of hash for the key
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
struct Dict {
    length: Cell<ArraySize>,
    data: Cell<RawArray<DictItem>>,
}

impl Dict {
    /// Return a bounds-checked pointer to the entry at the given index
    fn get_offset(&self, index: ArraySize) -> Result<*mut DictItem, RuntimeError> {
        let data = self.data.get();

        if index >= data.capacity() {
            Err(RuntimeError::new(ErrorKind::BoundsError))
        } else {
            let ptr = data
                .as_ptr()
                .ok_or(RuntimeError::new(ErrorKind::BoundsError))?;

            let dest_ptr = unsafe { ptr.offset(index as isize) as *mut DictItem };

            Ok(dest_ptr)
        }
    }

    /// Return a bounts-checked mutable reference to the entyr at the given index
    fn get_item_ref<'guard>(
        &self,
        _guard: &'guard dyn MutatorScope,
        index: ArraySize,
    ) -> Result<&'guard mut DictItem, RuntimeError> {
        Ok(unsafe { &mut *self.get_offset(index)? })
    }

    /// Given a key, generate the hash and search for an entry that either matches this hash
    /// or the next available blank entry.
    /// Cribbed from http://craftinginterpreters.com/hash-tables.html
    fn find_entry<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        key: TaggedScopedPtr
    ) -> Result<(&'guard mut DictItem, u64), RuntimeError> {
        // the inefficiency of my implementation is offensive but for now...

        let mut hasher = FnvHasher::default();
        match *key {
            Value::Symbol(s) => s.hash(guard, &mut hasher),
            _ => return Err(RuntimeError::new(ErrorKind::UnhashableError))
        }
        let hash = hasher.finish();

        let data = self.data.get();
        let mut index = (hash % data.capacity() as u64) as ArraySize;

        loop {
            let mut entry = self.get_item_ref(guard, index)?;

            if entry.hash == hash || entry.key.is_nil() {
                return Ok((entry, hash))
            }

            index = (index + 1) % data.capacity();
        }
    }

    fn fill_with_blank_entries<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
    ) -> Result<(), RuntimeError> {
        let data = self.data.get();
        let blank_entry = DictItem::blank();

        for index in 0..data.capacity() {
            *self.get_item_ref(guard, index)? = blank_entry.clone();
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
        let (mut entry, hash) = self.find_entry(guard, key)?;

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
        let (mut entry, hash) = self.find_entry(mem, key)?;

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
        mem: &'guard MutatorView,
        key: TaggedScopedPtr,
    ) -> Result<TaggedScopedPtr<'guard>, RuntimeError> {
        let (mut entry, hash) = self.find_entry(mem, key)?;

        if entry.key.is_nil() {
            return Err(RuntimeError::new(ErrorKind::KeyError))
        }

        self.length.set(self.length.get() - 1);
        entry.key.set_to_nil();

        Ok(entry.value.get(mem))
    }

    fn exists<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        key: TaggedScopedPtr,
    ) -> Result<bool, RuntimeError> {

        Ok(true)
    }
}

#[cfg(test)]
mod test {
    use super::{Container, ContainerFromPairList, Dict, HashIndexedAnyContainer};
    use crate::error::{ErrorKind, RuntimeError};
    use crate::memory::{Memory, Mutator, MutatorView};
    use crate::pair::Pair;
    use crate::primitives::ArrayAny;
    use crate::taggedptr::Value;
}
