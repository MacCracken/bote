//! Cross-crate integration tests — exercise bote's public API as a downstream
//! consumer would. These tests simulate the patterns used by daimon, agnoshi,
//! t-ron, and other AGNOS ecosystem crates.

use bote::dispatch::Dispatcher;
use bote::protocol::{JsonRpcRequest, JsonRpcResponse};
use bote::registry::{ToolDef, ToolRegistry, ToolSchema};
use bote::schema::CompiledSchema;
use bote::stream::CancellationToken;
use bote::transport;
use bote::{DispatchOutcome, StreamContext};
use std::collections::HashMap;
use std::sync::Arc;

// ===========================================================================
// Helpers
// ===========================================================================

fn empty_schema() -> ToolSchema {
    ToolSchema::new("object", HashMap::new(), vec![])
}

fn typed_schema() -> ToolSchema {
    ToolSchema::new(
        "object",
        HashMap::from([
            (
                "path".into(),
                serde_json::json!({"type": "string", "description": "File path"}),
            ),
            (
                "mode".into(),
                serde_json::json!({"type": "string", "enum": ["read", "write"]}),
            ),
            (
                "retries".into(),
                serde_json::json!({"type": "integer", "minimum": 0, "maximum": 10}),
            ),
        ]),
        vec!["path".into(), "mode".into()],
    )
}

fn make_dispatcher(n_tools: usize) -> Dispatcher {
    let mut reg = ToolRegistry::new();
    for i in 0..n_tools {
        reg.register(ToolDef::new(
            format!("app_tool_{i}"),
            format!("Test tool {i}"),
            empty_schema(),
        ));
    }
    let mut d = Dispatcher::new(reg);
    for i in 0..n_tools {
        d.handle(
            format!("app_tool_{i}"),
            Arc::new(move |params| {
                serde_json::json!({
                    "content": [{"type": "text", "text": format!("tool_{i}: {params}")}]
                })
            }),
        );
    }
    d
}

// ===========================================================================
// Consumer pattern: daimon-style (tool registry + dispatch)
// ===========================================================================

#[test]
fn daimon_pattern_register_and_dispatch() {
    let d = make_dispatcher(5);

    // Initialize
    let resp = d.dispatch(&JsonRpcRequest::new(1, "initialize")).unwrap();
    let result = resp.result.unwrap();
    assert!(result["protocolVersion"].is_string());
    assert_eq!(result["serverInfo"]["name"], "bote");

    // List tools
    let resp = d.dispatch(&JsonRpcRequest::new(2, "tools/list")).unwrap();
    let result = resp.result.unwrap();
    assert_eq!(result["tools"].as_array().unwrap().len(), 5);

    // Call a tool
    let req = JsonRpcRequest::new(3, "tools/call")
        .with_params(serde_json::json!({"name": "app_tool_2", "arguments": {"key": "value"}}));
    let resp = d.dispatch(&req).unwrap();
    let result = resp.result.unwrap();
    assert!(result.to_string().contains("tool_2"));
}

// ===========================================================================
// Consumer pattern: t-ron-style (security gate wrapping dispatcher)
// ===========================================================================

#[test]
fn tron_pattern_intercept_and_delegate() {
    let d = make_dispatcher(1);

    // Simulate t-ron: intercept tools/call, check policy, then delegate
    let req = JsonRpcRequest::new(1, "tools/call")
        .with_params(serde_json::json!({"name": "app_tool_0", "arguments": {}}));

    // Security check: extract tool name from request
    let tool_name = req.params.get("name").and_then(|v| v.as_str()).unwrap();
    assert_eq!(tool_name, "app_tool_0");

    // Policy says allow — delegate to bote
    let resp = d.dispatch(&req).unwrap();
    assert!(resp.result.is_some());
    assert!(resp.error.is_none());
}

#[test]
fn tron_pattern_deny_produces_valid_response() {
    // t-ron denies by constructing a JsonRpcResponse directly
    let resp = JsonRpcResponse::error(
        serde_json::json!(42),
        -32001,
        "security: rate limit exceeded [rate_limited]",
    );
    assert_eq!(resp.id, serde_json::json!(42));
    assert!(resp.error.is_some());
    assert!(resp.result.is_none());

    // Should serialize correctly
    let json = serde_json::to_string(&resp).unwrap();
    assert!(json.contains("-32001"));
    assert!(json.contains("rate_limited"));
}

// ===========================================================================
// Consumer pattern: agnoshi-style (protocol types for HTTP calls)
// ===========================================================================

