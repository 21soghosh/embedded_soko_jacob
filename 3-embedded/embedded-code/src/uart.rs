use core::fmt::Write;
use tudelft_lm3s6965_pac::interrupt;
use tudelft_lm3s6965_pac::UART0;

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
