// Copyright 2025 BuffDB
// Licensed under the Apache License, Version 2.0

//! Simple inference helpers for ML model storage
//!
//! This module provides simple utilities for storing and retrieving ML models
//! using BuffDB's KV and BLOB storage capabilities.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Model metadata structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub version: String,
    pub framework: String,
    pub description: String,
    pub input_shape: Vec<i64>,
    pub output_shape: Vec<i64>,
    pub blob_ids: Vec<u64>,
    pub created_at: String,
    pub parameters: HashMap<String, String>,
}

/// Model storage key format
#[derive(Debug, Clone, Copy)]
pub struct ModelKeys;

impl ModelKeys {
    /// Get the key for model metadata
    pub fn metadata_key(name: &str, version: &str) -> String {
        format!("model:{name}:{version}:metadata")
    }

    /// Get the key for model index
    pub fn index_key(name: &str) -> String {
        format!("model:{name}:index")
    }

    /// Get the key for model blob references
    pub fn blob_key(name: &str, version: &str) -> String {
        format!("model:{name}:{version}:blobs")
    }

    /// Get the key for global model list
    pub fn global_index_key() -> String {
        "model:global:index".to_string()
    }
}

/// Helper to format blob metadata for model storage
pub fn format_blob_metadata(model_name: &str, version: &str, part_index: usize) -> String {
    serde_json::json!({
        "model": model_name,
        "version": version,
        "part": part_index,
        "type": "weights"
    })
    .to_string()
}
