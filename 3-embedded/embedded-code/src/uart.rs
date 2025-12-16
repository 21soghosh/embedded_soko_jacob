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

static BUFFER: Mutex<RingBuffer> = Mutex::new(RingBuffer::new());

pub struct Uart {
    uart: UART0,
}

impl Uart {
    pub fn new(uart: UART0) -> Self {
        // enable receive interrupts and timeout interrupts
        uart.im
            .write(|w| w.uart_im_rxim().set_bit().uart_im_rtim().set_bit());

        unsafe {
            // Why its safe: Since the only shared state is the BUFFER, which is controlled
            // via a Mutex, it is safe to unmask the interrupt here.
            cortex_m::peripheral::NVIC::unmask(tudelft_lm3s6965_pac::Interrupt::UART0);
        }

        Self { uart }
    }

    pub fn write(&mut self, value: &[u8]) {
        for &b in value {
            // Wait until TX FIFO is not full
            while self.uart.fr.read().uart_fr_txff().bit_is_set() {}
            // SAFETY: writing any u8 value to the data register is valid.
            self.uart.dr.write(|w| unsafe { w.uart_dr_data().bits(b) });
        }
    }

    pub fn read(&mut self) -> Option<u8> {
        BUFFER.update(|buf| buf.pop())
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
    let uart = &*tudelft_lm3s6965_pac::UART0::ptr();

    // Clear interrupts
    uart.icr
        .write(|w| w.uart_icr_rxic().set_bit().uart_icr_rtic().set_bit());

    // Read all available bytes from FIFO
    while uart.fr.read().uart_fr_rxfe().bit_is_clear() {
        let byte = uart.dr.read().uart_dr_data().bits() as u8;
        BUFFER.update(|buf| buf.push(byte));
    }
}
