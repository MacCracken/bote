//! JSON Schema validation — compile `ToolSchema` into a typed representation
//! and validate parameters against it.
//!
//! Supports type checking (string, number, integer, boolean, array, object),
//! enum constraints, numeric bounds, and default value injection.

use std::collections::HashMap;

use crate::registry::ToolSchema;

/// Property type with constraints.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum SchemaType {
    String {
        enum_values: Option<Vec<String>>,
        default: Option<String>,
    },
    Number {
        minimum: Option<f64>,
        maximum: Option<f64>,
        default: Option<f64>,
    },
    Integer {
        minimum: Option<i64>,
        maximum: Option<i64>,
        default: Option<i64>,
    },
    Boolean {
        default: Option<bool>,
    },
    Array {
        items: Option<Box<PropertyDef>>,
    },
    Object {
        properties: HashMap<String, PropertyDef>,
        required: Vec<String>,
    },
    /// Fallback for unrecognized schemas — accepts any value.
    Any,
}

/// A property definition with type and optional description.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct PropertyDef {
    pub schema_type: SchemaType,
    pub description: Option<String>,
}

impl PropertyDef {
    #[must_use]
    pub fn new(schema_type: SchemaType) -> Self {
        Self {
            schema_type,
            description: None,
        }
    }

    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// A compiled schema for fast validation.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct CompiledSchema {
    pub properties: HashMap<String, PropertyDef>,
    pub required: Vec<String>,
}

impl CompiledSchema {
    /// Compile a `ToolSchema` into a `CompiledSchema`.
    ///
    /// Properties that cannot be parsed fall back to `SchemaType::Any`.
    pub fn compile(schema: &ToolSchema) -> crate::Result<Self> {
        let mut properties = HashMap::with_capacity(schema.properties.len());

        for (name, value) in &schema.properties {
            let prop = parse_property(name, value);
            properties.insert(name.clone(), prop);
        }

        Ok(Self {
            properties,
            required: schema.required.clone(),
        })
    }

    /// Validate parameters against this schema.
    ///
    /// Collects all violations rather than failing on the first.
    pub fn validate(&self, params: &serde_json::Value) -> std::result::Result<(), Vec<String>> {
        let map = match params.as_object() {
            Some(m) => m,
            None => return Err(vec!["params must be an object".into()]),
        };

        let mut violations = Vec::new();

        // Check required fields.
        for req in &self.required {
            if !map.contains_key(req) {
                violations.push(format!("missing required field: {req}"));
            }
        }

        // Type-check provided fields.
        for (name, value) in map {
            if let Some(prop) = self.properties.get(name) {
                validate_value(name, value, &prop.schema_type, &mut violations);
            }
            // Extra fields without schema are allowed (permissive).
        }

        if violations.is_empty() {
            Ok(())
        } else {
            Err(violations)
        }
    }

    /// Inject default values for missing optional fields.
    pub fn apply_defaults(&self, params: &mut serde_json::Value) {
        let map = match params.as_object_mut() {
            Some(m) => m,
            None => return,
        };

        for (name, prop) in &self.properties {
            if map.contains_key(name) {
                continue;
            }
            if let Some(default) = default_value(&prop.schema_type) {
                map.insert(name.clone(), default);
            }
        }
    }
}

/// Parse a JSON Schema property value into a `PropertyDef`.
fn parse_property(name: &str, value: &serde_json::Value) -> PropertyDef {
    let description = value
        .get("description")
        .and_then(|v| v.as_str())
        .map(String::from);

    let schema_type = match value.get("type").and_then(|v| v.as_str()) {
        Some("string") => SchemaType::String {
            enum_values: value.get("enum").and_then(|v| {
                v.as_array().map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
            }),
            default: value
                .get("default")
                .and_then(|v| v.as_str())
                .map(String::from),
        },
        Some("number") => SchemaType::Number {
            minimum: value.get("minimum").and_then(|v| v.as_f64()),
            maximum: value.get("maximum").and_then(|v| v.as_f64()),
            default: value.get("default").and_then(|v| v.as_f64()),
        },
        Some("integer") => SchemaType::Integer {
            minimum: value.get("minimum").and_then(|v| v.as_i64()),
            maximum: value.get("maximum").and_then(|v| v.as_i64()),
            default: value.get("default").and_then(|v| v.as_i64()),
        },
        Some("boolean") => SchemaType::Boolean {
            default: value.get("default").and_then(|v| v.as_bool()),
        },
        Some("array") => SchemaType::Array {
            items: value
                .get("items")
                .map(|v| Box::new(parse_property(&format!("{name}[]"), v))),
        },
        Some("object") => {
            let props = value
                .get("properties")
                .and_then(|v| v.as_object())
                .map(|obj| {
                    obj.iter()
                        .map(|(k, v)| (k.clone(), parse_property(k, v)))
                        .collect()
                })
                .unwrap_or_default();
            let required = value
                .get("required")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            SchemaType::Object {
                properties: props,
                required,
            }
        }
        Some(other) => {
            tracing::warn!(
                field = name,
                schema_type = other,
                "unknown schema type, using Any"
            );
            SchemaType::Any
        }
        None => SchemaType::Any,
    };

    PropertyDef {
        schema_type,
        description,
    }
}

