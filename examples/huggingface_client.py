#!/usr/bin/env python3
"""
Example: Using BuffDB as a cache for Hugging Face models

This example demonstrates how to:
1. Download models from Hugging Face
2. Cache them in BuffDB
3. Serve them efficiently to multiple clients
"""

import grpc
import asyncio
import hashlib
import json
from typing import Optional, Dict, Any
import aiohttp

# Import BuffDB proto definitions
# In production, these would be generated from the .proto files
import buffdb_pb2
import buffdb_pb2_grpc


class HuggingFaceBuffDB:
    """Client for managing Hugging Face models in BuffDB"""
    
    def __init__(self, buffdb_address: str = "localhost:9313"):
        self.channel = grpc.insecure_channel(buffdb_address)
        self.kv_stub = buffdb_pb2_grpc.KvStub(self.channel)
        self.blob_stub = buffdb_pb2_grpc.BlobStub(self.channel)
    
    async def download_from_huggingface(self, model_id: str, filename: str) -> bytes:
        """Download a model file from Hugging Face Hub"""
        url = f"https://huggingface.co/{model_id}/resolve/main/{filename}"
        
        async with aiohttp.ClientSession() as session:
            async with session.get(url) as response:
                if response.status != 200:
                    raise Exception(f"Failed to download {model_id}: {response.status}")
                
                model_data = await response.read()
                print(f"Downloaded {len(model_data) / 1024 / 1024:.2f} MB from Hugging Face")
                return model_data
    
    async def cache_model(self, model_id: str, filename: str, model_data: bytes) -> int:
        """Store model in BuffDB blob store"""
        # Calculate SHA256
        sha256 = hashlib.sha256(model_data).hexdigest()
        
        # Store blob
        metadata = json.dumps({
            "source": "huggingface",
            "model_id": model_id,
            "filename": filename,
            "sha256": sha256,
            "size": len(model_data)
        })
        
        request = buffdb_pb2.StoreRequest(
            bytes=model_data,
            metadata=metadata
        )
        
        response = self.blob_stub.Store(iter([request]))
        blob_id = None
        async for result in response:
            blob_id = result.id
            break
        
        if blob_id is None:
            raise Exception("Failed to store blob")
        
        # Store metadata in KV
        kv_key = f"hf:{model_id.replace('/', '_')}:info"
        kv_value = json.dumps({
            "model_id": model_id,
            "filename": filename,
            "blob_id": blob_id,
            "sha256": sha256,
            "size": len(model_data)
        })
        
        kv_request = buffdb_pb2.SetRequest(key=kv_key, value=kv_value)
        self.kv_stub.Set(iter([kv_request]))
        
        print(f"Cached model with blob ID: {blob_id}")
        return blob_id
    
    async def get_model(self, model_id: str) -> Optional[bytes]:
        """Retrieve model from BuffDB cache"""
        # Get blob ID from KV
        kv_key = f"hf:{model_id.replace('/', '_')}:info"
        kv_request = buffdb_pb2.GetRequest(key=kv_key)
        
        response = self.kv_stub.Get(iter([kv_request]))
        info = None
        async for result in response:
            info = json.loads(result.value)
            break
        
        if not info:
            return None
        
        # Get blob data
        blob_request = buffdb_pb2.GetRequest(id=info["blob_id"])
        response = self.blob_stub.Get(iter([blob_request]))
        
        model_data = bytearray()
        async for chunk in response:
            model_data.extend(chunk.bytes)
        
        print(f"Retrieved {len(model_data) / 1024 / 1024:.2f} MB from cache")
        return bytes(model_data)
    
    async def sync_model(self, model_id: str, filename: str) -> bytes:
        """Get model from cache or download from Hugging Face"""
        # Try cache first
        cached = await self.get_model(model_id)
        if cached:
            print(f"Using cached version of {model_id}")
            return cached
        
        # Download and cache
        print(f"Downloading {model_id} from Hugging Face...")
        model_data = await self.download_from_huggingface(model_id, filename)
        await self.cache_model(model_id, filename, model_data)
        return model_data


async def main():
    """Example usage"""
    client = HuggingFaceBuffDB()
    
    # Example models to cache
    models = [
        ("bert-base-uncased", "pytorch_model.bin"),
        ("gpt2", "pytorch_model.bin"),
        ("microsoft/resnet-50", "pytorch_model.bin"),
    ]
    
    print("=== BuffDB + Hugging Face Integration ===\n")
    
    for model_id, filename in models:
        print(f"Syncing {model_id}...")
        try:
            model_data = await client.sync_model(model_id, filename)
            print(f"✓ Model ready: {model_id} ({len(model_data) / 1024 / 1024:.2f} MB)\n")
        except Exception as e:
            print(f"✗ Failed to sync {model_id}: {e}\n")
    
    print("\n=== Benefits ===")
    print("• Models cached locally in BuffDB")
    print("• Fast serving to multiple clients")
    print("• Reduced bandwidth usage")
    print("• Offline availability")
    print("• Version tracking and integrity checks")


if __name__ == "__main__":
    asyncio.run(main())