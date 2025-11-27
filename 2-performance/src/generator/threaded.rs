use crate::generator::{Callback, Generator};
use crate::util::camera::Camera;
use crate::util::outputbuffer::OutputBuffer;
use std::sync::{Arc, Mutex};
use std::thread;

use log::info;

#[derive(Debug)]
pub struct ThreadedGenerator {
    threads: usize,
}

impl ThreadedGenerator {
    pub fn new(threads: usize) -> Self {
        Self { threads }
    }
}

impl Generator for ThreadedGenerator {
    fn generate(&self, camera: &Camera, callback: &Callback) -> OutputBuffer {
        // Shared output buffer, same as before
        let output = Arc::new(Mutex::new(OutputBuffer::with_size(
            camera.width,
            camera.height,
            "backup.rgb",
        )));

        thread::scope(|s| {
            // How many rows each thread should handle
            let rows_per_thread = (camera.height / self.threads)
                + if camera.height % self.threads == 0 { 0 } else { 1 };

            // Ceiling division: how many chunks we need
            let chunks = (camera.height + rows_per_thread - 1) / rows_per_thread;

            for index in 0..chunks {
                let start_y = index * rows_per_thread;

                let local_output = Arc::clone(&output);
                let width = camera.width;
                let height = camera.height;

                s.spawn(move || {
                    for y in start_y..(start_y + rows_per_thread) {
                        if y >= height {
                            break;
                        }

                        // 1) Compute the row without holding the lock
                        let mut row_pixels = Vec::with_capacity(width as usize);
                        for x in 0..width {
                            let color = callback(x, y);
                            row_pixels.push((x, color));
                        }

                        // 2) Lock once and write the whole row
                        let mut guard = local_output.lock().unwrap();
                        for (x, color) in row_pixels {
                            guard.set_at(x, y, color);
                        }
                        // guard is dropped here -> lock released
                    }
                });
            }
        });
        // First, consume the Arc and get the Mutex<OutputBuffer>
        let mutex = Arc::try_unwrap(output).unwrap_or_else(|_| {
            panic!("More than one Arc reference to output exists");
        });

        // Then, consume the Mutex and get the OutputBuffer
        let buffer = mutex.into_inner().unwrap_or_else(|_| {
            panic!("Mutex poisoned");
        });

        buffer
    }
}

