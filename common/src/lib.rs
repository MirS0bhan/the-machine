pub mod drm_sysfs;
pub mod error;
pub mod mcp;
pub mod paths;
pub mod provenance;
pub mod secrets;
pub mod token;
pub mod types;

pub use drm_sysfs::*;
pub use error::*;
pub use mcp::*;
pub use paths::*;
pub use provenance::*;
pub use secrets::*;
pub use token::*;
pub use types::*;

pub use serde::{Deserialize, Serialize};
pub use uuid::Uuid;

/// Current Unix timestamp in seconds.
pub fn current_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
