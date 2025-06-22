//! ROB (Redguard Object) file structures and parsing
//!
//! This module contains the data structures and parsing logic for ROB files.

use log::warn;
use nom::{
    IResult, Parser,
    bytes::complete::{tag, take},
    combinator::opt,
    number::complete::le_u32,
};

/// Header structure for ROB files
#[derive(Debug, Clone)]
pub struct RobHeader {
    pub unknown1: u32,
    pub num_segments: u32,
    pub unknown2: u32,
}

/// Segment structure within ROB files
#[derive(Debug, Clone)]
pub struct RobSegment {
    pub unknown1: u32,
    pub segment_name: [u8; 8],
    pub unknown2: u32,
    pub unknown3: [u32; 15],
    pub size: u32,
    pub data: Vec<u8>,
}

/// Complete ROB file structure
#[derive(Debug, Clone)]
pub struct RobFile {
    pub header: RobHeader,
    pub segments: Vec<RobSegment>,
}

impl RobSegment {
    /// Get the segment name as a string, trimming null bytes
    pub fn name(&self) -> String {
        String::from_utf8_lossy(&self.segment_name)
            .trim_matches('\0')
            .to_string()
    }

    /// Check if this segment contains embedded 3D data
    pub fn has_embedded_3d_data(&self) -> bool {
        self.unknown2 == 0 && self.size > 0
    }

    /// Check if this segment points to an external 3DC file
    pub fn points_to_external_file(&self) -> bool {
        self.unknown2 == 512
    }
}

/// Parse the ROB header (36 bytes), warning if OARC or OARD are missing
pub fn parse_rob_header(input: &[u8]) -> IResult<&[u8], RobHeader> {
    let (input, oarc) = opt(tag("OARC")).parse(input)?;
    if oarc.is_none() {
        warn!("OARC header not found where expected");
    }
    let (input, unknown1) = le_u32(input)?;
    let (input, num_segments) = le_u32(input)?;
    let (input, oard) = opt(tag("OARD")).parse(input)?;
    if oard.is_none() {
        warn!("OARD header not found where expected");
    }
    let (input, unknown2) = le_u32(input)?;

    Ok((
        input,
        RobHeader {
            unknown1,
            num_segments,
            unknown2,
        },
    ))
}

/// Parse a single ROB segment (80-byte header + data)
pub fn parse_rob_segment(input: &[u8]) -> IResult<&[u8], RobSegment> {
    let (input, (unknown1, segment_name, unknown2)) = (
        le_u32,    // Unknown1 (4 bytes)
        take(8u8), // SegmentName (8 bytes)
        le_u32,    // Unknown2 (4 bytes)
    )
        .parse(input)?;

    let (input, unknown3) = take(60u8)(input)?; // Unknown3 (15 * 4 = 60 bytes)
    let (input, size) = le_u32(input)?; // Size (4 bytes)

    // Parse the 15 unknown3 values
    let mut unknown3_array = [0u32; 15];
    for (i, chunk) in unknown3.chunks(4).enumerate() {
        if i < 15 && chunk.len() == 4 {
            unknown3_array[i] = u32::from_le_bytes(chunk.try_into().unwrap());
        }
    }

    // Parse the data if size > 0 and unknown2 != 512
    let (input, data) = if size > 0 && unknown2 != 512 {
        take(size as usize)(input)?
    } else {
        (input, &[] as &[u8])
    };

    Ok((
        input,
        RobSegment {
            unknown1,
            segment_name: segment_name.try_into().unwrap(),
            unknown2,
            unknown3: unknown3_array,
            size,
            data: data.to_vec(),
        },
    ))
}

/// Parse the entire ROB file, warning if END is missing
pub fn parse_rob_file(input: &[u8]) -> IResult<&[u8], RobFile> {
    let (input, header) = parse_rob_header(input)?;

    let mut segments = Vec::new();
    let mut remaining_input = input;

    for _ in 0..header.num_segments {
        let (input, segment) = parse_rob_segment(remaining_input)?;
        segments.push(segment);
        remaining_input = input;
    }

    // Check for END marker ("END ")
    let (remaining_input, end_marker) = opt(tag("END ")).parse(remaining_input)?;
    if end_marker.is_none() {
        warn!("END marker not found at end of file");
    }

    Ok((remaining_input, RobFile { header, segments }))
}
