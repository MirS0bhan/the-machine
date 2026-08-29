use serde::{Deserialize, Serialize};

/// Display mode information
#[derive(Debug, Serialize, Deserialize)]
pub struct DisplayMode {
    pub width: u32,
    pub height: u32,
    pub refresh: f32,
    pub current: bool,
}

/// Network interface information
#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub name: String,
    pub r#type: String,
    pub state: String,
}

/// Audio device information
#[derive(Debug, Serialize, Deserialize)]
pub struct AudioDevice {
    pub name: String,
    pub r#type: String,
    pub default: bool,
}
