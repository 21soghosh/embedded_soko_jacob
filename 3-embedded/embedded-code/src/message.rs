use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum Message {
    Move { dx: i8, dy: i8},
    MoveTo { x: u8, y: u8 },
}

