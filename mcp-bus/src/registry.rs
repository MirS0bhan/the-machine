//! Method registry mapping MCP method names to handler component ids.
//!
//! Supports exact keys and wildcard patterns (`calc.*`) across namespaces:
//! `mcp-intent`, `event-handler`, `system-op`, `state-op`.

use std::collections::HashMap;

/// Registry namespace per mcp-bus-spec.md §2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Namespace {
    McpIntent,
    EventHandler,
    SystemOp,
    StateOp,
}

impl Namespace {
    pub fn as_str(self) -> &'static str {
        match self {
            Namespace::McpIntent => "mcp-intent",
            Namespace::EventHandler => "event-handler",
            Namespace::SystemOp => "system-op",
            Namespace::StateOp => "state-op",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "mcp-intent" => Some(Namespace::McpIntent),
            "event-handler" => Some(Namespace::EventHandler),
            "system-op" => Some(Namespace::SystemOp),
            "state-op" => Some(Namespace::StateOp),
            _ => None,
        }
    }
}

/// A single routing entry in the registry.
#[derive(Debug, Clone)]
pub struct RouteEntry {
    pub namespace: Namespace,
    /// Exact key or wildcard pattern (e.g. `calc.*`).
    pub pattern: String,
    /// Component id to forward to (e.g. `lambda-server`).
    pub handler: String,
    pub registered_by: String,
    /// Optional manifest reference (lambda name, event handler id, …).
    pub manifest_ref: Option<String>,
    pub trusted: bool,
}

/// Resolved route for a concrete method call.
#[derive(Debug, Clone)]
pub struct ResolvedRoute {
    pub namespace: Namespace,
    pub handler: String,
    pub pattern: String,
    pub manifest_ref: Option<String>,
}

/// Maps method names/patterns to the component responsible for handling them.
pub struct Registry {
    /// Exact routes: full method name → entry.
    exact: HashMap<String, RouteEntry>,
    /// Wildcard routes per namespace, checked after exact lookup.
    wildcards: Vec<RouteEntry>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            exact: HashMap::new(),
            wildcards: Vec::new(),
        }
    }

    /// Register `method` as handled by `handler` (trusted flag reserved for future use).
    pub fn register(&mut self, method: &str, handler: &str, trusted: bool) -> anyhow::Result<()> {
        self.register_route(RouteEntry {
            namespace: infer_namespace(method),
            pattern: method.to_string(),
            handler: handler.to_string(),
            registered_by: "boot".into(),
            manifest_ref: None,
            trusted,
        })
    }

    /// Register a route with full metadata (internal `_bus.register` path).
    pub fn register_route(&mut self, entry: RouteEntry) -> anyhow::Result<()> {
        if entry.pattern.contains('*') {
            // Reject duplicate wildcard in same namespace.
            if self
                .wildcards
                .iter()
                .any(|e| e.namespace == entry.namespace && e.pattern == entry.pattern)
            {
                anyhow::bail!(
                    "route collision: {} in {:?}",
                    entry.pattern,
                    entry.namespace
                );
            }
            self.wildcards.push(entry);
        } else {
            if self.exact.contains_key(&entry.pattern) {
                anyhow::bail!("route collision: {}", entry.pattern);
            }
            self.exact.insert(entry.pattern.clone(), entry);
        }
        Ok(())
    }

    /// Resolve a method name to its handler component id (legacy API).
    pub fn resolve(&self, method: &str) -> Option<String> {
        self.resolve_full(method).map(|r| r.handler)
    }

    /// Full resolution including namespace and manifest reference.
    pub fn resolve_full(&self, method: &str) -> Option<ResolvedRoute> {
        if let Some(e) = self.exact.get(method) {
            return Some(ResolvedRoute {
                namespace: e.namespace,
                handler: e.handler.clone(),
                pattern: e.pattern.clone(),
                manifest_ref: e.manifest_ref.clone(),
            });
        }
        let ns = infer_namespace(method);
        // Wildcards: longest-prefix match wins.
        let mut best: Option<&RouteEntry> = None;
        for e in &self.wildcards {
            if e.namespace != ns && e.namespace != Namespace::McpIntent {
                continue;
            }
            if pattern_matches(&e.pattern, method) {
                match best {
                    None => best = Some(e),
                    Some(prev) => {
                        if e.pattern.len() > prev.pattern.len() {
                            best = Some(e);
                        }
                    }
                }
            }
        }
        best.map(|e| ResolvedRoute {
            namespace: e.namespace,
            handler: e.handler.clone(),
            pattern: e.pattern.clone(),
            manifest_ref: e.manifest_ref.clone(),
        })
    }

    /// Enumerate all routes, optionally filtered by namespace.
    pub fn list_routes(&self, namespace: Option<Namespace>) -> Vec<RouteEntry> {
        let mut out: Vec<RouteEntry> = self.exact.values().cloned().collect();
        out.extend(self.wildcards.iter().cloned());
        if let Some(ns) = namespace {
            out.retain(|e| e.namespace == ns);
        }
        out.sort_by(|a, b| a.pattern.cmp(&b.pattern));
        out
    }

    /// Remove a route by namespace + pattern (internal `_bus.deregister`).
    pub fn deregister_route(&mut self, namespace: Namespace, pattern: &str) -> bool {
        if pattern.contains('*') {
            let before = self.wildcards.len();
            self.wildcards
                .retain(|e| !(e.namespace == namespace && e.pattern == pattern));
            self.wildcards.len() < before
        } else {
            self.exact.remove(pattern).is_some()
        }
    }
}

