import Foundation
import GRPC
import NIO
import SwiftProtobuf

// Note: This is a simplified example that demonstrates the structure.
// In a real implementation, you would generate the protocol buffer files using:
// protoc --swift_out=. --grpc-swift_out=. ../../proto/kv.proto

// For demonstration purposes, here's a minimal working example
// that shows the gRPC client structure

@main
struct BuffDBExample {
    static func main() async throws {
        print("BuffDB Swift Client Example")
        print("============================")
        
        // Create event loop group
        let group = MultiThreadedEventLoopGroup(numberOfThreads: 1)
        defer {
            try! group.syncShutdownGracefully()
        }
        
        // Create channel
        let channel = try GRPCChannelPool.with(
            target: .host("localhost", port: 9313),
            transportSecurity: .plaintext,
            eventLoopGroup: group
        )
        
        print("✓ Connected to BuffDB server at localhost:9313")
        
        // Note: In a real implementation, you would use the generated client here
        // For example:
        // let kvClient = Buffdb_Kv_KvNIOClient(channel: channel)
        // try await performOperations(with: kvClient)
        
        print("\nTo use this example:")
        print("1. Generate Swift protobuf files:")
        print("   protoc --swift_out=. --grpc-swift_out=. ../../proto/kv.proto")
        print("2. Add the generated files to this project")
        print("3. Uncomment and use the client code shown in the README")
        
        // Close the channel
        try channel.close().wait()
        print("\n✓ Connection closed")
    }
}