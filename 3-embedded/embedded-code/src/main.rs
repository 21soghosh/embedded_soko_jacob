#![no_std]
#![no_main]

extern crate cortex_m_rt as rt;
extern crate tudelft_lm3s6965_pac as _;

use crate::uart::Uart;
use core::arch::asm;
use cortex_m_semihosting::hprintln;
use drawing::brightness::Brightness;
use drawing::font::NUMBERS;
use drawing::screen::Screen;
use message::{checksum_for, DisplayMode, Envelope, Message};
use rt::entry;
use tudelft_lm3s6965_pac::Peripherals;

const TRAIL_BRIGHTNESS: Brightness = Brightness::new(6); // lighter gray line
const PLAYER_BRIGHTNESS: Brightness = Brightness::new(0); // darkest marker

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
    let mut display_mode = DisplayMode::Trail;
    let mut total_steps: u32 = 0;
    let mut trail = Trail::new(pos_x, pos_y);

    screen.draw_pixel(pos_x, pos_y, PLAYER_BRIGHTNESS);
    // initialize the UART.
    let mut uart = Uart::new(dp.UART0);

    // buffer incoming bytes until we see the COBS frame delimiter (0x00)
    let mut rx_buf = [0u8; 32];
    let mut rx_len = 0usize;

    loop {
        while let Some(byte) = uart.read() {
            if byte == 0 {
                if rx_len > 0 {
                    match postcard::from_bytes_cobs::<Envelope>(&mut rx_buf[..rx_len]) {
                        Ok(envelope) => match checksum_for(&envelope.msg) {
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

        trail.push(*pos_x, *pos_y);

        if display_mode == DisplayMode::Trail {
            screen.draw_line(prev_x, prev_y, *pos_x, *pos_y, TRAIL_BRIGHTNESS);
            screen.draw_pixel(*pos_x, *pos_y, PLAYER_BRIGHTNESS);
        }
    }

    *total_steps = total_steps.wrapping_add(1);

    if display_mode == DisplayMode::Steps {
        render_step_counter(screen, *total_steps);
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
            let remaining_x = x as i16 - *pos_x as i16;
            let remaining_y = y as i16 - *pos_y as i16;
            let steps = (remaining_x.abs() + remaining_y.abs()) as u16;
            hprintln!("Instruction: move_to x {}, y {}, steps {}", x, y, steps);
            move_player(
                screen,
                pos_x,
                pos_y,
                remaining_x,
                remaining_y,
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
                DisplayMode::Trail => redraw_trail(screen, trail),
                DisplayMode::Steps => render_step_counter(screen, *total_steps),
            }
        }
        Message::SetDisplayMode(mode) => {
            *display_mode = mode;
            match mode {
                DisplayMode::Trail => redraw_trail(screen, trail),
                DisplayMode::Steps => render_step_counter(screen, *total_steps),
            }
        }
    }
}

fn render_step_counter(screen: &mut Screen, total_steps: u32) {
    screen.clear(Brightness::WHITE);
    draw_number(screen, 4, 4, total_steps);
}

fn redraw_trail(screen: &mut Screen, trail: &Trail) {
    screen.clear(Brightness::WHITE);
    if trail.len == 0 {
        return;
    }

    let mut prev = trail.points[0];
    for i in 1..trail.len {
        let curr = trail.points[i];
        screen.draw_line(prev.0, prev.1, curr.0, curr.1, TRAIL_BRIGHTNESS);
        prev = curr;
    }
    screen.draw_pixel(prev.0, prev.1, PLAYER_BRIGHTNESS);
}

fn draw_number(screen: &mut Screen, origin_x: u8, origin_y: u8, mut value: u32) {
    if value == 0 {
        screen.draw_character(origin_x, origin_y, &NUMBERS[0], PLAYER_BRIGHTNESS);
        return;
    }

    let mut digits = [0u8; 10];
    let mut count = 0;
    while value > 0 && count < digits.len() {
        digits[count] = (value % 10) as u8;
        value /= 10;
        count += 1;
    }

    let mut x = origin_x;
    for idx in (0..count).rev() {
        let digit = digits[idx] as usize;
        screen.draw_character(x, origin_y, &NUMBERS[digit], PLAYER_BRIGHTNESS);
        x = x.saturating_add(9); // 8px wide + 1px spacing
    }
}

struct Trail {
    points: [(u8, u8); Trail::MAX_POINTS],
    len: usize,
}

impl Trail {
    const MAX_POINTS: usize = 512;

    fn new(x: u8, y: u8) -> Self {
        let mut points = [(0u8, 0u8); Trail::MAX_POINTS];
        points[0] = (x, y);
        Self { points, len: 1 }
    }

    fn push(&mut self, x: u8, y: u8) {
        if self.len < Trail::MAX_POINTS {
            self.points[self.len] = (x, y);
            self.len += 1;
        } else {
            // drop oldest point to make room for new one
            for i in 1..Trail::MAX_POINTS {
                self.points[i - 1] = self.points[i];
            }
            self.points[Trail::MAX_POINTS - 1] = (x, y);
        }
    }

    fn clear(&mut self, x: u8, y: u8) {
        self.points[0] = (x, y);
        self.len = 1;
    }
}
