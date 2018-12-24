use stickyimmix::StickyImmixHeap;

use crate::taggedptr::ObjectHeader;

pub type Heap = StickyImmixHeap<ObjectHeader>;
