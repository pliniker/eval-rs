/// A memory arena implemented as an ever growing pool of blocks.
/// Currently implemented on top of stickyimmix without any gc which includes unnecessary
/// overhead.

use stickyimmix::{AllocError, AllocHeader, AllocRaw, Mark, RawPtr, SizeClass, StickyImmixHeap};

use taggedptr::TypeList;


/// Allocation header for an Arena-allocated value
pub struct ArenaHeader {
    // TODO
}


/// Since we're not using this functionality in an Arena, the impl is just
/// a set of no-ops.
impl AllocHeader for ArenaHeader {
    type TypeId = TypeList;

    fn mark(&mut self) {}

    fn is_marked(&self) -> bool { true }

    fn size_class(&self) -> SizeClass { SizeClass::Small }
}


/// A non-garbage-collected pool of memory blocks for interned values.
/// These values are not dropped on Arena deallocation.
/// Values must be "atomic", that is, not composed of other object
/// pointers that need to be traced.
pub struct Arena {
    heap: StickyImmixHeap<ArenaHeader>
}


impl Arena {
    pub fn new() -> Arena {
        Arena {
            heap: StickyImmixHeap::new()
        }
    }
}


impl AllocRaw for Arena {
    type Header = ArenaHeader;

    fn alloc<T>(&self, object: T) -> Result<RawPtr<T>, AllocError> {
        self.heap.alloc(object)
    }

    fn get_header(_object: *const ()) -> *const Self::Header {
        unimplemented!()
    }

    fn get_object(_header: *const Self::Header) -> *const () {
        unimplemented!()
    }
}
