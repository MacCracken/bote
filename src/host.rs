//! MCP hosting layer — types and registry for hosting MCP tools in a service.
//!
//! This module provides the server-side types for exposing tools via MCP.
//! A host (e.g. daimon) registers built-in and external tools, builds
//! manifests for discovery, and dispatches tool calls.
//!
//! ## Types
//!
//! - [`McpToolDescription`] — tool name, description, and input schema for discovery
//! - [`McpToolManifest`] — complete tool listing
//! - [`McpToolCall`] — incoming tool invocation request
//! - [`McpToolResult`] — tool execution result with content blocks
//! - [`ExternalMcpTool`] — externally registered tool with callback URL
//! - [`RegisterMcpToolRequest`] — request to register an external tool
//! - [`McpHostRegistry`] — registry for built-in + external tools

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tracing::{debug, info};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Description of a single MCP tool (schema for discovery).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct McpToolDescription {
    /// Tool name (unique identifier).
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// JSON Schema for the tool's input parameters.
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
}

impl McpToolDescription {
    /// Create a new tool description.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: serde_json::Value,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema,
        }
    }
}

/// Complete tool manifest returned by the discovery endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct McpToolManifest {
    /// All available tools.
    pub tools: Vec<McpToolDescription>,
}

/// A request to call a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct McpToolCall {
    /// Tool name to invoke.
    pub name: String,
    /// Arguments to pass.
    #[serde(default)]
    pub arguments: serde_json::Value,
}

impl McpToolCall {
    /// Create a new tool call.
    #[must_use]
    pub fn new(name: impl Into<String>, arguments: serde_json::Value) -> Self {
        Self {
            name: name.into(),
            arguments,
        }
    }
}

/// A content block in a tool result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct McpContentBlock {
    /// Content type (e.g. "text/plain").
    #[serde(rename = "type")]
    pub content_type: String,
    /// Text content.
    pub text: String,
}

/// Result of a tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct McpToolResult {
    /// Content blocks.
    pub content: Vec<McpContentBlock>,
    /// Whether this result represents an error.
    #[serde(rename = "isError")]
    pub is_error: bool,
}

impl McpToolResult {
    /// Create a success result with a single text block.
    #[must_use]
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            content: vec![McpContentBlock {
                content_type: "text/plain".into(),
                text: text.into(),
            }],
            is_error: false,
        }
    }

    /// Create an error result.
    #[must_use]
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            content: vec![McpContentBlock {
                content_type: "text/plain".into(),
                text: message.into(),
            }],
            is_error: true,
        }
    }

    /// Create a success result with JSON content.
    #[must_use]
    pub fn json(value: &serde_json::Value) -> Self {
        Self {
            content: vec![McpContentBlock {
                content_type: "text/plain".into(),
                text: serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".into()),
            }],
            is_error: false,
        }
    }
}

impl RegisterMcpToolRequest {
    /// Create a new registration request.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: serde_json::Value,
        callback_url: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema,
            callback_url: callback_url.into(),
            source: None,
        }
    }

    /// Set the source identifier.
    #[must_use]
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }
}

/// An externally registered MCP tool with a callback URL for dispatch.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ExternalMcpTool {
    /// Tool definition (name, description, input_schema).
    pub tool: McpToolDescription,
    /// HTTP endpoint to POST tool calls to.
    pub callback_url: String,
    /// Source service that registered this tool.
    pub source: String,
}

/// Request to register an external tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RegisterMcpToolRequest {
    /// Tool name.
    pub name: String,
    /// Description.
    pub description: String,
    /// JSON Schema for input.
    pub input_schema: serde_json::Value,
    /// Callback URL for tool execution.
    pub callback_url: String,
    /// Optional source identifier.
    pub source: Option<String>,
}

// ---------------------------------------------------------------------------
// SSRF validation
// ---------------------------------------------------------------------------

