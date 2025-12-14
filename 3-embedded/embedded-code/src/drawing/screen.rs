use crate::drawing::brightness::Brightness;
use crate::drawing::font::{Character, NUMBERS};
use tudelft_lm3s6965_pac::{GPIO_PORTC, SSI0};

pub struct Screen<'p> {
    ssi: &'p mut SSI0,
    gpio: &'p mut GPIO_PORTC,
    fb: [[u8; (Screen::WIDTH / 2) as usize]; Screen::HEIGHT as usize],
}

impl<'p> Screen<'p> {
    pub const WIDTH: u8 = 128;
    pub const HEIGHT: u8 = 80;

    pub fn new(ssi: &'p mut SSI0, gpio: &'p mut GPIO_PORTC) -> Self {
        // 1. Ensure that the SSE bit in the SSICR1 register is disabled before making any configuration changes.
        ssi.cr1.write(|w| w.ssi_cr1_sse().clear_bit());

        // 2. Select whether the SSI is a master or slave:
        //     a. For master operations, set the SSICR1 register to 0x0000.0000.
        //     b. For slave mode (output enabled), set the SSICR1 register to 0x0000.0004.
        //     c. For slave mode (output disabled), set the SSICR1 register to 0x0000.000C.
        ssi.cr1.write(|w| w.ssi_cr1_ms().clear_bit());

        // 3. Configure the clock prescale divisor by writing the SSICPSR register.
        // SAFETY: according to the docs, 2 is a valid value for this register
        ssi.cpsr.write(|w| unsafe { w.ssi_cpsr_cpsdvsr().bits(2) });

        // 4. Write the SSICR0 register with the following configuration:
        //     ■ Serial clock rate (SCR)
        //     ■ Desired clock phase/polarity, if using Freescale SPI mode (SPH and SPO)
        //     ■ The protocol mode: Freescale SPI, TI SSF, MICROWIRE (FRF)
        //     ■ The data size (DSS)
        // SAFETY: according to the docs, 9 is a valid value for this register
        ssi.cr0.write(|w| unsafe { w.ssi_cr0_scr().bits(9) });

        // 5. Enable the SSI by setting the SSE bit in the SSICR1 register.
        ssi.cr1.write(|w| w.ssi_cr1_sse().set_bit());

        // 6. set the bitmask
        ssi.cr0.write(|w| w.ssi_cr0_dss().ssi_cr0_dss_16());

        // SAFETY: according to the docs, these are both valid values for these two registers
        gpio.den.write(|w| unsafe { w.bits(1) });
        gpio.dir.write(|w| unsafe { w.bits(0xff) });

        Self {
            ssi,
            gpio,
            fb: [[0; (Self::WIDTH / 2) as usize]; Self::HEIGHT as usize],
        }
    }

    fn write_ssi(&mut self, data: u16) {
        self.ssi.dr.write(|w| unsafe { w.ssi_dr_data().bits(data) });
        let _ = self.ssi.dr.read();
        while self.ssi.sr.read().ssi_sr_bsy().bit_is_set() {}
    }

    fn change_mode(&mut self, mode: Mode) {
        // SAFETY: these two values registers can have any 7 bit value.
        // the exact values correspond with the ones qemu expects here
        // which we checked by reading qemu's source code.
        match mode {
            Mode::Cmd => self.gpio.data.write(|w| unsafe { w.bits(0x00) }),
            Mode::Data => self.gpio.data.write(|w| unsafe { w.bits(0xa0) }),
        }
    }

    /// Assumes we are in command mode and min/max are in bounds
    fn set_col(&mut self, min_curr: u8, max: u8) {
        self.write_ssi(0x15);
        self.write_ssi(min_curr as u16);
        self.write_ssi(max as u16);
    }

    /// Assumes we are in command mode and min/max are in bounds
    fn set_row(&mut self, min_curr: u8, max: u8) {
        self.write_ssi(0x75);
        self.write_ssi(min_curr as u16);
        self.write_ssi(max as u16);
    }

