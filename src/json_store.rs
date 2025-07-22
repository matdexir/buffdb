//! JSON document store with JSONPath query support
//!
//! This module provides a document-oriented storage layer on top of BuffDB's
//! key-value store, with support for JSONPath queries and partial updates.
//!
//! TODO: The json_store module needs to be refactored to work properly with the
//! streaming API. The current implementation is temporarily disabled as it requires
//! adapting between streaming and non-streaming interfaces, which is complex due
//! to the nature of tonic::Streaming being created by the gRPC framework.
//!
//! Possible solutions:
//! 1. Use the transitive client approach to create a proper gRPC client connection
//! 2. Implement a non-streaming backend specifically for internal use
//! 3. Refactor the entire module to work with streaming natively

use crate::backend::{DatabaseBackend, KvBackend};
use crate::fts::FtsManager;
use crate::index::{IndexConfig, IndexManager, IndexType};
use crate::interop::IntoTonicStatus;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::sync::Arc;

/// JSON document store errors
#[derive(Debug, thiserror::Error)]
pub enum JsonStoreError {
    #[error("Document not found: {id}")]
    DocumentNotFound { id: String },

    #[error("Invalid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),

    #[error("Invalid JSONPath expression: {0}")]
    InvalidJsonPath(String),

    #[error("Type mismatch at path {path}: expected {expected}, got {actual}")]
    TypeMismatch {
        path: String,
        expected: String,
        actual: String,
    },

    #[error("Index error: {0}")]
    IndexError(#[from] crate::index::IndexError),

    #[error("Backend error: {0}")]
    BackendError(String),

    #[error("Not implemented: json_store module needs refactoring for streaming API")]
    NotImplemented,
}

/// JSON document metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub version: u64,
    pub schema: Option<String>,
}

/// A JSON document with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonDocument {
    #[serde(flatten)]
    pub metadata: DocumentMetadata,
    pub data: Value,
}

/// JSON document store - temporarily disabled
///
/// This struct is kept for API compatibility but all methods return NotImplemented error
pub struct JsonStore<Backend> {
    _backend: std::marker::PhantomData<Backend>,
    index_manager: Arc<IndexManager>,
    fts_manager: Arc<FtsManager>,
    collection: String,
}

impl<Backend> JsonStore<Backend> {
    /// Create a new JSON document store
    pub fn new(_backend: Backend, collection: String) -> Self {
        Self {
            _backend: std::marker::PhantomData,
            index_manager: Arc::new(IndexManager::new()),
            fts_manager: Arc::new(FtsManager::new()),
            collection,
        }
    }

    /// Create a JSON path index
    pub fn create_json_index(
        &self,
        name: String,
        json_path: String,
        index_type: IndexType,
        unique: bool,
    ) -> Result<(), crate::index::IndexError> {
        let config = IndexConfig {
            name: format!("{}_{}", self.collection, name),
            index_type,
            unique,
            filter: Some(json_path),
        };

        self.index_manager.create_index(config)
    }

    /// Create a full-text search index on a JSON path
    pub fn create_fts_index(
        &self,
        name: String,
        _json_path: String,
    ) -> Result<(), crate::index::IndexError> {
        let index_name = format!("{}_{}", self.collection, name);
        self.fts_manager.create_index(index_name)?;

        // Store the JSON path mapping
        // In a real implementation, we'd store this persistently
        Ok(())
    }

    /// Generate document key
    fn doc_key(&self, id: &str) -> String {
        format!("{}:doc:{}", self.collection, id)
    }

    /// Generate metadata key
    fn meta_key(&self, id: &str) -> String {
        format!("{}:meta:{}", self.collection, id)
    }
}

/// JSONPath query evaluation
pub struct JsonPath;

