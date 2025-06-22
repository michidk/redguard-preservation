//! 3D Model file structures and parsing
//!
//! This module contains the data structures and parsing logic for 3DC/3D model files.

use log::{debug, info};
use nom::{
    IResult, Parser,
    bytes::complete::take,
    number::complete::{le_f32, le_i16, le_i32, le_u8, le_u16, le_u32},
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
    pub unknown4: u32, // always 0
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

impl FaceData {
    // Calculate the size of the face data in bytes
    pub fn size_in_bytes(&self, version: &ModelVersion) -> usize {
        let texture_data_size = if matches!(version, ModelVersion::V40 | ModelVersion::V50) {
            1 + 2 + 2 // unk_01 + u1 + u2 for v4/v5
        } else {
            1 + 1 + 2 // unk_01 + u1 + u2 for v2.6/v2.7
        };

        let vertex_data_size = self.face_vertices.len() * (4 + 2 + 2); // vertex_index + u + v
        1 + texture_data_size + 4 + vertex_data_size // vertex_count + texture_data + unk_04 + vertices
    }
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
        le_u32,    // NumTextures (previously Unknown4)
        le_u32,    // OffsetUVOffsets
        le_u32,    // OffsetUVData
        le_u32,    // OffsetVertexCoors
        le_u32,    // OffsetFaceNormals
        le_u32,    // NumUVOffsets2
        le_u32,    // OffsetFaceData
    )
        .parse(input)?;

    let mut header = Model3DHeader {
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
    };

    // Fixup for v2.7 and below
    let version_str = String::from_utf8_lossy(&header.version);
    if version_str.starts_with("v2.6") || version_str.starts_with("v2.7") {
        // Swap/fix UV offset/data fields
        let tmp = header.num_uv_offsets;
        header.num_uv_offsets = 0;
        header.offset_uv_offsets = tmp;
        let tmp2 = header.offset_uv_data;
        header.offset_uv_data = header.offset_uv_offsets;
        header.offset_uv_offsets = tmp2;
    }

    Ok((input, header))
}

/// Parse face data with version-specific texture data parsing
pub fn parse_face_data<'a>(input: &'a [u8], version: &ModelVersion) -> IResult<&'a [u8], FaceData> {
    debug!(
        "Starting face data parsing, first 16 bytes: {:02x?}",
        &input[..input.len().min(16)]
    );

    let (input, vertex_count) = le_u8(input)?;
    debug!("Read vertex_count: {}", vertex_count);

    let (input, texture_data_raw, unk_01_val) =
        if matches!(version, ModelVersion::V40 | ModelVersion::V50) {
            // v4.0/5.0: U1(u16), U2(u16), U3(u8)
            let (input, u1) = le_u16(input)?;
            let (input, u2) = le_u16(input)?;
            let (input, u3) = le_u8(input)?;
            let raw = ((u3 as u32) << 24) | ((u2 as u32) << 8) | (u1 as u32);
            debug!(
                "v4.0/5.0 texture fields: u1=0x{:04x}, u2=0x{:04x}, u3=0x{:02x}, raw=0x{:08x}",
                u1, u2, u3, raw
            );
            (input, raw, u3)
        } else {
            // v2.6/2.7: U1(u8), U2(u16), U3=0
            let (input, u1) = le_u8(input)?;
            let (input, u2) = le_u16(input)?;
            let raw = ((u2 as u32) << 8) | (u1 as u32);
            debug!(
                "v2.6/2.7 texture fields: u1=0x{:02x}, u2=0x{:04x}, raw=0x{:08x}",
                u1, u2, raw
            );
            (input, raw, u1)
        };

    let (input, unk_04) = le_u32(input)?;
    debug!("Read unk_04: 0x{:08x}", unk_04);

    let texture_data = parse_texture_data_value(texture_data_raw, version);
    debug!("Parsed texture_data: {:?}", texture_data);

    if unk_04 != 0 {
        debug!("Note: Non-zero Unk_04 in face data: {}", unk_04);
    }

    debug!(
        "After face header, remaining bytes for vertices: {:02x?}",
        &input[..input.len().min(20)]
    );
    debug!("Expected to parse {} vertices", vertex_count);

    // Step 5: Parse face vertices with cumulative U/V (same for all versions!)
    let mut face_vertices = Vec::with_capacity(vertex_count as usize);
    let mut acc_u = 0i16;
    let mut acc_v = 0i16;
    let mut input = input;

    for i in 0..vertex_count {
        debug!(
            "Parsing face vertex {}, remaining bytes: {:02x?}",
            i,
            &input[..input.len().min(12)]
        );

        // Read vertex index (uint32 for all versions)
        let (input2, vertex_index_raw) = le_u32(input)?;

        // Read U and V (int16 for all versions)
        let (input2, u) = le_i16(input2)?;
        let (input2, v) = le_i16(input2)?;

        // Apply version-specific vertex index adjustment per documentation:
        // "This is multiplied by 12 in v2.6/2.7 so this must be divided by 12 to get correct index for those versions"
        let vertex_index = if matches!(
            version,
            ModelVersion::V26 | ModelVersion::V27 | ModelVersion::Unknown
        ) {
            // v2.7 and below: vertex_index is byte offset, divide by 12
            vertex_index_raw / 12
        } else {
            // v4.0+: vertex_index is already correct
            vertex_index_raw
        };

        debug!(
            "Raw vertex_index: {} (0x{:08x}), adjusted: {}, u: {}, v: {}",
            vertex_index_raw, vertex_index_raw, vertex_index, u, v
        );

        // Accumulate U/V values (reference: U = ReadInt16() + u; V = ReadInt16() + v)
        acc_u = acc_u.wrapping_add(u);
        acc_v = acc_v.wrapping_add(v);

        face_vertices.push(FaceVertex {
            vertex_index,
            u: acc_u,
            v: acc_v,
        });

        input = input2;
    }

    Ok((
        input,
        FaceData {
            vertex_count,
            unk_01: unk_01_val,
            texture_data,
            unk_04,
            face_vertices,
        },
    ))
}

