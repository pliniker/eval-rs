/// Basic mutable dict type:
use std::cell::Cell;
use std::fmt;
use std::ptr::{read, write};

use stickyimmix::ArraySize;

use crate::containers::{Container, ContainerFromPairList, HashIndexedAnyContainer};
use crate::error::{ErrorKind, RuntimeError};
use crate::memory::MutatorView;
use crate::printer::Print;
use crate::rawarray::{default_array_growth, RawArray, DEFAULT_ARRAY_SIZE};
use crate::safeptr::{MutatorScope, TaggedCellPtr, TaggedScopedPtr};
use crate::taggedptr::Value;

struct DictItem {
    hash: u64,
    key: TaggedPtr,
    value: TaggedPtr,
}

struct Dict {
    array: RawArray<DictItem>,
    size: ArraySize,
}
