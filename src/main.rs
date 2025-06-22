use nom::{
    IResult, Parser,
    bytes::complete::{tag, take},
    combinator::opt,
    number::complete::{le_f32, le_i16, le_u8, le_u32},
};

#[derive(Debug)]
pub struct RobHeader {
    pub unknown1: u32,
    pub num_segments: u32,
    pub unknown2: u32,
}

#[derive(Debug)]
pub struct RobSegment {
    pub unknown1: u32,
    pub segment_name: [u8; 8],
    pub unknown2: u32,
    pub unknown3: [u32; 15],
    pub size: u32,
    pub data: Vec<u8>,
}

#[derive(Debug)]
pub struct RobFile {
    pub header: RobHeader,
    pub segments: Vec<RobSegment>,
}

// 3DC/3D File structures
#[derive(Debug)]
pub struct Model3DHeader {
    pub version: [u8; 4],
    pub num_vertices: u32,
    pub num_faces: u32,
    pub radius: u32,
    pub num_frames: u32,
    pub offset_frame_data: u32,
    pub num_uv_offsets: u32,
    pub offset_section4: u32,
    pub section4_count: u32,
    pub unknown4: u32,
    pub offset_uv_offsets: u32,
    pub offset_uv_data: u32,
    pub offset_vertex_coords: u32,
    pub offset_face_normals: u32,
    pub num_uv_offsets2: u32,
    pub offset_face_data: u32,
}

#[derive(Debug)]
pub struct FaceVertex {
    pub vertex_index: u32,
    pub u: i16,
    pub v: i16,
}

#[derive(Debug)]
pub struct FaceData {
    pub vertex_count: u8,
    pub unk_01: u8,
    pub texture_data: u32,
    pub unk_04: u32,
    pub face_vertices: Vec<FaceVertex>,
}

#[derive(Debug)]
pub struct VertexCoord {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug)]
pub struct FaceNormal {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug)]
pub struct UVCoord {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug)]
pub struct Model3DFile {
    pub header: Model3DHeader,
    pub frame_data: Vec<u8>,
    pub face_data: Vec<FaceData>,
    pub vertex_coords: Vec<VertexCoord>,
    pub face_normals: Vec<FaceNormal>,
    pub uv_offsets: Vec<u32>,
    pub uv_coords: Vec<UVCoord>,
}

