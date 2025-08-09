# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.5.0] - 2025-09-09

### Added
- **ML Model Storage Support**: First-class support for storing and serving machine learning models
  - New `inference` module with `ModelInfo` and `ModelKeys` helpers
  - Dedicated examples for ML model management (`ml_model_inference.rs`, `ml_model_storage.rs`)
  - Integration with Hugging Face Hub for model downloads (`huggingface_integration.rs`)
- **Multi-language ML Examples**: Added ML model management examples for all supported languages:
  - TypeScript/JavaScript
  - Python
  - Java
  - Go
  - Swift (new)
  - Kotlin (new)

### Changed
- **License**: Changed from FSL-1.1-Apache-2.0 to Apache License 2.0
- **Focus**: Shifted primary focus to ML model storage and serving use cases
- Updated all documentation to reflect ML-first approach
- Improved examples to demonstrate real-world ML workflows

### Removed
- Deprecated transaction support over gRPC (still available in backends directly)
- Temporarily disabled DuckDB backend until stabilization
- Removed experimental features: FTS (Full-Text Search), MVCC, JSON Document Store

### Technical Improvements
- Enhanced blob storage for efficient binary model weight storage
- Optimized for edge deployment scenarios
- Improved build configuration for smaller binary sizes
- Better error handling and type safety

### Dependencies
- Added `reqwest` for HTTP downloads (dev dependency)
- Updated all dependencies to latest compatible versions

## [0.4.0] - Previous releases

See git history for changes prior to v0.5.0
