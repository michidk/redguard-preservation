//! 3D Model file structures and parsing
//!
//! This module contains the data structures and parsing logic for 3DC/3D model files.

use nom::{
    IResult, Parser,
    bytes::complete::take,
    number::complete::{le_f32, le_i16, le_u8, le_u32},
};

/// Header structure for 3DC/3D model files
#[derive(Debug, Clone)]
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

/// Vertex within a face
#[derive(Debug, Clone)]
pub struct FaceVertex {
    pub vertex_index: u32,
    pub u: i16,
    pub v: i16,
}

/// Face data structure
#[derive(Debug, Clone)]
pub struct FaceData {
    pub vertex_count: u8,
    pub unk_01: u8,
    pub texture_data: u32,
    pub unk_04: u32,
    pub face_vertices: Vec<FaceVertex>,
}

/// 3D vertex coordinate
#[derive(Debug, Clone)]
pub struct VertexCoord {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// Face normal vector
#[derive(Debug, Clone)]
pub struct FaceNormal {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// UV coordinate
#[derive(Debug, Clone)]
pub struct UVCoord {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// Complete 3D model file structure
#[derive(Debug, Clone)]
pub struct Model3DFile {
    pub header: Model3DHeader,
    pub frame_data: Vec<u8>,
    pub face_data: Vec<FaceData>,
    pub vertex_coords: Vec<VertexCoord>,
    pub face_normals: Vec<FaceNormal>,
    pub uv_offsets: Vec<u32>,
    pub uv_coords: Vec<UVCoord>,
}

impl Model3DHeader {
    /// Get the version as a string, trimming null bytes
    pub fn version_string(&self) -> String {
        String::from_utf8_lossy(&self.version)
            .trim_matches('\0')
            .to_string()
    }
}

impl Model3DFile {
    /// Get the total number of vertices across all faces
    pub fn total_face_vertices(&self) -> usize {
        self.face_data
            .iter()
            .map(|face| face.vertex_count as usize)
            .sum()
    }

    /// Get the bounding box of the model
    pub fn bounding_box(&self) -> Option<(VertexCoord, VertexCoord)> {
        if self.vertex_coords.is_empty() {
            return None;
        }

        let mut min = VertexCoord {
            x: f32::INFINITY,
            y: f32::INFINITY,
            z: f32::INFINITY,
        };
        let mut max = VertexCoord {
            x: f32::NEG_INFINITY,
            y: f32::NEG_INFINITY,
            z: f32::NEG_INFINITY,
        };

        for vertex in &self.vertex_coords {
            min.x = min.x.min(vertex.x);
            min.y = min.y.min(vertex.y);
            min.z = min.z.min(vertex.z);
            max.x = max.x.max(vertex.x);
            max.y = max.y.max(vertex.y);
            max.z = max.z.max(vertex.z);
        }

        Some((min, max))
    }
}

/// Parse 3DC/3D header (64 bytes)
pub fn parse_3d_header(input: &[u8]) -> IResult<&[u8], Model3DHeader> {
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

/// Parse a single face vertex
pub fn parse_face_vertex(input: &[u8]) -> IResult<&[u8], FaceVertex> {
    let (input, (vertex_index, u, v)) = (
        le_u32, // VertexIndex
        le_i16, // U
        le_i16, // V
    )
        .parse(input)?;

    Ok((input, FaceVertex { vertex_index, u, v }))
}

/// Parse face data
pub fn parse_face_data(input: &[u8]) -> IResult<&[u8], FaceData> {
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

/// Parse vertex coordinates
pub fn parse_vertex_coord(input: &[u8]) -> IResult<&[u8], VertexCoord> {
    let (input, (x, y, z)) = (
        le_f32, // x
        le_f32, // y
        le_f32, // z
    )
        .parse(input)?;

    Ok((input, VertexCoord { x, y, z }))
}

/// Parse face normal
pub fn parse_face_normal(input: &[u8]) -> IResult<&[u8], FaceNormal> {
    let (input, (x, y, z)) = (
        le_f32, // x
        le_f32, // y
        le_f32, // z
    )
        .parse(input)?;

    Ok((input, FaceNormal { x, y, z }))
}

/// Parse UV coordinate
pub fn parse_uv_coord(input: &[u8]) -> IResult<&[u8], UVCoord> {
    let (input, (x, y, z)) = (
        le_f32, // x
        le_f32, // y
        le_f32, // z
    )
        .parse(input)?;

    Ok((input, UVCoord { x, y, z }))
}

/// Parse complete 3DC/3D file
pub fn parse_3d_file(input: &[u8]) -> IResult<&[u8], Model3DFile> {
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
