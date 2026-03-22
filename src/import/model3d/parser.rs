use super::{
    FaceData, FaceNormal, FaceVertex, FrameDataEntry, FrameType, Model3DFile, Model3DHeader,
    ModelVersion, TextureData, VertexCoord, VertexNormal,
};
use log::trace;
use nom::{
    IResult, Parser,
    bytes::complete::take,
    number::complete::{le_f32, le_i16, le_i32, le_u8, le_u16, le_u32},
};

/// Parses the fixed-size model header from a 3D/3DC byte slice.
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
            total_face_vertices,
            offset_section4,
            section4_count,
            unused_24,
            offset_normal_indices,
            offset_vertex_normals,
            offset_vertex_coords,
            offset_face_normals,
            total_face_vertices_dup,
            offset_face_data,
        ),
    ) = (
        take(4u8),
        le_u32,
        le_u32,
        le_u32,
        le_u32,
        le_u32,
        le_u32,
        le_u32,
        le_u32,
        le_u32,
        le_u32,
        le_u32,
        le_u32,
        le_u32,
        le_u32,
        le_u32,
    )
        .parse(input)?;

    let mut version_bytes = [0_u8; 4];
    version_bytes.copy_from_slice(version);

    let mut header = Model3DHeader {
        version: version_bytes,
        num_vertices,
        num_faces,
        radius,
        num_frames,
        offset_frame_data,
        total_face_vertices,
        offset_section4,
        section4_count,
        _unused_24: unused_24,
        offset_normal_indices,
        offset_vertex_normals,
        offset_vertex_coords,
        offset_face_normals,
        total_face_vertices_dup,
        offset_face_data,
    };

    let version_str = String::from_utf8_lossy(&header.version);
    if version_str.starts_with("v2.6") || version_str.starts_with("v2.7") {
        let tmp = header.total_face_vertices;
        header.total_face_vertices = 0;
        header.offset_normal_indices = tmp;
        std::mem::swap(
            &mut header.offset_vertex_normals,
            &mut header.offset_normal_indices,
        );
    }

    Ok((input, header))
}

/// Parses one face record, including per-vertex index and accumulated UV deltas.
pub fn parse_face_data<'a>(input: &'a [u8], version: &ModelVersion) -> IResult<&'a [u8], FaceData> {
    trace!(
        "Starting face data parsing, first 16 bytes: {:02x?}",
        &input[..input.len().min(16)]
    );

    let (input, vertex_count) = le_u8(input)?;
    trace!("Read vertex_count: {vertex_count}");

    let (input, texture_data_raw, tex_hi_val) = if matches!(
        version,
        ModelVersion::V40 | ModelVersion::V50
    ) {
        let (input, tex_hi) = le_u8(input)?;
        let (input, raw) = le_u32(input)?;
        trace!("v4.0/5.0 texture fields: tex_hi=0x{tex_hi:02x}, raw=0x{raw:08x}");
        (input, raw, tex_hi)
    } else {
        let (input, u1) = le_u8(input)?;
        let (input, texture_data) = le_u16(input)?;
        let raw = u32::from(texture_data);
        trace!(
            "v2.6/2.7 texture fields: u1=0x{u1:02x}, texture_data=0x{texture_data:04x}, raw=0x{raw:08x}"
        );
        (input, raw, u1)
    };

    let (input, _unused_04) = le_u32(input)?;

    let texture_data = parse_texture_data_value(texture_data_raw, version);
    trace!("Parsed texture_data: {texture_data:?}");

    let mut face_vertices = Vec::with_capacity(usize::from(vertex_count));
    let mut acc_u = 0i16;
    let mut acc_v = 0i16;
    let mut input = input;

    for i in 0..vertex_count {
        trace!(
            "Parsing face vertex {}, remaining bytes: {:02x?}",
            i,
            &input[..input.len().min(12)]
        );

        let (input2, vertex_index_raw) = le_u32(input)?;
        let (input2, u) = le_i16(input2)?;
        let (input2, v) = le_i16(input2)?;

        let vertex_index = if matches!(
            version,
            ModelVersion::V26 | ModelVersion::V27 | ModelVersion::Unknown
        ) {
            vertex_index_raw / 12
        } else {
            vertex_index_raw
        };

        trace!(
            "Raw vertex_index: {vertex_index_raw} (0x{vertex_index_raw:08x}), adjusted: {vertex_index}, u: {u}, v: {v}"
        );

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
            tex_hi: tex_hi_val,
            texture_data,
            face_vertices,
        },
    ))
}

