use bytemuck::{Pod, Zeroable};

// --- Texture decode output ---

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct TextureHeader {
    pub width: i32,
    pub height: i32,
    pub frame_count: i32,
    pub rgba_size: i32,
}
// 16 bytes, align 4. Followed by `rgba_size` bytes of RGBA pixel data.

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct AllFramesHeader {
    pub width: i32,
    pub height: i32,
    pub frame_count: i32,
}
// 12 bytes, align 4. Followed by `frame_count` frames, each prefixed by an i32 size.

// --- RGMD (model/terrain mesh data) ---

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct RgmdHeader {
    pub magic: [u8; 4],
    pub version: [u8; 4],
    pub submesh_count: i32,
    pub frame_count: i32,
    pub total_vertex_count: i32,
    pub total_index_count: i32,
    pub radius: u32,
}
// 28 bytes, align 4.

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct RgmdSubmeshHeader {
    pub material_type: u8,
    pub color_index: u8,
    pub texture_id: u16,
    pub image_id: u8,
    pub _pad: [u8; 3],
    pub vertex_count: i32,
    pub index_count: i32,
}
// 16 bytes, align 4. material_type: 0 = solid color (color_index is palette index),
// 1 = textured (texture_id + image_id identify the TEXBSI image).

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct RgmdVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}
// 32 bytes, align 4. Followed by index_count × u32 indices.

// --- RGPL (placements + lights) ---

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct RgplHeader {
    pub magic: [u8; 4],
    pub placement_count: i32,
    pub light_count: i32,
}
// 12 bytes, align 4.

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct RgplPlacement {
    pub model_name: [u8; 32],
    pub source_id: [u8; 32],
    pub transform: [f32; 16],
    pub texture_id: u16,
    pub image_id: u8,
    pub object_type: u8,
}
// 132 bytes, align 4.

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct RgplLight {
    pub name: [u8; 32],
    pub color: [f32; 3],
    pub position: [f32; 3],
    pub range: f32,
}
// 60 bytes, align 4.

// --- ROB (segment archive) ---

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct RobHeader {
    pub segment_count: i32,
}
// 4 bytes, align 4.

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct RobSegmentHeader {
    pub segment_name: [u8; 8],
    pub has_model: u8,
    pub _pad: [u8; 3],
    pub model_data_size: i32,
}
// 16 bytes, align 4. If has_model == 1, followed by model_data_size bytes of RGMD data.
