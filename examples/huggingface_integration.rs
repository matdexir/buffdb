//! Example demonstrating BuffDB integration with Hugging Face models
//!
//! This example shows how to:
//! 1. Download models from Hugging Face Hub
//! 2. Store them in BuffDB for efficient distribution
//! 3. Serve models to edge devices through BuffDB's gRPC interface

use buffdb::{
    client::{blob::BlobClient, kv::KvClient},
    proto::{
        blob::{GetRequest as BlobGetRequest, StoreRequest},
        kv::{GetRequest, SetRequest},
    },
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio_stream::StreamExt;

#[derive(Debug, Serialize, Deserialize)]
struct HuggingFaceModelInfo {
    model_id: String,
    filename: String,
    size: usize,
    sha256: String,
}

/// Downloads a model from Hugging Face and stores it in BuffDB
async fn import_huggingface_model(
    hf_model_id: &str,
    filename: &str,
    kv_client: &mut KvClient<tonic::transport::Channel>,
    blob_client: &mut BlobClient<tonic::transport::Channel>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("📥 Importing {} from Hugging Face...", hf_model_id);
    
    // Construct Hugging Face URL
    let url = format!(
        "https://huggingface.co/{}/resolve/main/{}",
        hf_model_id, filename
    );
    
    println!("   Downloading from: {}", url);
    
    // Download model from Hugging Face
    let client = Client::new();
    let response = client
        .get(&url)
        .header("User-Agent", "BuffDB/0.5.0")
        .send()
        .await?;
    
    if !response.status().is_success() {
        return Err(format!("Failed to download: {}", response.status()).into());
    }
    
    let content_length = response
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);
    
    println!("   Model size: {} MB", content_length / 1024 / 1024);
    
    // Download model data
    let model_data = response.bytes().await?;
    println!("   Downloaded {} bytes", model_data.len());
    
    // Calculate SHA256
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(&model_data);
    let sha256 = format!("{:x}", hasher.finalize());
    
    // Store in BuffDB blob store
    println!("   Storing in BuffDB...");
    let metadata = Some(serde_json::json!({
        "source": "huggingface",
        "model_id": hf_model_id,
        "filename": filename,
        "sha256": sha256,
        "imported_at": chrono::Utc::now().to_rfc3339(),
    }).to_string());
    
    let request = tonic::Request::new(tokio_stream::once(StoreRequest {
        bytes: model_data.to_vec(),
        metadata,
        transaction_id: None,
    }));
    
    let mut response = blob_client.store(request).await?.into_inner();
    let blob_id = if let Some(result) = response.next().await {
        result?.id
    } else {
        return Err("Failed to store blob".into());
    };
    
    // Store metadata in KV store
    let model_info = HuggingFaceModelInfo {
        model_id: hf_model_id.to_string(),
        filename: filename.to_string(),
        size: model_data.len(),
        sha256: sha256.clone(),
    };
    
    let kv_entries = vec![
        (
            format!("hf:{}:info", hf_model_id.replace("/", "_")),
            serde_json::to_string(&model_info)?,
        ),
        (
            format!("hf:{}:blob_id", hf_model_id.replace("/", "_")),
            blob_id.to_string(),
        ),
        (
            format!("hf:{}:updated", hf_model_id.replace("/", "_")),
            chrono::Utc::now().to_rfc3339(),
        ),
    ];
    
    for (key, value) in kv_entries {
        let request = tonic::Request::new(tokio_stream::once(SetRequest {
            key,
            value,
            transaction_id: None,
        }));
        drop(kv_client.set(request).await?);
    }
    
    println!("   ✓ Stored with blob ID: {}", blob_id);
    println!("   ✓ SHA256: {}", sha256);
    
    Ok(())
}

