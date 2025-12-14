#![no_std]
#![no_main]

extern crate cortex_m_rt as rt;
extern crate tudelft_lm3s6965_pac as _;

use crate::uart::Uart;
use core::arch::asm;
use cortex_m_semihosting::hprintln;
use drawing::brightness::Brightness;
use drawing::screen::{Screen, Trail};
use message::{calculate_checksum, DisplayMode, Envelope, Message};
use rt::entry;
use tudelft_lm3s6965_pac::Peripherals;

const TRAIL_BRIGHTNESS: Brightness = Brightness::new(6);
const PLAYER_BRIGHTNESS: Brightness = Brightness::new(0);

mod drawing;
mod exceptions;
mod uart;

mod mutex;

#[entry]
fn main() -> ! {
    // hprintln is kind of like cheating. On real hardware this is (usually)
    // not possible, but because we are running inside an emulator, we can
    // actually talk to the emulator and print to the stdout fo the emulator.
    // This is useful for debugging, but again: it doesn't work on real hardware.
    hprintln!("code started");
    let mut dp = Peripherals::take().unwrap();

    // initialize the screen
    let mut screen = Screen::new(&mut dp.SSI0, &mut dp.GPIO_PORTC);
    screen.clear(Brightness::WHITE);
    let mut pos_x = Screen::WIDTH / 2;
    let mut pos_y = Screen::HEIGHT / 2;
    let mut display_mode = DisplayMode::Trail;
    let mut total_steps: u32 = 0;
    let mut trail = Trail::new(pos_x, pos_y);

    // draw the player
    screen.draw_pixel(pos_x, pos_y, PLAYER_BRIGHTNESS);

    // initialize the UART.
    let mut uart = Uart::new(dp.UART0);

    // initialize receive buffer
    let mut rx_buf = [0u8; 32];
    let mut rx_len = 0usize;

    // main loop
    loop {
        // read all available bytes
        while let Some(byte) = uart.read() {
            // check for end of message
            if byte == 0 {
                if rx_len > 0 {
                    match postcard::from_bytes_cobs::<Envelope>(&mut rx_buf[..rx_len]) {
                        Ok(envelope) => match calculate_checksum(&envelope.msg) {
                            Some(expected) if expected == envelope.checksum => {
                                handle_message(
                                    envelope.msg,
                                    &mut screen,
                                    &mut pos_x,
                                    &mut pos_y,
                                    &mut trail,
                                    &mut total_steps,
                                    &mut display_mode,
                                );
                            }
                            Some(_) => hprintln!("Checksum mismatch, discarding message"),
                            None => hprintln!("Failed to compute checksum, discarding message"),
                        },
                        Err(_) => {
                            hprintln!("Invalid message received");
                        }
                    }
                }
                rx_len = 0;
                continue;
            }

            // store byte in buffer
            if rx_len < rx_buf.len() {
                rx_buf[rx_len] = byte;
                rx_len += 1;
            } else {
                rx_len = 0;
                hprintln!("Message too long, discarding");
            }
        }
    }
}

fn move_player(
    screen: &mut Screen,
    pos_x: &mut u8,
    pos_y: &mut u8,
    mut dx: i16,
    mut dy: i16,
    steps: u16,
    display_mode: DisplayMode,
    trail: &mut Trail,
    total_steps: &mut u32,
) {
    if steps == 0 {
        return;
    }

    for _ in 0..steps {
        let prev_x = *pos_x;
        let prev_y = *pos_y;

        if dx != 0 {
            let step = dx.signum();
            let new_x = (*pos_x as i16 + step).clamp(0, Screen::WIDTH as i16 - 1) as u8;
            *pos_x = new_x;
            dx -= step;
        } else if dy != 0 {
            let step = dy.signum();
            let new_y = (*pos_y as i16 + step).clamp(0, Screen::HEIGHT as i16 - 1) as u8;
            *pos_y = new_y;
            dy -= step;
        }

        trail.push(*pos_x, *pos_y);

        if display_mode == DisplayMode::Trail {
            screen.draw_line(prev_x, prev_y, *pos_x, *pos_y, TRAIL_BRIGHTNESS);
            screen.draw_pixel(*pos_x, *pos_y, PLAYER_BRIGHTNESS);
        }
    }

    *total_steps = total_steps.wrapping_add(1);

    if display_mode == DisplayMode::Steps {
        screen.render_step_counter(*total_steps, PLAYER_BRIGHTNESS);
    }
}

fn handle_message(
    msg: Message,
    screen: &mut Screen,
    pos_x: &mut u8,
    pos_y: &mut u8,
    trail: &mut Trail,
    total_steps: &mut u32,
    display_mode: &mut DisplayMode,
) {
    match msg {
        Message::Move { dx, dy } => {
            let steps = (dx.abs() + dy.abs()) as u16;
            hprintln!("Instruction: move dx {}, dy {}, steps {}", dx, dy, steps);
            move_player(
                screen,
                pos_x,
                pos_y,
                dx as i16,
                dy as i16,
                steps,
                *display_mode,
                trail,
                total_steps,
            );
        }
        Message::MoveTo { x, y } => {
            let dx = x as i16 - *pos_x as i16;
            let dy = y as i16 - *pos_y as i16;
            let steps = (dx.abs() + dy.abs()) as u16;
            hprintln!("Instruction: move_to x {}, y {}, steps {}", x, y, steps);
            move_player(
                screen,
                pos_x,
                pos_y,
                dx,
                dy,
                steps,
                *display_mode,
                trail,
                total_steps,
            );
        }
        Message::Reset => {
            *pos_x = Screen::WIDTH / 2;
            *pos_y = Screen::HEIGHT / 2;
            *total_steps = 0;
            trail.clear(*pos_x, *pos_y);

            match *display_mode {
                DisplayMode::Trail => screen.draw_trail(trail, TRAIL_BRIGHTNESS, PLAYER_BRIGHTNESS),
                DisplayMode::Steps => screen.render_step_counter(*total_steps, PLAYER_BRIGHTNESS),
            }
        }
        Message::SetDisplayMode(mode) => {
            *display_mode = mode;
            match mode {
                DisplayMode::Trail => screen.draw_trail(trail, TRAIL_BRIGHTNESS, PLAYER_BRIGHTNESS),
                DisplayMode::Steps => screen.render_step_counter(*total_steps, PLAYER_BRIGHTNESS),
            }
        }
    }
}
