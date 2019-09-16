/// Container traits
///
/// TODO iterators/views
use stickyimmix::ArraySize;

use crate::error::RuntimeError;
use crate::memory::MutatorView;
use crate::safeptr::{CellPtr, MutatorScope, ScopedPtr};

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
}

/// Specialized stack trait. If implemented, the container can function as a stack
pub trait StackAnyContainer: StackContainer<CellPtr> {
    /// Push can trigger an underlying array resize, hence it requires the ability to allocate
    fn push<'guard>(
        &self,
        mem: &'guard MutatorView,
        item: ScopedPtr<'guard>,
    ) -> Result<(), RuntimeError>;

    /// Pop returns a bounds error if the container is empty, otherwise moves the last item of the
    /// array out to the caller.
    fn pop<'guard>(
        &self,
        _guard: &'guard dyn MutatorScope,
    ) -> Result<ScopedPtr<'guard>, RuntimeError>;
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

/// Specialized indexable interface for where CellPtr is used as T
pub trait IndexedAnyContainer: IndexedContainer<CellPtr> {
    /// Return a pointer to the object at the given index. Bounds-checked.
    fn get<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        index: ArraySize,
    ) -> Result<ScopedPtr<'guard>, RuntimeError>;

    /// Set the object pointer at the given index. Bounds-checked.
    fn set<'guard>(
        &self,
        _guard: &'guard dyn MutatorScope,
        index: ArraySize,
        item: ScopedPtr<'guard>,
    ) -> Result<(), RuntimeError>;
}

/// The implementor represents mutable changes via an internal version count
/// such that the use of any references to an older version return an error
pub trait VersionedContainer<T: Sized + Clone>: Container<T> {}

pub trait ImmutableContainer<T: Sized + Clone>: Container<T> {}

/// Experimental
pub trait SliceSafeContainer<T: Sized + Clone>: ImmutableContainer<T> {
    /// Give a closure a view of the container as a slice.
    /// Restricting to `Fn` means interior mutability rules can be maintained. The closure cannot
    /// safely escape a reference to an object inside the array.
    /// It _is_ possible to reallocate the array while a slice is held - the slice will continue
    /// to refer to the old memory. This is a problem but strictly not unsafe because the
    /// lifetime limitation guarantee is non-invalidation of memory during the mutator lifetime.
    fn slice_apply<'guard, F>(
        &self,
        _guard: &'guard dyn MutatorScope,
        op: F,
    ) -> Result<(), RuntimeError>
    where
        F: Fn(&[T]) -> Result<(), RuntimeError>;
}
/*
/// Experimental
/// Give a closure a view of the container as a slice.
/// Restricting to `Fn` means interior mutability rules can be maintained. The closure cannot
/// safely escape a reference to an object inside the array.
fn slice_apply<'guard, F>(
&self,
_guard: &'guard dyn MutatorScope,
op: F,
    ) -> Result<(), RuntimeError>
    where
        F: Fn(&[T]) -> Result<(), RuntimeError>,
    {
        if let Some(ptr) = self.data.get().as_ptr() {
            let as_slice = unsafe { from_raw_parts(ptr, self.length.get() as usize) };

            op(as_slice)
        } else {
            Ok(())
        }
    }
*/
