/// Container traits
///
/// TODO iterators/views
use stickyimmix::ArraySize;

use crate::error::RuntimeError;
use crate::memory::MutatorView;
use crate::safeptr::{MutatorScope, TaggedCellPtr, TaggedScopedPtr};

/// Base container-type trait. All container types are subtypes of `Container`.
///
/// All container operations _must_ follow interior mutability only rules.
/// Because there are no compile-time mutable aliasing guarantees, there can be no references
/// into arrays at all, unless there can be a guarantee that the array memory will not be
/// reallocated.
///
/// `T` cannot be restricted to `Copy` because of the use of `Cell` for interior mutability.
pub trait Container<T: Sized + Clone>: Sized {
    /// Create a new, empty container instance.
    fn new() -> Self;
    /// Create a new container instance with the given capacity.
    fn with_capacity<'guard>(
        mem: &'guard MutatorView,
        capacity: ArraySize,
    ) -> Result<Self, RuntimeError>;

    /// Reset the size of the container to zero - empty
    fn clear<'guard>(&self, mem: &'guard MutatorView) -> Result<(), RuntimeError>;

    /// Count of items in the container
    fn length(&self) -> ArraySize;
}

/// Generic stack trait. If implemented, the container can function as a stack
pub trait StackContainer<T: Sized + Clone>: Container<T> {
    /// Push can trigger an underlying array resize, hence it requires the ability to allocate
    fn push<'guard>(&self, mem: &'guard MutatorView, item: T) -> Result<(), RuntimeError>;

    /// Pop returns a bounds error if the container is empty, otherwise moves the last item of the
    /// array out to the caller.
    fn pop<'guard>(&self, _guard: &'guard dyn MutatorScope) -> Result<T, RuntimeError>;

    /// Return the value at the top of the stack without removing it
    fn top<'guard>(&self, _guard: &'guard dyn MutatorScope) -> Result<T, RuntimeError>;
}

/// Specialized stack trait. If implemented, the container can function as a stack
pub trait StackAnyContainer: StackContainer<TaggedCellPtr> {
    /// Push can trigger an underlying array resize, hence it requires the ability to allocate
    fn push<'guard>(
        &self,
        mem: &'guard MutatorView,
        item: TaggedScopedPtr<'guard>,
    ) -> Result<(), RuntimeError>;

    /// Pop returns a bounds error if the container is empty, otherwise moves the last item of the
    /// array out to the caller.
    fn pop<'guard>(
        &self,
        _guard: &'guard dyn MutatorScope,
    ) -> Result<TaggedScopedPtr<'guard>, RuntimeError>;

    /// Return the value at the top of the stack without removing it
    fn top<'guard>(
        &self,
        _guard: &'guard dyn MutatorScope,
    ) -> Result<TaggedScopedPtr<'guard>, RuntimeError>;
}

/// Generic indexed-access trait. If implemented, the container can function as an indexable vector
pub trait IndexedContainer<T: Sized + Clone>: Container<T> {
    /// Return a copy of the object at the given index. Bounds-checked.
    fn get<'guard>(
        &self,
        _guard: &'guard dyn MutatorScope,
        index: ArraySize,
    ) -> Result<T, RuntimeError>;

    /// Move an object into the array at the given index. Bounds-checked.
    fn set<'guard>(
        &self,
        _guard: &'guard dyn MutatorScope,
        index: ArraySize,
        item: T,
    ) -> Result<(), RuntimeError>;
}

/// A function pointer that takes a slice of type T and returns a value of type R.
pub type SliceOp<T, R> = fn(&mut [T]) -> R;

/// A trait that is implemented for containers that can represent their contents as a slice.
pub trait SliceableContainer<T: Sized + Clone>: IndexedContainer<T> {
    /// This function allows access to the interior of a container as a slice by way of a
    /// function, permitting direct access to the memory locations of objects in the container
    /// for the lifetime of the closure call.
    ///
    /// It is important to understand that the 'guard lifetime is not the same safe duration
    /// as the slice lifetime - the slice may be invalidated during the 'guard lifetime
    /// by operations on the container that cause reallocation.
    ///
    /// To prevent the function from modifying the container outside of the slice reference,
    /// the function may not capture it's environment: hence the requirement for a plain
    /// function pointer.
    ///
    /// This retains the core requirement of interior mutability.
    fn access_slice<'guard, R>(&self, _guard: &'guard dyn MutatorScope, f: SliceOp<T, R>) -> R;
}

/// Specialized indexable interface for where TaggedCellPtr is used as T
pub trait IndexedAnyContainer: IndexedContainer<TaggedCellPtr> {
    /// Return a pointer to the object at the given index. Bounds-checked.
    fn get<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        index: ArraySize,
    ) -> Result<TaggedScopedPtr<'guard>, RuntimeError>;

    /// Set the object pointer at the given index. Bounds-checked.
    fn set<'guard>(
        &self,
        _guard: &'guard dyn MutatorScope,
        index: ArraySize,
        item: TaggedScopedPtr<'guard>,
    ) -> Result<(), RuntimeError>;
}

/// Hashable-indexed interface. Objects used as keys must implement Hashable.
pub trait HashIndexedAnyContainer {
    /// Return a pointer to to the object associated with the given key.
    /// Absence of an association should return an error.
    fn lookup<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        key: TaggedScopedPtr,
    ) -> Result<TaggedScopedPtr<'guard>, RuntimeError>;

    /// Associate a key with a value.
    fn assoc<'guard>(
        &self,
        mem: &'guard MutatorView,
        key: TaggedScopedPtr<'guard>,
        value: TaggedScopedPtr<'guard>,
    ) -> Result<(), RuntimeError>;

    /// Remove an association by its key.
    fn dissoc<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        key: TaggedScopedPtr,
    ) -> Result<TaggedScopedPtr<'guard>, RuntimeError>;

    /// Returns true if the key exists in the container.
    fn exists<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        key: TaggedScopedPtr,
    ) -> Result<bool, RuntimeError>;
}

/// Convert a Pair list to a different container
pub trait ContainerFromPairList: Container<TaggedCellPtr> {
    fn from_pair_list<'guard>(
        &self,
        mem: &'guard MutatorView,
        pair_list: TaggedScopedPtr<'guard>,
    ) -> Result<(), RuntimeError>;
}

/// The implementor represents mutable changes via an internal version count
/// such that the use of any references to an older version return an error
pub trait VersionedContainer<T: Sized + Clone>: Container<T> {}

pub trait ImmutableContainer<T: Sized + Clone>: Container<T> {}
