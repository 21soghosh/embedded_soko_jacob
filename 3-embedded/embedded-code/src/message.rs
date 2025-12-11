use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    Move { dx: i8, dy: i8 },
    MoveTo { x: u8, y: u8 },
    SetDisplayMode(DisplayMode),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Envelope {
    pub msg: Message,
    pub checksum: u8,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
pub enum DisplayMode {
    Trail,
    Steps,
}

impl Envelope {
    pub fn new(msg: Message) -> Option<Self> {
        checksum_for(&msg).map(|checksum| Self { msg, checksum })
    }
}

pub fn checksum_for(msg: &Message) -> Option<u8> {
    // serialize the message and compute a simple wrapping sum
    let mut buf = [0u8; 32];
    let encoded = postcard::to_slice(msg, &mut buf).ok()?;
    Some(encoded.iter().fold(0u8, |acc, b| acc.wrapping_add(*b)))
}
