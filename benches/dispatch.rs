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
        reg.register(ToolDef {
            name: format!("tool_{i}"),
            description: format!("Tool {i}"),
            input_schema: ToolSchema {
                schema_type: "object".into(),
                properties: HashMap::new(),
                required: vec![],
            },
        });
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
    reg.register(ToolDef {
        name: "stream_tool".into(),
        description: "Streaming".into(),
        input_schema: ToolSchema {
            schema_type: "object".into(),
            properties: HashMap::new(),
            required: vec![],
        },
    });
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
    reg.register(ToolDef {
        name: "strict".into(),
        description: "Strict".into(),
        input_schema: ToolSchema {
            schema_type: "object".into(),
            properties: HashMap::new(),
            required: vec!["path".into(), "mode".into()],
        },
    });
    let params = serde_json::json!({"path": "/tmp/foo", "mode": "read"});

    c.bench_function("validate_params_2_required", |b| {
        b.iter(|| reg.validate_params("strict", &params))
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
);
criterion_main!(benches);
