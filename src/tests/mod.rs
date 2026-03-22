use crate::*;
use std::collections::HashMap;
use std::sync::Arc;

#[test]
fn full_mcp_flow() {
    let mut reg = registry::ToolRegistry::new();
    reg.register(registry::ToolDef {
        name: "test_action".into(),
        description: "A test tool".into(),
        input_schema: registry::ToolSchema {
            schema_type: "object".into(),
            properties: HashMap::new(),
            required: vec!["input".into()],
        },
    });

    let mut dispatcher = dispatch::Dispatcher::new(reg);
    dispatcher.handle("test_action", Arc::new(|params| {
        let input = params.get("input").and_then(|v| v.as_str()).unwrap_or("none");
        serde_json::json!({ "content": [{ "type": "text", "text": format!("processed: {input}") }] })
    }));

    // Initialize
    let init = dispatcher.dispatch(&protocol::JsonRpcRequest::new(1, "initialize")).unwrap();
    assert!(init.result.is_some());

    // List tools
    let list = dispatcher.dispatch(&protocol::JsonRpcRequest::new(2, "tools/list")).unwrap();
    let result = list.result.unwrap();
    let tools = result["tools"].as_array().unwrap().len();
    assert_eq!(tools, 1);

    // Call tool
    let call = dispatcher
        .dispatch(
            &protocol::JsonRpcRequest::new(3, "tools/call").with_params(
                serde_json::json!({"name": "test_action", "arguments": {"input": "hello"}}),
            ),
        )
        .unwrap();
    let text = call.result.unwrap()["content"][0]["text"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(text, "processed: hello");
}

#[test]
fn transport_roundtrip() {
    let req = protocol::JsonRpcRequest::new(42, "tools/list");
    let json = serde_json::to_string(&req).unwrap();
    let parsed = transport::parse_request(&json).unwrap();
    assert_eq!(parsed.method, "tools/list");

    let resp = protocol::JsonRpcResponse::success(serde_json::json!(42), serde_json::json!({}));
    let line = transport::serialize_response(&resp).unwrap();
    assert!(line.contains("42"));
}

#[test]
fn error_codes() {
    assert_eq!(BoteError::ToolNotFound("x".into()).rpc_code(), -32601);
    assert_eq!(
        BoteError::InvalidParams {
            tool: "x".into(),
            reason: "y".into()
        }
        .rpc_code(),
        -32602
    );
    assert_eq!(BoteError::Parse("bad".into()).rpc_code(), -32700);
}

#[test]
fn flow_validation_failure() {
    let mut reg = registry::ToolRegistry::new();
    reg.register(registry::ToolDef {
        name: "strict_tool".into(),
        description: "Needs input".into(),
        input_schema: registry::ToolSchema {
            schema_type: "object".into(),
            properties: HashMap::new(),
            required: vec!["input".into()],
        },
    });

    let mut dispatcher = dispatch::Dispatcher::new(reg);
    dispatcher.handle(
        "strict_tool",
        Arc::new(|_| serde_json::json!({"ok": true})),
    );

    let resp = dispatcher
        .dispatch(
            &protocol::JsonRpcRequest::new(1, "tools/call")
                .with_params(serde_json::json!({"name": "strict_tool", "arguments": {}})),
        )
        .unwrap();
    let err = resp.error.unwrap();
    assert_eq!(err.code, -32602);
    assert!(err.message.contains("input"));
}

#[test]
fn flow_unknown_tool_call() {
    let reg = registry::ToolRegistry::new();
    let dispatcher = dispatch::Dispatcher::new(reg);

    let resp = dispatcher
        .dispatch(
            &protocol::JsonRpcRequest::new(1, "tools/call")
                .with_params(serde_json::json!({"name": "ghost", "arguments": {}})),
        )
        .unwrap();
    let err = resp.error.unwrap();
    assert_eq!(err.code, -32601);
    assert!(err.message.contains("ghost"));
}

#[test]
fn flow_unknown_method() {
    let reg = registry::ToolRegistry::new();
    let dispatcher = dispatch::Dispatcher::new(reg);

    let resp = dispatcher
        .dispatch(&protocol::JsonRpcRequest::new(1, "bogus/rpc"))
        .unwrap();
    let err = resp.error.unwrap();
    assert_eq!(err.code, -32600);
    assert!(err.message.contains("bogus/rpc"));
}

#[test]
fn transport_parse_dispatch_serialize() {
    let mut reg = registry::ToolRegistry::new();
    reg.register(registry::ToolDef {
        name: "ping".into(),
        description: "Ping".into(),
        input_schema: registry::ToolSchema {
            schema_type: "object".into(),
            properties: HashMap::new(),
            required: vec![],
        },
    });
    let mut dispatcher = dispatch::Dispatcher::new(reg);
    dispatcher.handle("ping", Arc::new(|_| serde_json::json!({"pong": true})));

    // Full path: raw JSON → process_message → response string
    let raw = r#"{"jsonrpc":"2.0","id":99,"method":"tools/call","params":{"name":"ping","arguments":{}}}"#;
    let out = transport::process_message(raw, &dispatcher).unwrap();
    assert!(out.contains("\"pong\""));
    assert!(out.contains("99"));
}

#[test]
fn flow_notification_no_response() {
    let reg = registry::ToolRegistry::new();
    let dispatcher = dispatch::Dispatcher::new(reg);

    let resp = dispatcher.dispatch(&protocol::JsonRpcRequest::notification("notifications/initialized"));
    assert!(resp.is_none());
}

#[test]
fn flow_batch_via_process_message() {
    let mut reg = registry::ToolRegistry::new();
    reg.register(registry::ToolDef {
        name: "ping".into(),
        description: "Ping".into(),
        input_schema: registry::ToolSchema {
            schema_type: "object".into(),
            properties: HashMap::new(),
            required: vec![],
        },
    });
    let mut dispatcher = dispatch::Dispatcher::new(reg);
    dispatcher.handle("ping", Arc::new(|_| serde_json::json!({"pong": true})));

    let input = r#"[
        {"jsonrpc":"2.0","id":1,"method":"initialize"},
        {"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"ping","arguments":{}}}
    ]"#;
    let out = transport::process_message(input, &dispatcher).unwrap();
    let responses: Vec<protocol::JsonRpcResponse> = serde_json::from_str(&out).unwrap();
    assert_eq!(responses.len(), 2);
    assert!(responses[0].result.is_some());
    assert!(responses[1].result.is_some());
}