/// Infer namespace from method prefix per mcp-bus-spec.md §2.
pub fn infer_namespace(method: &str) -> Namespace {
    if method.starts_with("state.") || method.starts_with("state-op:") {
        Namespace::StateOp
    } else if method.starts_with("system-op:")
        || method.starts_with("systemd.")
        || method.starts_with("power.")
        || method.starts_with("display.")
        || method.starts_with("net.")
        || method.starts_with("kernel.")
    {
        Namespace::SystemOp
    } else if method.starts_with("event-handler:") {
        Namespace::EventHandler
    } else {
        Namespace::McpIntent
    }
}

/// Pattern match: supports literal, `*` (any), prefix `x.*` and suffix `*.x`.
pub fn pattern_matches(pattern: &str, candidate: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(p) = pattern.strip_suffix(".*") {
        return candidate == p || candidate.starts_with(&format!("{}.", p));
    }
    if let Some(s) = pattern.strip_prefix("*.") {
        return candidate == s || candidate.ends_with(&format!(".{}", s));
    }
    let pseg: Vec<&str> = pattern.split('.').collect();
    let cseg: Vec<&str> = candidate.split('.').collect();
    if pseg.len() != cseg.len() {
        return false;
    }
    pseg.iter()
        .zip(cseg.iter())
        .all(|(p, c)| *p == "*" || p == c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_resolve() {
        let mut r = Registry::new();
        r.register("state.get", "state-store", true).unwrap();
        assert_eq!(r.resolve("state.get").as_deref(), Some("state-store"));
    }

    #[test]
    fn wildcard_resolve() {
        let mut r = Registry::new();
        r.register_route(RouteEntry {
            namespace: Namespace::McpIntent,
            pattern: "calc.*".into(),
            handler: "lambda-server".into(),
            registered_by: "lambda-server".into(),
            manifest_ref: Some("calc.eval".into()),
            trusted: true,
        })
        .unwrap();
        let resolved = r.resolve_full("calc.add").unwrap();
        assert_eq!(resolved.handler, "lambda-server");
        assert_eq!(resolved.manifest_ref.as_deref(), Some("calc.eval"));
    }

    #[test]
    fn pattern_matches_prefix() {
        assert!(pattern_matches("calc.*", "calc.add"));
        assert!(pattern_matches("calc.*", "calc"));
        assert!(!pattern_matches("calc.*", "other.add"));
    }

    #[test]
    fn deregister_exact_and_wildcard() {
        let mut r = Registry::new();
        r.register("calc.add", "lambda-server", true).unwrap();
        r.register_route(RouteEntry {
            namespace: Namespace::McpIntent,
            pattern: "math.*".into(),
            handler: "lambda-server".into(),
            registered_by: "lambda-server".into(),
            manifest_ref: None,
            trusted: true,
        })
        .unwrap();
        assert!(r.deregister_route(Namespace::McpIntent, "calc.add"));
        assert!(r.resolve("calc.add").is_none());
        assert!(r.deregister_route(Namespace::McpIntent, "math.*"));
        assert!(r.resolve("math.add").is_none());
        assert!(!r.deregister_route(Namespace::McpIntent, "missing"));
    }
}
