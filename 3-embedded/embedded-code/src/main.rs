#![no_std]
#![no_main]

extern crate cortex_m_rt as rt;
extern crate tudelft_lm3s6965_pac as _;

use crate::uart::Uart;
use core::arch::asm;
use cortex_m_semihosting::hprintln;
use drawing::brightness::Brightness;
use drawing::screen::Screen;
use message::Message;
use rt::entry;
use tudelft_lm3s6965_pac::Peripherals;

mod drawing;
mod exceptions;
mod uart;

mod message;
mod mutex;

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

    screen.draw_filled_box(
        pos_x - 1,
        pos_y - 1,
        pos_x + 1,
        pos_y + 1,
        Brightness::new(0),
    );
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
                        Ok(Message::Move { dx, dy}) => {
                            let steps = (dx.abs() + dy.abs()) as u16;
                            hprintln!("Instruction: move dx {}, dy {}, steps {}", dx, dy, steps);
                            move_player(
                                &mut screen,
                                &mut pos_x,
                                &mut pos_y,
                                dx as i16,
                                dy as i16,
                                steps as u16,
                            );
                        }
                        Ok(Message::MoveTo { x, y }) => {
                            let remaining_x = x as i16 - pos_x as i16;
                            let remaining_y = y as i16 - pos_y as i16;
                            let steps = (remaining_x.abs() + remaining_y.abs()) as u16;
                            hprintln!("Instruction: move_to x {}, y {}, steps {}", x, y, steps);
                            move_player(
                                &mut screen,
                                &mut pos_x,
                                &mut pos_y,
                                remaining_x,
                                remaining_y,
                                steps,
                            );
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

fn move_player(
    screen: &mut Screen,
    pos_x: &mut u8,
    pos_y: &mut u8,
    mut remaining_x: i16,
    mut remaining_y: i16,
    steps: u16,
) {

    for _ in 0..steps {
        screen.clear(Brightness::WHITE);

        if remaining_x != 0 {
            let step = remaining_x.signum();
            let new_x = (*pos_x as i16 + step).clamp(0, Screen::WIDTH as i16 - 1) as u8;
            *pos_x = new_x;
            remaining_x -= step;
        } else if remaining_y != 0 {
            let step = remaining_y.signum();
            let new_y = (*pos_y as i16 + step).clamp(0, Screen::HEIGHT as i16 - 1) as u8;
            *pos_y = new_y;
            remaining_y -= step;
        }

        let min_x = pos_x.saturating_sub(1);
        let min_y = pos_y.saturating_sub(1);
        let max_x = (*pos_x + 1).min(Screen::WIDTH - 1);
        let max_y = (*pos_y + 1).min(Screen::HEIGHT - 1);

        screen.draw_filled_box(min_x, min_y, max_x, max_y, Brightness::new(0));
    }
}
