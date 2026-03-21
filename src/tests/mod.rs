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
    let init = dispatcher.dispatch(&protocol::JsonRpcRequest::new(1, "initialize"));
    assert!(init.result.is_some());

    // List tools
    let list = dispatcher.dispatch(&protocol::JsonRpcRequest::new(2, "tools/list"));
    let result = list.result.unwrap();
    let tools = result["tools"].as_array().unwrap().len();
    assert_eq!(tools, 1);

    // Call tool
    let call =
        dispatcher.dispatch(&protocol::JsonRpcRequest::new(3, "tools/call").with_params(
            serde_json::json!({"name": "test_action", "arguments": {"input": "hello"}}),
        ));
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
