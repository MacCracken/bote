//! Tool sandboxing via kavach — execute tool commands in isolated environments.
//!
//! The sandbox module provides a decorator pattern for wrapping tool commands
//! in kavach sandboxes. Tool arguments are piped as JSON via stdin, and the
//! command's stdout is parsed as the JSON result.
//!
//! Enable the `sandbox` feature to use this module.
//!
//! # Example
//!
//! ```rust,no_run
//! use bote::sandbox::{ToolSandboxConfig, wrap_command};
//!
//! let handler = wrap_command("my-tool", ToolSandboxConfig::noop());
//! // handler can be passed to dispatcher.register_tool()
//! ```

use std::sync::{Arc, OnceLock};

use kavach::{Backend, ExecResult, Sandbox, SandboxConfig, SandboxPolicy, SandboxState};

use crate::dispatch::{Dispatcher, ToolHandler};
use crate::error::BoteError;
use crate::events::EventSink;
use crate::registry::ToolDef;
use crate::stream::{StreamContext, StreamingToolHandler};

/// Fallback tokio runtime for sync contexts without an active runtime.
static FALLBACK_RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

/// Get a tokio runtime handle, creating a fallback if needed.
fn runtime_handle() -> tokio::runtime::Handle {
    match tokio::runtime::Handle::try_current() {
        Ok(h) => h,
        Err(_) => FALLBACK_RT
            .get_or_init(|| {
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("BUG: failed to create fallback tokio runtime")
            })
            .handle()
            .clone(),
    }
}

/// Per-tool sandbox configuration.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ToolSandboxConfig {
    /// Isolation backend to use.
    pub backend: Backend,
    /// Security policy (seccomp, landlock, network, resources).
    pub policy: SandboxPolicy,
    /// Maximum execution time in milliseconds.
    pub timeout_ms: u64,
    /// Environment variables to inject into the sandbox.
    pub env: Vec<(String, String)>,
}

impl ToolSandboxConfig {
    /// Create a new sandbox config.
    #[must_use]
    pub fn new(backend: Backend, policy: SandboxPolicy, timeout_ms: u64) -> Self {
        Self {
            backend,
            policy,
            timeout_ms,
            env: Vec::new(),
        }
    }

    /// Basic config: Process backend, basic policy, 30s timeout.
    #[must_use]
    pub fn basic() -> Self {
        Self::new(Backend::Process, SandboxPolicy::basic(), 30_000)
    }

    /// Strict config: Process backend, strict policy, 10s timeout.
    #[must_use]
    pub fn strict() -> Self {
        Self::new(Backend::Process, SandboxPolicy::strict(), 10_000)
    }

    /// Noop config for testing: no actual isolation.
    #[must_use]
    pub fn noop() -> Self {
        Self::new(Backend::Noop, SandboxPolicy::minimal(), 30_000)
    }

    /// Set environment variables.
    #[must_use]
    pub fn with_env(mut self, env: Vec<(String, String)>) -> Self {
        self.env = env;
        self
    }

    /// Set timeout in milliseconds.
    #[must_use]
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }
}

/// Executor that runs commands inside kavach sandboxes.
pub struct SandboxExecutor {
    default_config: ToolSandboxConfig,
    events: Option<Arc<dyn EventSink>>,
}

impl SandboxExecutor {
    /// Create a new executor with the given default config.
    #[must_use]
    pub fn new(default_config: ToolSandboxConfig) -> Self {
        Self {
            default_config,
            events: None,
        }
    }

    /// Set the event sink for sandbox lifecycle events.
    #[must_use]
    pub fn with_events(mut self, events: Arc<dyn EventSink>) -> Self {
        self.events = Some(events);
        self
    }

    /// Execute a command in a sandbox, piping args as JSON via stdin.
    ///
    /// Returns the parsed JSON result from stdout, or wraps raw output
    /// as a text content block.
    pub fn execute(
        &self,
        tool_name: &str,
        command: &str,
        args: &serde_json::Value,
        config: Option<&ToolSandboxConfig>,
    ) -> crate::Result<serde_json::Value> {
        let cfg = config.unwrap_or(&self.default_config);
        let handle = runtime_handle();

        handle.block_on(self.execute_async(tool_name, command, args, cfg))
    }

