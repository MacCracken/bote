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
//! | `libro_proof` | Generate a Merkle inclusion proof for an entry | read-only |
//! | `libro_retention` | Apply a retention policy and report pruned entries | destructive |

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

    register_query(registry, &audit, &mut handlers);
    register_verify(registry, &audit, &mut handlers);
    register_export(registry, &audit, &mut handlers);
    register_proof(registry, &audit, &mut handlers);
    register_retention(registry, &audit, &mut handlers);

    handlers
}

// ---------------------------------------------------------------------------
// libro_query
// ---------------------------------------------------------------------------

fn register_query(
    registry: &mut ToolRegistry,
    audit: &Arc<LibroAudit>,
    handlers: &mut Vec<(String, ToolHandler)>,
) {
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
        version: Some("0.91.0".into()),
        deprecated: None,
        annotations: Some(ToolAnnotations::read_only()),
    });

    let audit_q = Arc::clone(audit);
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
}

// ---------------------------------------------------------------------------
// libro_verify — returns structured ChainReview JSON
// ---------------------------------------------------------------------------

fn register_verify(
    registry: &mut ToolRegistry,
    audit: &Arc<LibroAudit>,
    handlers: &mut Vec<(String, ToolHandler)>,
) {
    registry.register(ToolDef {
        name: "libro_verify".into(),
        description: "Verify the audit chain's cryptographic integrity — returns structured review with integrity status, entry count, time range, and source/severity/agent distributions".into(),
        input_schema: ToolSchema {
            schema_type: "object".into(),
            properties: HashMap::new(),
            required: vec![],
        },
        version: Some("0.91.0".into()),
        deprecated: None,
        annotations: Some(ToolAnnotations::read_only()),
    });

    let audit_v = Arc::clone(audit);
    handlers.push((
        "libro_verify".into(),
        Arc::new(move |_params: serde_json::Value| {
            let chain = audit_v.chain();
            let review = chain.review();

            // Return structured JSON — ChainReview is Serialize.
            let review_json = serde_json::to_value(&review).unwrap_or_default();

            serde_json::json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&review_json).unwrap_or_default()
                }],
                "_meta": {
                    "review": review_json
                }
            })
        }) as ToolHandler,
    ));
}

// ---------------------------------------------------------------------------
// libro_export
// ---------------------------------------------------------------------------

fn register_export(
    registry: &mut ToolRegistry,
    audit: &Arc<LibroAudit>,
    handlers: &mut Vec<(String, ToolHandler)>,
) {
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
        version: Some("0.91.0".into()),
        deprecated: None,
        annotations: Some(ToolAnnotations::read_only()),
    });

    let audit_e = Arc::clone(audit);
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
}

// ---------------------------------------------------------------------------
// libro_proof — Merkle inclusion proof
// ---------------------------------------------------------------------------

fn register_proof(
    registry: &mut ToolRegistry,
    audit: &Arc<LibroAudit>,
    handlers: &mut Vec<(String, ToolHandler)>,
) {
    registry.register(ToolDef {
        name: "libro_proof".into(),
        description: "Generate a Merkle inclusion proof for an audit entry by index — enables O(log N) verification without the full chain".into(),
        input_schema: ToolSchema {
            schema_type: "object".into(),
            properties: HashMap::from([(
                "index".into(),
                serde_json::json!({"type": "integer", "description": "Entry index (0-based)"}),
            )]),
            required: vec!["index".into()],
        },
        version: Some("0.91.0".into()),
        deprecated: None,
        annotations: Some(ToolAnnotations::read_only()),
    });

    let audit_p = Arc::clone(audit);
    handlers.push((
        "libro_proof".into(),
        Arc::new(move |params: serde_json::Value| {
            let chain = audit_p.chain();
            let entries = chain.entries();

            let index = match params.get("index").and_then(|v| v.as_u64()) {
                Some(i) => i as usize,
                None => {
                    return serde_json::json!({
                        "content": [{"type": "text", "text": "missing required 'index' parameter"}],
                        "isError": true
                    });
                }
            };

            if index >= entries.len() {
                return serde_json::json!({
                    "content": [{"type": "text", "text": format!("index {index} out of range (chain has {} entries)", entries.len())}],
                    "isError": true
                });
            }

            let tree = match libro::MerkleTree::build(entries) {
                Some(t) => t,
                None => {
                    return serde_json::json!({
                        "content": [{"type": "text", "text": "chain is empty — no Merkle tree"}],
                        "isError": true
                    });
                }
            };

            match tree.proof(index) {
                Some(proof) => {
                    let verified = libro::merkle::verify_proof(&proof);
                    let proof_json = serde_json::to_value(&proof).unwrap_or_default();
                    serde_json::json!({
                        "content": [{
                            "type": "text",
                            "text": format!(
                                "Merkle proof for entry {index}:\n  Leaf: {}\n  Root: {}\n  Path length: {}\n  Verified: {verified}",
                                proof.leaf_hash, proof.root, proof.path.len()
                            )
                        }],
                        "_meta": {
                            "proof": proof_json,
                            "verified": verified
                        }
                    })
                }
                None => serde_json::json!({
                    "content": [{"type": "text", "text": format!("failed to generate proof for index {index}")}],
                    "isError": true
                }),
            }
        }) as ToolHandler,
    ));
}

