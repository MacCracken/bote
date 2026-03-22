//! Tool call dispatcher — route JSON-RPC calls to registered handlers.

use std::collections::HashMap;
use std::sync::Arc;

use crate::audit::{AuditSink, ToolCallEvent};
use crate::error::BoteError;
use crate::events::{self, EventSink};
use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
use crate::registry::ToolRegistry;
use crate::stream::{self, ProgressUpdate, StreamContext, StreamingToolHandler};

/// Supported MCP protocol versions.
const SUPPORTED_VERSIONS: &[&str] = &["2024-11-05", "2025-03-26"];

/// Default protocol version when the client doesn't specify one.
const LATEST_VERSION: &str = "2025-03-26";

/// A tool handler function.
pub type ToolHandler = Arc<dyn Fn(serde_json::Value) -> serde_json::Value + Send + Sync>;

/// Outcome of `dispatch_streaming` — either an immediate response or a
/// streaming context that the transport drives.
pub enum DispatchOutcome {
    /// Immediate response (or `None` for notifications).
    Immediate(Option<JsonRpcResponse>),
    /// The tool supports streaming. The transport should spawn the handler,
    /// drain `progress_rx` for progress updates, and build the final response
    /// from the handler's return value.
    Streaming {
        request_id: serde_json::Value,
        progress_rx: std::sync::mpsc::Receiver<ProgressUpdate>,
        ctx: StreamContext,
        handler: StreamingToolHandler,
        arguments: serde_json::Value,
    },
}

/// Dispatcher: routes tool calls to handlers via the registry.
pub struct Dispatcher {
    registry: ToolRegistry,
    handlers: HashMap<String, ToolHandler>,
    streaming_handlers: HashMap<String, StreamingToolHandler>,
    audit: Option<Arc<dyn AuditSink>>,
    events: Option<Arc<dyn EventSink>>,
}

impl Dispatcher {
    pub fn new(registry: ToolRegistry) -> Self {
        Self {
            registry,
            handlers: HashMap::new(),
            streaming_handlers: HashMap::new(),
            audit: None,
            events: None,
        }
    }

    /// Set the audit sink for logging tool calls.
    pub fn set_audit(&mut self, sink: Arc<dyn AuditSink>) {
        self.audit = Some(sink);
    }

    /// Set the event sink for publishing tool events.
    pub fn set_events(&mut self, sink: Arc<dyn EventSink>) {
        self.events = Some(sink);
    }

    /// Log a tool call event to the audit and event sinks.
    /// Called automatically for sync dispatch; transports call this
    /// after streaming handlers complete.
    pub fn log_tool_call(&self, event: &ToolCallEvent) {
        if let Some(audit) = &self.audit {
            audit.log(event);
        }
        if let Some(events) = &self.events {
            let topic = if event.success {
                events::TOPIC_TOOL_COMPLETED
            } else {
                events::TOPIC_TOOL_FAILED
            };
            events.publish(topic, serde_json::to_value(event).unwrap_or_default());
        }
    }

    /// Register a handler for a tool.
    pub fn handle(&mut self, tool_name: impl Into<String>, handler: ToolHandler) {
        let name = tool_name.into();
        if let Some(events) = &self.events {
            events.publish(
                events::TOPIC_TOOL_REGISTERED,
                serde_json::json!({"tool_name": &name}),
            );
        }
        self.handlers.insert(name, handler);
    }

    /// Register a streaming handler for a tool.
    pub fn handle_streaming(
        &mut self,
        tool_name: impl Into<String>,
        handler: StreamingToolHandler,
    ) {
        let name = tool_name.into();
        if let Some(events) = &self.events {
            events.publish(
                events::TOPIC_TOOL_REGISTERED,
                serde_json::json!({"tool_name": &name}),
            );
        }
        self.streaming_handlers.insert(name, handler);
    }

    /// Returns `true` if the tool has a streaming handler registered.
    pub fn is_streaming_tool(&self, name: &str) -> bool {
        self.streaming_handlers.contains_key(name)
    }

