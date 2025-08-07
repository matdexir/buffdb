// Copyright 2025 BuffDB
// Licensed under the Apache License, Version 2.0

//! Example showing how to use BuffDB for ML model storage and retrieval
//!
//! This example demonstrates:
//! - Storing ML model weights as BLOBs
//! - Storing model metadata as KV pairs
//! - Retrieving and listing models
//! - Organizing models by name and version

use anyhow::Result;
use buffdb::client::{blob::BlobClient, kv::KvClient};
use buffdb::proto::{blob, kv};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tonic::transport::Channel;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ModelMetadata {
    name: String,
    version: String,
    framework: String,
    input_shape: Vec<i64>,
    output_shape: Vec<i64>,
    description: String,
    created_at: String,
}

/// Store a model in BuffDB
async fn store_model(
    kv_client: &mut KvClient<Channel>,
    blob_client: &mut BlobClient<Channel>,
    metadata: ModelMetadata,
    weights: Vec<u8>,
) -> Result<()> {
    println!("Storing model: {} v{}", metadata.name, metadata.version);

    // Store model weights as blob
    let store_request = blob::StoreRequest {
        bytes: weights,
        metadata: Some(format!(
            r#"{{"model": "{}", "version": "{}"}}"#,
            metadata.name, metadata.version
        )),
        transaction_id: None,
    };

    let mut response = blob_client
        .store(tokio_stream::once(store_request))
        .await?
        .into_inner();

    let blob_id = if let Some(Ok(resp)) = response.next().await {
        resp.id
    } else {
        return Err(anyhow::anyhow!("Failed to store model weights"));
    };

    println!("Stored model weights with blob ID: {}", blob_id);

    // Store model metadata and blob reference in KV store
    let model_key = format!("model:{}:{}", metadata.name, metadata.version);
    let model_data = serde_json::json!({
        "metadata": metadata,
        "blob_id": blob_id,
    });

    let set_request = kv::SetRequest {
        key: model_key.clone(),
        value: model_data.to_string(),
        transaction_id: None,
    };

    let mut response = kv_client
        .set(tokio_stream::once(set_request))
        .await?
        .into_inner();

    if let Some(Ok(_)) = response.next().await {
        println!("Stored model metadata at key: {}", model_key);
    }

    // Also store in model index for easy listing
    let index_key = format!("model_index:{}", metadata.name);
    let get_request = kv::GetRequest {
        key: index_key.clone(),
        transaction_id: None,
    };

    let mut response = kv_client
        .get(tokio_stream::once(get_request))
        .await?
        .into_inner();

    let mut versions: Vec<String> = if let Some(Ok(resp)) = response.next().await {
        serde_json::from_str(&resp.value).unwrap_or_else(|_| Vec::new())
    } else {
        Vec::new()
    };

    if !versions.contains(&metadata.version) {
        versions.push(metadata.version.clone());
        versions.sort();

        let set_request = kv::SetRequest {
            key: index_key,
            value: serde_json::to_string(&versions)?,
            transaction_id: None,
        };

        drop(kv_client.set(tokio_stream::once(set_request)).await?);
    }

    Ok(())
}

/// Load a model from BuffDB
async fn load_model(
    kv_client: &mut KvClient<Channel>,
    blob_client: &mut BlobClient<Channel>,
    name: &str,
    version: &str,
) -> Result<(ModelMetadata, Vec<u8>)> {
    println!("Loading model: {} v{}", name, version);

    // Get model metadata and blob ID
    let model_key = format!("model:{}:{}", name, version);
    let get_request = kv::GetRequest {
        key: model_key,
        transaction_id: None,
    };

    let mut response = kv_client
        .get(tokio_stream::once(get_request))
        .await?
        .into_inner();

    let model_data = if let Some(Ok(resp)) = response.next().await {
        serde_json::from_str::<serde_json::Value>(&resp.value)?
    } else {
        return Err(anyhow::anyhow!("Model not found"));
    };

    let metadata: ModelMetadata = serde_json::from_value(model_data["metadata"].clone())?;
    let blob_id = model_data["blob_id"].as_u64().unwrap();

    // Get model weights from blob store
    let get_request = blob::GetRequest {
        id: blob_id,
        transaction_id: None,
    };

    let mut response = blob_client
        .get(tokio_stream::once(get_request))
        .await?
        .into_inner();

    let weights = if let Some(Ok(resp)) = response.next().await {
        resp.bytes
    } else {
        return Err(anyhow::anyhow!("Model weights not found"));
    };

    println!("Loaded model with {} bytes", weights.len());
    Ok((metadata, weights))
}