    async fn execute_async(
        &self,
        tool_name: &str,
        command: &str,
        args: &serde_json::Value,
        config: &ToolSandboxConfig,
    ) -> crate::Result<serde_json::Value> {
        let sandbox_config = SandboxConfig::builder()
            .backend(config.backend)
            .policy(config.policy.clone())
            .timeout_ms(config.timeout_ms)
            .build();

        let mut sandbox =
            Sandbox::create(sandbox_config)
                .await
                .map_err(|e| BoteError::SandboxError {
                    tool: tool_name.into(),
                    reason: format!("failed to create sandbox: {e}"),
                })?;

        self.publish_event(
            crate::events::TOPIC_SANDBOX_CREATED,
            serde_json::json!({"tool_name": tool_name, "backend": format!("{:?}", config.backend)}),
        );

        sandbox
            .transition(SandboxState::Running)
            .map_err(|e| BoteError::SandboxError {
                tool: tool_name.into(),
                reason: format!("failed to start sandbox: {e}"),
            })?;

        // Pipe JSON args via stdin: echo '<json>' | <command>
        let json_args = serde_json::to_string(args).unwrap_or_default();
        let full_command = format!("echo '{}' | {}", json_args.replace('\'', "'\\''"), command);

        let result: ExecResult =
            sandbox
                .exec(&full_command)
                .await
                .map_err(|e| BoteError::SandboxError {
                    tool: tool_name.into(),
                    reason: format!("sandbox exec failed: {e}"),
                })?;

        sandbox.destroy().await.map_err(|e| {
            tracing::warn!(tool = tool_name, error = %e, "sandbox destroy failed");
            BoteError::SandboxError {
                tool: tool_name.into(),
                reason: format!("sandbox destroy failed: {e}"),
            }
        })?;

        self.publish_event(
            crate::events::TOPIC_SANDBOX_DESTROYED,
            serde_json::json!({
                "tool_name": tool_name,
                "exit_code": result.exit_code,
                "duration_ms": result.duration_ms,
                "timed_out": result.timed_out,
            }),
        );

        if result.timed_out {
            return Err(BoteError::SandboxError {
                tool: tool_name.into(),
                reason: "execution timed out".into(),
            });
        }

        if result.exit_code != 0 {
            let msg = if result.stderr.is_empty() {
                format!("exit code {}", result.exit_code)
            } else {
                result.stderr.trim().to_string()
            };
            return Err(BoteError::SandboxError {
                tool: tool_name.into(),
                reason: msg,
            });
        }

        // Parse stdout as JSON, fallback to text content block.
        let stdout = result.stdout.trim();
        match serde_json::from_str(stdout) {
            Ok(v) => Ok(v),
            Err(_) => Ok(serde_json::json!({
                "content": [{"type": "text", "text": stdout}]
            })),
        }
    }

    fn publish_event(&self, topic: &str, payload: serde_json::Value) {
        if let Some(events) = &self.events {
            events.publish(topic, payload);
        }
    }
}

/// Wrap a command as a sandboxed `ToolHandler`.
///
/// The handler serializes arguments as JSON, pipes them via stdin to the
/// command running inside a kavach sandbox, and parses the JSON stdout
/// as the result.
#[must_use]
pub fn wrap_command(command: impl Into<String>, config: ToolSandboxConfig) -> ToolHandler {
    let command = command.into();
    let executor = Arc::new(SandboxExecutor::new(config));

    Arc::new(move |args: serde_json::Value| -> serde_json::Value {
        match executor.execute("", &command, &args, None) {
            Ok(result) => result,
            Err(e) => {
                tracing::error!(error = %e, "sandboxed tool execution failed");
                serde_json::json!({
                    "content": [{"type": "text", "text": e.to_string()}],
                    "isError": true
                })
            }
        }
    })
}

/// Wrap a command as a sandboxed `StreamingToolHandler`.
///
/// Reports progress during sandbox lifecycle phases, then returns the result.
#[must_use]
pub fn wrap_streaming_command(
    command: impl Into<String>,
    config: ToolSandboxConfig,
) -> StreamingToolHandler {
    let command = command.into();
    let executor = Arc::new(SandboxExecutor::new(config));

    Arc::new(
        move |args: serde_json::Value, ctx: StreamContext| -> serde_json::Value {
            ctx.progress.report_msg(1, 3, "creating sandbox");

            if ctx.cancellation.is_cancelled() {
                return serde_json::json!({"content": [{"type": "text", "text": "cancelled"}], "isError": true});
            }

            ctx.progress.report_msg(2, 3, "executing command");

            let result = executor.execute("", &command, &args, None);

            ctx.progress.report_msg(3, 3, "sandbox complete");

            match result {
                Ok(v) => v,
                Err(e) => {
                    tracing::error!(error = %e, "sandboxed streaming tool execution failed");
                    serde_json::json!({
                        "content": [{"type": "text", "text": e.to_string()}],
                        "isError": true
                    })
                }
            }
        },
    )
}

// --- Dispatcher extension ---

impl Dispatcher {
    /// Register a tool that executes a command inside a kavach sandbox.
    ///
    /// The command receives JSON arguments via stdin and must produce
    /// JSON output on stdout.
    pub fn register_sandboxed_tool(
        &self,
        tool: ToolDef,
        command: impl Into<String>,
        config: ToolSandboxConfig,
    ) -> crate::Result<()> {
        let handler = wrap_command(command, config);
        self.register_tool(tool, handler)
    }

