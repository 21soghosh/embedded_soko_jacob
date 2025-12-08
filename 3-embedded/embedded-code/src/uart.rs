use core::fmt::Write;
use tudelft_lm3s6965_pac::interrupt;
use tudelft_lm3s6965_pac::UART0;

use crate::mutex::Mutex;

const BUFFER_SIZE: usize = 256;

struct RingBuffer {
    buffer: [u8; BUFFER_SIZE],
    head: usize,
    tail: usize,
    full: bool,
}

impl RingBuffer {
    pub const fn new() -> Self {
        Self {
            buffer: [0; BUFFER_SIZE],
            head: 0,
            tail: 0,
            full: false,
        }
    }
    fn is_empty(&self) -> bool {
        if self.full {
            false
        } else if self.head == self.tail {
            true
        } else {
            false
        }
    }
    pub fn push(&mut self, byte: u8) {
        self.buffer[self.head] = byte;
        if self.full {
            self.tail = (self.tail + 1) % BUFFER_SIZE;
        }
        self.head = (self.head + 1) % BUFFER_SIZE;
        self.full = self.head == self.tail;
    }
    pub fn pop(&mut self) -> Option<u8> {
        if self.is_empty() {
            return None;
        }
        let byte = self.buffer[self.tail];
        self.full = false;
        self.tail = (self.tail + 1) % BUFFER_SIZE;
        Some(byte)
    }
}

pub struct Uart {}

impl Uart {
    pub fn new(uart: UART0) -> Self {
        todo!()
    }

    pub fn write(&mut self, value: &[u8]) {
        todo!()
    }

    pub fn read(&mut self) -> Option<u8> {
        todo!()
    }
}

impl Write for Uart {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write(s.as_bytes());
        Ok(())
    }
}

#[interrupt]
unsafe fn UART0() {
    todo!()
}
