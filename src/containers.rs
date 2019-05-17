use std::cell::Cell;

use stickyimmix::ArraySize;

use crate::rawarray::RawArray;

pub trait Container<T: Sized> {
    fn new() -> Self;
}

pub trait StackContainer<T: Sized>: Container<T> {
    fn push(&self, item: T) -> &T;
    fn pop(&self) -> T;
}

pub trait IndexedContainer<T: Sized>: Container<T> {
    fn get(&self, index: ArraySize) -> &T;
    fn set(&self, index: ArraySize, item: T) -> &T;
}

pub struct Array<T: Sized> {
    length: Cell<ArraySize>,
    data: RawArray<T>,
}

impl<T: Sized> Container<T> for Array<T> {
    fn new() -> Array<T> {
        Array {
            length: Cell::new(0),
            data: RawArray::new(),
        }
    }
}

impl<T: Sized> StackContainer<T> for Array<T> {
    fn push(&self, item: T) -> &T {
        self.length.set(self.length.get() + 1);
        // TODO
        unimplemented!()
    }

    fn pop(&self) -> T {
        unimplemented!()
    }
}

impl<T: Sized> IndexedContainer<T> for Array<T> {
    fn get(&self, index: ArraySize) -> &T {
        unimplemented!()
    }

    fn set(&self, index: ArraySize, item: T) -> &T {
        unimplemented!()
    }
}