/// Validate a value against a schema type.
fn validate_value(
    path: &str,
    value: &serde_json::Value,
    schema_type: &SchemaType,
    violations: &mut Vec<String>,
) {
    match schema_type {
        SchemaType::String {
            enum_values,
            default: _,
        } => {
            if let Some(s) = value.as_str() {
                if let Some(allowed) = enum_values
                    && !allowed.iter().any(|a| a == s)
                {
                    violations.push(format!(
                        "{path}: value '{s}' not in enum [{}]",
                        allowed.join(", ")
                    ));
                }
            } else {
                violations.push(format!("{path}: expected string"));
            }
        }
        SchemaType::Number {
            minimum,
            maximum,
            default: _,
        } => {
            if let Some(n) = value.as_f64() {
                if let Some(min) = minimum
                    && n < *min
                {
                    violations.push(format!("{path}: {n} is less than minimum {min}"));
                }
                if let Some(max) = maximum
                    && n > *max
                {
                    violations.push(format!("{path}: {n} is greater than maximum {max}"));
                }
            } else {
                violations.push(format!("{path}: expected number"));
            }
        }
        SchemaType::Integer {
            minimum,
            maximum,
            default: _,
        } => {
            if let Some(n) = value.as_i64() {
                if let Some(min) = minimum
                    && n < *min
                {
                    violations.push(format!("{path}: {n} is less than minimum {min}"));
                }
                if let Some(max) = maximum
                    && n > *max
                {
                    violations.push(format!("{path}: {n} is greater than maximum {max}"));
                }
            } else {
                violations.push(format!("{path}: expected integer"));
            }
        }
        SchemaType::Boolean { default: _ } => {
            if !value.is_boolean() {
                violations.push(format!("{path}: expected boolean"));
            }
        }
        SchemaType::Array { items } => {
            if let Some(arr) = value.as_array() {
                if let Some(item_schema) = items {
                    for (i, item) in arr.iter().enumerate() {
                        validate_value(
                            &format!("{path}[{i}]"),
                            item,
                            &item_schema.schema_type,
                            violations,
                        );
                    }
                }
            } else {
                violations.push(format!("{path}: expected array"));
            }
        }
        SchemaType::Object {
            properties,
            required,
        } => {
            if let Some(obj) = value.as_object() {
                for req in required {
                    if !obj.contains_key(req) {
                        violations.push(format!("{path}.{req}: missing required field"));
                    }
                }
                for (key, val) in obj {
                    if let Some(prop) = properties.get(key) {
                        validate_value(
                            &format!("{path}.{key}"),
                            val,
                            &prop.schema_type,
                            violations,
                        );
                    }
                }
            } else {
                violations.push(format!("{path}: expected object"));
            }
        }
        SchemaType::Any => {}
    }
}

