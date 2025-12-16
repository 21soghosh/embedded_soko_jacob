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

struct GameState {
    x: u8,
    y: u8,
    trail: Trail,
    total_steps: u32,
    display_mode: DisplayMode,
}

impl GameState {
    fn new() -> Self {
        let x = Screen::WIDTH / 2;
        let y = Screen::HEIGHT / 2;
        Self {
            x,
            y,
            trail: Trail::new(x, y),
            total_steps: 0,
            display_mode: DisplayMode::Trail, // Standard visning
        }
    }

    // Flytt logikken for å tilbakestille inn her
    fn reset(&mut self, screen: &mut Screen) {
        self.x = Screen::WIDTH / 2;
        self.y = Screen::HEIGHT / 2;
        self.total_steps = 0;
        self.trail.clear(self.x, self.y);
        self.refresh_display(screen);
    }

    fn refresh_display(&self, screen: &mut Screen) {
        match self.display_mode {
            DisplayMode::Trail => {
                screen.draw_trail(&self.trail, TRAIL_BRIGHTNESS, PLAYER_BRIGHTNESS)
            }
            DisplayMode::Steps => screen.render_step_counter(self.total_steps, PLAYER_BRIGHTNESS),
        }
    }

    // Flytt move_player logikken inn her for å unngå å sende 9 argumenter
    fn move_player(&mut self, screen: &mut Screen, mut dx: i16, mut dy: i16, steps: u16) {
        if steps == 0 {
            return;
        }

        for _ in 0..steps {
            let prev_x = self.x;
            let prev_y = self.y;

            if dx != 0 {
                let step = dx.signum();
                // Bruk saturating casts eller sjekker for klarhet
                self.x = (self.x as i16 + step).clamp(0, Screen::WIDTH as i16 - 1) as u8;
                dx -= step;
            } else if dy != 0 {
                let step = dy.signum();
                self.y = (self.y as i16 + step).clamp(0, Screen::HEIGHT as i16 - 1) as u8;
                dy -= step;
            }

            self.trail.push(self.x, self.y);

            if self.display_mode == DisplayMode::Trail {
                screen.draw_line(prev_x, prev_y, self.x, self.y, TRAIL_BRIGHTNESS);
                screen.draw_pixel(self.x, self.y, PLAYER_BRIGHTNESS);
            }
        }
        self.total_steps = self.total_steps.wrapping_add(1);

        if self.display_mode == DisplayMode::Steps {
            screen.render_step_counter(self.total_steps, PLAYER_BRIGHTNESS);
        }
    }
}

#[entry]
fn main() -> ! {
    // hprintln is kind of like cheating. On real hardware this is (usually)
    // not possible, but because we are running inside an emulator, we can
    // actually talk to the emulator and print to the stdout fo the emulator.
    // This is useful for debugging, but again: it doesn't work on real hardware.
    hprintln!("code started");
    let mut dp = Peripherals::take().unwrap();

   let mut screen = Screen::new(&mut dp.SSI0, &mut dp.GPIO_PORTC);
    screen.clear(Brightness::WHITE);
    
    // Initialize game
    let mut game = GameState::new();
    
    // Draw starting position
    screen.draw_pixel(game.x, game.y, PLAYER_BRIGHTNESS);

    // UART setup
    let mut uart = Uart::new(dp.UART0);
    let mut rx_buf = [0u8; 32];
    let mut rx_len = 0usize;
    // main loop
    loop {
        // read all available bytes
        while let Some(byte) = uart.read() {
            // check for end of message
            if byte == 0 {
                if rx_len > 0 {
                    process_packet(&mut rx_buf[..rx_len], &mut game, &mut screen);
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

fn process_packet(data: &mut [u8], game: &mut GameState, screen: &mut Screen) {
     match postcard::from_bytes_cobs::<Envelope>(data) {
        Ok(envelope) => {
             if let Some(expected) = calculate_checksum(&envelope.msg) {
                 if expected == envelope.checksum {
                     handle_message(envelope.msg, game, screen);
                 } else {
                     hprintln!("Error: Checksum mismatch");
                 }
             } else {
                 hprintln!("Error: Could not calculate checksum");
             }
        }
        Err(_) => hprintln!("Error: Malformed packet"),
    }
}


fn handle_message(msg: Message, game: &mut GameState, screen: &mut Screen) {
    match msg {
        Message::Move { dx, dy } => {
            let steps = (dx.abs() + dy.abs()) as u16;
            game.move_player(screen, dx as i16, dy as i16, steps);
        }
        Message::MoveTo { x, y } => {
            let dx = x as i16 - game.x as i16;
            let dy = y as i16 - game.y as i16;
            let steps = (dx.abs() + dy.abs()) as u16;
            game.move_player(screen, dx, dy, steps);
        }
        Message::Reset => game.reset(screen),
        Message::SetDisplayMode(mode) => {
            game.display_mode = mode;
            game.refresh_display(screen);
        }
    }
}