#!/bin/bash

# Script to test BuffDB language examples

echo "BuffDB Example Test Script"
echo "=========================="

# Check if BuffDB is running
if ! nc -z localhost 9313 2>/dev/null; then
    echo "⚠️  BuffDB server is not running on localhost:9313"
    echo "   Please start it with: buffdb run"
    exit 1
fi

echo "✓ BuffDB server is running"

# Test Swift example
echo -e "\n📱 Testing Swift Example..."
cd swift
if command -v swift &> /dev/null; then
    echo "Building Swift example..."
    swift build
    if [ $? -eq 0 ]; then
        echo "✓ Swift example builds successfully"
        echo "Running Swift example..."
        swift run
    else
        echo "✗ Swift example build failed"
    fi
else
    echo "⚠️  Swift not installed. Skipping Swift example."
fi
cd ..

# Test Kotlin example
echo -e "\n🤖 Testing Kotlin Example..."
cd kotlin
if command -v gradle &> /dev/null || [ -f "./gradlew" ]; then
    echo "Building Kotlin example..."
    if [ -f "./gradlew" ]; then
        ./gradlew build
    else
        gradle build
    fi
    if [ $? -eq 0 ]; then
        echo "✓ Kotlin example builds successfully"
        echo "Running Kotlin example..."
        if [ -f "./gradlew" ]; then
            ./gradlew run
        else
            gradle run
        fi
    else
        echo "✗ Kotlin example build failed"
    fi
else
    echo "⚠️  Gradle not installed. Skipping Kotlin example."
fi
cd ..

echo -e "\n✓ Example testing complete!"