/// Helper to parse texture_data value into TextureData enum
pub fn parse_texture_data_value(raw: u32, version: &ModelVersion) -> TextureData {
    match version {
        ModelVersion::V26 | ModelVersion::V27 | ModelVersion::Unknown => {
            // Reference: faceData.TextureId = (faceData.TextureData >> 7);
            let texture_id = (raw >> 7) as u16;

            // Reference: if(faceData.TextureId < 2)
            if texture_id < 2 {
                // Reference: faceData.ColorIndex = (byte)(faceData.TextureData);
                let color_index = raw as u8;
                TextureData::SolidColor(color_index)
            } else {
                // Reference: faceData.ImageId = (byte)(faceData.TextureData & 0x7f);
                let image_id = (raw & 0x7F) as u8;
                TextureData::Texture {
                    texture_id,
                    image_id,
                }
            }
        }
        ModelVersion::V40 | ModelVersion::V50 => {
            // Reference: if((faceData.TextureData >> 20) == 0x0FFF)
            if (raw >> 20) == 0x0FFF {
                // Reference: faceData.ColorIndex = (byte)(faceData.TextureData>>8);
                let color_index = (raw >> 8) as u8;
                TextureData::SolidColor(color_index)
            } else {
                // Reference complex texture ID calculation:
                // uint tmp = (faceData.TextureData >>8)-4000000;
                // uint one = (tmp/250)%40;
                // uint ten = ((tmp-(one*250))/1000)%100;
                // uint hundred = (tmp-(one*250)-(ten*1000))/4000;
                // faceData.TextureId = one+ten+hundred;
                let tmp = (raw >> 8).wrapping_sub(4000000);
                let one = (tmp / 250) % 40;
                let ten = ((tmp - (one * 250)) / 1000) % 100;
                let hundred = (tmp - (one * 250) - (ten * 1000)) / 4000;
                let texture_id = (one + ten + hundred) as u16;

                // Reference image ID calculation:
                // one = (faceData.TextureData& 0xFF)%10;
                // ten = ((faceData.TextureData& 0xFF)/40)*10;
                // faceData.ImageId = one+ten;
                let one = (raw & 0xFF) % 10;
                let ten = ((raw & 0xFF) / 40) * 10;
                let image_id = (one + ten) as u8;

                TextureData::Texture {
                    texture_id,
                    image_id,
                }
            }
        }
    }
}

