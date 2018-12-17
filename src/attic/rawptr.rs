
/// Wrapper around a bare pointer type
pub struct RawPtr<T> {
    raw: *mut T
}


impl<T> Clone for RawPtr<T> {
    fn clone(&self) -> RawPtr<T> {
        RawPtr {
            raw: self.raw
        }
    }
}


impl<T> Copy for RawPtr<T> {}


impl<T> RawPtr<T> {
    /// From a bare pointer
    pub fn from_bare(object: *mut T) -> RawPtr<T> {
        RawPtr {
            raw: object
        }
    }

    pub unsafe fn deref(&self) -> &T {
        &*self.raw
    }

    pub unsafe fn deref_mut(&mut self) -> &mut T {
        &mut *self.raw
    }

    pub fn to_bare(&self) -> *mut T {
        self.raw
    }
}