    pub fn draw_pixel(&mut self, x: u8, y: u8, brightness: Brightness) {
        assert!(x < 128, "x larger than width");
        assert!(y < 64, "y larger than height");

        self.change_mode(Mode::Cmd);
        self.set_col(x / 2, Self::WIDTH - 1);
        self.set_row(y, Self::HEIGHT - 1);

        self.change_mode(Mode::Data);

        let current = &mut self.fb[y as usize][x as usize / 2];
        if x % 2 == 1 {
            *current &= 0xf0;
            *current |= Into::<u8>::into(brightness);
        } else {
            *current &= 0x0f;
            *current |= Into::<u8>::into(brightness) << 4;
        }

        let value = *current;
        self.write_ssi(value as u16);
    }

    pub fn clear(&mut self, brightness: Brightness) {
        self.change_mode(Mode::Cmd);
        self.set_row(0, Self::HEIGHT - 1);
        self.set_col(0, Self::WIDTH - 1);

        self.change_mode(Mode::Data);

        let brightness: u8 = brightness.into();
        let pix = brightness | (brightness << 4);

        for x in 0..(Self::WIDTH / 2) as usize {
            for y in 0..Self::HEIGHT as usize {
                self.write_ssi(pix as u16);
                self.fb[y][x] = pix;
            }
        }
    }
    pub fn draw_line(&mut self, x0: u8, y0: u8, x1: u8, y1: u8, brightness: Brightness) {
        let mut x0 = x0 as i16;
        let mut y0 = y0 as i16;
        let x1 = x1 as i16;
        let y1 = y1 as i16;

        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            self.draw_pixel(x0 as u8, y0 as u8, brightness);
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    pub fn draw_character(&mut self, x: u8, y: u8, character: &Character, brightness: Brightness) {
        for i in 0..16 {
            for j in 0..8 {
                if character[i][j] {
                    self.draw_pixel(x + j as u8, y + i as u8, brightness);
                }
            }
        }
    }

    pub fn draw_filled_box(&mut self, x0: u8, y0: u8, x1: u8, y1: u8, brightness: Brightness) {
        for x in x0..=x1 {
            for y in y0..=y1 {
                self.draw_pixel(x, y, brightness);
            }
        }
    }

    pub fn render_step_counter(&mut self, total_steps: u32, player_brightness: Brightness) {
        self.clear(Brightness::WHITE);
        self.draw_number(4, 4, total_steps, player_brightness);
    }

    pub fn draw_trail(
        &mut self,
        trail: &Trail,
        trail_brightness: Brightness,
        player_brightness: Brightness,
    ) {
        self.clear(Brightness::WHITE);
        if trail.is_empty() {
            return;
        }

        let mut prev = trail.points[0];
        for idx in 1..trail.len {
            let curr = trail.points[idx];
            self.draw_line(prev.0, prev.1, curr.0, curr.1, trail_brightness);
            prev = curr;
        }
        self.draw_pixel(prev.0, prev.1, player_brightness);
    }

    fn draw_number(
        &mut self,
        origin_x: u8,
        origin_y: u8,
        mut value: u32,
        player_brightness: Brightness,
    ) {
        if value == 0 {
            self.draw_character(origin_x, origin_y, &NUMBERS[0], player_brightness);
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
            self.draw_character(x, origin_y, &NUMBERS[digit], player_brightness);
            x = x.saturating_add(9); // 8px wide + 1px spacing
        }
    }
}

pub struct Trail {
    points: [(u8, u8); Trail::MAX_POINTS],
    len: usize,
}

impl Trail {
    pub const MAX_POINTS: usize = 512;

    pub fn new(x: u8, y: u8) -> Self {
        let mut points = [(0u8, 0u8); Trail::MAX_POINTS];
        points[0] = (x, y);
        Self { points, len: 1 }
    }

    pub fn push(&mut self, x: u8, y: u8) {
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

    pub fn clear(&mut self, x: u8, y: u8) {
        self.points[0] = (x, y);
        self.len = 1;
    }

    fn is_empty(&self) -> bool {
        self.len == 0
    }
}

enum Mode {
    Cmd,
    Data,
}