/// Parse vertex coordinates
pub fn parse_vertex_coord(input: &[u8]) -> IResult<&[u8], VertexCoord> {
    // According to UESP documentation and reference implementation,
    // vertex coordinates are stored as dword (32-bit integers)
    // But they might be signed values
    let (input, (x_raw, y_raw, z_raw)) = (
        le_i32, // x - signed dword
        le_i32, // y - signed dword
        le_i32, // z - signed dword
    )
        .parse(input)?;

    // Use the exact scaling factor from reference implementation
    let scale = 1.0 / 256.0;
    let x = x_raw as f32 * scale;
    let y = y_raw as f32 * scale;
    let z = z_raw as f32 * scale;

    Ok((input, VertexCoord { x, y, z }))
}

/// Parse face normal
pub fn parse_face_normal(input: &[u8]) -> IResult<&[u8], FaceNormal> {
    // According to UESP documentation and reference implementation,
    // face normals are stored as dword (32-bit integers)
    let (input, (x_raw, y_raw, z_raw)) = (
        le_i32, // x - signed dword
        le_i32, // y - signed dword
        le_i32, // z - signed dword
    )
        .parse(input)?;

    // Use the exact scaling factor from reference implementation
    let scale = 1.0 / 256.0;
    let x = x_raw as f32 * scale;
    let y = y_raw as f32 * scale;
    let z = z_raw as f32 * scale;

    Ok((input, FaceNormal { x, y, z }))
}

