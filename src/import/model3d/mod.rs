/// Low-level parser for Redguard 3D/3DC binary model data.
pub mod parser;
pub mod subobjects;

#[derive(serde::Serialize, Debug, Clone, PartialEq, Eq)]
/// Recognized model format versions from 3D/3DC headers.
pub enum ModelVersion {
    V26,
    V27,
    V40,
    V50,
    Unknown,
}

#[derive(serde::Serialize, Debug, Clone)]
/// Face material reference as either a palette index or texture/image pair.
pub enum TextureData {
    SolidColor(u8),
    Texture { texture_id: u16, image_id: u8 },
}

#[derive(serde::Serialize, Debug, Clone)]
/// Parsed 64-byte 3D model header.
pub struct Model3DHeader {
    pub version: [u8; 4],
    pub num_vertices: u32,
    pub num_faces: u32,
    pub radius: u32,
    pub num_frames: u32,
    pub offset_frame_data: u32,
    pub total_face_vertices: u32,
    pub offset_section4: u32,
    pub section4_count: u32,
    #[allow(
        clippy::pub_underscore_fields,
        reason = "field name mirrors reverse-engineered binary layout"
    )]
    /// Always 0 in v4.0/v5.0; bulk-copied as part of the 64-byte header but never
    /// individually read at runtime.
    pub _unused_24: u32,
    pub offset_normal_indices: u32,
    pub offset_vertex_normals: u32,
    pub offset_vertex_coords: u32,
    pub offset_face_normals: u32,
    pub total_face_vertices_dup: u32,
    pub offset_face_data: u32,
}

#[derive(serde::Serialize, Debug, Clone)]
/// One vertex reference and UV delta entry inside a face.
pub struct FaceVertex {
    pub vertex_index: u32,
    pub u: i16,
    pub v: i16,
}

#[derive(serde::Serialize, Debug, Clone)]
/// Decoded face record with vertices and material information.
pub struct FaceData {
    pub vertex_count: u8,
    /// Unsigned face brightness added to ambient RGB by Glide.
    pub tex_hi: u8,
    pub texture_data: TextureData,
    pub face_vertices: Vec<FaceVertex>,
}

impl FaceData {
    /// Returns the encoded byte size for this face in the given model version.
    #[must_use]
    pub const fn size_in_bytes(&self, version: &ModelVersion) -> usize {
        let texture_header_size = if matches!(version, ModelVersion::V40 | ModelVersion::V50) {
            1 + 4 // tex_hi (u8) + texture_raw (u32)
        } else {
            1 + 2 // u1 (u8) + texture_data (u16)
        };
        let vertex_data_size = self.face_vertices.len() * (4 + 2 + 2);
        1 + texture_header_size + 4 + vertex_data_size
    }
}

#[derive(serde::Serialize, Debug, Clone, Copy)]
/// Vertex position in model space.
pub struct VertexCoord {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(serde::Serialize, Debug, Clone)]
/// Face normal vector.
pub struct FaceNormal {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(serde::Serialize, Debug, Clone)]
/// Vertex normal vector.
pub struct VertexNormal {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(serde::Serialize, Debug, Clone, Copy, PartialEq, Eq)]
/// Frame encoding used by frame-table entries.
pub enum FrameType {
    Static3D,
    AnimatedI16,
    /// Signed i16 deltas in whole model units (frame type 0x12).
    AnimatedI16Unscaled,
    AnimatedI32,
    Static3DC,
    Unknown(u32),
}

#[derive(serde::Serialize, Debug, Clone)]
/// Parsed frame table entry from animated model data.
pub struct FrameDataEntry {
    pub vertex_offset: u32,
    pub normal_offset: u32,
    pub reserved: u32,
    pub frame_type: FrameType,
}

/// Absolute per-frame vertex positions in model units, reconstructed from frame-zero deltas.
#[derive(serde::Serialize, Debug, Clone)]
pub struct FrameVertexData {
    pub coords: Vec<VertexCoord>,
}

/// Parsed per-frame face normals.
#[derive(serde::Serialize, Debug, Clone)]
pub struct FrameNormalData {
    pub face_normals: Vec<FaceNormal>,
}

/// Parsed 3D/3DC model including geometry and frame metadata.
#[derive(serde::Serialize, Debug, Clone)]
pub struct Model3DFile {
    pub header: Model3DHeader,
    pub version: ModelVersion,
    pub frame_data: Vec<FrameDataEntry>,
    pub face_data: Vec<FaceData>,
    pub vertex_coords: Vec<VertexCoord>,
    pub face_normals: Vec<FaceNormal>,
    pub normal_indices: Vec<u32>,
    pub vertex_normals: Vec<VertexNormal>,
    /// Per-frame vertex positions. Empty if no animation frames.
    pub frame_vertex_data: Vec<FrameVertexData>,
    /// Per-frame face normals. Empty if no animation frames.
    pub frame_normal_data: Vec<FrameNormalData>,
}