/// Validate that a callback URL is safe from SSRF attacks.
///
/// Rejects: private IPs, non-http(s) schemes, credentials in URL,
/// and link-local addresses.
pub fn validate_callback_url(url: &str) -> std::result::Result<(), String> {
    // Must parse as a URL
    let parsed = url::Url::parse(url).map_err(|e| format!("invalid URL: {e}"))?;

    // Scheme must be http or https
    match parsed.scheme() {
        "http" | "https" => {}
        other => return Err(format!("unsupported scheme: {other}")),
    }

    // No credentials in URL
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err("credentials in URL not allowed".into());
    }

    // Must have a host
    let host = parsed
        .host_str()
        .ok_or_else(|| "missing host".to_string())?;

    // Block private/loopback ranges (except localhost for local dev)
    if host == "0.0.0.0" || host == "[::]" || host.starts_with("169.254.") {
        return Err(format!("blocked host: {host}"));
    }

    // Parse as IP and reject private ranges
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        match ip {
            std::net::IpAddr::V4(v4) => {
                if v4.is_link_local()
                    || v4.is_broadcast()
                    || (v4.octets()[0] == 10)
                    || (v4.octets()[0] == 172
                        && (16..=31).contains(&v4.octets()[1]))
                    || (v4.octets()[0] == 192 && v4.octets()[1] == 168)
                {
                    // Allow localhost (127.x) for local development
                    if !v4.is_loopback() {
                        return Err(format!("private IP not allowed: {v4}"));
                    }
                }
            }
            std::net::IpAddr::V6(v6) => {
                if v6.is_loopback() {
                    // Allow ::1 for local dev
                } else if v6.segments()[0] & 0xfe00 == 0xfc00 {
                    return Err(format!("private IPv6 not allowed: {v6}"));
                }
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// McpHostRegistry
// ---------------------------------------------------------------------------

/// Registry for built-in and external MCP tools.
///
/// The host registry manages two sets of tools:
/// - **Built-in**: tools registered at startup by the host application
/// - **External**: tools registered dynamically via the API
///
/// Built-in tools take precedence over external tools with the same name.
pub struct McpHostRegistry {
    builtin: HashMap<String, McpToolDescription>,
    external: HashMap<String, ExternalMcpTool>,
}

impl McpHostRegistry {
    /// Create a new empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            builtin: HashMap::new(),
            external: HashMap::new(),
        }
    }

    /// Register a built-in tool.
    pub fn register_builtin(&mut self, tool: McpToolDescription) {
        debug!(name = %tool.name, "registered built-in MCP tool");
        self.builtin.insert(tool.name.clone(), tool);
    }

    /// Register an external tool from a request.
    ///
    /// Validates that the name and callback URL are non-empty.
    /// Optionally validates the callback URL against SSRF rules.
    pub fn register_external(
        &mut self,
        req: RegisterMcpToolRequest,
        validate_ssrf: bool,
    ) -> std::result::Result<(), String> {
        if req.name.is_empty() {
            return Err("tool name cannot be empty".into());
        }
        if req.callback_url.is_empty() {
            return Err("callback URL cannot be empty".into());
        }

        // Reject names that collide with built-in tools
        if self.builtin.contains_key(&req.name) {
            return Err(format!(
                "tool '{}' conflicts with a built-in tool",
                req.name
            ));
        }

        if validate_ssrf {
            validate_callback_url(&req.callback_url)?;
        }

        let tool = McpToolDescription {
            name: req.name.clone(),
            description: req.description,
            input_schema: req.input_schema,
        };

        let external = ExternalMcpTool {
            tool,
            callback_url: req.callback_url,
            source: req.source.unwrap_or_else(|| "unknown".into()),
        };

        info!(name = %req.name, "registered external MCP tool");
        self.external.insert(req.name, external);
        Ok(())
    }

    /// Deregister an external tool by name.
    pub fn deregister(&mut self, name: &str) -> std::result::Result<(), String> {
        if self.external.remove(name).is_none() {
            return Err(format!("external tool not found: {name}"));
        }
        info!(name = %name, "deregistered external MCP tool");
        Ok(())
    }

    /// Build the complete tool manifest (built-in + external), sorted by name.
    #[must_use]
    pub fn manifest(&self) -> McpToolManifest {
        let mut tools: Vec<McpToolDescription> = self.builtin.values().cloned().collect();
        tools.extend(self.external.values().map(|e| e.tool.clone()));
        tools.sort_by(|a, b| a.name.cmp(&b.name));
        McpToolManifest { tools }
    }

    /// Look up a tool by name (built-in first, then external).
    #[must_use]
    pub fn find_tool(&self, name: &str) -> Option<&McpToolDescription> {
        self.builtin
            .get(name)
            .or_else(|| self.external.get(name).map(|e| &e.tool))
    }

    /// Get the external tool entry (includes callback URL).
    #[must_use]
    pub fn get_external(&self, name: &str) -> Option<&ExternalMcpTool> {
        self.external.get(name)
    }

    /// Get callback URL for an external tool.
    #[must_use]
    pub fn external_callback(&self, name: &str) -> Option<&str> {
        self.external.get(name).map(|e| e.callback_url.as_str())
    }

    /// Number of registered tools (built-in + external).
    #[must_use]
    pub fn tool_count(&self) -> usize {
        self.builtin.len() + self.external.len()
    }

    /// Number of built-in tools.
    #[must_use]
    pub fn builtin_count(&self) -> usize {
        self.builtin.len()
    }

    /// Number of external tools.
    #[must_use]
    pub fn external_count(&self) -> usize {
        self.external.len()
    }

    /// Check if a tool name exists (built-in or external).
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.builtin.contains_key(name) || self.external.contains_key(name)
    }
}