/// Extract the default value from a schema type, if any.
fn default_value(schema_type: &SchemaType) -> Option<serde_json::Value> {
    match schema_type {
        SchemaType::String { default, .. } => default.as_ref().map(|v| serde_json::json!(v)),
        SchemaType::Number { default, .. } => default.map(|v| serde_json::json!(v)),
        SchemaType::Integer { default, .. } => default.map(|v| serde_json::json!(v)),
        SchemaType::Boolean { default } => default.map(|v| serde_json::json!(v)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema_with_properties(props: serde_json::Value) -> ToolSchema {
        let properties: HashMap<String, serde_json::Value> = props
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        ToolSchema {
            schema_type: "object".into(),
            properties,
            required: vec![],
        }
    }

    // --- Type checking ---

    #[test]
    fn validate_string_type() {
        let schema = schema_with_properties(serde_json::json!({
            "name": {"type": "string"}
        }));
        let compiled = CompiledSchema::compile(&schema).unwrap();

        assert!(
            compiled
                .validate(&serde_json::json!({"name": "alice"}))
                .is_ok()
        );
        assert!(compiled.validate(&serde_json::json!({"name": 42})).is_err());
    }

    #[test]
    fn validate_number_type() {
        let schema = schema_with_properties(serde_json::json!({
            "score": {"type": "number"}
        }));
        let compiled = CompiledSchema::compile(&schema).unwrap();

        assert!(
            compiled
                .validate(&serde_json::json!({"score": 3.15}))
                .is_ok()
        );
        assert!(compiled.validate(&serde_json::json!({"score": 42})).is_ok());
        assert!(
            compiled
                .validate(&serde_json::json!({"score": "abc"}))
                .is_err()
        );
    }

    #[test]
    fn validate_integer_type() {
        let schema = schema_with_properties(serde_json::json!({
            "count": {"type": "integer"}
        }));
        let compiled = CompiledSchema::compile(&schema).unwrap();

        assert!(compiled.validate(&serde_json::json!({"count": 42})).is_ok());
        assert!(
            compiled
                .validate(&serde_json::json!({"count": 3.15}))
                .is_err()
        );
    }

    #[test]
    fn validate_boolean_type() {
        let schema = schema_with_properties(serde_json::json!({
            "flag": {"type": "boolean"}
        }));
        let compiled = CompiledSchema::compile(&schema).unwrap();

        assert!(
            compiled
                .validate(&serde_json::json!({"flag": true}))
                .is_ok()
        );
        assert!(
            compiled
                .validate(&serde_json::json!({"flag": "yes"}))
                .is_err()
        );
    }

    #[test]
    fn validate_array_type() {
        let schema = schema_with_properties(serde_json::json!({
            "tags": {"type": "array", "items": {"type": "string"}}
        }));
        let compiled = CompiledSchema::compile(&schema).unwrap();

        assert!(
            compiled
                .validate(&serde_json::json!({"tags": ["a", "b"]}))
                .is_ok()
        );
        assert!(
            compiled
                .validate(&serde_json::json!({"tags": [1, 2]}))
                .is_err()
        );
        assert!(
            compiled
                .validate(&serde_json::json!({"tags": "not array"}))
                .is_err()
        );
    }

    #[test]
    fn validate_nested_object() {
        let schema = schema_with_properties(serde_json::json!({
            "config": {
                "type": "object",
                "properties": {
                    "host": {"type": "string"},
                    "port": {"type": "integer"}
                },
                "required": ["host"]
            }
        }));
        let compiled = CompiledSchema::compile(&schema).unwrap();

        assert!(
            compiled
                .validate(&serde_json::json!({"config": {"host": "localhost", "port": 8080}}))
                .is_ok()
        );
        assert!(
            compiled
                .validate(&serde_json::json!({"config": {"port": 8080}}))
                .is_err()
        ); // missing required host
        assert!(
            compiled
                .validate(&serde_json::json!({"config": {"host": 42}}))
                .is_err()
        ); // wrong type
    }

    // --- Enum constraints ---

    #[test]
    fn validate_string_enum() {
        let schema = schema_with_properties(serde_json::json!({
            "mode": {"type": "string", "enum": ["read", "write", "append"]}
        }));
        let compiled = CompiledSchema::compile(&schema).unwrap();

        assert!(
            compiled
                .validate(&serde_json::json!({"mode": "read"}))
                .is_ok()
        );
        assert!(
            compiled
                .validate(&serde_json::json!({"mode": "delete"}))
                .is_err()
        );
    }

    // --- Numeric bounds ---

    #[test]
    fn validate_number_bounds() {
        let schema = schema_with_properties(serde_json::json!({
            "age": {"type": "number", "minimum": 0, "maximum": 150}
        }));
        let compiled = CompiledSchema::compile(&schema).unwrap();

        assert!(compiled.validate(&serde_json::json!({"age": 25})).is_ok());
        assert!(compiled.validate(&serde_json::json!({"age": -1})).is_err());
        assert!(compiled.validate(&serde_json::json!({"age": 200})).is_err());
    }

    #[test]
    fn validate_integer_bounds() {
        let schema = schema_with_properties(serde_json::json!({
            "retries": {"type": "integer", "minimum": 0, "maximum": 5}
        }));
        let compiled = CompiledSchema::compile(&schema).unwrap();

        assert!(
            compiled
                .validate(&serde_json::json!({"retries": 3}))
                .is_ok()
        );
        assert!(
            compiled
                .validate(&serde_json::json!({"retries": -1}))
                .is_err()
        );
        assert!(
            compiled
                .validate(&serde_json::json!({"retries": 10}))
                .is_err()
        );
    }

    // --- Required fields ---

    #[test]
    fn validate_required_fields() {
        let mut schema = schema_with_properties(serde_json::json!({
            "name": {"type": "string"},
            "age": {"type": "integer"}
        }));
        schema.required = vec!["name".into()];
        let compiled = CompiledSchema::compile(&schema).unwrap();

        assert!(
            compiled
                .validate(&serde_json::json!({"name": "alice"}))
                .is_ok()
        );
        assert!(compiled.validate(&serde_json::json!({"age": 25})).is_err());
    }

    // --- Multiple violations ---

    #[test]
    fn multiple_violations_reported() {
        let mut schema = schema_with_properties(serde_json::json!({
            "name": {"type": "string"},
            "age": {"type": "integer"}
        }));
        schema.required = vec!["name".into(), "age".into()];
        let compiled = CompiledSchema::compile(&schema).unwrap();

        let result = compiled.validate(&serde_json::json!({}));
        let violations = result.unwrap_err();
        assert_eq!(violations.len(), 2);
    }

    // --- Default values ---

    #[test]
    fn apply_defaults_fills_missing() {
        let schema = schema_with_properties(serde_json::json!({
            "mode": {"type": "string", "default": "read"},
            "retries": {"type": "integer", "default": 3},
            "verbose": {"type": "boolean", "default": false}
        }));
        let compiled = CompiledSchema::compile(&schema).unwrap();

        let mut params = serde_json::json!({"mode": "write"});
        compiled.apply_defaults(&mut params);

        assert_eq!(params["mode"], "write"); // not overwritten
        assert_eq!(params["retries"], 3);
        assert_eq!(params["verbose"], false);
    }

    #[test]
    fn apply_defaults_no_op_for_non_object() {
        let schema = schema_with_properties(serde_json::json!({}));
        let compiled = CompiledSchema::compile(&schema).unwrap();

        let mut params = serde_json::json!("not an object");
        compiled.apply_defaults(&mut params);
        assert_eq!(params, serde_json::json!("not an object"));
    }

    // --- Backward compat ---

    #[test]
    fn empty_properties_still_validates_required() {
        let schema = ToolSchema {
            schema_type: "object".into(),
            properties: HashMap::new(),
            required: vec!["path".into()],
        };
        let compiled = CompiledSchema::compile(&schema).unwrap();

        assert!(
            compiled
                .validate(&serde_json::json!({"path": "/tmp"}))
                .is_ok()
        );
        assert!(compiled.validate(&serde_json::json!({})).is_err());
    }

    #[test]
    fn unknown_type_falls_back_to_any() {
        let schema = schema_with_properties(serde_json::json!({
            "data": {"type": "custom_type"}
        }));
        let compiled = CompiledSchema::compile(&schema).unwrap();

        // Any value should be accepted.
        assert!(compiled.validate(&serde_json::json!({"data": 42})).is_ok());
        assert!(
            compiled
                .validate(&serde_json::json!({"data": "hello"}))
                .is_ok()
        );
    }

    #[test]
    fn extra_fields_allowed() {
        let schema = schema_with_properties(serde_json::json!({
            "name": {"type": "string"}
        }));
        let compiled = CompiledSchema::compile(&schema).unwrap();

        // Extra field "extra" is not in schema but should be accepted.
        assert!(
            compiled
                .validate(&serde_json::json!({"name": "alice", "extra": 42}))
                .is_ok()
        );
    }

    #[test]
    fn non_object_params_rejected() {
        let schema = schema_with_properties(serde_json::json!({}));
        let compiled = CompiledSchema::compile(&schema).unwrap();

        let result = compiled.validate(&serde_json::json!("not an object"));
        assert!(result.is_err());
        assert!(result.unwrap_err()[0].contains("params must be an object"));
    }
}
