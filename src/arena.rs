

use stickyimmix::{AllocError, AllocHeader, AllocRaw, Heap, Mark, RawPtr, SizeClass};


/// Allocation header for an Arena-allocated value
struct ArenaHeader {
    // TODO
}


/// Since we're not using this functionality in an Arena, the impl is just
/// a set of no-ops.
impl AllocHeader for ArenaHeader {
    fn new(_size_class: SizeClass, _mark_bit: Mark) -> Self {
        ArenaHeader {}
    }

    fn mark(&mut self) {}

    fn is_marked(&self) -> bool { true }

    fn size_class(&self) -> SizeClass { SizeClass::Small }
}


/// A non-garbage-collected pool of memory blocks for interned values.
/// These values are not dropped on Arena deallocation.
/// Values must be "atomic", that is, not composed of other object
/// pointers that need to be traced.
pub struct Arena {
    heap: Heap<ArenaHeader>
}


impl Arena {
    pub fn new() -> Arena {
        Arena {
            heap: Heap::new()
        }
    }
}


impl AllocRaw for Arena {
    type Header = ArenaHeader;

    fn alloc<T>(&self, object: T) -> Result<RawPtr<T>, AllocError> {
        self.heap.alloc(object)
    }

    fn get_header(_object: *const ()) -> Self::Header {
        unimplemented!() // TODO
    }
}