/// Decodes a raw texture field into `TextureData` for a given model version.
#[must_use]
pub const fn parse_texture_data_value(raw: u32, version: &ModelVersion) -> TextureData {
    #[allow(clippy::cast_possible_truncation)]
    // Bitfield extraction is range-limited by masks/shifts.
    match version {
        ModelVersion::V26 | ModelVersion::V27 | ModelVersion::Unknown => {
            let texture_id = (raw >> 7) as u16;
            if texture_id < 2 {
                let color_index = raw as u8;
                TextureData::SolidColor(color_index)
            } else {
                let image_id = (raw & 0x7F) as u8;
                TextureData::Texture {
                    texture_id,
                    image_id,
                }
            }
        }
        ModelVersion::V40 | ModelVersion::V50 => {
            if (raw >> 20) == 0x0FFF {
                let color_index = (raw >> 8) as u8;
                TextureData::SolidColor(color_index)
            } else {
                let tmp = (raw >> 8).wrapping_sub(4_000_000);
                let one = (tmp / 250) % 40;
                let ten = ((tmp - (one * 250)) / 1000) % 100;
                let hundred = (tmp - (one * 250) - (ten * 1000)) / 4000;
                let texture_id = (one + ten + hundred) as u16;

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

/// Parses one vertex coordinate triplet and applies fixed-point scaling.
pub fn parse_vertex_coord(input: &[u8]) -> IResult<&[u8], VertexCoord> {
    let (input, (x_raw, y_raw, z_raw)) = (le_i32, le_i32, le_i32).parse(input)?;
    let scale = 1.0 / 256.0;
    #[allow(clippy::cast_precision_loss)] // Source format stores fixed-point i32 positions.
    Ok((
        input,
        VertexCoord {
            x: x_raw as f32 * scale,
            y: y_raw as f32 * scale,
            z: z_raw as f32 * scale,
        },
    ))
}

/// Parses one face normal vector and applies fixed-point scaling.
pub fn parse_face_normal(input: &[u8]) -> IResult<&[u8], FaceNormal> {
    let (input, (x_raw, y_raw, z_raw)) = (le_i32, le_i32, le_i32).parse(input)?;
    let scale = 1.0 / 256.0;
    #[allow(clippy::cast_precision_loss)] // Source format stores fixed-point i32 normals.
    Ok((
        input,
        FaceNormal {
            x: x_raw as f32 * scale,
            y: y_raw as f32 * scale,
            z: z_raw as f32 * scale,
        },
    ))
}

/// Parses one vertex normal vector.
pub fn parse_vertex_normal(input: &[u8]) -> IResult<&[u8], VertexNormal> {
    let (input, (x, y, z)) = (le_f32, le_f32, le_f32).parse(input)?;
    Ok((input, VertexNormal { x, y, z }))
}

fn parse_frame_data_section(input: &[u8], header: &Model3DHeader) -> Vec<FrameDataEntry> {
    let mut entries = Vec::new();
    let Ok(mut off) = usize::try_from(header.offset_frame_data) else {
        return entries;
    };

    for _ in 0..header.num_frames {
        if off + 16 > input.len() {
            break;
        }
        let vertex_offset =
            u32::from_le_bytes([input[off], input[off + 1], input[off + 2], input[off + 3]]);
        let normal_offset = u32::from_le_bytes([
            input[off + 4],
            input[off + 5],
            input[off + 6],
            input[off + 7],
        ]);
        let reserved = u32::from_le_bytes([
            input[off + 8],
            input[off + 9],
            input[off + 10],
            input[off + 11],
        ]);
        let raw_type = u32::from_le_bytes([
            input[off + 12],
            input[off + 13],
            input[off + 14],
            input[off + 15],
        ]);

        let frame_type = match raw_type {
            0 => FrameType::Static3D,
            2 => FrameType::AnimatedI16,
            4 => FrameType::AnimatedI32,
            8 => FrameType::Static3DC,
            other => FrameType::Unknown(other),
        };

        entries.push(FrameDataEntry {
            vertex_offset,
            normal_offset,
            reserved,
            frame_type,
        });
        off += 16;
    }

    trace!(
        "Parsed {} frame data entries (num_frames={})",
        entries.len(),
        header.num_frames
    );
    entries
}

fn parse_section4_data_section(input: &[u8], header: &Model3DHeader) -> Vec<u8> {
    if header.offset_section4 == 0 || header.section4_count == 0 {
        return Vec::new();
    }
    let Ok(start) = usize::try_from(header.offset_section4) else {
        return Vec::new();
    };
    if start >= input.len() {
        return Vec::new();
    }
    input[start..].to_vec()
}

fn parse_face_data_section(
    input: &[u8],
    header: &Model3DHeader,
    version: &ModelVersion,
) -> Vec<FaceData> {
    let mut face_data = Vec::new();
    let input_len_u32 = u32::try_from(input.len()).unwrap_or(u32::MAX);

    if header.offset_face_data < input_len_u32 {
        let Ok(offset_face_data) = usize::try_from(header.offset_face_data) else {
            return face_data;
        };
        let mut face_data_input = &input[offset_face_data..];

        for face_index in 0..header.num_faces {
            if face_data_input.len() < 20 {
                trace!(
                    "Face {}: Not enough data ({} bytes remaining)",
                    face_index,
                    face_data_input.len()
                );
                break;
            }

            match parse_face_data(face_data_input, version) {
                Ok((remaining, face)) => {
                    trace!(
                        "Face {}: vertex_count={}, tex_hi={}, texture_data={:?}",
                        face_index, face.vertex_count, face.tex_hi, face.texture_data,
                    );
                    face_data.push(face);
                    face_data_input = remaining;
                }
                Err(e) => {
                    trace!("Face {face_index}: Failed to parse face data: {e:?}");
                    break;
                }
            }
        }
    }

    trace!("Successfully parsed {} faces", face_data.len());
    face_data
}

fn parse_vertex_coords_section(
    input: &[u8],
    header: &Model3DHeader,
    adjusted_offset_vertex_coords: u32,
) -> Vec<VertexCoord> {
    let mut vertex_coords = Vec::new();
    let input_len_u32 = u32::try_from(input.len()).unwrap_or(u32::MAX);

    if adjusted_offset_vertex_coords < input_len_u32 {
        let Ok(offset_vertex_coords) = usize::try_from(adjusted_offset_vertex_coords) else {
            return vertex_coords;
        };
        let mut vertex_input = &input[offset_vertex_coords..];

        for vertex_index in 0..header.num_vertices {
            match parse_vertex_coord(vertex_input) {
                Ok((remaining, vertex)) => {
                    if vertex_index < 5 {
                        trace!(
                            "Vertex {}: ({:.2}, {:.2}, {:.2})",
                            vertex_index, vertex.x, vertex.y, vertex.z
                        );
                    }
                    vertex_coords.push(vertex);
                    vertex_input = remaining;
                }
                Err(e) => {
                    trace!("Vertex {vertex_index}: Failed to parse vertex: {e:?}");
                    break;
                }
            }
        }
    }

    trace!("Successfully parsed {} vertices", vertex_coords.len());
    vertex_coords
}

fn parse_face_normals_section(input: &[u8], header: &Model3DHeader) -> Vec<FaceNormal> {
    let mut face_normals = Vec::new();
    let input_len_u32 = u32::try_from(input.len()).unwrap_or(u32::MAX);

    if header.offset_face_normals < input_len_u32 {
        let Ok(offset_face_normals) = usize::try_from(header.offset_face_normals) else {
            return face_normals;
        };
        let mut normal_input = &input[offset_face_normals..];

        for normal_index in 0..header.num_faces {
            match parse_face_normal(normal_input) {
                Ok((remaining, normal)) => {
                    if normal_index < 5 {
                        trace!(
                            "Normal {}: ({:.2}, {:.2}, {:.2})",
                            normal_index, normal.x, normal.y, normal.z
                        );
                    }
                    face_normals.push(normal);
                    normal_input = remaining;
                }
                Err(e) => {
                    trace!("Normal {normal_index}: Failed to parse normal: {e:?}");
                    break;
                }
            }
        }
    }

    trace!("Successfully parsed {} face normals", face_normals.len());
    face_normals
}

fn parse_normal_indices_section(input: &[u8], header: &Model3DHeader) -> Vec<u32> {
    let mut indices = Vec::new();
    let input_len_u32 = u32::try_from(input.len()).unwrap_or(u32::MAX);

    if header.offset_normal_indices == 0 || header.offset_normal_indices >= input_len_u32 {
        return indices;
    }

    let Ok(offset_normal_indices) = usize::try_from(header.offset_normal_indices) else {
        return indices;
    };
    let mut idx_input = &input[offset_normal_indices..];

    for i in 0..header.total_face_vertices {
        match le_u32::<_, nom::error::Error<_>>(idx_input) {
            Ok((remaining, offset)) => {
                if i < 5 {
                    trace!("Normal index {i}: {offset}");
                }
                indices.push(offset);
                idx_input = remaining;
            }
            Err(e) => {
                trace!("Normal index {i}: Failed to parse: {e:?}");
                break;
            }
        }
    }

    trace!("Successfully parsed {} normal indices", indices.len());
    indices
}

fn parse_vertex_normals_section(
    input: &[u8],
    header: &Model3DHeader,
    offset: u32,
) -> Vec<VertexNormal> {
    let mut normals = Vec::new();
    let input_len_u32 = u32::try_from(input.len()).unwrap_or(u32::MAX);

    if offset == 0 || offset >= input_len_u32 {
        return normals;
    }

    let Ok(normals_offset) = usize::try_from(offset) else {
        return normals;
    };
    let mut norm_input = &input[normals_offset..];

    for i in 0..header.num_vertices {
        match parse_vertex_normal(norm_input) {
            Ok((remaining, normal)) => {
                if i < 5 {
                    trace!(
                        "Vertex normal {}: ({:.4}, {:.4}, {:.4})",
                        i, normal.x, normal.y, normal.z
                    );
                }
                normals.push(normal);
                norm_input = remaining;
            }
            Err(e) => {
                trace!("Vertex normal {i}: Failed to parse: {e:?}");
                break;
            }
        }
    }

    trace!("Successfully parsed {} vertex normals", normals.len());
    normals
}

/// Parses a full 3D/3DC model and returns decoded geometry sections.
pub fn parse_3d_file(input: &[u8]) -> IResult<&[u8], Model3DFile> {
    trace!(
        "First 128 bytes of file: {:02x?}",
        &input[..128.min(input.len())]
    );
    let (_input_after_header, header) = parse_3d_header(input)?;
    let version = header.parse_version();

    trace!(
        "Parsing 3D file - Version: {:?}, Vertices: {}, Faces: {}",
        version, header.num_vertices, header.num_faces
    );

    let (adjusted_offset_normals, adjusted_offset_vertex_coords) = if header.is_v27_or_earlier() {
        (header.total_face_vertices, header.offset_vertex_normals)
    } else {
        (header.offset_vertex_normals, header.offset_vertex_coords)
    };

    let frame_data = parse_frame_data_section(input, &header);
    let _ = parse_section4_data_section(input, &header);
    let face_data = parse_face_data_section(input, &header, &version);

    let vertex_coords = parse_vertex_coords_section(input, &header, adjusted_offset_vertex_coords);
    let face_normals = parse_face_normals_section(input, &header);
    let raw_normal_indices = parse_normal_indices_section(input, &header);
    let vertex_normals = parse_vertex_normals_section(input, &header, adjusted_offset_normals);

    // Convert normal_indices from file offsets to vertex_normals array indices.
    // Each raw entry is a byte offset into the vertex-normal section; dividing
    // by 12 (size of one f32×3 normal) yields the array index.
    let normal_indices: Vec<u32> = if adjusted_offset_normals > 0 {
        raw_normal_indices
            .into_iter()
            .map(|offset| {
                if offset >= adjusted_offset_normals {
                    (offset - adjusted_offset_normals) / 12
                } else {
                    u32::MAX
                }
            })
            .collect()
    } else {
        raw_normal_indices
    };

    let remaining = &[];

    Ok((
        remaining,
        Model3DFile {
            header,
            version,
            frame_data,
            face_data,
            vertex_coords,
            face_normals,
            normal_indices,
            vertex_normals,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_v27_face_bytes(u1: u8, texture_data: u16, vertex_count: u8) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.push(vertex_count);
        buf.push(u1);
        buf.extend_from_slice(&texture_data.to_le_bytes());
        buf.extend_from_slice(&0u32.to_le_bytes());
        for _ in 0..vertex_count {
            buf.extend_from_slice(&0u32.to_le_bytes());
            buf.extend_from_slice(&0i16.to_le_bytes());
            buf.extend_from_slice(&0i16.to_le_bytes());
        }
        buf
    }

    #[test]
    fn v27_texture_decode_uses_u16_not_combined_with_u1() {
        let data = make_v27_face_bytes(0xFF, 0x0180, 3);
        let (_, face) = parse_face_data(&data, &ModelVersion::V27).unwrap();
        match face.texture_data {
            TextureData::Texture {
                texture_id,
                image_id,
            } => {
                assert_eq!(texture_id, 3, "texture_id should be 0x0180 >> 7 = 3");
                assert_eq!(image_id, 0, "image_id should be 0x0180 & 0x7F = 0");
            }
            TextureData::SolidColor(_) => panic!("expected Texture, got SolidColor"),
        }
    }

    #[test]
    fn v27_u1_byte_does_not_affect_texture_id() {
        let data_a = make_v27_face_bytes(0x00, 0x0400, 3);
        let data_b = make_v27_face_bytes(0xFF, 0x0400, 3);
        let (_, face_a) = parse_face_data(&data_a, &ModelVersion::V27).unwrap();
        let (_, face_b) = parse_face_data(&data_b, &ModelVersion::V27).unwrap();
        match (&face_a.texture_data, &face_b.texture_data) {
            (
                TextureData::Texture {
                    texture_id: tid_a,
                    image_id: iid_a,
                },
                TextureData::Texture {
                    texture_id: tid_b,
                    image_id: iid_b,
                },
            ) => {
                assert_eq!(tid_a, tid_b, "u1 must not contaminate texture_id");
                assert_eq!(iid_a, iid_b, "u1 must not contaminate image_id");
            }
            _ => panic!("expected Texture variants"),
        }
    }

    #[test]
    fn v27_solid_color_when_texture_id_below_2() {
        let data = make_v27_face_bytes(0x00, 0x0042, 3);
        let (_, face) = parse_face_data(&data, &ModelVersion::V27).unwrap();
        match face.texture_data {
            TextureData::SolidColor(idx) => {
                assert_eq!(idx, 0x42, "color_index should be low byte of texture_data");
            }
            TextureData::Texture { .. } => panic!("expected SolidColor for texture_id < 2"),
        }
    }
}
