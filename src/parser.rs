//! Combined parser functionality
//!
//! This module provides high-level parsing functions that combine ROB and 3D model parsing.

use crate::{ParseResult, model3d::Model3DFile, rob::RobSegment};
use log::warn;

/// Parse embedded 3D data from a ROB segment
pub fn parse_embedded_3d_data(segment: &RobSegment) -> ParseResult<Model3DFile> {
    if !segment.has_embedded_3d_data() {
        return Err("Segment does not contain embedded 3D data".to_string());
    }

    match crate::model3d::parse_3d_file(&segment.data) {
        Ok((remaining, model)) => {
            if !remaining.is_empty() {
                warn!("{} bytes remaining in embedded 3D data", remaining.len());
            }
            Ok(model)
        }
        Err(e) => Err(format!("Failed to parse embedded 3D data: {:?}", e)),
    }
}

/// Parse a ROB file from bytes
pub fn parse_rob_file(input: &[u8]) -> Result<crate::rob::RobFile, String> {
    match crate::rob::parse_rob_file(input) {
        Ok((remaining, rob_file)) => {
            if !remaining.is_empty() {
                warn!("{} bytes remaining unparsed", remaining.len());
            }
            Ok(rob_file)
        }
        Err(e) => Err(format!("Failed to parse ROB file: {:?}", e)),
    }
}

/// Parse a 3D model file from bytes
pub fn parse_3d_file(input: &[u8]) -> ParseResult<Model3DFile> {
    match crate::model3d::parse_3d_file(input) {
        Ok((remaining, model)) => {
            if !remaining.is_empty() {
                warn!("{} bytes remaining in 3D model data", remaining.len());
            }
            Ok(model)
        }
        Err(e) => Err(format!("Failed to parse 3D model file: {:?}", e)),
    }
}

/// Parse a ROB file and extract all embedded 3D models
pub fn parse_rob_with_models(
    input: &[u8],
) -> Result<(crate::rob::RobFile, Vec<Model3DFile>), String> {
    let rob_file = parse_rob_file(input)?;
    let mut models = Vec::new();

    for segment in &rob_file.segments {
        if segment.has_embedded_3d_data() {
            match parse_embedded_3d_data(segment) {
                Ok(model) => models.push(model),
                Err(e) => {
                    warn!(
                        "Failed to parse embedded 3D data in segment '{}': {}",
                        segment.name(),
                        e
                    );
                }
            }
        }
    }

    Ok((rob_file, models))
}
