use bote::dispatch::Dispatcher;
use bote::protocol::JsonRpcRequest;
use bote::registry::{ToolDef, ToolRegistry, ToolSchema};
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

fn bench_dispatch(c: &mut Criterion) {
    let d = make_dispatcher(100);
    let req = JsonRpcRequest::new(1, "tools/call")
        .with_params(serde_json::json!({"name": "tool_50", "arguments": {}}));

    c.bench_function("dispatch_call_100_tools", |b| b.iter(|| d.dispatch(&req)));
}

fn bench_list(c: &mut Criterion) {
    let d = make_dispatcher(100);
    let req = JsonRpcRequest::new(1, "tools/list");

    c.bench_function("dispatch_list_100_tools", |b| b.iter(|| d.dispatch(&req)));
}

criterion_group!(benches, bench_dispatch, bench_list);
criterion_main!(benches);