/// Parse UV coordinate
pub fn parse_uv_coord(input: &[u8]) -> IResult<&[u8], UVCoord> {
    let (input, (x_raw, y_raw, z_raw)) = (
        le_f32, // x
        le_f32, // y
        le_f32, // z
    )
        .parse(input)?;
    // If you ever parse as integer, use 1/4096.0 scaling
    Ok((
        input,
        UVCoord {
            x: x_raw,
            y: y_raw,
            z: z_raw,
        },
    ))
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

/// Parse frame data section from the 3D file
fn parse_frame_data_section(input: &[u8], header: &Model3DHeader) -> Vec<u8> {
    let frame_data_size = if header.offset_frame_data > 64 {
        header.offset_frame_data - 64
    } else {
        0
    };

    let frame_data = if frame_data_size > 0 && header.offset_frame_data < input.len() as u32 {
        let start = header.offset_frame_data as usize;
        let end = (header.offset_frame_data + frame_data_size) as usize;
        if end <= input.len() {
            &input[start..end]
        } else {
            &[]
        }
    } else {
        &[]
    };

    debug!(
        "Frame data size: {}, actual data: {} bytes",
        frame_data_size,
        frame_data.len()
    );
    frame_data.to_vec()
}

/// Parse Section4 data from the 3D file
fn parse_section4_data_section(input: &[u8], header: &Model3DHeader) -> Vec<u8> {
    let section4_data = if header.offset_section4 > 0 && header.offset_section4 < input.len() as u32
    {
        let start = header.offset_section4 as usize;
        let end = if header.section4_count > 0 {
            start + (header.section4_count as usize * 4) // Assuming 4 bytes per entry
        } else {
            input.len()
        };
        if end <= input.len() {
            &input[start..end]
        } else {
            &[]
        }
    } else {
        &[]
    };

    debug!(
        "Section4 data: {} entries, {} bytes",
        header.section4_count,
        section4_data.len()
    );
    section4_data.to_vec()
}

/// Parse all face data from the 3D file
fn parse_face_data_section(
    input: &[u8],
    header: &Model3DHeader,
    version: &ModelVersion,
) -> Vec<FaceData> {
    let mut face_data = Vec::new();

    if header.offset_face_data < input.len() as u32 {
        let mut face_data_input = &input[header.offset_face_data as usize..];
        debug!(
            "Face data starts at offset {}, input size: {}",
            header.offset_face_data,
            face_data_input.len()
        );
        debug!(
            "First 32 bytes of face data: {:02x?}",
            &face_data_input[..face_data_input.len().min(32)]
        );
        debug!(
            "Next 128 bytes at face_data (offset {}): {:02x?}",
            header.offset_face_data,
            &input[header.offset_face_data as usize
                ..(header.offset_face_data as usize + 128).min(input.len())]
        );

        for face_index in 0..header.num_faces {
            // Stop if we don't have enough data for even a minimal face
            if face_data_input.len() < 20 {
                debug!(
                    "Face {}: Not enough data for minimal face ({} bytes remaining)",
                    face_index,
                    face_data_input.len()
                );
                break;
            }

            match parse_face_data(face_data_input, version) {
                Ok((remaining, face)) => {
                    debug!(
                        "Face {}: vertex_count={}, unk_01={}, texture_data={:?}, unk_04={}, vertices={}",
                        face_index,
                        face.vertex_count,
                        face.unk_01,
                        face.texture_data,
                        face.unk_04,
                        face.face_vertices.len()
                    );

                    face_data.push(face);
                    face_data_input = remaining;
                }
                Err(e) => {
                    debug!("Face {}: Failed to parse face data: {:?}", face_index, e);
                    break;
                }
            }
        }
    }

    debug!("Successfully parsed {} faces", face_data.len());
    face_data
}

/// Parse all vertex coordinates from the 3D file
fn parse_vertex_coords_section(
    input: &[u8],
    header: &Model3DHeader,
    adjusted_offset_vertex_coords: u32,
) -> Vec<VertexCoord> {
    let mut vertex_coords = Vec::new();

    if adjusted_offset_vertex_coords < input.len() as u32 {
        let mut vertex_input = &input[adjusted_offset_vertex_coords as usize..];
        debug!(
            "Vertex data starts at offset {}, input size: {}",
            adjusted_offset_vertex_coords,
            vertex_input.len()
        );

        for vertex_index in 0..header.num_vertices {
            match parse_vertex_coord(vertex_input) {
                Ok((remaining, vertex)) => {
                    if vertex_index < 5 {
                        debug!(
                            "Vertex {}: ({:.2}, {:.2}, {:.2})",
                            vertex_index, vertex.x, vertex.y, vertex.z
                        );
                    }
                    vertex_coords.push(vertex);
                    vertex_input = remaining;
                }
                Err(e) => {
                    debug!("Vertex {}: Failed to parse vertex: {:?}", vertex_index, e);
                    break;
                }
            }
        }
    }

    debug!("Successfully parsed {} vertices", vertex_coords.len());
    vertex_coords
}

/// Parse all face normals from the 3D file
fn parse_face_normals_section(input: &[u8], header: &Model3DHeader) -> Vec<FaceNormal> {
    let mut face_normals = Vec::new();

    if header.offset_face_normals < input.len() as u32 {
        let mut normal_input = &input[header.offset_face_normals as usize..];
        debug!(
            "Face normal data starts at offset {}, input size: {}",
            header.offset_face_normals,
            normal_input.len()
        );

        for normal_index in 0..header.num_faces {
            match parse_face_normal(normal_input) {
                Ok((remaining, normal)) => {
                    if normal_index < 5 {
                        debug!(
                            "Normal {}: ({:.2}, {:.2}, {:.2})",
                            normal_index, normal.x, normal.y, normal.z
                        );
                    }
                    face_normals.push(normal);
                    normal_input = remaining;
                }
                Err(e) => {
                    debug!("Normal {}: Failed to parse normal: {:?}", normal_index, e);
                    break;
                }
            }
        }
    }

    debug!("Successfully parsed {} face normals", face_normals.len());
    face_normals
}

/// Parse all UV offsets from the 3D file
fn parse_uv_offsets_section(input: &[u8], header: &Model3DHeader) -> Vec<u32> {
    let mut uv_offsets = Vec::new();

    if header.offset_uv_offsets < input.len() as u32 {
        let mut uv_offset_input = &input[header.offset_uv_offsets as usize..];
        debug!(
            "UV offset data starts at offset {}, input size: {}",
            header.offset_uv_offsets,
            uv_offset_input.len()
        );

        for offset_index in 0..header.num_uv_offsets {
            match le_u32::<_, nom::error::Error<_>>(uv_offset_input) {
                Ok((remaining, offset)) => {
                    if offset_index < 5 {
                        debug!("UV offset {}: {}", offset_index, offset);
                    }
                    uv_offsets.push(offset);
                    uv_offset_input = remaining;
                }
                Err(e) => {
                    debug!(
                        "UV offset {}: Failed to parse offset: {:?}",
                        offset_index, e
                    );
                    break;
                }
            }
        }
    }

    debug!("Successfully parsed {} UV offsets", uv_offsets.len());
    uv_offsets
}

/// Parse all UV coordinates from the 3D file
fn parse_uv_coords_section(
    input: &[u8],
    header: &Model3DHeader,
    adjusted_offset_uv_data: u32,
) -> Vec<UVCoord> {
    let mut uv_coords = Vec::new();

    if adjusted_offset_uv_data < input.len() as u32 {
        let mut uv_input = &input[adjusted_offset_uv_data as usize..];
        debug!(
            "UV coordinate data starts at offset {}, input size: {}",
            adjusted_offset_uv_data,
            uv_input.len()
        );

        // For v2.7 or earlier, UV data might be all zeros, so we need to be careful
        if header.is_v27_or_earlier() {
            // In v2.7 or earlier, UV data might be all zeros, so we'll try to parse
            // but be more lenient about failures
            for coord_index in 0..header.num_uv_offsets {
                match parse_uv_coord(uv_input) {
                    Ok((remaining, coord)) => {
                        if coord_index < 5 {
                            debug!(
                                "UV coord {}: ({:.2}, {:.2}, {:.2})",
                                coord_index, coord.x, coord.y, coord.z
                            );
                        }
                        uv_coords.push(coord);
                        uv_input = remaining;
                    }
                    Err(e) => {
                        debug!("UV coord {}: Failed to parse coord: {:?}", coord_index, e);
                        // For v2.7 or earlier, UV data might be all zeros, so we'll stop
                        break;
                    }
                }
            }
        } else {
            // For v4.0 or later, parse UV coordinates normally
            for coord_index in 0..header.num_uv_offsets {
                match parse_uv_coord(uv_input) {
                    Ok((remaining, coord)) => {
                        if coord_index < 5 {
                            debug!(
                                "UV coord {}: ({:.2}, {:.2}, {:.2})",
                                coord_index, coord.x, coord.y, coord.z
                            );
                        }
                        uv_coords.push(coord);
                        uv_input = remaining;
                    }
                    Err(e) => {
                        debug!("UV coord {}: Failed to parse coord: {:?}", coord_index, e);
                        break;
                    }
                }
            }
        }
    }

    debug!("Successfully parsed {} UV coordinates", uv_coords.len());
    uv_coords
}

/// Calculate the remaining data after all sections have been parsed
fn calculate_remaining_data<'a>(
    input: &'a [u8],
    header: &Model3DHeader,
    face_data: &[FaceData],
    adjusted_offset_uv_data: u32,
    adjusted_offset_vertex_coords: u32,
    frame_data_size: u32,
) -> &'a [u8] {
    // Find the highest offset + size to determine where our data ends
    let mut max_offset = 0u32;

    // Check frame data end
    if header.offset_frame_data > 0 && frame_data_size > 0 {
        max_offset = max_offset.max(header.offset_frame_data + frame_data_size);
    }

    // Check section4 data end
    if header.offset_section4 > 0 && header.section4_count > 0 {
        max_offset = max_offset.max(header.offset_section4 + (header.section4_count * 4));
    }

    // Check face data end (approximate)
    if header.offset_face_data > 0 {
        // Estimate face data size: each face has variable size based on vertex count
        let estimated_face_size = face_data
            .iter()
            .map(|face| {
                8 + (face.vertex_count as u32 * 8) // 8 bytes header + 8 bytes per vertex
            })
            .sum::<u32>();
        max_offset = max_offset.max(header.offset_face_data + estimated_face_size);
    }

    // Check vertex coordinates end
    if adjusted_offset_vertex_coords > 0 {
        max_offset = max_offset.max(adjusted_offset_vertex_coords + (header.num_vertices * 12));
    }

    // Check face normals end
    if header.offset_face_normals > 0 {
        max_offset = max_offset.max(header.offset_face_normals + (header.num_faces * 12));
    }

    // Check UV offsets end
    if header.offset_uv_offsets > 0 {
        max_offset = max_offset.max(header.offset_uv_offsets + (header.num_uv_offsets * 4));
    }

    // Check UV coordinates end
    if adjusted_offset_uv_data > 0 {
        max_offset = max_offset.max(adjusted_offset_uv_data + (header.num_uv_offsets * 12));
    }

    // Return remaining bytes after the last parsed section
    let remaining = if max_offset > 0 && max_offset < input.len() as u32 {
        &input[max_offset as usize..]
    } else {
        &[]
    };

    debug!(
        "Parsing complete - max_offset: {}, remaining bytes: {}",
        max_offset,
        remaining.len()
    );
    remaining
}

