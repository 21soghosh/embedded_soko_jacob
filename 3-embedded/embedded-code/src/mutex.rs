use core::marker::PhantomData;

pub struct Mutex<T> {
    // TODO: remove this field. It's so it compiles.
    phantom: PhantomData<T>,
}

impl<T> Mutex<T> {
    // make a new mutex
    pub fn new(data: T) -> Self {
        todo!()
    }

    // "locks" the mutex, and runs v as a critical section
    pub fn update<U>(&self, v: impl FnOnce(&mut T) -> U) -> U {
        todo!()
    }
}