    /// Extract and validate the tool name from a tools/call request.
    fn extract_tool_name(request: &JsonRpcRequest) -> Result<&str, BoteError> {
        request
            .params
            .get("name")
            .and_then(|v| v.as_str())
            .filter(|n| !n.is_empty())
            .ok_or_else(|| BoteError::InvalidParams {
                tool: String::new(),
                reason: "missing or empty 'name' field".into(),
            })
    }

    /// Dispatch a JSON-RPC request. Returns `None` for notifications.
    pub fn dispatch(&self, request: &JsonRpcRequest) -> Option<JsonRpcResponse> {
        let id = request.id.clone().unwrap_or(serde_json::Value::Null);

        let response = match request.method.as_str() {
            "initialize" => {
                let negotiated = request
                    .params
                    .get("protocolVersion")
                    .and_then(|v| v.as_str())
                    .filter(|v| SUPPORTED_VERSIONS.contains(v))
                    .unwrap_or(LATEST_VERSION);

                JsonRpcResponse::success(
                    id,
                    serde_json::json!({
                        "protocolVersion": negotiated,
                        "capabilities": { "tools": {} },
                        "serverInfo": { "name": "bote", "version": env!("CARGO_PKG_VERSION") }
                    }),
                )
            }
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
                JsonRpcResponse::success(id, serde_json::json!({ "tools": tools }))
            }
            "tools/call" => {
                let tool_name = match Self::extract_tool_name(request) {
                    Ok(name) => name,
                    Err(e) => return Some(JsonRpcResponse::error(id, e.rpc_code(), e.to_string())),
                };
                let arguments = request
                    .params
                    .get("arguments")
                    .cloned()
                    .unwrap_or(serde_json::json!({}));

                // Validate
                if let Err(e) = self.registry.validate_params(tool_name, &arguments) {
                    tracing::warn!(tool = tool_name, error = %e, "param validation failed");
                    return Some(JsonRpcResponse::error(id, e.rpc_code(), e.to_string()));
                }

                // Dispatch with timing
                if let Some(handler) = self.handlers.get(tool_name) {
                    tracing::debug!(tool = tool_name, "dispatching tool call");
                    let start = std::time::Instant::now();
                    let result = handler(arguments);
                    let duration_ms = start.elapsed().as_millis() as u64;
                    tracing::info!(tool = tool_name, duration_ms, "tool call completed");

                    self.log_tool_call(&ToolCallEvent {
                        tool_name: tool_name.into(),
                        duration_ms,
                        success: true,
                        error: None,
                        caller_id: None,
                    });

                    JsonRpcResponse::success(id, result)
                } else {
                    tracing::warn!(tool = tool_name, "tool not found");
                    let err = BoteError::ToolNotFound(tool_name.into());
                    self.log_tool_call(&ToolCallEvent {
                        tool_name: tool_name.into(),
                        duration_ms: 0,
                        success: false,
                        error: Some(err.to_string()),
                        caller_id: None,
                    });
                    JsonRpcResponse::error(id, err.rpc_code(), err.to_string())
                }
            }
            _ => {
                let err = BoteError::Protocol(format!("unknown method: {}", request.method));
                JsonRpcResponse::error(id, err.rpc_code(), err.to_string())
            }
        };

