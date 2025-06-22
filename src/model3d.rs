//! 3D Model file structures and parsing
//!
//! This module contains the data structures and parsing logic for 3DC/3D model files.

use nom::{
    IResult, Parser,
    bytes::complete::take,
    number::complete::{le_f32, le_i16, le_u8, le_u16, le_u32},
};

/// Version information for 3DC/3D files
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelVersion {
    V26, // v2.6
    V27, // v2.7
    V40, // v4.0
    V50, // v5.0
    Unknown,
}

/// Texture data parsed from face data
#[derive(Debug, Clone)]
pub enum TextureData {
    SolidColor(u8),
    Texture { texture_id: u16, image_id: u8 },
}

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
    pub texture_data: TextureData,
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
    pub version: ModelVersion,
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

    /// Parse version string into ModelVersion enum
    pub fn parse_version(&self) -> ModelVersion {
        let version_str = self.version_string();
        match version_str.as_str() {
            "v2.6" => ModelVersion::V26,
            "v2.7" => ModelVersion::V27,
            "v4.0" => ModelVersion::V40,
            "v5.0" => ModelVersion::V50,
            _ => ModelVersion::Unknown,
        }
    }

    /// Check if this is a v2.7 or earlier version
    pub fn is_v27_or_earlier(&self) -> bool {
        matches!(self.parse_version(), ModelVersion::V26 | ModelVersion::V27)
    }

    /// Check if this is a v4.0 or later version
    pub fn is_v40_or_later(&self) -> bool {
        matches!(self.parse_version(), ModelVersion::V40 | ModelVersion::V50)
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

/// Parse texture data based on version
pub fn parse_texture_data<'a>(
    input: &'a [u8],
    version: &ModelVersion,
) -> IResult<&'a [u8], TextureData> {
    match version {
        ModelVersion::V26 | ModelVersion::V27 => {
            // v2.6 & 2.7 (3DART) - TextureData is u16
            let (input, texture_data) = le_u16(input)?;

            let texture_id = (texture_data >> 7) as u16;

            // Check if solid color - Note: TEXTURE.000/001 are not used
            if texture_id < 2 {
                let color_index = texture_data as u8;
                Ok((input, TextureData::SolidColor(color_index)))
            } else {
                let image_id = (texture_data & 0x7F) as u8;
                Ok((
                    input,
                    TextureData::Texture {
                        texture_id,
                        image_id,
                    },
                ))
            }
        }
        ModelVersion::V40 | ModelVersion::V50 => {
            // v4.0 & 5.0 (FXART) - TextureData is u32
            let (input, texture_data) = le_u32(input)?;

            // Check if solid color
            if (texture_data >> 20) == 0x0FFF {
                let color_index = (texture_data >> 8) as u8;
                Ok((input, TextureData::SolidColor(color_index)))
            } else {
                // Parse TextureID
                let temp_val = (texture_data >> 8) - 4000000;
                let one = (temp_val / 250) % 40;
                let ten = ((temp_val - (one * 250)) / 1000) % 100;
                let hundred = (temp_val - (one * 250) - (ten * 1000)) / 4000;
                let texture_id = one + ten + hundred;

                // Parse ImageID
                let one = (texture_data & 0xFF) % 10;
                let ten = ((texture_data & 0xFF) / 40) * 10;
                let image_id = (one + ten) as u8;

                Ok((
                    input,
                    TextureData::Texture {
                        texture_id: texture_id as u16,
                        image_id,
                    },
                ))
            }
        }
        ModelVersion::Unknown => {
            // Default to u32 for unknown versions
            let (input, texture_data) = le_u32(input)?;
            Ok((input, TextureData::SolidColor(texture_data as u8)))
        }
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

/// Parse a single face vertex with version-specific vertex index handling
pub fn parse_face_vertex<'a>(
    input: &'a [u8],
    version: &ModelVersion,
) -> IResult<&'a [u8], FaceVertex> {
    let (input, (vertex_index, u, v)) = (
        le_u32, // VertexIndex
        le_i16, // U
        le_i16, // V
    )
        .parse(input)?;

    // In v2.6/2.7, vertex_index is multiplied by 12, so we need to divide by 12
    let adjusted_vertex_index = if matches!(version, ModelVersion::V26 | ModelVersion::V27) {
        vertex_index / 12
    } else {
        vertex_index
    };

    Ok((
        input,
        FaceVertex {
            vertex_index: adjusted_vertex_index,
            u,
            v,
        },
    ))
}

/// Parse face data with version-specific texture data parsing
pub fn parse_face_data<'a>(input: &'a [u8], version: &ModelVersion) -> IResult<&'a [u8], FaceData> {
    let (input, (vertex_count, unk_01)) = (
        le_u8, // VertexCount
        le_u8, // Unk_01
    )
        .parse(input)?;

    let (input, texture_data) = parse_texture_data(input, version)?;

    let (input, unk_04) = le_u32(input)?; // Unk_04

    let mut face_vertices = Vec::new();
    let mut remaining_input = input;

    for _ in 0..vertex_count {
        let (input, vertex) = parse_face_vertex(remaining_input, version)?;
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

/// Parse frame data header (for v2.7 3DC files)
pub fn parse_frame_data_header(input: &[u8]) -> IResult<&[u8], (u32, u32, u32, u32)> {
    let (input, (u1, u2, u3, u4)) = (
        le_u32, // u1
        le_u32, // u2
        le_u32, // u3
        le_u32, // u4
    )
        .parse(input)?;

    Ok((input, (u1, u2, u3, u4)))
}

/// Parse complete 3DC/3D file with version-specific handling
pub fn parse_3d_file(input: &[u8]) -> IResult<&[u8], Model3DFile> {
    let (input, header) = parse_3d_header(input)?;
    let version = header.parse_version();

    // Handle version-specific header field interpretations
    let (_adjusted_offset_uv_data, adjusted_offset_vertex_coords) = if header.is_v27_or_earlier() {
        // For v2.7 or earlier:
        // - NumUVOffsets is the offset to UV Data (which is just all 0s)
        // - OffsetUVData is an offset to an unknown section
        (header.num_uv_offsets, header.offset_uv_data)
    } else {
        (header.offset_uv_data, header.offset_vertex_coords)
    };

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
        let (input, face) = parse_face_data(remaining_input, &version)?;
        face_data.push(face);
        remaining_input = input;
    }

    // Calculate actual vertex coordinates offset for v2.7 3DC files
    let _vertex_coords_offset = if header.is_v27_or_earlier() && frame_data_size > 0 {
        // For v2.7 3DC files, vertex coordinates start after frame data header
        // We need to parse the frame data header to get u3
        if let Ok((_, (_, _, u3, _))) = parse_frame_data_header(&frame_data) {
            header.offset_face_data + (face_data.len() as u32 * 8) + u3 // Approximate face data size
        } else {
            adjusted_offset_vertex_coords
        }
    } else {
        adjusted_offset_vertex_coords
    };

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
            version,
            frame_data: frame_data.to_vec(),
            face_data,
            vertex_coords,
            face_normals,
            uv_offsets,
            uv_coords,
        },
    ))
}
