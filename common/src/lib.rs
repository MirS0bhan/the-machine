pub mod error;
pub mod mcp;
pub mod provenance;
pub mod token;
pub mod types;

pub use error::*;
pub use mcp::*;
pub use provenance::*;
pub use token::*;
pub use types::*;

pub use uuid::Uuid;
pub use serde::{Deserialize, Serialize};

/// Current Unix timestamp in seconds.
pub fn current_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
