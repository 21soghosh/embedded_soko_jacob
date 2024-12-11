#[derive(Copy, Clone)]
pub struct Brightness(u8);

impl From<Brightness> for u8 {
    fn from(b: Brightness) -> u8 {
        b.0
    }
}

impl Brightness {
    pub const WHITE: Brightness = Brightness::new(15);
    pub const BLACK: Brightness = Brightness::new(0);

    /// Create a brightness from a value between 0..16
    ///
    /// # Panics
    /// if v >= 16
    pub const fn new(v: u8) -> Self {
        assert!(v < 16);
        Self(v)
    }
}