// ---------------------------------------------------------------------------
// libro_retention — apply compliance retention policies
// ---------------------------------------------------------------------------

fn register_retention(
    registry: &mut ToolRegistry,
    audit: &Arc<LibroAudit>,
    handlers: &mut Vec<(String, ToolHandler)>,
) {
    registry.register(ToolDef {
        name: "libro_retention".into(),
        description: "Apply a retention policy to the audit chain — archives entries outside the retention window. Supports PCI-DSS (1yr), HIPAA (6yr), SOX (7yr), or custom count/duration policies.".into(),
        input_schema: ToolSchema {
            schema_type: "object".into(),
            properties: HashMap::from([
                (
                    "policy".into(),
                    serde_json::json!({"type": "string", "enum": ["pci_dss", "hipaa", "sox", "keep_count"], "description": "Retention policy preset"}),
                ),
                (
                    "count".into(),
                    serde_json::json!({"type": "integer", "description": "Number of entries to keep (for keep_count policy)"}),
                ),
            ]),
            required: vec!["policy".into()],
        },
        version: Some("0.91.0".into()),
        deprecated: None,
        annotations: None, // destructive — not read-only
    });

    let audit_r = Arc::clone(audit);
    handlers.push((
        "libro_retention".into(),
        Arc::new(move |params: serde_json::Value| {
            let policy_name = params.get("policy").and_then(|v| v.as_str()).unwrap_or("");

            let policy = match policy_name {
                "pci_dss" => libro::RetentionPolicy::pci_dss(),
                "hipaa" => libro::RetentionPolicy::hipaa(),
                "sox" => libro::RetentionPolicy::sox(),
                "keep_count" => {
                    let count = params.get("count").and_then(|v| v.as_u64()).unwrap_or(1000) as usize;
                    libro::RetentionPolicy::KeepCount(count)
                }
                _ => {
                    return serde_json::json!({
                        "content": [{"type": "text", "text": format!("unknown policy: {policy_name}. Use: pci_dss, hipaa, sox, keep_count")}],
                        "isError": true
                    });
                }
            };

            let mut chain = audit_r.chain();
            let before = chain.len();
            let archive = chain.apply_retention(&policy);
            let after = chain.len();
            let archived_count = archive.map(|a| a.entries.len()).unwrap_or(0);

            serde_json::json!({
                "content": [{
                    "type": "text",
                    "text": format!(
                        "Retention policy '{policy_name}' applied:\n  Before: {before} entries\n  After: {after} entries\n  Archived: {archived_count} entries"
                    )
                }],
                "_meta": {
                    "policy": policy_name,
                    "before": before,
                    "after": after,
                    "archived": archived_count
                }
            })
        }) as ToolHandler,
    ));
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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

    // --- libro_query ---

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
        assert!(text.contains("5 entries matched"));
    }

    #[test]
    fn libro_query_with_severity_filter() {
        let audit = make_audit_with_entries(6);
        let mut registry = ToolRegistry::new();
        let handlers = register(&mut registry, Arc::clone(&audit));
        let handler = &handlers[0].1;

        let result = handler(serde_json::json!({"severity": "Error"}));
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("2 entries matched"));
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

    // --- libro_verify ---

    #[test]
    fn libro_verify_returns_structured_review() {
        let audit = make_audit_with_entries(3);
        let mut registry = ToolRegistry::new();
        let handlers = register(&mut registry, Arc::clone(&audit));
        let handler = &handlers[1].1;

        let result = handler(serde_json::json!({}));
        // Text output contains structured JSON
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("\"integrity\""));
        assert!(text.contains("\"entry_count\""));

        // _meta contains structured review
        let meta = &result["_meta"]["review"];
        assert_eq!(meta["entry_count"], 3);
        assert_eq!(meta["integrity"], "Valid");
        assert!(meta["head_hash"].is_string());
    }

    #[test]
    fn libro_verify_empty_chain() {
        let audit = make_audit_with_entries(0);
        let mut registry = ToolRegistry::new();
        let handlers = register(&mut registry, Arc::clone(&audit));
        let handler = &handlers[1].1;

        let result = handler(serde_json::json!({}));
        let meta = &result["_meta"]["review"];
        assert_eq!(meta["entry_count"], 0);
        assert_eq!(meta["integrity"], "Empty");
    }

    // --- libro_export ---

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

    // --- libro_proof ---

    #[test]
    fn libro_proof_generates_valid_proof() {
        let audit = make_audit_with_entries(5);
        let mut registry = ToolRegistry::new();
        let handlers = register(&mut registry, Arc::clone(&audit));
        let handler = &handlers[3].1;

        let result = handler(serde_json::json!({"index": 2}));
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Merkle proof for entry 2"));
        assert!(text.contains("Verified: true"));

        // _meta contains structured proof
        let meta = &result["_meta"];
        assert_eq!(meta["verified"], true);
        assert!(meta["proof"]["leaf_hash"].is_string());
        assert!(meta["proof"]["root"].is_string());
        assert!(meta["proof"]["path"].is_array());
    }

    #[test]
    fn libro_proof_index_out_of_range() {
        let audit = make_audit_with_entries(3);
        let mut registry = ToolRegistry::new();
        let handlers = register(&mut registry, Arc::clone(&audit));
        let handler = &handlers[3].1;

        let result = handler(serde_json::json!({"index": 10}));
        assert_eq!(result["isError"], true);
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("out of range"));
    }

    #[test]
    fn libro_proof_empty_chain() {
        let audit = make_audit_with_entries(0);
        let mut registry = ToolRegistry::new();
        let handlers = register(&mut registry, Arc::clone(&audit));
        let handler = &handlers[3].1;

        let result = handler(serde_json::json!({"index": 0}));
        assert_eq!(result["isError"], true);
    }

    #[test]
    fn libro_proof_missing_index() {
        let audit = make_audit_with_entries(3);
        let mut registry = ToolRegistry::new();
        let handlers = register(&mut registry, Arc::clone(&audit));
        let handler = &handlers[3].1;

        let result = handler(serde_json::json!({}));
        assert_eq!(result["isError"], true);
    }

    // --- libro_retention ---

    #[test]
    fn libro_retention_keep_count() {
        let audit = make_audit_with_entries(10);
        let mut registry = ToolRegistry::new();
        let handlers = register(&mut registry, Arc::clone(&audit));
        let handler = &handlers[4].1;

        let result = handler(serde_json::json!({"policy": "keep_count", "count": 3}));
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Before: 10"));
        assert!(text.contains("After: 3"));
        assert!(text.contains("Archived: 7"));

        let meta = &result["_meta"];
        assert_eq!(meta["before"], 10);
        assert_eq!(meta["after"], 3);
        assert_eq!(meta["archived"], 7);
    }

    #[test]
    fn libro_retention_unknown_policy() {
        let audit = make_audit_with_entries(3);
        let mut registry = ToolRegistry::new();
        let handlers = register(&mut registry, Arc::clone(&audit));
        let handler = &handlers[4].1;

        let result = handler(serde_json::json!({"policy": "nonsense"}));
        assert_eq!(result["isError"], true);
    }

    #[test]
    fn libro_retention_pci_dss_on_fresh_chain() {
        // Fresh chain — all entries are recent, nothing to prune
        let audit = make_audit_with_entries(5);
        let mut registry = ToolRegistry::new();
        let handlers = register(&mut registry, Arc::clone(&audit));
        let handler = &handlers[4].1;

        let result = handler(serde_json::json!({"policy": "pci_dss"}));
        let meta = &result["_meta"];
        assert_eq!(meta["before"], 5);
        assert_eq!(meta["after"], 5); // nothing pruned, all recent
        assert_eq!(meta["archived"], 0);
    }

    // --- registration ---

    #[test]
    fn all_tools_registered() {
        let audit = make_audit_with_entries(0);
        let mut registry = ToolRegistry::new();
        let _ = register(&mut registry, audit);

        assert!(registry.get("libro_query").is_some());
        assert!(registry.get("libro_verify").is_some());
        assert!(registry.get("libro_export").is_some());
        assert!(registry.get("libro_proof").is_some());
        assert!(registry.get("libro_retention").is_some());
    }

    #[test]
    fn read_only_tools_annotated() {
        let audit = make_audit_with_entries(0);
        let mut registry = ToolRegistry::new();
        let _ = register(&mut registry, audit);

        for name in ["libro_query", "libro_verify", "libro_export", "libro_proof"] {
            let def = registry.get(name).unwrap();
            assert_eq!(
                def.annotations.as_ref().unwrap().read_only_hint,
                Some(true),
                "{name} should be read-only"
            );
        }

        // libro_retention is NOT read-only (destructive)
        let ret_def = registry.get("libro_retention").unwrap();
        assert!(ret_def.annotations.is_none());
    }
}