/// Helper function to print debug information about file offsets
fn debug_file_offsets(input: &[u8], header: &Model3DHeader) {
    debug!("Raw header values:");
    debug!("  offset_frame_data: {}", header.offset_frame_data);
    debug!("  offset_section4: {}", header.offset_section4);
    debug!("  offset_face_data: {}", header.offset_face_data);
    debug!("  offset_vertex_coords: {}", header.offset_vertex_coords);
    debug!("  offset_face_normals: {}", header.offset_face_normals);
    debug!("  offset_uv_offsets: {}", header.offset_uv_offsets);
    debug!("  offset_uv_data: {}", header.offset_uv_data);
    debug!("  num_uv_offsets: {}", header.num_uv_offsets);
    debug!("  num_uv_offsets2: {}", header.num_uv_offsets2);

    debug!(
        "Header offsets - Frame: {}, Section4: {}, Face: {}, Vertex: {}, Normal: {}, UV: {}",
        header.offset_frame_data,
        header.offset_section4,
        header.offset_face_data,
        header.offset_vertex_coords,
        header.offset_face_normals,
        header.offset_uv_data
    );

    // Hex dump at each major offset
    let dump = |label: &str, offset: u32| {
        if offset < input.len() as u32 {
            let end = ((offset as usize) + 32).min(input.len());
            debug!(
                "Bytes at {} (offset {}): {:02x?}",
                label,
                offset,
                &input[offset as usize..end]
            );
        }
    };

    dump("face_data", header.offset_face_data);
    dump("vertex_coords", header.offset_vertex_coords);
    dump("face_normals", header.offset_face_normals);
    dump("uv_offsets", header.offset_uv_offsets);
    dump("uv_data", header.offset_uv_data);
}

