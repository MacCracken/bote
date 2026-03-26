use bote::dispatch::Dispatcher;
use bote::protocol::JsonRpcRequest;
use bote::registry::{ToolDef, ToolRegistry, ToolSchema};
use bote::transport;
use criterion::{Criterion, criterion_group, criterion_main};
use std::collections::HashMap;
use std::sync::Arc;

fn make_dispatcher(n_tools: usize) -> Dispatcher {
    let mut reg = ToolRegistry::new();
    for i in 0..n_tools {
        reg.register(ToolDef::new(
            format!("tool_{i}"),
            format!("Tool {i}"),
            ToolSchema::new("object", HashMap::new(), vec![]),
        ));
    }
    let mut d = Dispatcher::new(reg);
    for i in 0..n_tools {
        d.handle(
            format!("tool_{i}"),
            Arc::new(|_| serde_json::json!({"ok": true})),
        );
    }
    d
}

fn bench_dispatch_call(c: &mut Criterion) {
    let d = make_dispatcher(100);
    let req = JsonRpcRequest::new(1, "tools/call")
        .with_params(serde_json::json!({"name": "tool_50", "arguments": {}}));

    c.bench_function("dispatch_call_100_tools", |b| b.iter(|| d.dispatch(&req)));
}

fn bench_dispatch_list(c: &mut Criterion) {
    let d = make_dispatcher(100);
    let req = JsonRpcRequest::new(1, "tools/list");

    c.bench_function("dispatch_list_100_tools", |b| b.iter(|| d.dispatch(&req)));
}

fn bench_dispatch_initialize(c: &mut Criterion) {
    let d = make_dispatcher(1);
    let req = JsonRpcRequest::new(1, "initialize")
        .with_params(serde_json::json!({"protocolVersion": "2024-11-05"}));

    c.bench_function("dispatch_initialize", |b| b.iter(|| d.dispatch(&req)));
}

fn bench_dispatch_notification(c: &mut Criterion) {
    let d = make_dispatcher(100);
    let req = JsonRpcRequest::notification("notifications/initialized");

    c.bench_function("dispatch_notification", |b| b.iter(|| d.dispatch(&req)));
}

fn bench_process_message_single(c: &mut Criterion) {
    let d = make_dispatcher(100);
    let input = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"tool_50","arguments":{}}}"#;

    c.bench_function("process_message_single", |b| {
        b.iter(|| transport::process_message(input, &d))
    });
}

fn bench_process_message_batch(c: &mut Criterion) {
    let d = make_dispatcher(100);
    let requests: Vec<String> = (0..10)
        .map(|i| {
            format!(
                r#"{{"jsonrpc":"2.0","id":{i},"method":"tools/call","params":{{"name":"tool_{i}","arguments":{{}}}}}}"#
            )
        })
        .collect();
    let input = format!("[{}]", requests.join(","));

    c.bench_function("process_message_batch_10", |b| {
        b.iter(|| transport::process_message(&input, &d))
    });
}

fn bench_dispatch_streaming_setup(c: &mut Criterion) {
    let mut reg = ToolRegistry::new();
    reg.register(ToolDef::new(
        "stream_tool",
        "Streaming",
        ToolSchema::new("object", HashMap::new(), vec![]),
    ));
    let mut d = Dispatcher::new(reg);
    d.handle_streaming(
        "stream_tool",
        Arc::new(|_params, _ctx| serde_json::json!({"ok": true})),
    );
    let req = JsonRpcRequest::new(1, "tools/call")
        .with_params(serde_json::json!({"name": "stream_tool", "arguments": {}}));

    c.bench_function("dispatch_streaming_setup", |b| {
        b.iter(|| d.dispatch_streaming(&req))
    });
}

fn bench_validate_params(c: &mut Criterion) {
    let mut reg = ToolRegistry::new();
    reg.register(ToolDef::new(
        "strict",
        "Strict",
        ToolSchema::new("object", HashMap::new(), vec!["path".into(), "mode".into()]),
    ));
    let params = serde_json::json!({"path": "/tmp/foo", "mode": "read"});

    c.bench_function("validate_params_2_required", |b| {
        b.iter(|| reg.validate_params("strict", &params))
    });
}

fn bench_wrap_tool_result(c: &mut Criterion) {
    let raw = serde_json::json!({"answer": 42, "data": [1, 2, 3]});
    let already_wrapped = serde_json::json!({
        "content": [{"type": "text", "text": "hello"}]
    });

    c.bench_function("wrap_tool_result_raw", |b| {
        b.iter(|| bote::bridge::wrap_tool_result(&raw))
    });
    c.bench_function("wrap_tool_result_passthrough", |b| {
        b.iter(|| bote::bridge::wrap_tool_result(&already_wrapped))
    });
}

fn bench_validate_params_typed(c: &mut Criterion) {
    let mut reg = ToolRegistry::new();
    let mut props = HashMap::new();
    props.insert("path".into(), serde_json::json!({"type": "string"}));
    props.insert(
        "mode".into(),
        serde_json::json!({"type": "string", "enum": ["read", "write"]}),
    );
    props.insert(
        "retries".into(),
        serde_json::json!({"type": "integer", "minimum": 0, "maximum": 10}),
    );
    reg.register(ToolDef::new(
        "typed",
        "Typed",
        ToolSchema::new("object", props, vec!["path".into(), "mode".into()]),
    ));
    let params = serde_json::json!({"path": "/tmp/foo", "mode": "read", "retries": 3});

    c.bench_function("validate_params_typed_schema", |b| {
        b.iter(|| reg.validate_params("typed", &params))
    });
}

fn bench_schema_compile(c: &mut Criterion) {
    let mut props = HashMap::new();
    props.insert("name".into(), serde_json::json!({"type": "string"}));
    props.insert(
        "count".into(),
        serde_json::json!({"type": "integer", "minimum": 0}),
    );
    props.insert(
        "tags".into(),
        serde_json::json!({"type": "array", "items": {"type": "string"}}),
    );
    let schema = ToolSchema::new("object", props, vec!["name".into()]);

    c.bench_function("schema_compile", |b| {
        b.iter(|| bote::schema::CompiledSchema::compile(&schema))
    });
}

fn bench_dispatch_call_rwlock(c: &mut Criterion) {
    let d = make_dispatcher(100);
    let req = JsonRpcRequest::new(1, "tools/call")
        .with_params(serde_json::json!({"name": "tool_50", "arguments": {}}));

    c.bench_function("dispatch_call_100_tools_rwlock", |b| {
        b.iter(|| d.dispatch(&req))
    });
}

criterion_group!(
    benches,
    bench_dispatch_call,
    bench_dispatch_list,
    bench_dispatch_initialize,
    bench_dispatch_notification,
    bench_process_message_single,
    bench_process_message_batch,
    bench_dispatch_streaming_setup,
    bench_validate_params,
    bench_wrap_tool_result,
    bench_validate_params_typed,
    bench_schema_compile,
    bench_dispatch_call_rwlock,
);
criterion_main!(benches);