impl JsonPath {
    /// Evaluate a JSONPath expression against a JSON value
    pub fn query<'a>(json: &'a Value, path: &str) -> Result<Vec<&'a Value>, JsonStoreError> {
        // Simple JSONPath implementation
        // Full implementation would use a proper JSONPath parser

        let parts: Vec<&str> = path.split('.').collect();
        if parts.is_empty() || parts[0] != "$" {
            return Err(JsonStoreError::InvalidJsonPath(
                "Path must start with $".to_string(),
            ));
        }

        let mut current = vec![json];

        for part in parts.iter().skip(1) {
            let mut next = Vec::new();

            for value in current {
                if part.ends_with(']') {
                    // Array access
                    let (field, index_str) = part.split_once('[').unwrap();
                    let index_str = index_str.trim_end_matches(']');

                    if let Some(obj) = value.get(field) {
                        if let Some(arr) = obj.as_array() {
                            if index_str == "*" {
                                // All elements
                                next.extend(arr.iter());
                            } else if let Ok(index) = index_str.parse::<usize>() {
                                // Specific index
                                if let Some(elem) = arr.get(index) {
                                    next.push(elem);
                                }
                            }
                        }
                    }
                } else if part == &"*" {
                    // Wildcard
                    match value {
                        Value::Object(map) => next.extend(map.values()),
                        Value::Array(arr) => next.extend(arr.iter()),
                        _ => {}
                    }
                } else {
                    // Object field access
                    if let Some(field_value) = value.get(part) {
                        next.push(field_value);
                    }
                }
            }

            current = next;
        }

        Ok(current)
    }

    /// Set a value at a JSONPath location
    pub fn set(json: &mut Value, path: &str, new_value: Value) -> Result<(), JsonStoreError> {
        let parts: Vec<&str> = path.split('.').collect();
        if parts.is_empty() || parts[0] != "$" {
            return Err(JsonStoreError::InvalidJsonPath(
                "Path must start with $".to_string(),
            ));
        }

        // Handle path with only "$"
        if parts.len() == 1 {
            *json = new_value;
            return Ok(());
        }

        // Navigate to the parent of the target location
        let mut current = json;
        let target_path = &parts[1..parts.len() - 1];
        let final_key = parts[parts.len() - 1];

        for part in target_path {
            if part.ends_with(']') {
                // Array access
                let (field, index_str) = part.split_once('[').unwrap();
                let index_str = index_str.trim_end_matches(']');
                let index: usize = index_str.parse().unwrap();

                // Ensure we have an object to work with
                if !current.is_object() {
                    *current = Value::Object(Map::new());
                }

                // Ensure field exists and is an array
                {
                    let obj = current.as_object_mut().unwrap();
                    if !obj.contains_key(field) || !obj[field].is_array() {
                        obj.insert(field.to_string(), Value::Array(Vec::new()));
                    }
                }

                // Now work with the array
                {
                    let obj = current.as_object_mut().unwrap();
                    let arr = obj.get_mut(field).unwrap().as_array_mut().unwrap();

                    // Extend array if needed
                    while arr.len() <= index {
                        arr.push(Value::Null);
                    }
                }

                // Move to the array element
                current = current
                    .get_mut(field)
                    .unwrap()
                    .as_array_mut()
                    .unwrap()
                    .get_mut(index)
                    .unwrap();
            } else {
                // Object field access
                if !current.is_object() {
                    *current = Value::Object(Map::new());
                }

                let obj = current.as_object_mut().unwrap();
                if !obj.contains_key(&part.to_string()) {
                    obj.insert(part.to_string(), Value::Object(Map::new()));
                }
                current = obj.get_mut(&part.to_string()).unwrap();
            }
        }

        // Set the final value
        if final_key.ends_with(']') {
            // Array access
            let (field, index_str) = final_key.split_once('[').unwrap();
            let index_str = index_str.trim_end_matches(']');
            let index: usize = index_str.parse().unwrap();

            // Ensure we have an object to work with
            if !current.is_object() {
                *current = Value::Object(Map::new());
            }

            // Ensure field exists and is an array
            {
                let obj = current.as_object_mut().unwrap();
                if !obj.contains_key(field) || !obj[field].is_array() {
                    obj.insert(field.to_string(), Value::Array(Vec::new()));
                }
            }

            // Set the value in the array
            let obj = current.as_object_mut().unwrap();
            let arr = obj.get_mut(field).unwrap().as_array_mut().unwrap();

            // Extend array if needed
            while arr.len() <= index {
                arr.push(Value::Null);
            }
            arr[index] = new_value;
        } else {
            // Object field access
            if !current.is_object() {
                *current = Value::Object(Map::new());
            }

            let obj = current.as_object_mut().unwrap();
            obj.insert(final_key.to_string(), new_value);
        }

        Ok(())
    }
}

