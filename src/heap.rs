
use stickyimmix::StickyImmixHeap;

use taggedptr::{ObjectHeader, TypeList};


pub type Heap = StickyImmixHeap<ObjectHeader>;
