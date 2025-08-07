# BuffDB Language Examples

This directory contains example implementations of BuffDB clients in various programming languages.

## Prerequisites

1. **BuffDB Server**: Ensure BuffDB is running locally:
   ```bash
   buffdb run
   ```

2. **Protocol Buffers**: You'll need `protoc` installed to generate language-specific code:
   ```bash
   # macOS
   brew install protobuf
   
   # Ubuntu/Debian
   sudo apt-get install protobuf-compiler
   ```

## Language Examples

### 🍎 Swift

The Swift example demonstrates how to connect to BuffDB using grpc-swift.

```bash
cd swift

# Generate protobuf files (first time only)
protoc --swift_out=Sources --grpc-swift_out=Sources ../../proto/kv.proto

# Build and run
swift build
swift run
```

**Requirements:**
- Swift 5.9+
- Xcode 15+ (for macOS)

### 🤖 Kotlin

The Kotlin example shows how to use BuffDB with Kotlin coroutines and gRPC.

```bash
cd kotlin

# Generate protobuf files (handled by Gradle)
./gradlew generateProto

# Build and run
./gradlew run
```

**Requirements:**
- JDK 17+
- Gradle 8.0+ (wrapper included)

## Testing All Examples

Run the included test script to verify all examples:

```bash
./test_examples.sh
```

This script will:
1. Check if BuffDB server is running
2. Build and test each language example
3. Report any issues

## Common Patterns

All examples demonstrate:
- Connecting to BuffDB server
- Basic CRUD operations (Create, Read, Update, Delete)
- Streaming API usage
- Error handling
- Graceful shutdown

## Troubleshooting

### Connection Refused
- Ensure BuffDB is running: `buffdb run`
- Check the port (default: 9313)
- Verify no firewall is blocking the connection

### Protobuf Generation Issues
- Ensure `protoc` is installed and in PATH
- Check that proto files exist in `../proto/`
- For language-specific plugins, see each example's README

### Build Failures
- Verify language SDK/runtime versions
- Check dependency versions in build files
- See language-specific error messages for details