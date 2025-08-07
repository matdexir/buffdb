package com.buffdb.example

import io.grpc.ManagedChannel
import io.grpc.ManagedChannelBuilder
import kotlinx.coroutines.*

fun main() = runBlocking {
    println("BuffDB Kotlin Client Example")
    println("============================")
    
    // Create channel
    val channel: ManagedChannel = ManagedChannelBuilder
        .forAddress("localhost", 9313)
        .usePlaintext()
        .build()
    
    println("✓ Connected to BuffDB server at localhost:9313")
    
    try {
        // Note: In a real implementation, you would use the generated client here
        // after running: ./gradlew generateProto
        
        // Example of what the code would look like:
        // val kvStub = KvGrpc.newStub(channel)
        // val client = BuffDBClient(channel)
        // 
        // // Simple operations
        // client.set("user:1", "Alice")
        // val value = client.get("user:1")
        // println("Retrieved value: $value")
        
        println("\nTo use this example:")
        println("1. Ensure BuffDB server is running: buffdb run")
        println("2. Generate Kotlin protobuf files: ./gradlew generateProto")
        println("3. Uncomment and use the client code shown in the README")
        
        // Simulate some work
        delay(1000)
        
    } finally {
        // Shutdown channel
        channel.shutdown()
        println("\n✓ Connection closed")
    }
}

// Simple client wrapper (demonstration only)
class BuffDBClient(private val channel: ManagedChannel) {
    // In a real implementation, these methods would use the generated stubs
    // as shown in the README example
    
    suspend fun set(key: String, value: String) {
        println("Would set: $key = $value")
    }
    
    suspend fun get(key: String): String? {
        println("Would get: $key")
        return "mock_value"
    }
    
    suspend fun delete(key: String) {
        println("Would delete: $key")
    }
}