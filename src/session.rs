//! MCP session management (MCP 2025-11-25).
//!
//! Tracks server-side sessions via `MCP-Session-Id` headers.
//! Sessions are created on `initialize` and invalidated on timeout or explicit close.

use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// MCP protocol version header name.
pub const MCP_PROTOCOL_VERSION_HEADER: &str = "MCP-Protocol-Version";

/// MCP session ID header name.
pub const MCP_SESSION_ID_HEADER: &str = "MCP-Session-Id";

/// Supported protocol versions for header validation.
pub const SUPPORTED_PROTOCOL_VERSIONS: &[&str] = &["2024-11-05", "2025-03-26", "2025-11-25"];

/// Session state for a connected MCP client.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct McpSession {
    /// Unique session identifier.
    pub session_id: String,
    /// Negotiated protocol version.
    pub protocol_version: String,
    /// When the session was created.
    #[serde(skip)]
    pub created_at: Option<Instant>,
    /// When the session was last active.
    #[serde(skip)]
    pub last_active: Option<Instant>,
}

/// Session store — manages active MCP sessions.
pub struct SessionStore {
    sessions: RwLock<HashMap<String, McpSession>>,
    /// Session timeout duration. Sessions inactive longer than this are pruned.
    timeout: Duration,
}

impl SessionStore {
    /// Create a new session store with the given timeout.
    #[must_use]
    pub fn new(timeout: Duration) -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            timeout,
        }
    }

    /// Create a new session and return its ID.
    #[must_use]
    pub fn create(&self, protocol_version: String) -> String {
        let session_id = uuid::Uuid::new_v4().to_string();
        let now = Instant::now();
        let session = McpSession {
            session_id: session_id.clone(),
            protocol_version,
            created_at: Some(now),
            last_active: Some(now),
        };
        info!(session_id = %session_id, "MCP session created");
        self.sessions
            .write()
            .expect("session lock poisoned")
            .insert(session_id.clone(), session);
        session_id
    }

    /// Validate and touch a session. Returns the session if valid.
    pub fn validate(&self, session_id: &str) -> Option<McpSession> {
        let mut sessions = self.sessions.write().expect("session lock poisoned");
        if let Some(session) = sessions.get_mut(session_id) {
            session.last_active = Some(Instant::now());
            Some(session.clone())
        } else {
            debug!(session_id = %session_id, "Unknown session ID");
            None
        }
    }

    /// Remove a session.
    pub fn remove(&self, session_id: &str) -> bool {
        let removed = self
            .sessions
            .write()
            .expect("session lock poisoned")
            .remove(session_id)
            .is_some();
        if removed {
            info!(session_id = %session_id, "MCP session removed");
        }
        removed
    }

    /// Prune expired sessions. Returns the number pruned.
    pub fn prune_expired(&self) -> usize {
        let now = Instant::now();
        let timeout = self.timeout;
        let mut sessions = self.sessions.write().expect("session lock poisoned");
        let before = sessions.len();
        sessions.retain(|id, s| {
            let alive = s
                .last_active
                .map(|la| now.duration_since(la) < timeout)
                .unwrap_or(false);
            if !alive {
                warn!(session_id = %id, "Pruning expired MCP session");
            }
            alive
        });
        before - sessions.len()
    }

    /// Number of active sessions.
    #[must_use]
    pub fn active_count(&self) -> usize {
        self.sessions
            .read()
            .expect("session lock poisoned")
            .len()
    }
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::new(Duration::from_secs(3600)) // 1 hour default
    }
}

/// Validate the `MCP-Protocol-Version` header value.
///
/// Returns `Ok(version)` if the version is supported, `Err(msg)` otherwise.
pub fn validate_protocol_version(version: &str) -> Result<&str, String> {
    if SUPPORTED_PROTOCOL_VERSIONS.contains(&version) {
        Ok(version)
    } else {
        Err(format!(
            "unsupported MCP protocol version: {version}. Supported: {}",
            SUPPORTED_PROTOCOL_VERSIONS.join(", ")
        ))
    }
}

// ---------------------------------------------------------------------------
// Origin validation (MCP 2025-11-25)
// ---------------------------------------------------------------------------

/// Validate an HTTP `Origin` header for DNS rebinding protection.
///
/// Returns `Ok(())` if the origin is allowed, `Err(reason)` if it should
/// be rejected with HTTP 403.
///
/// `allowed_origins` — list of allowed origin strings (e.g. `["http://localhost:8090"]`).
/// An empty list means **reject all origins** (strict mode).
/// `"*"` in the list means allow any origin (development only).
pub fn validate_origin(origin: &str, allowed_origins: &[String]) -> Result<(), String> {
    if origin.is_empty() {
        // No Origin header — allow (same-origin requests may omit it)
        return Ok(());
    }

    if allowed_origins.iter().any(|o| o == "*") {
        return Ok(());
    }

    if allowed_origins.is_empty() {
        return Err("no origins allowed (strict mode)".into());
    }

    if allowed_origins.iter().any(|o| o == origin) {
        Ok(())
    } else {
        Err(format!("origin not allowed: {origin}"))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_validate_session() {
        let store = SessionStore::default();
        let id = store.create("2025-11-25".into());
        assert_eq!(store.active_count(), 1);
        let session = store.validate(&id);
        assert!(session.is_some());
        assert_eq!(session.unwrap().protocol_version, "2025-11-25");
    }

    #[test]
    fn validate_unknown_session_returns_none() {
        let store = SessionStore::default();
        assert!(store.validate("nonexistent").is_none());
    }

    #[test]
    fn remove_session() {
        let store = SessionStore::default();
        let id = store.create("2025-11-25".into());
        assert!(store.remove(&id));
        assert_eq!(store.active_count(), 0);
        assert!(!store.remove(&id)); // already gone
    }

    #[test]
    fn prune_expired() {
        let store = SessionStore::new(Duration::from_millis(1));
        store.create("2025-11-25".into());
        std::thread::sleep(Duration::from_millis(10));
        let pruned = store.prune_expired();
        assert_eq!(pruned, 1);
        assert_eq!(store.active_count(), 0);
    }

    #[test]
    fn validate_protocol_version_supported() {
        assert!(validate_protocol_version("2025-11-25").is_ok());
        assert!(validate_protocol_version("2025-03-26").is_ok());
        assert!(validate_protocol_version("2024-11-05").is_ok());
    }

    #[test]
    fn validate_protocol_version_unsupported() {
        assert!(validate_protocol_version("1999-01-01").is_err());
        assert!(validate_protocol_version("").is_err());
    }

    // -- Origin validation --

    #[test]
    fn origin_empty_allowed() {
        assert!(validate_origin("", &[]).is_ok());
    }

    #[test]
    fn origin_wildcard_allows_any() {
        assert!(validate_origin("http://evil.com", &["*".into()]).is_ok());
    }

    #[test]
    fn origin_strict_rejects_all() {
        assert!(validate_origin("http://localhost", &[]).is_err());
    }

    #[test]
    fn origin_matched() {
        let allowed = vec!["http://localhost:8090".into()];
        assert!(validate_origin("http://localhost:8090", &allowed).is_ok());
    }

    #[test]
    fn origin_not_matched() {
        let allowed = vec!["http://localhost:8090".into()];
        assert!(validate_origin("http://evil.com", &allowed).is_err());
    }

    #[test]
    fn origin_multiple_allowed() {
        let allowed = vec![
            "http://localhost:8090".into(),
            "http://localhost:3000".into(),
        ];
        assert!(validate_origin("http://localhost:3000", &allowed).is_ok());
        assert!(validate_origin("http://other.com", &allowed).is_err());
    }
}
