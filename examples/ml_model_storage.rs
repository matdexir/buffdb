//! Example demonstrating BuffDB for ML model storage and serving from buffdb.dev
//!
//! This example shows how to:
//! 1. Connect to a remote BuffDB server (buffdb.dev) to download models
//! 2. Cache models locally for edge inference
//! 3. Manage model versions and metadata
//! 4. Handle offline scenarios with local cache

use buffdb::{
    client::{blob::BlobClient, kv::KvClient},
    proto::{
        blob::{GetRequest as BlobGetRequest, StoreRequest},
        kv::{GetRequest, SetRequest},
    },
};
use std::time::Instant;
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== BuffDB ML Model Edge Deployment Example ===\n");

    // Configuration
    let remote_server = "https://buffdb.dev:9313"; // Your model repository
    let local_server = "http://[::1]:9313"; // Local BuffDB instance for caching

    // Try to connect to remote server first
    println!("1. Connecting to model repository at {}...", remote_server);
    let remote_channel = match tonic::transport::Channel::from_static(remote_server)
        .connect()
        .await
    {
        Ok(channel) => {
            println!("   ✓ Connected to remote model repository");
            Some(channel)
        }
        Err(e) => {
            println!("   ✗ Could not connect to remote: {}", e);
            println!("   → Falling back to local cache only mode");
            None
        }
    };

    // Always connect to local BuffDB for caching
    println!("\n2. Connecting to local BuffDB at {}...", local_server);
    let local_channel = tonic::transport::Channel::from_static(local_server)
        .connect()
        .await?;
    println!("   ✓ Connected to local BuffDB");

    let mut local_kv = KvClient::new(local_channel.clone());
    let mut local_blob = BlobClient::new(local_channel);

    // Example: Sync a specific model from remote to local
    if let Some(remote_channel) = remote_channel {
        let model_id = "resnet50-v2.1";
        println!("\n3. Checking for model updates: {}", model_id);

        let mut remote_kv = KvClient::new(remote_channel.clone());
        let mut remote_blob = BlobClient::new(remote_channel);

        // Check model metadata on remote
        let remote_version_key = format!("model:{}:version", model_id);
        let request = tonic::Request::new(tokio_stream::once(GetRequest {
            key: remote_version_key.clone(),
            transaction_id: None,
        }));

        let mut response = remote_kv.get(request).await?.into_inner();
        if let Some(result) = response.next().await {
            let remote_version = result?.value;
            println!("   Remote version: {}", remote_version);

            // Check local version
            let request = tonic::Request::new(tokio_stream::once(GetRequest {
                key: remote_version_key.clone(),
                transaction_id: None,
            }));

            let mut local_response = local_kv.get(request).await?.into_inner();
            let local_version = if let Some(result) = local_response.next().await {
                Some(result?.value)
            } else {
                None
            };

            if local_version.as_ref() != Some(&remote_version) {
                println!(
                    "   Local version: {} (outdated)",
                    local_version.unwrap_or_else(|| "none".to_string())
                );
                println!("   → Downloading updated model...");

                // Download model blob from remote
                let blob_id_key = format!("model:{}:blob_id", model_id);
                let request = tonic::Request::new(tokio_stream::once(GetRequest {
                    key: blob_id_key,
                    transaction_id: None,
                }));

                let mut response = remote_kv.get(request).await?.into_inner();
                if let Some(result) = response.next().await {
                    let blob_id = result?.value.parse::<u64>()?;

                    // Download the blob
                    let start = Instant::now();
                    let request = tonic::Request::new(tokio_stream::once(BlobGetRequest {
                        id: blob_id,
                        transaction_id: None,
                    }));

                    let mut blob_response = remote_blob.get(request).await?.into_inner();
                    let mut model_data = Vec::new();
                    let mut metadata = None;

                    while let Some(chunk) = blob_response.next().await {
                        let chunk = chunk?;
                        model_data.extend(&chunk.bytes);
                        if let Some(chunk_metadata) = chunk.metadata {
                            metadata = Some(chunk_metadata);
                        }
                    }

                    let download_time = start.elapsed();
                    println!(
                        "   Downloaded {} MB in {:?} ({:.2} MB/s)",
                        model_data.len() / 1024 / 1024,
                        download_time,
                        (model_data.len() as f64 / 1024.0 / 1024.0) / download_time.as_secs_f64()
                    );

                    // Store in local cache
                    println!("   → Caching model locally...");
                    let request = tonic::Request::new(tokio_stream::once(StoreRequest {
                        bytes: model_data,
                        metadata,
                        transaction_id: None,
                    }));

                    let mut store_response = local_blob.store(request).await?.into_inner();
                    if let Some(result) = store_response.next().await {
                        let local_blob_id = result?.id;

                        // Update local metadata
                        let updates = vec![
                            (remote_version_key, remote_version),
                            (
                                format!("model:{}:blob_id", model_id),
                                local_blob_id.to_string(),
                            ),
                            (
                                format!("model:{}:last_sync", model_id),
                                chrono::Utc::now().to_rfc3339(),
                            ),
                        ];

                        for (key, value) in updates {
                            let request = tonic::Request::new(tokio_stream::once(SetRequest {
                                key,
                                value,
                                transaction_id: None,
                            }));
                            let _ = local_kv.set(request).await?;
                        }

                        println!("   ✓ Model cached successfully");
                    }
                }
            } else {
                println!("   ✓ Local cache is up to date");
            }
        }
    }

    // Example: Load model from local cache for inference
    println!("\n4. Loading model from local cache for inference...");
    let model_id = "resnet50-v2.1";
    let blob_id_key = format!("model:{}:blob_id", model_id);

    let request = tonic::Request::new(tokio_stream::once(GetRequest {
        key: blob_id_key,
        transaction_id: None,
    }));

    let mut response = local_kv.get(request).await?.into_inner();
    if let Some(result) = response.next().await {
        let blob_id = result?.value.parse::<u64>()?;

        let start = Instant::now();
        let request = tonic::Request::new(tokio_stream::once(BlobGetRequest {
            id: blob_id,
            transaction_id: None,
        }));

        let mut blob_response = local_blob.get(request).await?.into_inner();
        let mut total_bytes = 0;

        while let Some(chunk) = blob_response.next().await {
            let chunk = chunk?;
            total_bytes += chunk.bytes.len();
            // In a real scenario, you would feed this to your ML framework
        }

        println!(
            "   ✓ Loaded {} MB in {:?} (ready for inference)",
            total_bytes / 1024 / 1024,
            start.elapsed()
        );
    } else {
        println!("   ✗ Model not found in local cache");
    }

    // Show cache status
    println!("\n5. Local cache status:");
    let cache_keys = vec![
        "model:resnet50-v2.1:version",
        "model:resnet50-v2.1:last_sync",
        "model:bert-base:version",
        "model:bert-base:last_sync",
    ];

    for key in cache_keys {
        let request = tonic::Request::new(tokio_stream::once(GetRequest {
            key: key.to_string(),
            transaction_id: None,
        }));

        let response = local_kv.get(request).await;
        match response {
            Ok(stream) => {
                if let Some(Ok(result)) = stream.into_inner().next().await {
                    println!("   {} = {}", key, result.value);
                }
            }
            _ => {}
        }
    }

    println!("\n=== Example Complete ===");
    println!("\nKey benefits demonstrated:");
    println!("- Automatic model synchronization from buffdb.dev");
    println!("- Local caching for offline operation");
    println!("- Version management to avoid unnecessary downloads");
    println!("- Fast local model loading for inference");
    println!("- Resilient operation when remote is unavailable");

    Ok(())
}
