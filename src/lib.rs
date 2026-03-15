//! Redguard Preservation - A library for parsing ROB and 3D model files
//!
//! This library provides parsers for ROB (Redguard Object) files and embedded 3D model data.
//! It can be used both as a library and as a command-line tool.

/// Error types used by parsing and conversion operations.
pub mod error;
/// GLTF/GLB conversion utilities for parsed Redguard assets.
pub mod gltf;
/// Parsers and helpers for Redguard file formats.
pub mod import;

// Re-export main types for convenience
pub use gltf::{
    convert_models_to_gltf, convert_positioned_models_to_gltf, convert_wld_scene_to_gltf, to_glb,
};
pub use import::{model3d, rob};

/// Result type for parsing operations
pub type Result<T> = std::result::Result<T, error::Error>;
