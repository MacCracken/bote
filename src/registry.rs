//! Tool registry — register, discover, and validate MCP tools.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Tool input schema (JSON Schema subset).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ToolSchema {
    #[serde(rename = "type")]
    pub schema_type: String,
    #[serde(default)]
    pub properties: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub required: Vec<String>,
}

/// Definition of a registered MCP tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub input_schema: ToolSchema,
}

impl ToolSchema {
    #[must_use]
    pub fn new(
        schema_type: impl Into<String>,
        properties: HashMap<String, serde_json::Value>,
        required: Vec<String>,
    ) -> Self {
        Self {
            schema_type: schema_type.into(),
            properties,
            required,
        }
    }
}

impl ToolDef {
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: ToolSchema,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema,
        }
    }
}

/// Registry of MCP tools.
#[derive(Debug, Default)]
pub struct ToolRegistry {
    tools: HashMap<String, ToolDef>,
}

impl ToolRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, tool: ToolDef) {
        tracing::debug!(tool = %tool.name, "tool registered");
        self.tools.insert(tool.name.clone(), tool);
    }

    #[inline]
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&ToolDef> {
        self.tools.get(name)
    }

    #[must_use]
    pub fn list(&self) -> Vec<&ToolDef> {
        self.tools.values().collect()
    }

    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.tools.len()
    }
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    #[inline]
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Validate params against a tool's schema (basic required-field check).
    pub fn validate_params(
        &self,
        tool_name: &str,
        params: &serde_json::Value,
    ) -> crate::Result<()> {
        let tool = self
            .get(tool_name)
            .ok_or_else(|| crate::BoteError::ToolNotFound(tool_name.into()))?;

        let map = match params {
            serde_json::Value::Object(map) => map,
            _ => {
                return Err(crate::BoteError::InvalidParams {
                    tool: tool_name.into(),
                    reason: "params must be an object".into(),
                });
            }
        };

        for req in &tool.input_schema.required {
            if !map.contains_key(req) {
                return Err(crate::BoteError::InvalidParams {
                    tool: tool_name.into(),
                    reason: format!("missing required field: {req}"),
                });
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tool(name: &str) -> ToolDef {
        ToolDef {
            name: name.into(),
            description: format!("{name} tool"),
            input_schema: ToolSchema {
                schema_type: "object".into(),
                properties: HashMap::new(),
                required: vec!["path".into()],
            },
        }
    }

    #[test]
    fn register_and_get() {
        let mut reg = ToolRegistry::new();
        reg.register(make_tool("test_tool"));
        assert!(reg.contains("test_tool"));
        assert!(!reg.contains("nope"));
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn list_tools() {
        let mut reg = ToolRegistry::new();
        reg.register(make_tool("a"));
        reg.register(make_tool("b"));
        assert_eq!(reg.list().len(), 2);
    }

    #[test]
    fn validate_params_ok() {
        let mut reg = ToolRegistry::new();
        reg.register(make_tool("scan"));
        let params = serde_json::json!({"path": "/tmp"});
        assert!(reg.validate_params("scan", &params).is_ok());
    }

    #[test]
    fn validate_params_missing() {
        let mut reg = ToolRegistry::new();
        reg.register(make_tool("scan"));
        let params = serde_json::json!({});
        assert!(reg.validate_params("scan", &params).is_err());
    }

    #[test]
    fn validate_unknown_tool() {
        let reg = ToolRegistry::new();
        assert!(reg.validate_params("nope", &serde_json::json!({})).is_err());
    }

    #[test]
    fn validate_rejects_non_object_params() {
        let mut reg = ToolRegistry::new();
        reg.register(make_tool("scan"));
        assert!(
            reg.validate_params("scan", &serde_json::json!(null))
                .is_err()
        );
        assert!(
            reg.validate_params("scan", &serde_json::json!("string"))
                .is_err()
        );
        assert!(
            reg.validate_params("scan", &serde_json::json!([1, 2]))
                .is_err()
        );
        assert!(reg.validate_params("scan", &serde_json::json!(42)).is_err());
        assert!(
            reg.validate_params("scan", &serde_json::json!(true))
                .is_err()
        );
    }

    #[test]
    fn empty_registry() {
        let reg = ToolRegistry::new();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
        assert!(reg.list().is_empty());
        assert!(reg.get("anything").is_none());
    }

    #[test]
    fn get_returns_correct_tool() {
        let mut reg = ToolRegistry::new();
        reg.register(make_tool("alpha"));
        reg.register(make_tool("beta"));
        let tool = reg.get("alpha").unwrap();
        assert_eq!(tool.name, "alpha");
        assert_eq!(tool.description, "alpha tool");
    }

    #[test]
    fn register_overwrites_duplicate() {
        let mut reg = ToolRegistry::new();
        reg.register(make_tool("dup"));
        reg.register(ToolDef {
            name: "dup".into(),
            description: "updated".into(),
            input_schema: ToolSchema {
                schema_type: "object".into(),
                properties: HashMap::new(),
                required: vec![],
            },
        });
        assert_eq!(reg.len(), 1);
        assert_eq!(reg.get("dup").unwrap().description, "updated");
        // Overwrite also removed the required field
        assert!(reg.validate_params("dup", &serde_json::json!({})).is_ok());
    }

    #[test]
    fn validate_passes_with_no_required_fields() {
        let mut reg = ToolRegistry::new();
        reg.register(ToolDef {
            name: "open".into(),
            description: "no required".into(),
            input_schema: ToolSchema {
                schema_type: "object".into(),
                properties: HashMap::new(),
                required: vec![],
            },
        });
        assert!(reg.validate_params("open", &serde_json::json!({})).is_ok());
    }
}