#[test]
fn agnoshi_pattern_build_jsonrpc_request() {
    // agnoshi builds a JsonRpcRequest and sends it over HTTP
    let request = JsonRpcRequest::new(1, "tools/call").with_params(serde_json::json!({
        "name": "synapse_deploy",
        "arguments": {"model": "gpt-4", "replicas": 3}
    }));

    // Serialize for HTTP body
    let json = serde_json::to_string(&request).unwrap();
    assert!(json.contains("tools/call"));
    assert!(json.contains("synapse_deploy"));

    // Deserialize response
    let response_json =
        r#"{"jsonrpc":"2.0","id":1,"result":{"content":[{"type":"text","text":"deployed"}]}}"#;
    let resp: JsonRpcResponse = serde_json::from_str(response_json).unwrap();
    assert!(resp.result.is_some());
    assert!(resp.error.is_none());
}

// ===========================================================================
// Consumer pattern: tool definition via ToolDef::new() (all consumers)
// ===========================================================================

#[test]
fn consumer_pattern_tool_def_construction() {
    // This is the pattern every consumer uses — no struct literals
    let def = ToolDef::new(
        "my_tool",
        "My custom tool",
        ToolSchema::new(
            "object",
            HashMap::from([
                (
                    "input".into(),
                    serde_json::json!({"type": "string", "description": "Input data"}),
                ),
                (
                    "verbose".into(),
                    serde_json::json!({"type": "boolean", "default": false}),
                ),
            ]),
            vec!["input".into()],
        ),
    );

    assert_eq!(def.name, "my_tool");
    assert_eq!(def.input_schema.schema_type, "object");
    assert_eq!(def.input_schema.properties.len(), 2);
    assert_eq!(def.input_schema.required, vec!["input"]);

    // Serde roundtrip (used by all consumers for MCP wire format)
    let json = serde_json::to_value(&def).unwrap();
    let back: ToolDef = serde_json::from_value(json).unwrap();
    assert_eq!(back.name, "my_tool");
}

// ===========================================================================
// Consumer pattern: schema validation (bote validates before handler runs)
// ===========================================================================

#[test]
fn consumer_pattern_schema_validation_rejects_bad_params() {
    let mut reg = ToolRegistry::new();
    reg.register(ToolDef::new("strict_tool", "Strict", typed_schema()));

    // Valid params
    let valid = serde_json::json!({"path": "/tmp/foo", "mode": "read", "retries": 3});
    assert!(reg.validate_params("strict_tool", &valid).is_ok());

    // Missing required field
    let missing = serde_json::json!({"path": "/tmp/foo"});
    assert!(reg.validate_params("strict_tool", &missing).is_err());

    // Wrong type
    let wrong_type = serde_json::json!({"path": 42, "mode": "read"});
    assert!(reg.validate_params("strict_tool", &wrong_type).is_err());

    // Invalid enum value
    let bad_enum = serde_json::json!({"path": "/tmp", "mode": "delete"});
    assert!(reg.validate_params("strict_tool", &bad_enum).is_err());

    // Out of bounds
    let oob = serde_json::json!({"path": "/tmp", "mode": "read", "retries": 99});
    assert!(reg.validate_params("strict_tool", &oob).is_err());
}

// ===========================================================================
// Consumer pattern: CompiledSchema for direct validation
// ===========================================================================

#[test]
fn consumer_pattern_compiled_schema_with_defaults() {
    let schema = ToolSchema::new(
        "object",
        HashMap::from([
            (
                "mode".into(),
                serde_json::json!({"type": "string", "default": "auto"}),
            ),
            (
                "limit".into(),
                serde_json::json!({"type": "integer", "default": 10}),
            ),
        ]),
        vec![],
    );
    let compiled = CompiledSchema::compile(&schema).unwrap();

    // Apply defaults to sparse params
    let mut params = serde_json::json!({});
    compiled.apply_defaults(&mut params);
    assert_eq!(params["mode"], "auto");
    assert_eq!(params["limit"], 10);
}

// ===========================================================================
// Consumer pattern: transport process_message (stdio servers)
// ===========================================================================

