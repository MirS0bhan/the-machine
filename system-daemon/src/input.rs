//! Input forwarding: delivers trusted input events to the compositor/agents.
//!
//! Placeholder stub; real implementation reads evdev and attaches
//! [`common::ProvenanceMarker`]s before forwarding.

pub struct InputForwarder;

impl InputForwarder {
    pub fn new() -> Self {
        Self
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        // Placeholder: event loop that forwards input with provenance.
        Ok(())
    }
}
