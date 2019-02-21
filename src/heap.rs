use stickyimmix::StickyImmixHeap;

use crate::headers::ObjectHeader;

/// The heap implementation
pub type Heap = StickyImmixHeap<ObjectHeader>;
