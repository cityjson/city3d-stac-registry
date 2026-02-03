//! Attribute schema definitions for semantic attributes

use serde::{Deserialize, Serialize};

/// Schema definition for a semantic attribute
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AttributeDefinition {
    /// Attribute name
    pub name: String,

    /// Data type
    #[serde(rename = "type")]
    pub attr_type: AttributeType,

    /// Human-readable description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Whether attribute is always present
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
}

impl AttributeDefinition {
    /// Create a new attribute definition
    pub fn new(name: impl Into<String>, attr_type: AttributeType) -> Self {
        Self {
            name: name.into(),
            attr_type,
            description: None,
            required: None,
        }
    }

    /// Set the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set whether the attribute is required
    pub fn with_required(mut self, required: bool) -> Self {
        self.required = Some(required);
        self
    }
}

/// Data type for semantic attributes
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AttributeType {
    String,
    Number,
    Boolean,
    Date,
    Array,
    Object,
}

impl AttributeType {
    /// Infer attribute type from a JSON value
    pub fn from_json_value(value: &serde_json::Value) -> Self {
        match value {
            serde_json::Value::String(_) => AttributeType::String,
            serde_json::Value::Number(_) => AttributeType::Number,
            serde_json::Value::Bool(_) => AttributeType::Boolean,
            serde_json::Value::Array(_) => AttributeType::Array,
            serde_json::Value::Object(_) => AttributeType::Object,
            serde_json::Value::Null => AttributeType::String, // Default to string for null
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attribute_definition() {
        let attr = AttributeDefinition::new("yearOfConstruction", AttributeType::Number)
            .with_description("Year the building was constructed")
            .with_required(false);

        assert_eq!(attr.name, "yearOfConstruction");
        assert_eq!(attr.attr_type, AttributeType::Number);
        assert_eq!(
            attr.description,
            Some("Year the building was constructed".to_string())
        );
        assert_eq!(attr.required, Some(false));
    }

    #[test]
    fn test_attribute_type_from_json() {
        use serde_json::json;

        assert_eq!(
            AttributeType::from_json_value(&json!("hello")),
            AttributeType::String
        );
        assert_eq!(
            AttributeType::from_json_value(&json!(42)),
            AttributeType::Number
        );
        assert_eq!(
            AttributeType::from_json_value(&json!(true)),
            AttributeType::Boolean
        );
        assert_eq!(
            AttributeType::from_json_value(&json!([1, 2, 3])),
            AttributeType::Array
        );
        assert_eq!(
            AttributeType::from_json_value(&json!({"key": "value"})),
            AttributeType::Object
        );
    }

    #[test]
    fn test_attribute_serialization() {
        let attr = AttributeDefinition::new("function", AttributeType::String);
        let json = serde_json::to_string(&attr).unwrap();
        assert!(json.contains("\"name\":\"function\""));
        assert!(json.contains("\"type\":\"String\""));
    }
}
