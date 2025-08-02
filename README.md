# 🦁 BuffDB

[![License: MIT/Apache](https://img.shields.io/badge/License-MIT%2FApache-blue.svg)](LICENSE-MIT)
[![Tests](https://github.com/buffdb/buffdb/actions/workflows/test.yml/badge.svg)](https://github.com/buffdb/buffdb/actions/workflows/test.yml)
[![Discord](https://img.shields.io/discord/1267505649198305384?label=Discord&logo=discord)](https://discord.gg/4Pzv6sB8)
[![Crates.io](https://img.shields.io/crates/v/buffdb.svg)](https://crates.io/crates/buffdb)

BuffDB is a lightweight, high-performance embedded database with networking capabilities, designed for edge computing and offline-first applications. Built in Rust with <2MB binary size.

**⚠️ Experimental:** Join our [Discord](https://discord.gg/4Pzv6sB8) for help. See [known issues](https://github.com/buffdb/buffdb/issues/11).

## ✨ Key Features

- 🚀 **High Performance** - Optimized for speed with multiple backend options
- 🌐 **gRPC Network API** - Access your database over the network
- 🔄 **Transactions** - ACID compliance with isolation levels
- 🔍 **Full-Text Search** - Built-in FTS with BM25 ranking
- 📊 **Secondary Indexes** - B-tree and hash indexes for fast queries
- 📄 **JSON Documents** - Document store with JSONPath queries
- 🔐 **MVCC** - Multi-version concurrency control for better performance
- 📦 **Tiny Size** - Under 2MB binary with SQLite backend

## 🚀 Quick Start

### Prerequisites

BuffDB requires `protoc` (Protocol Buffers compiler):

```bash
# Ubuntu/Debian
sudo apt-get install protobuf-compiler

# macOS
brew install protobuf

# Windows
choco install protoc
```

### Start BuffDB

```bash
cargo install buffdb
buffdb run
```

### Language Examples

<details>
<summary><b>🦀 Rust</b></summary>

```rust
use buffdb::{Connection, proto::kv::*};
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to BuffDB
    let mut client = Connection::connect("http://[::1]:9313").await?;
    
    // Simple key-value operations
    client.kv_set("user:1", "Alice").await?;
    let value = client.kv_get("user:1").await?;
    println!("Value: {}", value.unwrap());
    
    // Use transactions
    let tx_id = client.begin_transaction().await?;
    client.kv_set_tx("counter", "100", &tx_id).await?;
    client.commit_transaction(&tx_id).await?;
    
    Ok(())
}
```

Add to `Cargo.toml`:
```toml
[dependencies]
buffdb = "0.4"
tokio = { version = "1", features = ["full"] }
```
</details>

<details>
<summary><b>🟦 TypeScript / Node.js</b></summary>

```typescript
import * as grpc from '@grpc/grpc-js';
import * as protoLoader from '@grpc/proto-loader';

// Load proto definitions
const packageDefinition = protoLoader.loadSync('buffdb.proto');
const buffdb = grpc.loadPackageDefinition(packageDefinition).buffdb;

// Connect to BuffDB
const client = new buffdb.kv.Kv(
  '[::1]:9313',
  grpc.credentials.createInsecure()
);

// Simple key-value operations
const setStream = client.Set();
setStream.write({ key: 'user:1', value: 'Alice' });
setStream.end();

const getStream = client.Get();
getStream.on('data', (data) => {
  console.log('Value:', data.value);
});
getStream.write({ key: 'user:1' });
getStream.end();
```

Install dependencies:
```bash
npm install @grpc/grpc-js @grpc/proto-loader
```
</details>

<details>
<summary><b>🐍 Python</b></summary>

```python
import grpc
import buffdb_pb2
import buffdb_pb2_grpc

# Connect to BuffDB
channel = grpc.insecure_channel('[::1]:9313')
stub = buffdb_pb2_grpc.KvStub(channel)

# Simple key-value operations
def set_key_value():
    request = buffdb_pb2.SetRequest(key='user:1', value='Alice')
    for response in stub.Set(iter([request])):
        print(f'Set key: {response.key}')

def get_value():
    request = buffdb_pb2.GetRequest(key='user:1')
    for response in stub.Get(iter([request])):
        print(f'Value: {response.value}')

set_key_value()
get_value()
```

Install dependencies:
```bash
pip install grpcio grpcio-tools
```
</details>

<details>
<summary><b>☕ Java</b></summary>

```java
import io.grpc.ManagedChannel;
import io.grpc.ManagedChannelBuilder;
import buffdb.KvGrpc;
import buffdb.KvOuterClass.*;

public class BuffDBExample {
    public static void main(String[] args) {
        // Connect to BuffDB
        ManagedChannel channel = ManagedChannelBuilder
            .forAddress("localhost", 9313)
            .usePlaintext()
            .build();
        
        KvGrpc.KvStub stub = KvGrpc.newStub(channel);
        
        // Simple key-value operations
        StreamObserver<SetRequest> setStream = stub.set(
            new StreamObserver<SetResponse>() {
                @Override
                public void onNext(SetResponse response) {
                    System.out.println("Set key: " + response.getKey());
                }
                // ... implement other methods
            }
        );
        
        setStream.onNext(SetRequest.newBuilder()
            .setKey("user:1")
            .setValue("Alice")
            .build());
        setStream.onCompleted();
    }
}
```

Add to `build.gradle`:
```gradle
dependencies {
    implementation 'io.grpc:grpc-netty:1.58.0'
    implementation 'io.grpc:grpc-protobuf:1.58.0'
    implementation 'io.grpc:grpc-stub:1.58.0'
}
```
</details>

<details>
<summary><b>🔷 Go</b></summary>

```go
package main

import (
    "context"
    "log"
    pb "your-module/buffdb"
    "google.golang.org/grpc"
)

func main() {
    // Connect to BuffDB
    conn, err := grpc.Dial("[::1]:9313", grpc.WithInsecure())
    if err != nil {
        log.Fatal(err)
    }
    defer conn.Close()
    
    client := pb.NewKvClient(conn)
    
    // Simple key-value operations
    setStream, err := client.Set(context.Background())
    if err != nil {
        log.Fatal(err)
    }
    
    err = setStream.Send(&pb.SetRequest{
        Key:   "user:1",
        Value: "Alice",
    })
    if err != nil {
        log.Fatal(err)
    }
    setStream.CloseSend()
    
    // Get value
    getStream, err := client.Get(context.Background())
    getStream.Send(&pb.GetRequest{Key: "user:1"})
    
    resp, err := getStream.Recv()
    if err == nil {
        log.Printf("Value: %s", resp.Value)
    }
}
```

Install dependencies:
```bash
go get google.golang.org/grpc
```
</details>

## 🛠️ Installation

```bash
# Install from crates.io
cargo install buffdb

# Docker
docker run -p 9313:9313 ghcr.io/buffdb/buffdb:latest

# From source
git clone https://github.com/buffdb/buffdb
cd buffdb
cargo build --release
./target/release/buffdb run
```

## 📚 Advanced Features

### Transactions
```rust
// Begin transaction
let tx_id = client.begin_transaction(false, Some(5000)).await?;

// Operations within transaction
client.set_in_transaction(&tx_id, "key1", "value1").await?;
client.set_in_transaction(&tx_id, "key2", "value2").await?;

// Commit or rollback
client.commit_transaction(&tx_id).await?;
```

### Secondary Indexes
```rust
// Create index
store.create_index(IndexConfig {
    name: "email_idx".to_string(),
    index_type: IndexType::Hash,
    unique: true,
    filter: None,
})?;

// Query using index (automatic on KV operations)
```

### Full-Text Search
```rust
// Create FTS index
let fts = FtsIndex::new("articles".to_string());
fts.index_document("doc1", "Search content here")?;

// Search
let results = fts.search("content", 10);
```

### JSON Documents
```rust
// Store JSON document
let doc = json!({
    "name": "Alice",
    "email": "alice@example.com"
});
json_store.insert("user:1", doc).await?;

// Query with JSONPath
json_store.patch("user:1", "$.email", json!("newemail@example.com")).await?;
```

## 🔧 Configuration

### CLI Options
```bash
buffdb run --addr [::1]:9313 --kv-store kv.db --blob-store blob.db
```

### Backends
| Backend | Feature Flag | Performance | Use Case | Status |
|---------|-------------|-------------|----------|--------|
| SQLite | `vendored-sqlite` | Balanced | General purpose | ✅ Stable |
| DuckDB | `vendored-duckdb` | Analytics | OLAP workloads | ⚠️ Experimental |

## 🏗️ Architecture

BuffDB combines embedded database efficiency with network accessibility:

```
┌─────────────┐     gRPC      ┌─────────────┐
│   Client    │ ◄──────────► │   BuffDB    │
│ (Any Lang)  │               │   Server    │
└─────────────┘               └──────┬──────┘
                                     │
                              ┌──────┴──────┐
                              │   Backend   │
                              │  (SQLite/   │
                              │   DuckDB)   │
                              └─────────────┘
```

## 📊 Performance

- **Binary Size**: <2MB (SQLite backend)
- **Startup Time**: <10ms
- **Throughput**: 100K+ ops/sec (varies by backend)
- **Latency**: <1ms for local operations

## 🤝 Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## 📄 License

Licensed under either [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.

## 🔧 Backend Support

**⚠️ DuckDB Support (Experimental)**

DuckDB backend is currently experimental due to:
- Platform-specific linking issues (especially on macOS)
- Incomplete Rust bindings ([duckdb/duckdb-rs#368](https://github.com/duckdb/duckdb-rs/issues/368))
- Performance optimizations still in progress

For production use, we recommend SQLite backend which is stable and well-tested.

By default, SQLite backend is included and vendored. To use DuckDB (experimental), enable it with `--features duckdb`.

### Command line interface

You can use `buffdb help` to see the commands and flags permitted. The following operations are
currently supported:

- `buffdb run [ADDR]`, starting the server. The default address is `[::1]:9313`.
- `buffdb kv get <KEY>`, printing the value to stdout.
- `buffdb kv set <KEY> <VALUE>`, setting the value.
- `buffdb kv delete <KEY>`, deleting the value.
- `buffdb kv eq [KEYS]...`, exiting successfully if the values for all provided keys are equal.
    Exits with an error code if any two values are not equal.
- `buffdb kv not-eq [KEYS]...`, exiting successfully if the values for all provided keys are
    unique. Exits with an error code if any two values are equal.
- `buffdb blob get <ID>`, printing the data to stdout. Note that this is arbitrary bytes!
- `buffdb blob store <FILE> [METADATA]`, storing the file (use `-` for stdin) and printing the ID
    to stdout. Metadata is optional.
- `buffdb blob update <ID> data <FILE>`, updating the data of the blob. Use `-` for stdin. Metadata
    is unchanged.
- `buffdb blob update <ID> metadata [METADATA]`, updating the metadata of the blob. Data is
    unchanged. Omitting `[METADATA]` will set the metadata to null.
- `buffdb blob update <ID> all <FILE> [METADATA]`, updating both the data and metadata of the blob.
    For `<FILE>`, use `-` for stdin. Omitting `[METADATA]` will set the metadata to null.
- `buffdb blob delete <ID>`, deleting the blob.
- `buffdb blob eq-data [IDS]...`, exiting successfully if the blobs for all provided IDs are equal.
    Exits with an error code if any two blobs are not equal.
- `buffdb blob not-eq-data [IDS]...`, exiting successfully if the blobs for all provided IDs are
    unique. Exits with an error code if any two blobs are equal.

Commands altering a store will exit with an error code if the key/id does not exist. An exception
to this is updating the metadata of a blob to be null, as it is not required to exist beforehand.

All commands for `kv` and `blob` can use `-s`/`--store` to specify which store to use. The defaults
are `kv_store.db` and `blob_store.db` respectively. To select a backend, use `-b`/`--backend`. The
default varies by which backends are enabled.

## 📚 Using BuffDB as a Library

See the [Rust example](#-rust) above for library usage.

## 🎯 Use Cases

- **Offline-First Apps**: Note-taking, games, fieldwork applications, airline systems, collaborative documents
- **IoT & Edge Computing**: Managing device configurations and states locally before cloud sync
- **Low-Bandwidth Environments**: Reducing serialization overhead with Protocol Buffers
- **Embedded Analytics**: Local data processing with optional network access

## 🙏 Acknowledgments

This project is inspired by conversations with Michael Cahill, Professor of Practice, School of Computer Science, University of Sydney, and feedback from edge computing customers dealing with low-bandwidth, high-performance challenges.
