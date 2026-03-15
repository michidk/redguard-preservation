use super::{RobFile, RobHeader, RobSegment, SegmentBBox};
use log::warn;
use nom::{
    IResult, Parser,
    bytes::complete::{tag, take},
    combinator::opt,
    number::complete::{be_u32, le_u16, le_u32},
};

/// Parses the fixed ROB file header.
pub fn parse_rob_header(input: &[u8]) -> IResult<&[u8], RobHeader> {
    let (input, oarc) = opt(tag("OARC")).parse(input)?;
    if oarc.is_none() {
        warn!("OARC header not found where expected");
    }
    let (input, unknown_04) = be_u32(input)?;
    let (input, num_segments) = le_u32(input)?;
    let (input, oard) = opt(tag("OARD")).parse(input)?;
    if oard.is_none() {
        warn!("OARD header not found where expected");
    }
    let (input, payload_size) = be_u32(input)?;

    Ok((
        input,
        RobHeader {
            unknown_04,
            num_segments,
            payload_size,
        },
    ))
}

/// Parses one ROB segment record and optional payload bytes.
pub fn parse_rob_segment(input: &[u8]) -> IResult<&[u8], RobSegment> {
    let (input, total_size) = le_u32(input)?;
    let (input, segment_name) = take(8u8)(input)?;
    let (input, segment_type) = le_u16(input)?;
    let (input, segment_flags) = le_u16(input)?;

    let (input, segment_attribs) = nom::number::complete::le_u8(input)?;
    let (input, _face_count_low) = take(3usize)(input)?;
    let (input, _unused_14) = be_u32(input)?;
    let (input, _reserved_18) = le_u32(input)?;

    let (input, extent_x) = le_u32(input)?;
    let (input, extent_y) = le_u32(input)?;
    let (input, extent_z) = le_u32(input)?;

    let (input, _reserved_28) = le_u32(input)?;
    let (input, _reserved_2c) = le_u32(input)?;
    let (input, _reserved_30) = le_u32(input)?;

    let (input, positive_x) = le_u32(input)?;
    let (input, positive_y) = le_u32(input)?;
    let (input, positive_z) = le_u32(input)?;

    let (input, negative_x) = le_u32(input)?;
    let (input, negative_y) = le_u32(input)?;
    let (input, negative_z) = le_u32(input)?;

    let (input, data_size) = le_u32(input)?;

    let (input, data) = if data_size > 0 && segment_type != 512 {
        take(data_size as usize)(input)?
    } else {
        (input, &[] as &[u8])
    };

    Ok((
        input,
        RobSegment {
            total_size,
            segment_name: segment_name
                .try_into()
                .expect("ROB segment name must be 8 bytes"),
            segment_type,
            segment_flags,
            segment_attribs,
            _face_count_low: _face_count_low
                .try_into()
                .expect("ROB face_count_low must be 3 bytes"),
            _unused_14,
            bbox: SegmentBBox {
                extent_x,
                extent_y,
                extent_z,
                positive_x,
                positive_y,
                positive_z,
                negative_x,
                negative_y,
                negative_z,
            },
            data_size,
            data: data.to_vec(),
        },
    ))
}

/// Parses a complete ROB file into header and segment records.
pub fn parse_rob_file(input: &[u8]) -> IResult<&[u8], RobFile> {
    let file_size = input.len() as u32;
    let (input, header) = parse_rob_header(input)?;

    // Validated across all 72 shipped ROB files: payload_size == file_size - 24
    // (24 = 20-byte header + 4-byte "END " footer)
    let expected_payload = file_size.saturating_sub(24);
    if header.payload_size != expected_payload {
        warn!(
            "ROB payload_size {} does not match expected {} (file_size {} - 24)",
            header.payload_size, expected_payload, file_size
        );
    }

    let mut segments = Vec::new();
    let mut remaining_input = input;

    for _ in 0..header.num_segments {
        let (input, segment) = parse_rob_segment(remaining_input)?;
        segments.push(segment);
        remaining_input = input;
    }

    let (remaining_input, end_marker) = opt(tag("END ")).parse(remaining_input)?;
    if end_marker.is_none() {
        warn!("END marker not found at end of file");
    }

    Ok((remaining_input, RobFile { header, segments }))
}
