use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Message {
    pub dx: i8,
    pub dy: i8,
    pub steps: u8,
}

impl Message {
    pub fn new() -> Self {
        Self {
            dx: 0,
            dy: 0,
            steps: 0,
        }
    }
}


