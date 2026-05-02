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
    pub radius: f32,
}
// 28 bytes, align 4.

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct RgmdSubmeshHeader {
    pub textured: u8,
    pub color_r: u8,
    pub color_g: u8,
    pub color_b: u8,
    pub texture_id: u16,
    pub image_id: u8,
    pub _pad: u8,
    pub vertex_count: i32,
    pub index_count: i32,
}
// 16 bytes, align 4. textured: 0 = solid color (color_r/g/b hold resolved RGB),
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
// 132 bytes, align 4. object_type: 0 = mesh, 1 = flat sprite, 2 = rope link.
// texture_id/image_id: TEXBSI texture for flat sprites; 0 for mesh/rope placements.

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct RgplLight {
    pub name: [u8; 32],
    pub color: [f32; 3],
    pub position: [f32; 3],
    pub range: f32,
    pub light_type: u8,
    pub _pad: [u8; 3],
}
// 64 bytes, align 4. light_type: 0 = point. color is linear RGB 0.0–1.0.

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

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct RgWorldDescriptor {
    pub world_id: i32,
    pub has_wld: u8,
    pub _pad: [u8; 3],
    pub texbsi_id: u16,
    pub _pad2: [u8; 2],
    pub rgm_path: [u8; 64],
    pub wld_path: [u8; 64],
    pub palette_path: [u8; 64],
}
// 204 bytes, align 4. Paths are null-terminated ASCII, zero-padded.