#[test]
fn consumer_pattern_stdio_process_message() {
    let d = make_dispatcher(1);

    // Single request
    let input = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"app_tool_0","arguments":{"x":1}}}"#;
    let output = transport::process_message(input, &d).unwrap();
    let resp: JsonRpcResponse = serde_json::from_str(&output).unwrap();
    assert!(resp.result.is_some());

    // Batch request
    let batch = r#"[
        {"jsonrpc":"2.0","id":1,"method":"initialize"},
        {"jsonrpc":"2.0","id":2,"method":"tools/list"}
    ]"#;
    let output = transport::process_message(batch, &d).unwrap();
    let responses: Vec<JsonRpcResponse> = serde_json::from_str(&output).unwrap();
    assert_eq!(responses.len(), 2);

    // Notification (no response)
    let notif = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
    assert!(transport::process_message(notif, &d).is_none());
}

// ===========================================================================
// Consumer pattern: dynamic registration (runtime hot-reload)
// ===========================================================================

#[test]
fn consumer_pattern_dynamic_registration() {
    let d = Dispatcher::new(ToolRegistry::new());

    // Register at runtime
    let tool = ToolDef::new("plugin_scan", "Scan", empty_schema());
    d.register_tool(tool, Arc::new(|_| serde_json::json!({"scanned": true})))
        .unwrap();

    // Use it
    let req = JsonRpcRequest::new(1, "tools/call")
        .with_params(serde_json::json!({"name": "plugin_scan", "arguments": {}}));
    let resp = d.dispatch(&req).unwrap();
    assert_eq!(resp.result.unwrap()["scanned"], true);

    // Hot-reload handler
    let tool2 = ToolDef::new("plugin_scan", "Scan v2", empty_schema());
    d.register_tool(
        tool2,
        Arc::new(|_| serde_json::json!({"scanned": true, "version": 2})),
    )
    .unwrap();
    let resp = d.dispatch(&req).unwrap();
    assert_eq!(resp.result.unwrap()["version"], 2);

    // Deregister
    d.deregister_tool("plugin_scan").unwrap();
    let resp = d.dispatch(&req).unwrap();
    assert!(resp.error.is_some());
}

// ===========================================================================
// Consumer pattern: streaming dispatch
// ===========================================================================

#[test]
fn consumer_pattern_streaming_dispatch() {
    let mut reg = ToolRegistry::new();
    reg.register(ToolDef::new("long_task", "Long task", empty_schema()));
    let mut d = Dispatcher::new(reg);
    d.handle_streaming(
        "long_task",
        Arc::new(|_params, ctx: StreamContext| {
            for i in 1..=5 {
                if ctx.cancellation.is_cancelled() {
                    return serde_json::json!({"cancelled_at": i});
                }
                ctx.progress.report(i, 5);
            }
            serde_json::json!({"content": [{"type": "text", "text": "complete"}]})
        }),
    );

    let req = JsonRpcRequest::new(1, "tools/call")
        .with_params(serde_json::json!({"name": "long_task", "arguments": {}}));

    match d.dispatch_streaming(&req) {
        DispatchOutcome::Streaming {
            handler,
            arguments,
            ctx,
            progress_rx,
            ..
        } => {
            let result = handler(arguments, ctx);
            assert_eq!(result["content"][0]["text"], "complete");

            // Verify progress was emitted
            let mut updates = vec![];
            while let Ok(u) = progress_rx.try_recv() {
                updates.push(u);
            }
            assert_eq!(updates.len(), 5);
            assert_eq!(updates[4].progress, 5);
        }
        _ => panic!("expected Streaming outcome"),
    }
}

// ===========================================================================
// Consumer pattern: cancellation
// ===========================================================================

#[test]
fn consumer_pattern_cancellation_token() {
    let token = CancellationToken::new();
    let clone = token.clone();

    // Spawn a thread that checks cancellation
    let handle = std::thread::spawn(move || {
        while !clone.is_cancelled() {
            std::thread::yield_now();
        }
        true
    });

    token.cancel();
    assert!(handle.join().unwrap());
}

// ===========================================================================
// Consumer pattern: versioning and deprecation
// ===========================================================================

#[test]
fn consumer_pattern_versioned_tools() {
    let mut reg = ToolRegistry::new();
    reg.register(ToolDef::new("my_tool", "v1", empty_schema()).with_version("1.0.0"));
    reg.register(
        ToolDef::new("my_tool", "v2", empty_schema())
            .with_version("2.0.0")
            .with_deprecated("use v3 when available"),
    );

    // Current tool is v2
    let current = reg.get("my_tool").unwrap();
    assert_eq!(current.deprecated.as_deref(), Some("use v3 when available"));

    // Can look up specific versions
    assert!(reg.get_versioned("my_tool", "1.0.0").is_some());
    assert!(reg.get_versioned("my_tool", "2.0.0").is_some());
    assert!(reg.get_versioned("my_tool", "3.0.0").is_none());
}
