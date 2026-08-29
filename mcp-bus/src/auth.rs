//! Authorization for internal `_bus.register` / `_bus.deregister`.
//!
//! Client-supplied `registered_by` is not trusted for privilege. Runtime
//! callers may only register `mcp-intent` / `event-handler` routes for
//! *themselves* (handler must equal registrar). `boot` is in-process only.

use crate::registry::Namespace;

/// Identities allowed to call `_bus.register` / `_bus.deregister` over the socket.
pub const SOCKET_REGISTRARS: &[&str] = &["lambda-server", "event-bus", "policy-broker"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterAuthError {
    Forbidden,
    InvalidNamespace,
    HandlerMismatch,
}

/// Whether `registered_by` may use the internal registration MCP methods.
pub fn authorize_registrar(registered_by: &str) -> bool {
    SOCKET_REGISTRARS.contains(&registered_by)
}

/// Authorize a runtime `_bus.register` request.
///
/// `boot` is rejected on the socket path — boot routes are seeded in-process.
pub fn authorize_register(
    registered_by: &str,
    handler: &str,
    namespace: Namespace,
) -> Result<(), RegisterAuthError> {
    if !authorize_registrar(registered_by) {
        return Err(RegisterAuthError::Forbidden);
    }
    if !matches!(namespace, Namespace::McpIntent | Namespace::EventHandler) {
        return Err(RegisterAuthError::InvalidNamespace);
    }
    if handler != registered_by {
        return Err(RegisterAuthError::HandlerMismatch);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lambda_server_may_register_own_intent() {
        assert_eq!(
            authorize_register("lambda-server", "lambda-server", Namespace::McpIntent),
            Ok(())
        );
    }

    #[test]
    fn event_bus_may_register_event_handler() {
        assert_eq!(
            authorize_register("event-bus", "event-bus", Namespace::EventHandler),
            Ok(())
        );
    }

    #[test]
    fn spoofed_boot_is_rejected() {
        assert_eq!(
            authorize_register("boot", "lambda-server", Namespace::McpIntent),
            Err(RegisterAuthError::Forbidden)
        );
    }

    #[test]
    fn unknown_registrar_is_rejected() {
        assert_eq!(
            authorize_register("agent-core", "agent-core", Namespace::McpIntent),
            Err(RegisterAuthError::Forbidden)
        );
    }

    #[test]
    fn cannot_claim_another_handler() {
        assert_eq!(
            authorize_register("lambda-server", "agent-core", Namespace::McpIntent),
            Err(RegisterAuthError::HandlerMismatch)
        );
    }

    #[test]
    fn cannot_register_fixed_namespaces() {
        assert_eq!(
            authorize_register("lambda-server", "lambda-server", Namespace::SystemOp),
            Err(RegisterAuthError::InvalidNamespace)
        );
        assert_eq!(
            authorize_register("lambda-server", "lambda-server", Namespace::StateOp),
            Err(RegisterAuthError::InvalidNamespace)
        );
    }
}