/// Parse complete 3DC/3D file with offset-based section parsing
pub fn parse_3d_file(input: &[u8]) -> IResult<&[u8], Model3DFile> {
    debug!(
        "First 128 bytes of file: {:02x?}",
        &input[..128.min(input.len())]
    );
    let (_input_after_header, mut header) = parse_3d_header(input)?;
    let version = header.parse_version();

    debug!(
        "Parsing 3D file - Version: {:?}, Vertices: {}, Faces: {}",
        version, header.num_vertices, header.num_faces
    );

    // Debug file offsets
    debug_file_offsets(input, &header);

    // Handle version-specific header field interpretations
    let (adjusted_offset_uv_data, adjusted_offset_vertex_coords) = if header.is_v27_or_earlier() {
        // For v2.7 or earlier:
        // - NumUVOffsets is the offset to UV Data (which is just all 0s)
        // - OffsetUVData is an offset to an unknown section
        (header.num_uv_offsets, header.offset_uv_data)
    } else {
        (header.offset_uv_data, header.offset_vertex_coords)
    };

    debug!(
        "Adjusted offsets - UV: {}, Vertex: {}",
        adjusted_offset_uv_data, adjusted_offset_vertex_coords
    );

    // Parse all sections using dedicated functions
    let frame_data = parse_frame_data_section(input, &header);
    let _section4_data = parse_section4_data_section(input, &header);
    let face_data = parse_face_data_section(input, &header, &version);

    if header.is_v40_or_later() {
        let face_data_size: u32 = face_data
            .iter()
            .map(|f| f.size_in_bytes(&version) as u32)
            .sum();
        header.offset_uv_offsets = header.offset_face_data + face_data_size;
    }

    let vertex_coords =
        parse_vertex_coords_section(input, &header, adjusted_offset_vertex_coords);
    let face_normals = parse_face_normals_section(input, &header);
    let uv_offsets = parse_uv_offsets_section(input, &header);
    let uv_coords = parse_uv_coords_section(input, &header, adjusted_offset_uv_data);

    // Calculate remaining data
    let frame_data_size = if header.offset_frame_data > 64 {
        header.offset_frame_data - 64
    } else {
        0
    };

    let remaining = calculate_remaining_data(
        input,
        &header,
        &face_data,
        adjusted_offset_uv_data,
        adjusted_offset_vertex_coords,
        frame_data_size,
    );

    Ok((
        remaining,
        Model3DFile {
            header,
            version,
            frame_data,
            face_data,
            vertex_coords,
            face_normals,
            uv_offsets,
            uv_coords,
        },
    ))
}
