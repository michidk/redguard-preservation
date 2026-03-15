/// Low-level ROB binary parser.
pub mod parser;

use crate::{
    Result,
    error::Error,
    model3d::{self, Model3DFile},
};
use log::warn;

#[derive(Debug, Clone)]
/// Parsed ROB file header.
pub struct RobHeader {
    pub unknown_04: u32,
    pub num_segments: u32,
    pub payload_size: u32,
}

#[derive(Debug, Clone)]
/// Axis extents and positive/negative bounds for a ROB segment.
pub struct SegmentBBox {
    pub extent_x: u32,
    pub extent_y: u32,
    pub extent_z: u32,
    pub positive_x: u32,
    pub positive_y: u32,
    pub positive_z: u32,
    pub negative_x: u32,
    pub negative_y: u32,
    pub negative_z: u32,
}

#[derive(Debug, Clone)]
/// Parsed ROB segment record with metadata and optional payload bytes.
pub struct RobSegment {
    pub total_size: u32,
    pub segment_name: [u8; 8],
    pub segment_type: u16,
    pub segment_flags: u16,
    /// Attribute flags byte at offset 0x10. 0x02 = texture preload, 0x40 = special object.
    pub segment_attribs: u8,
    /// Build-tool artifact (face_count mod 256); not read at runtime.
    pub _face_count_low: [u8; 3],
    /// Never read at runtime.
    pub _unused_14: u32,
    pub bbox: SegmentBBox,
    pub data_size: u32,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
/// Parsed ROB archive containing header and segment table.
pub struct RobFile {
    pub header: RobHeader,
    pub segments: Vec<RobSegment>,
}

impl RobSegment {
    /// Returns the segment name decoded from its 8-byte field.
    pub fn name(&self) -> String {
        String::from_utf8_lossy(&self.segment_name)
            .trim_matches('\0')
            .to_string()
    }

    /// Render mode derived from the high byte of `segment_flags`.
    /// Value 0 or >0xFD defaults to 0xFF (standard rendering).
    pub fn render_mode(&self) -> u8 {
        let hi = (self.segment_flags >> 8) as u8;
        if hi == 0 || hi > 0xFD { 0xFF } else { hi }
    }

    /// Returns `true` when this segment stores embedded 3D model bytes.
    pub fn has_embedded_3d_data(&self) -> bool {
        self.segment_type == 0 && self.data_size > 0
    }

    /// Returns `true` when this segment stores embedded special-object bytes.
    pub fn has_special_embedded_data(&self) -> bool {
        self.segment_type == 256 && self.data_size > 0
    }

    /// Returns `true` when this segment references an external file.
    pub fn points_to_external_file(&self) -> bool {
        self.segment_type == 512
    }

    /// Parses embedded 3D bytes in this segment into a `Model3DFile`.
    pub fn parse_embedded_3d_data(&self) -> Result<Model3DFile> {
        if !self.has_embedded_3d_data() {
            return Err(Error::Parse(
                "Segment does not contain embedded 3D data".to_string(),
            ));
        }
        model3d::parse_3d_file(&self.data)
    }
}

/// Parses a ROB file from raw bytes.
pub fn parse_rob_file(input: &[u8]) -> Result<RobFile> {
    match parser::parse_rob_file(input) {
        Ok((remaining, rob_file)) => {
            if !remaining.is_empty() {
                warn!("{} bytes remaining unparsed", remaining.len());
            }
            Ok(rob_file)
        }
        Err(e) => Err(Error::Parse(format!("Failed to parse ROB file: {e:?}"))),
    }
}

/// Parses a ROB file and extracts all segments with embedded 3D models.
pub fn parse_rob_with_models(input: &[u8]) -> Result<(RobFile, Vec<Model3DFile>)> {
    let rob_file = parse_rob_file(input)?;
    let mut models = Vec::new();

    for segment in &rob_file.segments {
        if segment.has_embedded_3d_data() {
            match segment.parse_embedded_3d_data() {
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