/// List all models
async fn list_models(kv_client: &mut KvClient<Channel>) -> Result<()> {
    println!("\nAvailable models:");

    // In a real implementation, you'd use a more sophisticated index
    // For now, we'll list some known model names
    let model_names = vec!["resnet50", "bert-base", "gpt2"];

    for name in model_names {
        let index_key = format!("model_index:{}", name);
        let get_request = kv::GetRequest {
            key: index_key,
            transaction_id: None,
        };

        let mut response = kv_client
            .get(tokio_stream::once(get_request))
            .await?
            .into_inner();

        if let Some(Ok(resp)) = response.next().await {
            let versions: Vec<String> = serde_json::from_str(&resp.value)?;
            println!("  {} - versions: {:?}", name, versions);
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Connect to BuffDB
    let addr = "[::1]:9313";
    println!("Connecting to BuffDB at {}", addr);

    let channel = Channel::from_static("http://[::1]:9313").connect().await?;

    let mut kv_client = KvClient::new(channel.clone());
    let mut blob_client = BlobClient::new(channel);

    // Example 1: Store a ResNet50 model
    let resnet_metadata = ModelMetadata {
        name: "resnet50".to_string(),
        version: "1.0".to_string(),
        framework: "pytorch".to_string(),
        input_shape: vec![1, 3, 224, 224],
        output_shape: vec![1, 1000],
        description: "ResNet50 trained on ImageNet".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    // Simulate model weights (in practice, these would be actual model parameters)
    let resnet_weights = vec![0u8; 1024 * 1024]; // 1MB dummy weights

    store_model(
        &mut kv_client,
        &mut blob_client,
        resnet_metadata.clone(),
        resnet_weights,
    )
    .await?;

    // Example 2: Store a BERT model
    let bert_metadata = ModelMetadata {
        name: "bert-base".to_string(),
        version: "2.0".to_string(),
        framework: "tensorflow".to_string(),
        input_shape: vec![1, 512],       // sequence length
        output_shape: vec![1, 512, 768], // hidden size
        description: "BERT base model for NLP tasks".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    let bert_weights = vec![1u8; 2 * 1024 * 1024]; // 2MB dummy weights

    store_model(
        &mut kv_client,
        &mut blob_client,
        bert_metadata,
        bert_weights,
    )
    .await?;

    // Example 3: List all models
    list_models(&mut kv_client).await?;

    // Example 4: Load a model
    let (loaded_metadata, loaded_weights) =
        load_model(&mut kv_client, &mut blob_client, "resnet50", "1.0").await?;

    println!("\nLoaded model details:");
    println!("  Name: {}", loaded_metadata.name);
    println!("  Version: {}", loaded_metadata.version);
    println!("  Framework: {}", loaded_metadata.framework);
    println!("  Input shape: {:?}", loaded_metadata.input_shape);
    println!("  Output shape: {:?}", loaded_metadata.output_shape);
    println!("  Description: {}", loaded_metadata.description);
    println!("  Weights size: {} bytes", loaded_weights.len());

    // Example 5: Update model metadata
    let mut updated_metadata = resnet_metadata;
    updated_metadata.version = "1.1".to_string();
    updated_metadata.description = "ResNet50 trained on ImageNet - optimized version".to_string();

    store_model(
        &mut kv_client,
        &mut blob_client,
        updated_metadata,
        vec![2u8; 1024 * 1024],
    )
    .await?;

    println!("\n✅ All operations completed successfully!");

    Ok(())
}
