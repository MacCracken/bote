//! Tool registry — register, discover, and validate MCP tools.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::schema::CompiledSchema;

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<String>,
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
            version: None,
            deprecated: None,
        }
    }

    #[must_use]
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    #[must_use]
    pub fn with_deprecated(mut self, message: impl Into<String>) -> Self {
        self.deprecated = Some(message.into());
        self
    }
}

/// A registered tool entry with optional compiled schema.
#[derive(Debug)]
struct ToolEntry {
    def: ToolDef,
    compiled: Option<CompiledSchema>,
}

/// Registry of MCP tools.
#[derive(Debug, Default)]
pub struct ToolRegistry {
    entries: HashMap<String, ToolEntry>,
    versions: HashMap<String, Vec<ToolDef>>,
}

impl ToolRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, tool: ToolDef) {
        tracing::debug!(tool = %tool.name, version = ?tool.version, "tool registered");
        let compiled = match CompiledSchema::compile(&tool.input_schema) {
            Ok(compiled) => Some(compiled),
            Err(e) => {
                tracing::warn!(tool = %tool.name, error = %e, "failed to compile schema, using fallback");
                None
            }
        };
        // Track versioned tools.
        if tool.version.is_some() {
            self.versions
                .entry(tool.name.clone())
                .or_default()
                .push(tool.clone());
        }
        self.entries.insert(
            tool.name.clone(),
            ToolEntry {
                def: tool,
                compiled,
            },
        );
    }

    #[inline]
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&ToolDef> {
        self.entries.get(name).map(|e| &e.def)
    }

    #[must_use]
    pub fn list(&self) -> Vec<&ToolDef> {
        self.entries.values().map(|e| &e.def).collect()
    }

    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[inline]
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.entries.contains_key(name)
    }

    /// Look up a specific version of a tool.
    #[must_use]
    pub fn get_versioned(&self, name: &str, version: &str) -> Option<&ToolDef> {
        self.versions
            .get(name)?
            .iter()
            .find(|t| t.version.as_deref() == Some(version))
    }

    /// List all registered versions of a tool.
    #[must_use]
    pub fn list_versions(&self, name: &str) -> Vec<&ToolDef> {
        self.versions
            .get(name)
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }

    /// Remove a tool from the registry.
    pub fn deregister(&mut self, name: &str) -> Option<ToolDef> {
        self.versions.remove(name);
        let removed = self.entries.remove(name).map(|e| e.def);
        if removed.is_some() {
            tracing::debug!(tool = name, "tool deregistered");
        }
        removed
    }

    /// Mark a tool as deprecated with a message.
    pub fn deprecate(&mut self, name: &str, message: impl Into<String>) {
        if let Some(entry) = self.entries.get_mut(name) {
            let msg = message.into();
            tracing::info!(tool = name, message = %msg, "tool deprecated");
            entry.def.deprecated = Some(msg);
        }
    }

    /// Validate params against a tool's compiled schema.
    ///
    /// Performs full type checking, enum validation, and bounds checking
    /// when property schemas are defined. Falls back to required-field-only
    /// validation when no compiled schema is available.
    pub fn validate_params(
        &self,
        tool_name: &str,
        params: &serde_json::Value,
    ) -> crate::Result<()> {
        let entry = self
            .entries
            .get(tool_name)
            .ok_or_else(|| crate::BoteError::ToolNotFound(tool_name.into()))?;

        // Use compiled schema if available.
        if let Some(compiled) = &entry.compiled {
            if let Err(violations) = compiled.validate(params) {
                return Err(crate::BoteError::SchemaViolation {
                    tool: tool_name.into(),
                    violations,
                });
            }
            return Ok(());
        }

        // Fallback: basic required-field check.
        let map = match params {
            serde_json::Value::Object(map) => map,
            _ => {
                return Err(crate::BoteError::InvalidParams {
                    tool: tool_name.into(),
                    reason: "params must be an object".into(),
                });
            }
        };

        for req in &entry.def.input_schema.required {
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
            version: None,
            deprecated: None,
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
            version: None,
            deprecated: None,
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
            version: None,
            deprecated: None,
        });
        assert!(reg.validate_params("open", &serde_json::json!({})).is_ok());
    }
}
