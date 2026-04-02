//! Built-in MCP tools for querying, verifying, and exporting libro audit chains.
//!
//! Requires the `audit` feature. These tools operate on the audit chain
//! attached to the dispatcher via [`LibroAudit`](crate::audit::LibroAudit).
//!
//! ## Tools
//!
//! | Tool | Description | Annotations |
//! |------|-------------|-------------|
//! | `libro_query` | Query audit entries by source, severity, action, agent, or time range | read-only |
//! | `libro_verify` | Verify the audit chain's cryptographic integrity | read-only |
//! | `libro_export` | Export the audit chain as JSON Lines or CSV | read-only |

use std::collections::HashMap;
use std::sync::Arc;

use crate::audit::LibroAudit;
use crate::dispatch::ToolHandler;
use crate::registry::{ToolAnnotations, ToolDef, ToolRegistry, ToolSchema};

/// Register the libro audit tools on a registry and return their handlers.
///
/// The caller should attach these handlers to the dispatcher:
/// ```rust,ignore
/// let audit = Arc::new(LibroAudit::new());
/// let handlers = libro_tools::register(&mut registry, Arc::clone(&audit));
/// for (name, handler) in handlers {
///     dispatcher.handle(name, handler);
/// }
/// ```
pub fn register(registry: &mut ToolRegistry, audit: Arc<LibroAudit>) -> Vec<(String, ToolHandler)> {
    let mut handlers = Vec::new();

    // --- libro_query ---
    registry.register(ToolDef {
        name: "libro_query".into(),
        description: "Query audit chain entries by source, severity, action, agent, or time range"
            .into(),
        input_schema: ToolSchema {
            schema_type: "object".into(),
            properties: HashMap::from([
                (
                    "source".into(),
                    serde_json::json!({"type": "string", "description": "Filter by event source"}),
                ),
                (
                    "severity".into(),
                    serde_json::json!({"type": "string", "description": "Filter by exact severity (Debug, Info, Warning, Error, Critical, Security)"}),
                ),
                (
                    "action".into(),
                    serde_json::json!({"type": "string", "description": "Filter by action"}),
                ),
                (
                    "agent_id".into(),
                    serde_json::json!({"type": "string", "description": "Filter by agent ID"}),
                ),
                (
                    "min_severity".into(),
                    serde_json::json!({"type": "string", "description": "Filter by minimum severity (inclusive)"}),
                ),
                (
                    "limit".into(),
                    serde_json::json!({"type": "integer", "description": "Maximum entries to return (default: 100)"}),
                ),
            ]),
            required: vec![],
        },
        version: Some("0.90.0".into()),
        deprecated: None,
        annotations: Some(ToolAnnotations::read_only()),
    });

    let audit_q = Arc::clone(&audit);
    handlers.push((
        "libro_query".into(),
        Arc::new(move |params: serde_json::Value| {
            let chain = audit_q.chain();
            let mut filter = libro::QueryFilter::new();

            if let Some(s) = params.get("source").and_then(|v| v.as_str()) {
                filter = filter.source(s);
            }
            if let Some(s) = params.get("action").and_then(|v| v.as_str()) {
                filter = filter.action(s);
            }
            if let Some(s) = params.get("agent_id").and_then(|v| v.as_str()) {
                filter = filter.agent_id(s);
            }
            if let Some(s) = params.get("severity").and_then(|v| v.as_str())
                && let Some(sev) = parse_severity(s)
            {
                filter = filter.severity(sev);
            }
            if let Some(s) = params.get("min_severity").and_then(|v| v.as_str())
                && let Some(sev) = parse_severity(s)
            {
                filter = filter.min_severity(sev);
            }

            let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(100) as usize;

            let results: Vec<&libro::AuditEntry> =
                chain.query(&filter).into_iter().take(limit).collect();
            let json = serde_json::to_string_pretty(&results).unwrap_or_default();

            serde_json::json!({
                "content": [{
                    "type": "text",
                    "text": format!("{} entries matched\n{json}", results.len())
                }]
            })
        }) as ToolHandler,
    ));

    // --- libro_verify ---
    registry.register(ToolDef {
        name: "libro_verify".into(),
        description: "Verify the audit chain's cryptographic integrity — checks hash linking and entry self-hashes".into(),
        input_schema: ToolSchema {
            schema_type: "object".into(),
            properties: HashMap::new(),
            required: vec![],
        },
        version: Some("0.90.0".into()),
        deprecated: None,
        annotations: Some(ToolAnnotations::read_only()),
    });

    let audit_v = Arc::clone(&audit);
    handlers.push((
        "libro_verify".into(),
        Arc::new(move |_params: serde_json::Value| {
            let chain = audit_v.chain();
            let review = chain.review();
            let text = format!("{review}");

            serde_json::json!({
                "content": [{
                    "type": "text",
                    "text": text
                }]
            })
        }) as ToolHandler,
    ));

    // --- libro_export ---
    registry.register(ToolDef {
        name: "libro_export".into(),
        description: "Export the audit chain as JSON Lines or CSV".into(),
        input_schema: ToolSchema {
            schema_type: "object".into(),
            properties: HashMap::from([(
                "format".into(),
                serde_json::json!({"type": "string", "enum": ["jsonl", "csv"], "description": "Export format (default: jsonl)"}),
            )]),
            required: vec![],
        },
        version: Some("0.90.0".into()),
        deprecated: None,
        annotations: Some(ToolAnnotations::read_only()),
    });

    let audit_e = Arc::clone(&audit);
    handlers.push((
        "libro_export".into(),
        Arc::new(move |params: serde_json::Value| {
            let chain = audit_e.chain();
            let format = params
                .get("format")
                .and_then(|v| v.as_str())
                .unwrap_or("jsonl");

            let mut buf = Vec::new();
            let result = match format {
                "csv" => libro::to_csv(chain.entries(), &mut buf),
                _ => libro::to_jsonl(chain.entries(), &mut buf),
            };

            match result {
                Ok(()) => {
                    let text = String::from_utf8_lossy(&buf).into_owned();
                    serde_json::json!({
                        "content": [{
                            "type": "text",
                            "text": text
                        }]
                    })
                }
                Err(e) => serde_json::json!({
                    "content": [{
                        "type": "text",
                        "text": format!("export failed: {e}")
                    }],
                    "isError": true
                }),
            }
        }) as ToolHandler,
    ));

    handlers
}