        if request.is_notification() {
            None
        } else {
            Some(response)
        }
    }

    /// Dispatch with streaming support. Returns `DispatchOutcome::Streaming` for
    /// tools with streaming handlers, `DispatchOutcome::Immediate` otherwise.
    pub fn dispatch_streaming(&self, request: &JsonRpcRequest) -> DispatchOutcome {
        // Only tools/call can be streaming.
        if request.method != "tools/call" {
            return DispatchOutcome::Immediate(self.dispatch(request));
        }

        let id = request.id.clone().unwrap_or(serde_json::Value::Null);
        let tool_name = match Self::extract_tool_name(request) {
            Ok(name) => name,
            Err(e) => {
                return DispatchOutcome::Immediate(Some(JsonRpcResponse::error(
                    id,
                    e.rpc_code(),
                    e.to_string(),
                )));
            }
        };
        let arguments = request
            .params
            .get("arguments")
            .cloned()
            .unwrap_or(serde_json::json!({}));

        // Validate params.
        if let Err(e) = self.registry.validate_params(tool_name, &arguments) {
            return DispatchOutcome::Immediate(Some(JsonRpcResponse::error(
                id,
                e.rpc_code(),
                e.to_string(),
            )));
        }

        // Streaming handler takes priority.
        if let Some(handler) = self.streaming_handlers.get(tool_name) {
            if request.is_notification() {
                return DispatchOutcome::Immediate(None);
            }

            let (ctx, progress_rx) = stream::make_stream_context();
            return DispatchOutcome::Streaming {
                request_id: id,
                progress_rx,
                ctx,
                handler: Arc::clone(handler),
                arguments,
            };
        }

        // Fall back to sync dispatch.
        DispatchOutcome::Immediate(self.dispatch(request))
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
        let resp = d.dispatch(&req).unwrap();
        assert!(resp.result.is_some());
    }

    #[test]
    fn dispatch_tools_list() {
        let d = make_dispatcher();
        let req = JsonRpcRequest::new(1, "tools/list");
        let resp = d.dispatch(&req).unwrap();
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
        let resp = d.dispatch(&req).unwrap();
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
    }

    #[test]
    fn dispatch_unknown_method() {
        let d = make_dispatcher();
        let req = JsonRpcRequest::new(1, "bogus/method");
        let resp = d.dispatch(&req).unwrap();
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, -32600);
    }

    #[test]
    fn dispatch_unknown_tool() {
        let d = make_dispatcher();
        let req = JsonRpcRequest::new(1, "tools/call")
            .with_params(serde_json::json!({"name": "nope", "arguments": {}}));
        let resp = d.dispatch(&req).unwrap();
        let err = resp.error.unwrap();
        assert_eq!(err.code, -32601);
        assert!(err.message.contains("nope"));
    }

    #[test]
    fn initialize_response_structure() {
        let d = make_dispatcher();
        let req = JsonRpcRequest::new(1, "initialize");
        let resp = d.dispatch(&req).unwrap();
        let result = resp.result.unwrap();
        assert_eq!(result["protocolVersion"], LATEST_VERSION);
        assert_eq!(result["serverInfo"]["name"], "bote");
        assert!(result["serverInfo"]["version"].is_string());
        assert!(result["capabilities"]["tools"].is_object());
    }

    #[test]
    fn initialize_version_negotiation_supported() {
        let d = make_dispatcher();
        let req = JsonRpcRequest::new(1, "initialize")
            .with_params(serde_json::json!({"protocolVersion": "2024-11-05"}));
        let resp = d.dispatch(&req).unwrap();
        let result = resp.result.unwrap();
        assert_eq!(result["protocolVersion"], "2024-11-05");
    }

    #[test]
    fn initialize_version_negotiation_unsupported() {
        let d = make_dispatcher();
        let req = JsonRpcRequest::new(1, "initialize")
            .with_params(serde_json::json!({"protocolVersion": "2099-01-01"}));
        let resp = d.dispatch(&req).unwrap();
        let result = resp.result.unwrap();
        assert_eq!(result["protocolVersion"], LATEST_VERSION);
    }

    #[test]
    fn initialize_version_negotiation_missing() {
        let d = make_dispatcher();
        let req = JsonRpcRequest::new(1, "initialize");
        let resp = d.dispatch(&req).unwrap();
        let result = resp.result.unwrap();
        assert_eq!(result["protocolVersion"], LATEST_VERSION);
    }

    #[test]
    fn dispatch_notification_returns_none() {
        let d = make_dispatcher();
        let req = JsonRpcRequest::notification("notifications/initialized");
        assert!(d.dispatch(&req).is_none());
    }

    #[test]
    fn dispatch_call_missing_name() {
        let d = make_dispatcher();
        let req =
            JsonRpcRequest::new(1, "tools/call").with_params(serde_json::json!({"arguments": {}}));
        let resp = d.dispatch(&req).unwrap();
        let err = resp.error.unwrap();
        assert_eq!(err.code, -32602);
        assert!(err.message.contains("missing or empty 'name'"));
    }

    #[test]
    fn dispatch_call_empty_name() {
        let d = make_dispatcher();
        let req = JsonRpcRequest::new(1, "tools/call")
            .with_params(serde_json::json!({"name": "", "arguments": {}}));
        let resp = d.dispatch(&req).unwrap();
        let err = resp.error.unwrap();
        assert_eq!(err.code, -32602);
        assert!(err.message.contains("missing or empty 'name'"));
    }

    #[test]
    fn dispatch_call_name_is_number() {
        let d = make_dispatcher();
        let req = JsonRpcRequest::new(1, "tools/call")
            .with_params(serde_json::json!({"name": 42, "arguments": {}}));
        let resp = d.dispatch(&req).unwrap();
        let err = resp.error.unwrap();
        assert_eq!(err.code, -32602);
    }

    #[test]
    fn dispatch_call_defaults_empty_arguments() {
        let mut reg = ToolRegistry::new();
        reg.register(ToolDef {
            name: "noop".into(),
            description: "No args".into(),
            input_schema: ToolSchema {
                schema_type: "object".into(),
                properties: HashMap::new(),
                required: vec![],
            },
        });
        let mut d = Dispatcher::new(reg);
        d.handle("noop", Arc::new(|_| serde_json::json!({"ok": true})));

        let req =
            JsonRpcRequest::new(1, "tools/call").with_params(serde_json::json!({"name": "noop"}));
        let resp = d.dispatch(&req).unwrap();
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
    }

    #[test]
    fn dispatch_call_with_invalid_params() {
        let mut reg = ToolRegistry::new();
        reg.register(ToolDef {
            name: "strict".into(),
            description: "Requires path".into(),
            input_schema: ToolSchema {
                schema_type: "object".into(),
                properties: HashMap::new(),
                required: vec!["path".into()],
            },
        });
        let mut d = Dispatcher::new(reg);
        d.handle("strict", Arc::new(|_| serde_json::json!({"ok": true})));

        let req = JsonRpcRequest::new(1, "tools/call")
            .with_params(serde_json::json!({"name": "strict", "arguments": {}}));
        let resp = d.dispatch(&req).unwrap();
        let err = resp.error.unwrap();
        assert_eq!(err.code, -32602);
        assert!(err.message.contains("path"));
    }

    #[test]
    fn dispatch_preserves_request_id() {
        let d = make_dispatcher();
        let req = JsonRpcRequest::new("req-abc", "initialize");
        let resp = d.dispatch(&req).unwrap();
        assert_eq!(resp.id, serde_json::json!("req-abc"));
    }

    // --- Streaming dispatch tests ---

    fn make_streaming_dispatcher() -> Dispatcher {
        let mut reg = ToolRegistry::new();
        reg.register(ToolDef {
            name: "slow".into(),
            description: "Slow streaming tool".into(),
            input_schema: ToolSchema {
                schema_type: "object".into(),
                properties: HashMap::new(),
                required: vec![],
            },
        });
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
        // Sync handler for echo.
        d.handle(
            "echo",
            Arc::new(|params| serde_json::json!({ "echoed": params })),
        );
        // Streaming handler for slow.
        d.handle_streaming(
            "slow",
            Arc::new(|_params, ctx| {
                ctx.progress.report(1, 3);
                ctx.progress.report(2, 3);
                ctx.progress.report(3, 3);
                serde_json::json!({"content": [{"type": "text", "text": "done"}]})
            }),
        );
        d
    }

    #[test]
    fn is_streaming_tool_check() {
        let d = make_streaming_dispatcher();
        assert!(d.is_streaming_tool("slow"));
        assert!(!d.is_streaming_tool("echo"));
        assert!(!d.is_streaming_tool("nonexistent"));
    }

    #[test]
    fn dispatch_streaming_returns_streaming_for_streaming_tool() {
        let d = make_streaming_dispatcher();
        let req = JsonRpcRequest::new(1, "tools/call")
            .with_params(serde_json::json!({"name": "slow", "arguments": {}}));
        match d.dispatch_streaming(&req) {
            DispatchOutcome::Streaming {
                request_id,
                handler,
                arguments,
                ctx,
                progress_rx,
            } => {
                assert_eq!(request_id, serde_json::json!(1));
                // Execute the handler and verify progress.
                let result = handler(arguments, ctx);
                assert_eq!(result["content"][0]["text"], "done");

                let mut updates = vec![];
                while let Ok(u) = progress_rx.try_recv() {
                    updates.push(u);
                }
                assert_eq!(updates.len(), 3);
                assert_eq!(updates[0].progress, 1);
                assert_eq!(updates[2].progress, 3);
            }
            _ => panic!("expected DispatchOutcome::Streaming"),
        }
    }

    #[test]
    fn dispatch_streaming_returns_immediate_for_sync_tool() {
        let d = make_streaming_dispatcher();
        let req = JsonRpcRequest::new(1, "tools/call")
            .with_params(serde_json::json!({"name": "echo", "arguments": {"msg": "hi"}}));
        match d.dispatch_streaming(&req) {
            DispatchOutcome::Immediate(Some(resp)) => {
                assert!(resp.result.is_some());
            }
            _ => panic!("expected DispatchOutcome::Immediate(Some)"),
        }
    }

    #[test]
    fn dispatch_streaming_returns_immediate_for_initialize() {
        let d = make_streaming_dispatcher();
        let req = JsonRpcRequest::new(1, "initialize");
        match d.dispatch_streaming(&req) {
            DispatchOutcome::Immediate(Some(resp)) => {
                assert!(resp.result.is_some());
            }
            _ => panic!("expected DispatchOutcome::Immediate(Some)"),
        }
    }

    #[test]
    fn dispatch_streaming_returns_none_for_notification() {
        let d = make_streaming_dispatcher();
        let req = JsonRpcRequest::notification("notifications/initialized");
        match d.dispatch_streaming(&req) {
            DispatchOutcome::Immediate(None) => {}
            _ => panic!("expected DispatchOutcome::Immediate(None)"),
        }
    }

    #[test]
    fn streaming_handler_sees_cancellation() {
        let d = {
            let mut reg = ToolRegistry::new();
            reg.register(ToolDef {
                name: "cancelable".into(),
                description: "Cancelable".into(),
                input_schema: ToolSchema {
                    schema_type: "object".into(),
                    properties: HashMap::new(),
                    required: vec![],
                },
            });
            let mut d = Dispatcher::new(reg);
            d.handle_streaming(
                "cancelable",
                Arc::new(|_params, ctx| {
                    for i in 0..100 {
                        if ctx.cancellation.is_cancelled() {
                            return serde_json::json!({"cancelled_at": i});
                        }
                        ctx.progress.report(i, 100);
                    }
                    serde_json::json!({"completed": true})
                }),
            );
            d
        };

        let req = JsonRpcRequest::new(1, "tools/call")
            .with_params(serde_json::json!({"name": "cancelable", "arguments": {}}));

        match d.dispatch_streaming(&req) {
            DispatchOutcome::Streaming {
                ctx,
                handler,
                arguments,
                ..
            } => {
                // Cancel immediately.
                ctx.cancellation.cancel();
                let result = handler(arguments, ctx);
                assert_eq!(result["cancelled_at"], 0);
            }
            _ => panic!("expected Streaming"),
        }
    }
}