// Document store operations - temporarily return NotImplemented
impl<Backend> JsonStore<Backend> {
    /// Insert a new document
    pub async fn insert(
        &self,
        _id: String,
        _data: Value,
        _schema: Option<String>,
    ) -> Result<JsonDocument, JsonStoreError> {
        Err(JsonStoreError::NotImplemented)
    }

    /// Get a document by ID
    pub async fn get(&self, _id: &str) -> Result<JsonDocument, JsonStoreError> {
        Err(JsonStoreError::NotImplemented)
    }

    /// Update a document
    pub async fn update(&self, _id: &str, _data: Value) -> Result<JsonDocument, JsonStoreError> {
        Err(JsonStoreError::NotImplemented)
    }

    /// Partially update a document using JSONPath
    pub async fn patch(
        &self,
        _id: &str,
        _path: &str,
        _value: Value,
    ) -> Result<JsonDocument, JsonStoreError> {
        Err(JsonStoreError::NotImplemented)
    }

    /// Delete a document
    pub async fn delete(&self, _id: &str) -> Result<(), JsonStoreError> {
        Err(JsonStoreError::NotImplemented)
    }

    /// Query documents using JSONPath
    pub async fn query(
        &self,
        _json_path: &str,
        _value: &Value,
    ) -> Result<Vec<String>, JsonStoreError> {
        Err(JsonStoreError::NotImplemented)
    }

    fn update_indexes(
        &self,
        _id: &str,
        _old_data: Option<&Value>,
        _new_data: &Value,
    ) -> Result<(), JsonStoreError> {
        // In a real implementation, we would:
        // 1. Extract values from JSON paths configured in indexes
        // 2. Update the index entries
        // For now, this is a placeholder
        Ok(())
    }

    fn remove_from_indexes(&self, _id: &str, _data: &Value) -> Result<(), JsonStoreError> {
        // In a real implementation, we would remove from all indexes
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_path_query() {
        let json = serde_json::json!({
            "user": {
                "name": "Alice",
                "age": 30,
                "emails": ["alice@example.com", "alice@work.com"],
                "address": {
                    "city": "New York",
                    "zip": "10001"
                }
            }
        });

        // Simple field access
        let results = JsonPath::query(&json, "$.user.name").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], &Value::String("Alice".to_string()));

        // Nested field access
        let results = JsonPath::query(&json, "$.user.address.city").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], &Value::String("New York".to_string()));

        // Array access
        let results = JsonPath::query(&json, "$.user.emails[0]").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], &Value::String("alice@example.com".to_string()));

        // Array wildcard
        let results = JsonPath::query(&json, "$.user.emails[*]").unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_json_path_set() {
        let mut json = serde_json::json!({
            "user": {
                "name": "Alice"
            }
        });

        // Set simple field
        JsonPath::set(&mut json, "$.user.age", Value::Number(30.into())).unwrap();
        assert_eq!(json["user"]["age"], 30);

        // Set nested field (creates path)
        JsonPath::set(
            &mut json,
            "$.user.address.city",
            Value::String("NYC".to_string()),
        )
        .unwrap();
        assert_eq!(json["user"]["address"]["city"], "NYC");

        // Set array element
        JsonPath::set(
            &mut json,
            "$.user.tags[0]",
            Value::String("admin".to_string()),
        )
        .unwrap();
        assert_eq!(json["user"]["tags"][0], "admin");
    }
}