fn parse_severity(s: &str) -> Option<libro::EventSeverity> {
    match s {
        "Debug" => Some(libro::EventSeverity::Debug),
        "Info" => Some(libro::EventSeverity::Info),
        "Warning" => Some(libro::EventSeverity::Warning),
        "Error" => Some(libro::EventSeverity::Error),
        "Critical" => Some(libro::EventSeverity::Critical),
        "Security" => Some(libro::EventSeverity::Security),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::AuditSink;

    fn make_audit_with_entries(n: usize) -> Arc<LibroAudit> {
        let audit = Arc::new(LibroAudit::new());
        for i in 0..n {
            let event = crate::audit::ToolCallEvent::new(
                format!("tool_{i}"),
                i as u64 * 10,
                i % 3 != 0, // every 3rd call fails
                if i % 3 == 0 {
                    Some("simulated failure".into())
                } else {
                    None
                },
                Some(format!("agent-{}", i % 2)),
            );
            audit.log(&event);
        }
        audit
    }

    #[test]
    fn libro_query_no_filter() {
        let audit = make_audit_with_entries(5);
        let mut registry = ToolRegistry::new();
        let handlers = register(&mut registry, Arc::clone(&audit));
        let handler = &handlers[0].1;

        let result = handler(serde_json::json!({}));
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.starts_with("5 entries matched"));
    }

    #[test]
    fn libro_query_with_source_filter() {
        let audit = make_audit_with_entries(5);
        let mut registry = ToolRegistry::new();
        let handlers = register(&mut registry, Arc::clone(&audit));
        let handler = &handlers[0].1;

        let result = handler(serde_json::json!({"source": "bote"}));
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("5 entries matched")); // all from "bote"
    }

    #[test]
    fn libro_query_with_severity_filter() {
        let audit = make_audit_with_entries(6);
        let mut registry = ToolRegistry::new();
        let handlers = register(&mut registry, Arc::clone(&audit));
        let handler = &handlers[0].1;

        let result = handler(serde_json::json!({"severity": "Error"}));
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("2 entries matched")); // i=0,3 fail
    }

    #[test]
    fn libro_query_with_limit() {
        let audit = make_audit_with_entries(10);
        let mut registry = ToolRegistry::new();
        let handlers = register(&mut registry, Arc::clone(&audit));
        let handler = &handlers[0].1;

        let result = handler(serde_json::json!({"limit": 3}));
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.starts_with("3 entries matched"));
    }

    #[test]
    fn libro_verify_returns_review() {
        let audit = make_audit_with_entries(3);
        let mut registry = ToolRegistry::new();
        let handlers = register(&mut registry, Arc::clone(&audit));
        let handler = &handlers[1].1;

        let result = handler(serde_json::json!({}));
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("VALID"));
        assert!(text.contains("Entries:"));
    }

    #[test]
    fn libro_export_jsonl() {
        let audit = make_audit_with_entries(3);
        let mut registry = ToolRegistry::new();
        let handlers = register(&mut registry, Arc::clone(&audit));
        let handler = &handlers[2].1;

        let result = handler(serde_json::json!({}));
        let text = result["content"][0]["text"].as_str().unwrap();
        let lines: Vec<&str> = text.trim().lines().collect();
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn libro_export_csv() {
        let audit = make_audit_with_entries(3);
        let mut registry = ToolRegistry::new();
        let handlers = register(&mut registry, Arc::clone(&audit));
        let handler = &handlers[2].1;

        let result = handler(serde_json::json!({"format": "csv"}));
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.starts_with("id,timestamp,severity,"));
        let lines: Vec<&str> = text.trim().lines().collect();
        assert_eq!(lines.len(), 4); // header + 3 entries
    }

    #[test]
    fn tools_registered_with_annotations() {
        let audit = make_audit_with_entries(0);
        let mut registry = ToolRegistry::new();
        let _ = register(&mut registry, audit);

        // All 3 tools should be registered
        assert!(registry.get("libro_query").is_some());
        assert!(registry.get("libro_verify").is_some());
        assert!(registry.get("libro_export").is_some());

        // All should be read-only
        let def = registry.get("libro_query").unwrap();
        assert_eq!(def.annotations.as_ref().unwrap().read_only_hint, Some(true));
    }
}
