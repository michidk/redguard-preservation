//! Redguard Preservation - A library for parsing ROB and 3D model files
//!
//! This library provides parsers for ROB (Redguard Object) files and embedded 3D model data.
//! It can be used both as a library and as a command-line tool.

pub mod model3d;
pub mod parser;
pub mod rob;

// Re-export main types for convenience
pub use model3d::{
    FaceData, FaceNormal, FaceVertex, Model3DFile, Model3DHeader, UVCoord, VertexCoord,
};
pub use parser::{parse_embedded_3d_data, parse_rob_file, parse_rob_with_models};
pub use rob::{RobFile, RobHeader, RobSegment};

/// Result type for parsing operations
pub type ParseResult<T> = Result<T, String>;
