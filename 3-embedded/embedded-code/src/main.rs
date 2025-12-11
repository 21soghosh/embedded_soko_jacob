#![no_std]
#![no_main]

extern crate cortex_m_rt as rt;
extern crate tudelft_lm3s6965_pac as _;

use crate::uart::Uart;
use core::arch::asm;
use cortex_m_semihosting::hprintln;
use drawing::brightness::Brightness;
use drawing::screen::Screen;
use rt::entry;
use tudelft_lm3s6965_pac::Peripherals;
use message::Message;

mod drawing;
mod exceptions;
mod uart;

mod mutex;
mod message;

#[entry]
fn main() -> ! {
    // hprintln is kind of like cheating. On real hardware this is (usually)
    // not possible, but because we are running inside an emulator, we can
    // actually talk to the emulator and print to the stdout fo the emulator.
    // This is useful for debugging, but again: it doesn't work on real hardware.
    hprintln!("code started");
    let mut dp = Peripherals::take().unwrap();

    // initialize the screen for drawing
    let mut screen = Screen::new(&mut dp.SSI0, &mut dp.GPIO_PORTC);
    screen.clear(Brightness::WHITE);
    let mut pos_x = Screen::WIDTH / 2;
    let mut pos_y = Screen::HEIGHT / 2;

    screen.draw_filled_box(pos_x - 1, pos_y - 1, pos_x + 1, pos_y + 1, Brightness::new(0));
    // initialize the UART.
    let mut uart = Uart::new(dp.UART0);

    // buffer incoming bytes until we see the COBS frame delimiter (0x00)
    let mut rx_buf = [0u8; 32];
    let mut rx_len = 0usize;

    loop {
        while let Some(byte) = uart.read() {
            if byte == 0 {
                if rx_len > 0 {
                    // postcard needs a mutable slice for in-place COBS deframing
                    match postcard::from_bytes_cobs::<Message>(&mut rx_buf[..rx_len]) {
                        Ok(msg) => {
                            hprintln!("Instruction: dx {}, dy {}, steps {}", msg.dx, msg.dy, msg.steps);
                            apply_instruction(&mut screen, &mut pos_x, &mut pos_y, &msg);
                        }
                        Err(_) => {
                            hprintln!("Invalid message received");
                        }
                    }
                }
                rx_len = 0;
                continue;
            }

            if rx_len < rx_buf.len() {
                rx_buf[rx_len] = byte;
                rx_len += 1;
            } else {
                rx_len = 0;
                hprintln!("Message too long, discarding");
            }
        }

        // wait for interrupts, before looping again to save cycles.
        unsafe { asm!("wfi") }
    }
}

fn apply_instruction(screen: &mut Screen, pos_x: &mut u8, pos_y: &mut u8, msg: &Message) {
    for _ in 0..msg.steps {
        screen.clear(Brightness::WHITE);

        let new_x = (*pos_x as i16 + msg.dx as i16).clamp(0, Screen::WIDTH as i16 - 1) as u8;
        let new_y = (*pos_y as i16 + msg.dy as i16).clamp(0, Screen::HEIGHT as i16 - 1) as u8;

        *pos_x = new_x;
        *pos_y = new_y;

        let min_x = pos_x.saturating_sub(1);
        let min_y = pos_y.saturating_sub(1);
        let max_x = (*pos_x + 1).min(Screen::WIDTH - 1);
        let max_y = (*pos_y + 1).min(Screen::HEIGHT - 1);

        screen.draw_filled_box(min_x, min_y, max_x, max_y, Brightness::new(0));
    }
}
