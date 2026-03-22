//! Tool call dispatcher — route JSON-RPC calls to registered handlers.

use std::collections::HashMap;
use std::sync::Arc;

use crate::error::BoteError;
use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
use crate::registry::ToolRegistry;

/// A tool handler function.
pub type ToolHandler = Arc<dyn Fn(serde_json::Value) -> serde_json::Value + Send + Sync>;

/// Dispatcher: routes tool calls to handlers via the registry.
pub struct Dispatcher {
    registry: ToolRegistry,
    handlers: HashMap<String, ToolHandler>,
}

impl Dispatcher {
    pub fn new(registry: ToolRegistry) -> Self {
        Self {
            registry,
            handlers: HashMap::new(),
        }
    }

    /// Register a handler for a tool.
    pub fn handle(&mut self, tool_name: impl Into<String>, handler: ToolHandler) {
        self.handlers.insert(tool_name.into(), handler);
    }

    /// Dispatch a JSON-RPC request.
    pub fn dispatch(&self, request: &JsonRpcRequest) -> JsonRpcResponse {
        match request.method.as_str() {
            "initialize" => JsonRpcResponse::success(
                request.id.clone(),
                serde_json::json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": { "tools": {} },
                    "serverInfo": { "name": "bote", "version": env!("CARGO_PKG_VERSION") }
                }),
            ),
            "tools/list" => {
                let tools: Vec<serde_json::Value> = self
                    .registry
                    .list()
                    .iter()
                    .map(|t| {
                        serde_json::json!({
                            "name": t.name,
                            "description": t.description,
                            "inputSchema": t.input_schema,
                        })
                    })
                    .collect();
                JsonRpcResponse::success(request.id.clone(), serde_json::json!({ "tools": tools }))
            }
            "tools/call" => {
                let tool_name = request
                    .params
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let arguments = request
                    .params
                    .get("arguments")
                    .cloned()
                    .unwrap_or(serde_json::json!({}));

                // Validate
                if let Err(e) = self.registry.validate_params(tool_name, &arguments) {
                    return JsonRpcResponse::error(request.id.clone(), e.rpc_code(), e.to_string());
                }

                // Dispatch
                if let Some(handler) = self.handlers.get(tool_name) {
                    let result = handler(arguments);
                    JsonRpcResponse::success(request.id.clone(), result)
                } else {
                    let err = BoteError::ToolNotFound(tool_name.into());
                    JsonRpcResponse::error(request.id.clone(), err.rpc_code(), err.to_string())
                }
            }
            _ => {
                let err = BoteError::Protocol(format!("unknown method: {}", request.method));
                JsonRpcResponse::error(request.id.clone(), err.rpc_code(), err.to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{ToolDef, ToolSchema};

    fn make_dispatcher() -> Dispatcher {
        let mut reg = ToolRegistry::new();
        reg.register(ToolDef {
            name: "echo".into(),
            description: "Echo input".into(),
            input_schema: ToolSchema {
                schema_type: "object".into(),
                properties: HashMap::new(),
                required: vec![],
            },
        });
        let mut d = Dispatcher::new(reg);
        d.handle("echo", Arc::new(|params| {
            serde_json::json!({ "content": [{ "type": "text", "text": params.to_string() }] })
        }));
        d
    }

    #[test]
    fn dispatch_initialize() {
        let d = make_dispatcher();
        let req = JsonRpcRequest::new(1, "initialize");
        let resp = d.dispatch(&req);
        assert!(resp.result.is_some());
    }

    #[test]
    fn dispatch_tools_list() {
        let d = make_dispatcher();
        let req = JsonRpcRequest::new(1, "tools/list");
        let resp = d.dispatch(&req);
        let result = resp.result.unwrap();
        let tools = result["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["name"], "echo");
    }

    #[test]
    fn dispatch_tools_call() {
        let d = make_dispatcher();
        let req = JsonRpcRequest::new(1, "tools/call")
            .with_params(serde_json::json!({"name": "echo", "arguments": {"msg": "hello"}}));
        let resp = d.dispatch(&req);
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
    }

    #[test]
    fn dispatch_unknown_method() {
        let d = make_dispatcher();
        let req = JsonRpcRequest::new(1, "bogus/method");
        let resp = d.dispatch(&req);
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, -32600);
    }

    #[test]
    fn dispatch_unknown_tool() {
        let d = make_dispatcher();
        let req = JsonRpcRequest::new(1, "tools/call")
            .with_params(serde_json::json!({"name": "nope", "arguments": {}}));
        let resp = d.dispatch(&req);
        assert!(resp.error.is_some());
    }
}
