//! RGM file structures and parsing.
pub mod parser;

mod metadata;
mod positioning;
mod shared;

use crate::{Result, error::Error, import::registry::Registry, model3d::Model3DFile};
use log::warn;
pub use positioning::{Placement, PlacementType};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
pub struct RgmSectionHeader {
    pub name: [u8; 4],
    pub data_length: u32,
    pub record_count: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct MpsRecord {
    pub id: u32,
    pub model_name: [u8; 12],
    pub pos_x: i32,
    pub pos_y: i32,
    pub pos_z: u32,
    pub rotation_matrix: [i32; 9],
    pub trailing: [u8; 2],
}

#[derive(Debug, Clone)]
pub struct MpobRecord {
    pub id: u32,
    pub object_type: u8,
    pub is_active: u8,
    pub script_name: [u8; 9],
    pub model_name: [u8; 9],
    pub is_static: u8,
    pub unknown1: i16,
    pub pos_x: i32,
    pub pad_x: u8,
    pub pos_y: i32,
    pub pad_y: u8,
    pub pos_z: u32,
    pub angle_x: u32,
    pub angle_y: u32,
    pub angle_z: u32,
    pub texture_data_raw: i16,
    pub texture_id: u8,
    pub image_id: u8,
    pub intensity: i16,
    pub radius: i16,
    pub model_id: i16,
    pub world_id: i16,
    pub red: i16,
    pub green: i16,
    pub blue: i16,
}

#[derive(Debug, Clone)]
pub struct RavcRecord {
    pub offset_x: i8,
    pub offset_y: i8,
    pub offset_z: i8,
    pub vertex: u16,
    pub radius: u32,
}

#[derive(Debug, Clone)]
pub struct RaexRecord {
    pub grip0: i16,
    pub grip1: i16,
    pub scabbard0: i16,
    pub scabbard1: i16,
    pub unknown_08: i16,
    pub texture_id: i16,
    pub v_vertex: i16,
    pub v_size: i16,
    pub taunt_id: i16,
    pub unknown_12: i16,
    pub unknown_14: i16,
    pub unknown_16: i16,
    pub range_min: i16,
    pub range_ideal: i16,
    pub range_max: i16,
}

#[derive(Debug, Clone, Copy)]
pub struct RagrCommand {
    pub raw: u32,
    pub opcode: u8,
}

#[derive(Debug, Clone)]
pub struct RagrAnimGroup {
    pub group_index: u16,
    pub anim_id: u16,
    pub flag: u8,
    pub frame_count: u16,
    pub commands: Vec<RagrCommand>,
}

#[derive(Debug, Clone)]
pub struct PositionedModel {
    pub model: Model3DFile,
    pub transform: [f32; 16],
    pub model_name: String,
    pub source_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PositionedLight {
    pub color: [f32; 3],
    pub position: [f32; 3],
    pub range: f32,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct MpslRecord {
    pub color_r: u8,
    pub color_g: u8,
    pub color_b: u8,
    pub color_flags: u8,
    pub unknown0: u32,
    pub pos_x: i32,
    pub pos_y: i32,
    pub pos_z: u32,
    pub param0: i16,
    pub param1: i16,
    pub unknown1: [u8; 18],
}

#[derive(Debug, Clone)]
pub struct MprpRecord {
    pub id: u32,
    pub unknown0: u8,
    pub pos_x: i32,
    pub pad_x: u8,
    pub pos_y: i32,
    pub pad_y: u8,
    pub pos_z: u32,
    pub angle_y: i32,
    pub rope_type: i32,
    pub swing: i32,
    pub speed: i32,
    pub length: i16,
    pub static_model: String,
    pub rope_model: String,
    pub unknown1: [i32; 7],
}

#[derive(Debug, Clone)]
pub struct RgmFile {
    pub sections: Vec<RgmSection>,
}

#[derive(Debug, Clone)]
pub enum RgmSection {
    Rahd(RgmSectionHeader, Vec<u8>),
    Rafs(RgmSectionHeader, Vec<u8>),
    Rast(RgmSectionHeader, Vec<u8>),
    Rasb(RgmSectionHeader, Vec<u8>),
    Rava(RgmSectionHeader, Vec<u8>),
    Rasc(RgmSectionHeader, Vec<u8>),
    Rahk(RgmSectionHeader, Vec<u8>),
    Ralc(RgmSectionHeader, Vec<u8>),
    Raex(RgmSectionHeader, Vec<u8>),
    RaexParsed(RgmSectionHeader, Vec<RaexRecord>),
    Raat(RgmSectionHeader, Vec<u8>),
    Raan(RgmSectionHeader, Vec<u8>),
    Ragr(RgmSectionHeader, Vec<u8>),
    Ranm(RgmSectionHeader, Vec<u8>),
    Ravc(RgmSectionHeader, Vec<u8>),
    RavcParsed(RgmSectionHeader, Vec<RavcRecord>),
    Mpob(RgmSectionHeader, Vec<u8>),
    MpobParsed(RgmSectionHeader, Vec<MpobRecord>),
    Mprp(RgmSectionHeader, Vec<u8>),
    MprpParsed(RgmSectionHeader, Vec<MprpRecord>),
    Mps(RgmSectionHeader, Vec<MpsRecord>),
    Mpl(RgmSectionHeader, Vec<u8>),
    MplParsed(RgmSectionHeader, Vec<MpslRecord>),
    Mpf(RgmSectionHeader, Vec<u8>),
    Mpm(RgmSectionHeader, Vec<u8>),
    Mpsz(RgmSectionHeader, Vec<u8>),
    Wdnm(RgmSectionHeader, Vec<u8>),
    Flat(RgmSectionHeader, Vec<u8>),
    End(RgmSectionHeader),
}

#[must_use]
pub fn parse_ragr_actor_groups(ragr_data: &[u8], ragr_offset: usize) -> Vec<RagrAnimGroup> {
    metadata::parse_ragr_actor_groups_impl(ragr_data, ragr_offset)
}

#[must_use]
pub fn parse_rahd_ragr_index(rahd_data: &[u8]) -> HashMap<String, usize> {
    metadata::parse_rahd_ragr_index_impl(rahd_data)
}

#[allow(clippy::missing_errors_doc)] // Public wrapper delegates parser errors into crate error type.
pub fn parse_rgm_file(input: &[u8]) -> Result<RgmFile> {
    match parser::parse_rgm_file(input) {
        Ok((remaining, rgm_file)) => {
            if !remaining.is_empty() {
                warn!("{} bytes remaining unparsed", remaining.len());
            }
            Ok(rgm_file)
        }
        Err(e) => Err(Error::Parse(format!("Failed to parse RGM file: {e:?}"))),
    }
}

#[allow(clippy::missing_errors_doc)] // Public wrapper combines parser/model-loading errors into crate error type.
pub fn parse_rgm_with_models(
    input: &[u8],
    registry: &Registry,
) -> Result<(RgmFile, Vec<PositionedModel>, Vec<PositionedLight>)> {
    let rgm_file = parse_rgm_file(input)?;
    let (positioned_models, positioned_lights) =
        positioning::extract_positioned_models(input, &rgm_file, registry);
    Ok((rgm_file, positioned_models, positioned_lights))
}

#[must_use]
pub fn export_rgm_metadata_json(rgm: &RgmFile) -> serde_json::Value {
    metadata::export_rgm_metadata_json_impl(rgm)
}

#[allow(clippy::missing_errors_doc)]
pub fn export_rgm_runtime_metadata_json(input: &[u8]) -> Result<serde_json::Value> {
    let rgm_file = parse_rgm_file(input)?;

    let mut root = match export_rgm_metadata_json(&rgm_file) {
        serde_json::Value::Object(map) => map,
        _ => serde_json::Map::new(),
    };

    let mut mpob_objects = Vec::new();
    for section in &rgm_file.sections {
        if let RgmSection::MpobParsed(_, records) = section {
            for (index, record) in records.iter().enumerate() {
                let position =
                    positioning::decode_position(record.pos_x, record.pos_y, record.pos_z);
                mpob_objects.push(serde_json::json!({
                    "index": index,
                    "id": record.id,
                    "object_type": record.object_type,
                    "is_active": record.is_active,
                    "is_static": record.is_static,
                    "script_name": record.script_name(),
                    "model_name": record.model_name(),
                    "texture_id": record.texture_id,
                    "image_id": record.image_id,
                    "position": position,
                    "angle_x": record.angle_x,
                    "angle_y": record.angle_y,
                    "angle_z": record.angle_z,
                    "model_id": record.model_id,
                    "world_id": record.world_id,
                }));
            }
        }
    }

    root.insert(
        "mpob_objects".into(),
        serde_json::Value::Array(mpob_objects),
    );
    Ok(serde_json::Value::Object(root))
}

#[allow(clippy::missing_errors_doc)] // Public wrapper extracts placements without loading models.
pub fn extract_rgm_placements(input: &[u8]) -> Result<(Vec<Placement>, Vec<PositionedLight>)> {
    let rgm_file = parse_rgm_file(input)?;
    Ok(positioning::extract_placements(input, &rgm_file))
}

#[allow(clippy::missing_errors_doc)] // Public wrapper delegates parse errors into crate error type.
pub fn dump_rgm(input: &[u8]) -> Result<String> {
    shared::dump_rgm_impl(input)
}

#[cfg(test)]
mod tests;
