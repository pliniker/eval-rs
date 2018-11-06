
use stickyimmix::StickyImmixHeap;

use crate::taggedptr::{ObjectHeader, TypeList};


pub type Heap = StickyImmixHeap<ObjectHeader>;