    /// Register a streaming tool that executes a command inside a kavach sandbox.
    pub fn register_sandboxed_streaming_tool(
        &self,
        tool: ToolDef,
        command: impl Into<String>,
        config: ToolSandboxConfig,
    ) -> crate::Result<()> {
        let handler = wrap_streaming_command(command, config);
        self.register_streaming_tool(tool, handler)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{ToolRegistry, ToolSchema};
    use std::collections::HashMap;

    fn noop_tool(name: &str) -> ToolDef {
        ToolDef {
            name: name.into(),
            description: format!("{name} sandboxed tool"),
            input_schema: ToolSchema {
                schema_type: "object".into(),
                properties: HashMap::new(),
                required: vec![],
            },
            version: None,
            deprecated: None,
        }
    }

    // --- ToolSandboxConfig tests ---

    #[test]
    fn config_noop_preset() {
        let cfg = ToolSandboxConfig::noop();
        assert_eq!(cfg.backend, Backend::Noop);
        assert_eq!(cfg.timeout_ms, 30_000);
    }

    #[test]
    fn config_basic_preset() {
        let cfg = ToolSandboxConfig::basic();
        assert_eq!(cfg.backend, Backend::Process);
        assert_eq!(cfg.timeout_ms, 30_000);
    }

    #[test]
    fn config_strict_preset() {
        let cfg = ToolSandboxConfig::strict();
        assert_eq!(cfg.backend, Backend::Process);
        assert_eq!(cfg.timeout_ms, 10_000);
    }

    #[test]
    fn config_with_env() {
        let cfg = ToolSandboxConfig::noop().with_env(vec![("KEY".into(), "val".into())]);
        assert_eq!(cfg.env.len(), 1);
        assert_eq!(cfg.env[0].0, "KEY");
    }

    #[test]
    fn config_with_timeout() {
        let cfg = ToolSandboxConfig::noop().with_timeout(5000);
        assert_eq!(cfg.timeout_ms, 5000);
    }

    // --- SandboxExecutor tests ---

    #[test]
    fn executor_creation() {
        let executor = SandboxExecutor::new(ToolSandboxConfig::noop());
        assert!(executor.events.is_none());
    }

    #[test]
    fn executor_with_events() {
        let executor = SandboxExecutor::new(ToolSandboxConfig::noop()).with_events(Arc::new(()));
        assert!(executor.events.is_some());
    }

    #[test]
    fn executor_execute_noop() {
        let executor = SandboxExecutor::new(ToolSandboxConfig::noop());
        let result = executor.execute(
            "test_tool",
            "echo '{\"ok\": true}'",
            &serde_json::json!({}),
            None,
        );
        // Noop backend returns empty stdout, which becomes a text block.
        assert!(result.is_ok());
    }

    // --- wrap_command tests ---

    #[test]
    fn wrap_command_produces_handler() {
        let handler = wrap_command("echo test", ToolSandboxConfig::noop());
        let result = handler(serde_json::json!({}));
        // Should return some value (noop backend returns empty, wrapped as text).
        assert!(result.is_object());
    }

    #[test]
    fn wrap_streaming_command_produces_handler() {
        let handler = wrap_streaming_command("echo test", ToolSandboxConfig::noop());
        let (ctx, rx) = crate::stream::make_stream_context();
        let result = handler(serde_json::json!({}), ctx);

        // Should have produced progress updates.
        let mut updates = vec![];
        while let Ok(u) = rx.try_recv() {
            updates.push(u);
        }
        assert_eq!(updates.len(), 3);
        assert!(result.is_object());
    }

    // --- Dispatcher integration tests ---

    #[test]
    fn register_sandboxed_tool_dispatches() {
        let reg = ToolRegistry::new();
        let d = Dispatcher::new(reg);

        d.register_sandboxed_tool(
            noop_tool("sandbox_echo"),
            "echo test",
            ToolSandboxConfig::noop(),
        )
        .unwrap();

        let req = crate::protocol::JsonRpcRequest::new(1, "tools/call")
            .with_params(serde_json::json!({"name": "sandbox_echo", "arguments": {}}));
        let resp = d.dispatch(&req).unwrap();
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
    }

    #[test]
    fn register_sandboxed_streaming_tool_dispatches() {
        let reg = ToolRegistry::new();
        let d = Dispatcher::new(reg);

        d.register_sandboxed_streaming_tool(
            noop_tool("sandbox_stream"),
            "echo test",
            ToolSandboxConfig::noop(),
        )
        .unwrap();

        assert!(d.is_streaming_tool("sandbox_stream"));
    }

    // --- Send+Sync assertions ---

    #[test]
    fn sandbox_types_are_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<ToolSandboxConfig>();
        assert_sync::<ToolSandboxConfig>();
        assert_send::<SandboxExecutor>();
        assert_sync::<SandboxExecutor>();
    }
}