// Parse the ROB header (36 bytes), warning if OARC or OARD are missing
fn parse_rob_header(input: &[u8]) -> IResult<&[u8], RobHeader> {
    let (input, oarc) = opt(tag("OARC")).parse(input)?;
    if oarc.is_none() {
        eprintln!("Warning: OARC header not found where expected");
    }
    let (input, unknown1) = le_u32(input)?;
    let (input, num_segments) = le_u32(input)?;
    let (input, oard) = opt(tag("OARD")).parse(input)?;
    if oard.is_none() {
        eprintln!("Warning: OARD header not found where expected");
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

// Parse a single ROB segment (80-byte header + data)
fn parse_rob_segment(input: &[u8]) -> IResult<&[u8], RobSegment> {
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

// Parse the entire ROB file, warning if END is missing
fn parse_rob_file(input: &[u8]) -> IResult<&[u8], RobFile> {
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
        eprintln!("Warning: END marker not found at end of file");
    }

    Ok((remaining_input, RobFile { header, segments }))
}

// Parse 3DC/3D header (64 bytes)
fn parse_3d_header(input: &[u8]) -> IResult<&[u8], Model3DHeader> {
    let (
        input,
        (
            version,
            num_vertices,
            num_faces,
            radius,
            num_frames,
            offset_frame_data,
            num_uv_offsets,
            offset_section4,
            section4_count,
            unknown4,
            offset_uv_offsets,
            offset_uv_data,
            offset_vertex_coords,
            offset_face_normals,
            num_uv_offsets2,
            offset_face_data,
        ),
    ) = (
        take(4u8), // Version[4]
        le_u32,    // NumVertices
        le_u32,    // NumFaces
        le_u32,    // Radius
        le_u32,    // NumFrames
        le_u32,    // OffsetFrameData
        le_u32,    // NumUVOffsets
        le_u32,    // OffsetSection4
        le_u32,    // Section4Count
        le_u32,    // Unknown4
        le_u32,    // OffsetUVOffsets
        le_u32,    // OffsetUVData
        le_u32,    // OffsetVertexCoors
        le_u32,    // OffsetFaceNormals
        le_u32,    // NumUVOffsets2
        le_u32,    // OffsetFaceData
    )
        .parse(input)?;

    Ok((
        input,
        Model3DHeader {
            version: version.try_into().unwrap(),
            num_vertices,
            num_faces,
            radius,
            num_frames,
            offset_frame_data,
            num_uv_offsets,
            offset_section4,
            section4_count,
            unknown4,
            offset_uv_offsets,
            offset_uv_data,
            offset_vertex_coords,
            offset_face_normals,
            num_uv_offsets2,
            offset_face_data,
        },
    ))
}

// Parse a single face vertex
fn parse_face_vertex(input: &[u8]) -> IResult<&[u8], FaceVertex> {
    let (input, (vertex_index, u, v)) = (
        le_u32, // VertexIndex
        le_i16, // U
        le_i16, // V
    )
        .parse(input)?;

    Ok((input, FaceVertex { vertex_index, u, v }))
}

// Parse face data
fn parse_face_data(input: &[u8]) -> IResult<&[u8], FaceData> {
    let (input, (vertex_count, unk_01, texture_data, unk_04)) = (
        le_u8,  // VertexCount
        le_u8,  // Unk_01
        le_u32, // TextureData
        le_u32, // Unk_04
    )
        .parse(input)?;

    let mut face_vertices = Vec::new();
    let mut remaining_input = input;

    for _ in 0..vertex_count {
        let (input, vertex) = parse_face_vertex(remaining_input)?;
        face_vertices.push(vertex);
        remaining_input = input;
    }

    Ok((
        remaining_input,
        FaceData {
            vertex_count,
            unk_01,
            texture_data,
            unk_04,
            face_vertices,
        },
    ))
}

// Parse vertex coordinates
fn parse_vertex_coord(input: &[u8]) -> IResult<&[u8], VertexCoord> {
    let (input, (x, y, z)) = (
        le_f32, // x
        le_f32, // y
        le_f32, // z
    )
        .parse(input)?;

    Ok((input, VertexCoord { x, y, z }))
}

// Parse face normal
fn parse_face_normal(input: &[u8]) -> IResult<&[u8], FaceNormal> {
    let (input, (x, y, z)) = (
        le_f32, // x
        le_f32, // y
        le_f32, // z
    )
        .parse(input)?;

    Ok((input, FaceNormal { x, y, z }))
}

// Parse UV coordinate
fn parse_uv_coord(input: &[u8]) -> IResult<&[u8], UVCoord> {
    let (input, (x, y, z)) = (
        le_f32, // x
        le_f32, // y
        le_f32, // z
    )
        .parse(input)?;

    Ok((input, UVCoord { x, y, z }))
}

// Parse complete 3DC/3D file
fn parse_3d_file(input: &[u8]) -> IResult<&[u8], Model3DFile> {
    let (input, header) = parse_3d_header(input)?;

    // Parse frame data (if any)
    let frame_data_size = if header.offset_frame_data > 64 {
        header.offset_frame_data - 64
    } else {
        0
    };

    let (input, frame_data) = if frame_data_size > 0 {
        take(frame_data_size as usize)(input)?
    } else {
        (input, &[] as &[u8])
    };

    // Parse face data
    let mut face_data = Vec::new();
    let mut remaining_input = input;

    for _ in 0..header.num_faces {
        let (input, face) = parse_face_data(remaining_input)?;
        face_data.push(face);
        remaining_input = input;
    }

    // Parse vertex coordinates
    let mut vertex_coords = Vec::new();
    for _ in 0..header.num_vertices {
        let (input, vertex) = parse_vertex_coord(remaining_input)?;
        vertex_coords.push(vertex);
        remaining_input = input;
    }

    // Parse face normals
    let mut face_normals = Vec::new();
    for _ in 0..header.num_faces {
        let (input, normal) = parse_face_normal(remaining_input)?;
        face_normals.push(normal);
        remaining_input = input;
    }

    // Parse UV offsets
    let mut uv_offsets = Vec::new();
    for _ in 0..header.num_uv_offsets {
        let (input, offset) = le_u32(remaining_input)?;
        uv_offsets.push(offset);
        remaining_input = input;
    }

    // Parse UV coordinates
    let mut uv_coords = Vec::new();
    let num_uv_coords = header.num_uv_offsets; // This might need adjustment based on actual data
    for _ in 0..num_uv_coords {
        let (input, coord) = parse_uv_coord(remaining_input)?;
        uv_coords.push(coord);
        remaining_input = input;
    }

    Ok((
        remaining_input,
        Model3DFile {
            header,
            frame_data: frame_data.to_vec(),
            face_data,
            vertex_coords,
            face_normals,
            uv_offsets,
            uv_coords,
        },
    ))
}

// Parse embedded 3D data from a ROB segment
fn parse_embedded_3d_data(segment: &RobSegment) -> Result<Model3DFile, String> {
    if segment.unknown2 != 0 {
        return Err("Segment does not contain embedded 3D data".to_string());
    }

    match parse_3d_file(&segment.data) {
        Ok((remaining, model)) => {
            if !remaining.is_empty() {
                eprintln!(
                    "Warning: {} bytes remaining in embedded 3D data",
                    remaining.len()
                );
            }
            Ok(model)
        }
        Err(e) => Err(format!("Failed to parse embedded 3D data: {:?}", e)),
    }
}

fn main() {
    // Read the ROB file
    let file_content = std::fs::read("BELLTOWR_glide.ROB").expect("Failed to read file");

    match parse_rob_file(&file_content) {
        Ok((remaining, rob_file)) => {
            println!("Successfully parsed ROB file!");
            println!("Header: {:?}", rob_file.header);
            println!("Number of segments: {}", rob_file.segments.len());

            // Print segment names and parse embedded 3D data
            for (i, segment) in rob_file.segments.iter().enumerate() {
                let name = String::from_utf8_lossy(&segment.segment_name);
                let clean_name = name.trim_matches('\0');

                if segment.unknown2 == 512 {
                    println!(
                        "Segment {}: '{}' points to external 3DC file",
                        i, clean_name
                    );
                } else {
                    println!(
                        "Segment {}: '{}' embeds data (size: {})",
                        i, clean_name, segment.size
                    );

                    // Try to parse embedded 3D data
                    match parse_embedded_3d_data(segment) {
                        Ok(model) => {
                            let version = String::from_utf8_lossy(&model.header.version);
                            println!(
                                "  -> Embedded 3D model: version {}, {} vertices, {} faces",
                                version.trim_matches('\0'),
                                model.header.num_vertices,
                                model.header.num_faces
                            );
                            println!("     - Vertex count: {}", model.vertex_coords.len());
                            println!("     - Face count:   {}", model.face_data.len());
                            println!("     - Normal count: {}", model.face_normals.len());
                            println!("     - UV count:     {}", model.uv_coords.len());
                        }
                        Err(e) => {
                            println!("  -> Failed to parse as 3D model: {}", e);
                        }
                    }
                }
            }

            if !remaining.is_empty() {
                println!("Warning: {} bytes remaining unparsed", remaining.len());
            }
        }
        Err(e) => {
            eprintln!("Failed to parse ROB file: {:?}", e);
        }
    }
}