impl Default for McpHostRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn test_tool(name: &str) -> McpToolDescription {
        McpToolDescription {
            name: name.into(),
            description: format!("Test tool: {name}"),
            input_schema: json!({"type": "object"}),
        }
    }

    fn test_register_req(name: &str) -> RegisterMcpToolRequest {
        RegisterMcpToolRequest {
            name: name.into(),
            description: format!("External: {name}"),
            input_schema: json!({"type": "object"}),
            callback_url: "http://localhost:9000/callback".into(),
            source: Some("test".into()),
        }
    }

    // -- McpToolResult --

    #[test]
    fn tool_result_text() {
        let r = McpToolResult::text("hello");
        assert!(!r.is_error);
        assert_eq!(r.content.len(), 1);
        assert_eq!(r.content[0].text, "hello");
        assert_eq!(r.content[0].content_type, "text/plain");
    }

    #[test]
    fn tool_result_error() {
        let r = McpToolResult::error("boom");
        assert!(r.is_error);
        assert_eq!(r.content[0].text, "boom");
    }

    #[test]
    fn tool_result_json() {
        let val = json!({"status": "ok"});
        let r = McpToolResult::json(&val);
        assert!(!r.is_error);
        assert!(r.content[0].text.contains("ok"));
    }

    // -- McpHostRegistry --

    #[test]
    fn register_builtin() {
        let mut reg = McpHostRegistry::new();
        reg.register_builtin(test_tool("scan"));
        assert_eq!(reg.tool_count(), 1);
        assert_eq!(reg.builtin_count(), 1);
        assert!(reg.find_tool("scan").is_some());
    }

    #[test]
    fn register_external() {
        let mut reg = McpHostRegistry::new();
        reg.register_external(test_register_req("custom"), false)
            .unwrap();
        assert_eq!(reg.tool_count(), 1);
        assert_eq!(reg.external_count(), 1);
        assert!(reg.find_tool("custom").is_some());
        assert!(reg.external_callback("custom").is_some());
    }

    #[test]
    fn register_external_empty_name_rejected() {
        let mut reg = McpHostRegistry::new();
        let mut req = test_register_req("x");
        req.name = String::new();
        assert!(reg.register_external(req, false).is_err());
    }

    #[test]
    fn register_external_empty_url_rejected() {
        let mut reg = McpHostRegistry::new();
        let mut req = test_register_req("x");
        req.callback_url = String::new();
        assert!(reg.register_external(req, false).is_err());
    }

    #[test]
    fn register_external_conflict_with_builtin() {
        let mut reg = McpHostRegistry::new();
        reg.register_builtin(test_tool("overlap"));
        assert!(reg
            .register_external(test_register_req("overlap"), false)
            .is_err());
    }

    #[test]
    fn deregister_external() {
        let mut reg = McpHostRegistry::new();
        reg.register_external(test_register_req("temp"), false)
            .unwrap();
        assert!(reg.deregister("temp").is_ok());
        assert_eq!(reg.tool_count(), 0);
    }

    #[test]
    fn deregister_nonexistent() {
        let mut reg = McpHostRegistry::new();
        assert!(reg.deregister("nope").is_err());
    }

    #[test]
    fn manifest_sorted() {
        let mut reg = McpHostRegistry::new();
        reg.register_builtin(test_tool("zebra"));
        reg.register_builtin(test_tool("alpha"));
        reg.register_external(test_register_req("middle"), false)
            .unwrap();

        let manifest = reg.manifest();
        let names: Vec<&str> = manifest.tools.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "middle", "zebra"]);
    }

    #[test]
    fn find_prefers_builtin() {
        let mut reg = McpHostRegistry::new();
        reg.register_builtin(test_tool("scan"));
        // External with same name is blocked
        assert!(reg
            .register_external(test_register_req("scan"), false)
            .is_err());
    }

    #[test]
    fn contains_check() {
        let mut reg = McpHostRegistry::new();
        assert!(!reg.contains("test"));
        reg.register_builtin(test_tool("test"));
        assert!(reg.contains("test"));
    }

    // -- Serde roundtrips --

    #[test]
    fn tool_call_serde_roundtrip() {
        let call = McpToolCall {
            name: "scan".into(),
            arguments: json!({"target": "localhost"}),
        };
        let json_str = serde_json::to_string(&call).unwrap();
        let back: McpToolCall = serde_json::from_str(&json_str).unwrap();
        assert_eq!(back.name, "scan");
    }

    #[test]
    fn tool_result_serde_roundtrip() {
        let result = McpToolResult::text("ok");
        let json_str = serde_json::to_string(&result).unwrap();
        let back: McpToolResult = serde_json::from_str(&json_str).unwrap();
        assert!(!back.is_error);
    }

    #[test]
    fn manifest_serde_roundtrip() {
        let mut reg = McpHostRegistry::new();
        reg.register_builtin(test_tool("t1"));
        let manifest = reg.manifest();
        let json_str = serde_json::to_string(&manifest).unwrap();
        let back: McpToolManifest = serde_json::from_str(&json_str).unwrap();
        assert_eq!(back.tools.len(), 1);
    }

    #[test]
    fn external_tool_serde_roundtrip() {
        let ext = ExternalMcpTool {
            tool: test_tool("ext"),
            callback_url: "http://example.com".into(),
            source: "test".into(),
        };
        let json_str = serde_json::to_string(&ext).unwrap();
        let back: ExternalMcpTool = serde_json::from_str(&json_str).unwrap();
        assert_eq!(back.callback_url, "http://example.com");
    }

    // -- SSRF validation --

    #[test]
    fn ssrf_allows_localhost() {
        assert!(validate_callback_url("http://127.0.0.1:9000/cb").is_ok());
        assert!(validate_callback_url("http://localhost:9000/cb").is_ok());
    }

    #[test]
    fn ssrf_allows_public_https() {
        assert!(validate_callback_url("https://api.example.com/tool").is_ok());
    }

    #[test]
    fn ssrf_blocks_private_ips() {
        assert!(validate_callback_url("http://10.0.0.1/cb").is_err());
        assert!(validate_callback_url("http://192.168.1.1/cb").is_err());
        assert!(validate_callback_url("http://172.16.0.1/cb").is_err());
    }

    #[test]
    fn ssrf_blocks_link_local() {
        assert!(validate_callback_url("http://169.254.1.1/cb").is_err());
    }

    #[test]
    fn ssrf_blocks_bad_scheme() {
        assert!(validate_callback_url("ftp://example.com/cb").is_err());
        assert!(validate_callback_url("file:///etc/passwd").is_err());
    }

    #[test]
    fn ssrf_blocks_credentials() {
        assert!(validate_callback_url("http://user:pass@example.com/cb").is_err());
    }

    #[test]
    fn ssrf_blocks_zero_addr() {
        assert!(validate_callback_url("http://0.0.0.0/cb").is_err());
    }

    #[test]
    fn ssrf_with_validation_flag() {
        let mut reg = McpHostRegistry::new();
        let mut req = test_register_req("bad");
        req.callback_url = "http://10.0.0.1/cb".into();
        // Without SSRF validation — passes
        assert!(reg.register_external(req.clone(), false).is_ok());
        reg.deregister("bad").unwrap();
        // With SSRF validation — rejected
        assert!(reg.register_external(req, true).is_err());
    }
}
