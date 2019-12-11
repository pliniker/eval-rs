/// Basic mutable dict type:
use std::cell::Cell;
use std::fmt;
use std::ptr::{read, write};

use fnv::FnvHasher;
use stickyimmix::ArraySize;

use crate::containers::{Container, ContainerFromPairList, HashIndexedAnyContainer};
use crate::error::{ErrorKind, RuntimeError};
use crate::memory::MutatorView;
use crate::printer::Print;
use crate::rawarray::{default_array_growth, RawArray, DEFAULT_ARRAY_SIZE};
use crate::safeptr::{MutatorScope, TaggedCellPtr, TaggedScopedPtr};
use crate::taggedptr::Value;

#[derive(Clone)]
struct DictItem {
    key: TaggedCellPtr,
    value: TaggedCellPtr,
    hash: u64,
}

struct Dict {
    length: Cell<ArraySize>,
    data: Cell<RawArray<DictItem>>,
}

impl Container<DictItem> for Dict {
    fn new() -> Dict {
        Dict {
            length: Cell::new(0),
            data: Cell::new(RawArray::new())
        }
    }

    fn with_capacity<'guard>(
        mem: &'guard MutatorView,
        capacity: ArraySize,
    ) -> Result<Self, RuntimeError> {
        Ok(Dict {
            length: Cell::new(0),
            data: Cell::new(RawArray::with_capacity(mem, capacity)?),
        })
    }

    fn clear<'guard>(&self, mem: &'guard MutatorView) -> Result<(), RuntimeError> {
        self.length.set(0);
        Ok(())
    }

    fn length(&self) -> ArraySize {
        self.length.get()
    }
}

#[cfg(test)]
mod test {
    use super::{Dict, Container, ContainerFromPairList, HashIndexedAnyContainer};
    use crate::error::{ErrorKind, RuntimeError};
    use crate::memory::{Memory, Mutator, MutatorView};
    use crate::pair::Pair;
    use crate::primitives::ArrayAny;
    use crate::taggedptr::Value;
}
