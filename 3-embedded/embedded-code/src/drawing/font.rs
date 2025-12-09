pub type Character = [[bool; 8]; 16];

// TODO: this is one way to get a simple font in your program.
//       finish this if you want to use it properly. Also add
//       functions to screen.rs to draw a `Character`
pub const ZERO: Character = [
    [false, true, true, true, true, true, true, false],
    [true, true, true, true, true, true, true, true],
    [true, true, true, false, false, true, true, true],
    [true, true, false, false, false, false, true, true],
    [true, true, false, false, false, false, true, true],
    [true, true, false, false, false, false, true, true],
    [true, true, false, false, false, false, true, true],
    [true, true, false, false, false, false, true, true],
    [true, true, false, false, false, false, true, true],
    [true, true, false, false, false, false, true, true],
    [true, true, false, false, false, false, true, true],
    [true, true, false, false, false, false, true, true],
    [true, true, false, false, false, false, true, true],
    [true, true, true, false, false, true, true, true],
    [true, true, true, true, true, true, true, true],
    [false, true, true, true, true, true, true, false],
];
pub const ONE: Character = [
    [false, false, false, false, false, false, false, false],
    [false, true, true, true, true, true, true, false],
    [false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false],
];

pub const NUMBERS: [Character; 2] = [
    ZERO,
    // TODO:
    ONE,
    // TWO,
    // THREE,
    // FOUR,
    // FIVE,
    // SIX,
    // SEVEN,
    // EIGHT,
    // NINE,
];
