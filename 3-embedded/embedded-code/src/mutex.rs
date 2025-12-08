use core::cell::UnsafeCell;
use cortex_m::interrupt;

pub struct Mutex<T> {
    data: UnsafeCell<T>,
}

impl<T> Mutex<T> {
    // make a new mutex
    pub const fn new(data: T) -> Self {
        Self {
            data: UnsafeCell::new(data),
        }
    }

    // "locks" the mutex, and runs v as a critical section
    pub fn update<U>(&self, v: impl FnOnce(&mut T) -> U) -> U {
        interrupt::free(|_| {
            let data = unsafe { &mut *self.data.get() };
            v(data)
        })
    }
}



unsafe impl<T> Sync for Mutex<T> where T: Send {}