impl Model3DHeader {
    /// Returns the header version bytes as a trimmed UTF-8 string.
    #[must_use]
    pub fn version_string(&self) -> String {
        String::from_utf8_lossy(&self.version)
            .trim_matches('\0')
            .to_string()
    }

    /// Maps the header version string to a known `ModelVersion` variant.
    #[must_use]
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

    /// Returns `true` for v2.6 and v2.7 layout variants.
    #[must_use]
    pub fn is_v27_or_earlier(&self) -> bool {
        matches!(self.parse_version(), ModelVersion::V26 | ModelVersion::V27)
    }

    /// Returns `true` for v4.0 and v5.0 layout variants.
    #[must_use]
    pub fn is_v40_or_later(&self) -> bool {
        matches!(self.parse_version(), ModelVersion::V40 | ModelVersion::V50)
    }
}

impl Model3DFile {
    /// Counts vertices referenced by all parsed faces.
    #[must_use]
    pub fn total_face_vertices(&self) -> usize {
        self.face_data
            .iter()
            .map(|face| usize::from(face.vertex_count))
            .sum()
    }

    /// Computes an axis-aligned bounding box from parsed vertex positions.
    #[must_use]
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

use crate::{Result, error::Error};
use log::warn;

fn parse_data_internal(input: &[u8], kind: &str) -> Result<Model3DFile> {
    match parser::parse_3d_file(input) {
        Ok((remaining, model)) => {
            if !remaining.is_empty() {
                warn!("{} bytes remaining in {} data", remaining.len(), kind);
            }
            Ok(model)
        }
        Err(e) => Err(Error::Parse(format!("Failed to parse {kind}: {e:?}"))),
    }
}

/// Parses a complete 3D/3DC model file from bytes.
pub fn parse_3d_file(input: &[u8]) -> Result<Model3DFile> {
    parse_data_internal(input, "3D model file")
}

impl Model3DFile {
    /// Resolves a frame to absolute positions in parser model units.
    ///
    /// The parser has already reconstructed compressed frame positions.
    /// Frame zero uses the parsed base geometry. Incomplete frames are rejected.
    /// The returned model is a static snapshot with no animation frame table.
    pub fn at_frame(&self, frame: usize) -> crate::Result<Self> {
        use crate::error::Error;
        let invalid = |message: &str| Error::Conversion(format!("model frame {frame}: {message}"));
        if frame >= (self.header.num_frames as usize).max(1) {
            return Err(invalid("outside frame table"));
        }
        let mut posed = self.clone();
        posed.header.num_frames = 1;
        posed.frame_data.clear();
        posed.frame_vertex_data.clear();
        posed.frame_normal_data.clear();
        if frame == 0 {
            return Ok(posed);
        }
        let encoding = self
            .frame_data
            .first()
            .ok_or_else(|| invalid("missing encoding"))?
            .frame_type;
        let positions = &self
            .frame_vertex_data
            .get(frame)
            .ok_or_else(|| invalid("missing positions"))?
            .coords;
        let normals = &self
            .frame_normal_data
            .get(frame)
            .ok_or_else(|| invalid("missing normals"))?
            .face_normals;
        if positions.len() != self.vertex_coords.len() || normals.len() != self.face_data.len() {
            return Err(invalid("incomplete geometry"));
        }
        posed.vertex_coords = match encoding {
            FrameType::AnimatedI16 | FrameType::AnimatedI16Unscaled | FrameType::AnimatedI32 => {
                positions.clone()
            }
            FrameType::Static3D
                if matches!(self.version, ModelVersion::V40 | ModelVersion::V50) =>
            {
                positions.clone()
            }
            _ => return Err(invalid("unsupported encoding")),
        };
        posed.face_normals = normals.clone();
        Ok(posed)
    }
}

/// Exports all decoded model fields and authored collision subsets for inspection.
/// Positions retain parser model units; face UVs are accumulated sixteenths of a texel.
/// Frame poses are absolute, as in `frame_vertex_data`; no compressed payload is included.
#[must_use]
pub fn export_model_metadata(
    model: &Model3DFile,
    subobjects: &[subobjects::Subobject],
) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "subobjects": subobjects,
        "scene_subobjects": subobjects.iter().map(|subobject| serde_json::json!({
            "center": crate::gltf::model_position(subobject.center.map(|value| value as f32 / 256.0)),
            "radius": subobject.radius as f32 / 256.0 / crate::gltf::ENGINE_UNIT_SCALE,
            "extent_factors": subobject.extents,
            "faces": subobject.faces,
        })).collect::<Vec<_>>(),
        "uv_units": "sixteenths_of_a_texel",
        "bounding_box": model.bounding_box(),
    })
}
