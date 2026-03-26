//! Audit logging — record every tool call for compliance and debugging.
//!
//! The [`AuditSink`] trait defines the interface. Enable the `audit` feature
//! for the [`LibroAudit`] implementation backed by libro's hash-linked chain.

use serde::Serialize;

/// A tool call event to be logged.
#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct ToolCallEvent {
    pub tool_name: String,
    pub duration_ms: u64,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caller_id: Option<String>,
}

impl ToolCallEvent {
    #[must_use]
    pub fn new(
        tool_name: impl Into<String>,
        duration_ms: u64,
        success: bool,
        error: Option<String>,
        caller_id: Option<String>,
    ) -> Self {
        Self {
            tool_name: tool_name.into(),
            duration_ms,
            success,
            error,
            caller_id,
        }
    }
}

/// Trait for audit logging backends.
pub trait AuditSink: Send + Sync {
    /// Log a tool call event.
    fn log(&self, event: &ToolCallEvent);
}

/// No-op audit sink (used when auditing is disabled).
impl AuditSink for () {
    fn log(&self, _event: &ToolCallEvent) {}
}

// --- libro integration (feature = "audit") ---

#[cfg(feature = "audit")]
mod libro_impl {
    use super::*;
    use libro::chain::AuditChain;
    use libro::entry::EventSeverity;
    use std::sync::Mutex;

    /// Audit sink backed by libro's hash-linked audit chain.
    pub struct LibroAudit {
        chain: Mutex<AuditChain>,
    }

    impl LibroAudit {
        #[must_use]
        pub fn new() -> Self {
            Self {
                chain: Mutex::new(AuditChain::new()),
            }
        }

        /// Create from an existing audit chain.
        #[must_use]
        pub fn with_chain(chain: AuditChain) -> Self {
            Self {
                chain: Mutex::new(chain),
            }
        }

        /// Access the underlying chain (e.g. for verification or export).
        #[must_use = "access the underlying audit chain"]
        pub fn chain(&self) -> std::sync::MutexGuard<'_, AuditChain> {
            self.chain.lock().unwrap_or_else(|e| e.into_inner())
        }
    }

    impl Default for LibroAudit {
        fn default() -> Self {
            Self::new()
        }
    }

    impl AuditSink for LibroAudit {
        fn log(&self, event: &ToolCallEvent) {
            let severity = if event.success {
                EventSeverity::Info
            } else {
                EventSeverity::Error
            };

            let action = if event.success {
                "tool.completed"
            } else {
                "tool.failed"
            };

            let details = serde_json::json!({
                "tool_name": event.tool_name,
                "duration_ms": event.duration_ms,
                "success": event.success,
                "error": event.error,
                "caller_id": event.caller_id,
            });

            let mut chain = self.chain.lock().unwrap_or_else(|e| e.into_inner());
            chain.append(severity, "bote", action, details);
        }
    }
}

#[cfg(feature = "audit")]
pub use libro_impl::LibroAudit;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_call_event_serializes() {
        let event = ToolCallEvent {
            tool_name: "echo".into(),
            duration_ms: 42,
            success: true,
            error: None,
            caller_id: Some("agent-1".into()),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"echo\""));
        assert!(json.contains("42"));
        assert!(!json.contains("\"error\""));
    }

    #[test]
    fn tool_call_event_with_error_serializes() {
        let event = ToolCallEvent {
            tool_name: "broken".into(),
            duration_ms: 5,
            success: false,
            error: Some("handler crashed".into()),
            caller_id: None,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"error\""));
        assert!(json.contains("handler crashed"));
        assert!(!json.contains("\"caller_id\""));
    }

    #[test]
    fn noop_sink_compiles() {
        let sink: &dyn AuditSink = &();
        sink.log(&ToolCallEvent {
            tool_name: "test".into(),
            duration_ms: 0,
            success: true,
            error: None,
            caller_id: None,
        });
    }
}

#[cfg(all(test, feature = "audit"))]
mod audit_tests {
    use super::*;

    #[test]
    fn libro_audit_logs_success() {
        let audit = LibroAudit::new();
        audit.log(&ToolCallEvent {
            tool_name: "echo".into(),
            duration_ms: 10,
            success: true,
            error: None,
            caller_id: Some("agent-1".into()),
        });

        let chain = audit.chain();
        assert_eq!(chain.len(), 1);
        let entry = &chain.entries()[0];
        assert_eq!(entry.source(), "bote");
        assert_eq!(entry.action(), "tool.completed");
        assert_eq!(entry.severity(), libro::entry::EventSeverity::Info);
        assert_eq!(entry.details()["tool_name"], "echo");
        assert_eq!(entry.details()["duration_ms"], 10);
    }

    #[test]
    fn libro_audit_logs_failure() {
        let audit = LibroAudit::new();
        audit.log(&ToolCallEvent {
            tool_name: "broken".into(),
            duration_ms: 5,
            success: false,
            error: Some("handler crashed".into()),
            caller_id: None,
        });

        let chain = audit.chain();
        assert_eq!(chain.len(), 1);
        let entry = &chain.entries()[0];
        assert_eq!(entry.action(), "tool.failed");
        assert_eq!(entry.severity(), libro::entry::EventSeverity::Error);
        assert_eq!(entry.details()["error"], "handler crashed");
    }

    #[test]
    fn libro_audit_chain_links() {
        let audit = LibroAudit::new();
        for i in 0..3 {
            audit.log(&ToolCallEvent {
                tool_name: format!("tool_{i}"),
                duration_ms: i as u64,
                success: true,
                error: None,
                caller_id: None,
            });
        }

        let chain = audit.chain();
        assert_eq!(chain.len(), 3);
        // Verify chain integrity.
        assert!(chain.verify().is_ok());
    }
}