/// Lists all Hugging Face models stored in BuffDB
async fn list_huggingface_models(
    kv_client: &mut KvClient<tonic::transport::Channel>,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut models = Vec::new();
    
    // In a real implementation, you'd use a prefix scan
    // For this example, we'll check a few known models
    let potential_models = vec![
        "bert-base-uncased",
        "gpt2",
        "sentence-transformers/all-MiniLM-L6-v2",
        "microsoft/resnet-50",
        "openai/clip-vit-base-patch32",
    ];
    
    for model in potential_models {
        let key = format!("hf:{}:info", model.replace("/", "_"));
        let request = tonic::Request::new(tokio_stream::once(GetRequest {
            key,
            transaction_id: None,
        }));
        
        match kv_client.get(request).await {
            Ok(stream) => {
                if let Some(Ok(_)) = stream.into_inner().next().await {
                    models.push(model.to_string());
                }
            }
            _ => {}
        }
    }
    
    Ok(models)
}

/// Serves a Hugging Face model from BuffDB
async fn serve_huggingface_model(
    model_id: &str,
    kv_client: &mut KvClient<tonic::transport::Channel>,
    blob_client: &mut BlobClient<tonic::transport::Channel>,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    println!("🚀 Serving {} from BuffDB...", model_id);
    
    // Get blob ID
    let key = format!("hf:{}:blob_id", model_id.replace("/", "_"));
    let request = tonic::Request::new(tokio_stream::once(GetRequest {
        key,
        transaction_id: None,
    }));
    
    let mut response = kv_client.get(request).await?.into_inner();
    let blob_id = if let Some(result) = response.next().await {
        result?.value.parse::<u64>()?
    } else {
        return Err(format!("Model {} not found", model_id).into());
    };
    
    // Stream model from blob store
    let request = tonic::Request::new(tokio_stream::once(BlobGetRequest {
        id: blob_id,
        transaction_id: None,
    }));
    
    let mut stream = blob_client.get(request).await?.into_inner();
    let mut model_data = Vec::new();
    
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        model_data.extend(chunk.bytes);
    }
    
    println!("   ✓ Served {} MB", model_data.len() / 1024 / 1024);
    Ok(model_data)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== BuffDB + Hugging Face Integration Example ===\n");
    
    // Connect to BuffDB
    let addr = "http://[::1]:9313";
    println!("Connecting to BuffDB at {}...", addr);
    
    let channel = tonic::transport::Channel::from_static(addr)
        .connect()
        .await?;
    
    let mut kv_client = KvClient::new(channel.clone());
    let mut blob_client = BlobClient::new(channel);
    
    // Example 1: Import a small model from Hugging Face
    println!("\n1. Importing model from Hugging Face Hub:");
    import_huggingface_model(
        "hf-internal-testing/tiny-random-bert",
        "pytorch_model.bin",
        &mut kv_client,
        &mut blob_client,
    ).await?;
    
    // Example 2: List stored models
    println!("\n2. Listing Hugging Face models in BuffDB:");
    let models = list_huggingface_models(&mut kv_client).await?;
    for model in &models {
        println!("   - {}", model);
    }
    
    // Example 3: Serve a model
    if !models.is_empty() {
        println!("\n3. Serving model for inference:");
        let _model_data = serve_huggingface_model(
            &models[0],
            &mut kv_client,
            &mut blob_client,
        ).await?;
        
        println!("\n✅ Model ready for inference!");
        println!("   In a real application, you would now:");
        println!("   - Load the model into your ML framework");
        println!("   - Run inference on input data");
        println!("   - Cache frequently used models locally");
    }
    
    // Example 4: Metadata management
    println!("\n4. Model metadata example:");
    let example_metadata = HashMap::from([
        ("framework".to_string(), "pytorch".to_string()),
        ("task".to_string(), "text-classification".to_string()),
        ("language".to_string(), "en".to_string()),
        ("license".to_string(), "apache-2.0".to_string()),
    ]);
    
    println!("   Example metadata structure:");
    for (key, value) in &example_metadata {
        println!("   - {}: {}", key, value);
    }
    
    println!("\n=== Integration Benefits ===");
    println!("✓ Direct access to Hugging Face's vast model repository");
    println!("✓ Efficient binary storage with BuffDB's blob store");
    println!("✓ Metadata tracking for model versioning");
    println!("✓ Fast local serving through gRPC");
    println!("✓ Edge deployment ready with caching");
    
    Ok(())
}