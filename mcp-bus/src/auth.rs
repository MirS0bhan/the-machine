//! Authorization for runtime `_bus.register` / `_bus.deregister`.
//!
//! The JSON `registered_by` field is caller-supplied. We never treat a
//! wire claim of `boot` as trusted — boot routes are installed in-process
//! at startup. Runtime callers may only add `mcp-intent` / `event-handler`
//! routes for themselves.

use crate::registry::Namespace;

/// Callers allowed to mutate the dynamic registry over the socket.
const RUNTIME_REGISTRANTS: &[&str] = &[
    "lambda-server",
    "event-bus",
    "policy-broker",
    "marketplace",
    "agent-core",
    "local-model-daemon",
];

pub fn authorize_register(
    registered_by: &str,
    namespace: Namespace,
    handler: &str,
) -> Result<(), &'static str> {
    if registered_by == "boot" {
        return Err("boot registration is reserved for in-process startup");
    }
    if !RUNTIME_REGISTRANTS.contains(&registered_by) {
        return Err("registration not allowed");
    }
    if !matches!(namespace, Namespace::McpIntent | Namespace::EventHandler) {
        return Err("runtime callers may only register mcp-intent or event-handler");
    }
    if handler != registered_by {
        return Err("cannot register routes for another handler");
    }
    Ok(())
}

pub fn authorize_deregister(registered_by: &str) -> Result<(), &'static str> {
    if registered_by == "boot" {
        return Err("boot deregistration is reserved for in-process startup");
    }
    if !RUNTIME_REGISTRANTS.contains(&registered_by) {
        return Err("deregistration not allowed");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_spoofed_boot() {
        let err = authorize_register("boot", Namespace::McpIntent, "lambda-server").unwrap_err();
        assert!(err.contains("boot"));
    }

    #[test]
    fn rejects_unknown_caller() {
        assert!(authorize_register("evil", Namespace::McpIntent, "evil").is_err());
    }

    #[test]
    fn rejects_system_op_from_runtime() {
        assert!(authorize_register("lambda-server", Namespace::SystemOp, "lambda-server").is_err());
    }

    #[test]
    fn rejects_handler_spoof() {
        assert!(
            authorize_register("lambda-server", Namespace::McpIntent, "policy-broker").is_err()
        );
    }

    #[test]
    fn allows_lambda_intent_for_self() {
        authorize_register("lambda-server", Namespace::McpIntent, "lambda-server").unwrap();
        authorize_register("event-bus", Namespace::EventHandler, "event-bus").unwrap();
    }

    #[test]
    fn deregister_rejects_boot() {
        assert!(authorize_deregister("boot").is_err());
        authorize_deregister("lambda-server").unwrap();
    }
}
