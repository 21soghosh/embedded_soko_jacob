
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::cell::RefCell;
use std::thread_local;

pub mod camera;
pub mod color;
pub mod consts;
pub mod outputbuffer;
pub mod ray;
pub mod vector;

thread_local! {
    static THREAD_RNG: RefCell<SmallRng> = RefCell::new(SmallRng::from_entropy());
}

/// Fast, per-thread random value.
pub fn random_f64() -> f64 {
    THREAD_RNG.with(|rng| rng.borrow_mut().gen())
}
