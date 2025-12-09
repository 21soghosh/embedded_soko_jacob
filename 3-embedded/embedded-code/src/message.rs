use crate::serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Message {
    pub dx: i8,
    pub dy: i8,
    pub steps: u8,
}
