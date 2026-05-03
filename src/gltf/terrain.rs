use log::warn;
use std::collections::BTreeMap;

use crate::{Result, import::wld::WldFile};

use super::{ENGINE_UNIT_SCALE, MaterialKey, UnrolledPrimitive};

pub(super) const WLD_SIZE_SCALE: f32 = 256.0 / ENGINE_UNIT_SCALE;

pub(super) const WLD_HEIGHT_TABLE: [u16; 128] = [
    0, 40, 40, 40, 80, 80, 80, 120, 120, 120, 160, 160, 160, 200, 200, 200, 240, 240, 240, 280,
    280, 320, 320, 320, 360, 360, 400, 400, 400, 440, 440, 480, 480, 480, 520, 520, 560, 560, 600,
    600, 600, 640, 640, 680, 680, 720, 720, 760, 760, 800, 800, 840, 840, 880, 880, 920, 920, 960,
    1000, 1000, 1040, 1040, 1080, 1120, 1120, 1160, 1160, 1200, 1240, 1240, 1280, 1320, 1320, 1360,
    1400, 1440, 1440, 1480, 1520, 1560, 1600, 1600, 1640, 1680, 1720, 1760, 1800, 1840, 1880, 1920,
    1960, 2000, 2040, 2080, 2120, 2200, 2240, 2280, 2320, 2400, 2440, 2520, 2560, 2640, 2680, 2760,
    2840, 2920, 3000, 3080, 3160, 3240, 3360, 3440, 3560, 3680, 3800, 3960, 4080, 4280, 4440, 4680,
    4920, 5200, 5560, 6040, 6680, 7760,
];

const UV_ROTATIONS: [[[f32; 2]; 6]; 4] = [
    [
        [1.0, 0.0],
        [1.0, 1.0],
        [0.0, 1.0],
        [0.0, 1.0],
        [0.0, 0.0],
        [1.0, 0.0],
    ],
    [
        [0.0, 0.0],
        [1.0, 0.0],
        [1.0, 1.0],
        [1.0, 1.0],
        [0.0, 1.0],
        [0.0, 0.0],
    ],
    [
        [0.0, 1.0],
        [0.0, 0.0],
        [1.0, 0.0],
        [1.0, 0.0],
        [1.0, 1.0],
        [0.0, 1.0],
    ],
    [
        [1.0, 1.0],
        [0.0, 1.0],
        [0.0, 0.0],
        [0.0, 0.0],
        [1.0, 0.0],
        [1.0, 1.0],
    ],
];

#[must_use]
fn map_side_from_square_len(len: usize) -> Option<usize> {
    if len == 0 {
        return None;
    }

    let mut side = 1usize;
    while side.saturating_mul(side) < len {
        side += 1;
    }

    (side.saturating_mul(side) == len).then_some(side)
}

fn scaled_grid_coordinate(value: usize) -> Result<f32> {
    let coord = u16::try_from(value).map_err(|_| {
        warn!("terrain grid coordinate {value} exceeds u16::MAX");
        crate::error::Error::Conversion("terrain grid coordinate exceeds u16::MAX".to_string())
    })?;
    Ok(f32::from(coord) * WLD_SIZE_SCALE)
}

fn terrain_position(heightmap: &[u8], map_side: usize, x: usize, y: usize) -> Result<[f32; 3]> {
    Ok([
        -scaled_grid_coordinate(x)?,
        wld_height(heightmap[x + y * map_side]),
        -scaled_grid_coordinate(y)?,
    ])
}

fn build_face_normals(
    heightmap: &[u8],
    map_side: usize,
    cells: usize,
) -> Result<Vec<([f32; 3], [f32; 3])>> {
    let mut face_normals: Vec<([f32; 3], [f32; 3])> = Vec::with_capacity(cells * cells);
    for y in 0..cells {
        for x in 0..cells {
            let top_left = terrain_position(heightmap, map_side, x, y)?;
            let top_right = terrain_position(heightmap, map_side, x + 1, y)?;
            let bottom_left = terrain_position(heightmap, map_side, x, y + 1)?;
            let bottom_right = terrain_position(heightmap, map_side, x + 1, y + 1)?;
            face_normals.push((
                triangle_normal(top_left, bottom_right, top_right),
                triangle_normal(bottom_right, top_left, bottom_left),
            ));
        }
    }
    Ok(face_normals)
}

#[must_use]
fn build_vertex_normals(
    face_normals: &[([f32; 3], [f32; 3])],
    map_side: usize,
    cells: usize,
) -> Vec<[f32; 3]> {
    let mut vertex_normals: Vec<[f32; 3]> = Vec::with_capacity(map_side * map_side);

    for gy in 0..map_side {
        for gx in 0..map_side {
            let mut acc = [0.0f32; 3];
            let cell =
                |cx: usize, cy: usize| -> &([f32; 3], [f32; 3]) { &face_normals[cx + cy * cells] };
            if gx > 0 && gy > 0 {
                let (tri_1, tri_2) = cell(gx - 1, gy - 1);
                acc[0] += tri_1[0] + tri_2[0];
                acc[1] += tri_1[1] + tri_2[1];
                acc[2] += tri_1[2] + tri_2[2];
            }
            if gx < cells && gy > 0 {
                let (_, tri_2) = cell(gx, gy - 1);
                acc[0] += tri_2[0];
                acc[1] += tri_2[1];
                acc[2] += tri_2[2];
            }
            if gx > 0 && gy < cells {
                let (tri_1, _) = cell(gx - 1, gy);
                acc[0] += tri_1[0];
                acc[1] += tri_1[1];
                acc[2] += tri_1[2];
            }
            if gx < cells && gy < cells {
                let (tri_1, tri_2) = cell(gx, gy);
                acc[0] += tri_1[0] + tri_2[0];
                acc[1] += tri_1[1] + tri_2[1];
                acc[2] += tri_1[2] + tri_2[2];
            }
            vertex_normals.push(normalize(acc));
        }
    }

    vertex_normals
}

#[must_use]
fn resolve_wld_material(
    primitive_groups: &mut BTreeMap<MaterialKey, UnrolledPrimitive>,
    texbsi_id: u16,
    tex_id: u8,
) -> &mut UnrolledPrimitive {
    let material_key = MaterialKey::TerrainTextured(texbsi_id, tex_id);
    primitive_groups
        .entry(material_key)
        .or_insert_with(|| UnrolledPrimitive {
            material_key,
            scale_uv_by_texture_dimensions: false,
            positions: Vec::new(),
            normals: Vec::new(),
            uvs: Vec::new(),
            indices: Vec::new(),
            min: [f32::INFINITY, f32::INFINITY, f32::INFINITY],
            max: [f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY],
        })
}

fn append_wld_cell(
    primitive: &mut UnrolledPrimitive,
    heightmap: &[u8],
    map_side: usize,
    vertex_normals: &[[f32; 3]],
    x: usize,
    y: usize,
    uv: [[f32; 2]; 6],
) -> Result<()> {
    let normal_at = |vx: usize, vy: usize| vertex_normals[vx + vy * map_side];
    let top_left = terrain_position(heightmap, map_side, x, y)?;
    let top_right = terrain_position(heightmap, map_side, x + 1, y)?;
    let bottom_left = terrain_position(heightmap, map_side, x, y + 1)?;
    let bottom_right = terrain_position(heightmap, map_side, x + 1, y + 1)?;
    let normal_top_left = normal_at(x, y);
    let normal_top_right = normal_at(x + 1, y);
    let normal_bottom_left = normal_at(x, y + 1);
    let normal_bottom_right = normal_at(x + 1, y + 1);

    push_terrain_vertex(primitive, top_left, normal_top_left, uv[2]);
    push_terrain_vertex(primitive, bottom_right, normal_bottom_right, uv[5]);
    push_terrain_vertex(primitive, top_right, normal_top_right, uv[1]);
    push_terrain_vertex(primitive, bottom_right, normal_bottom_right, uv[0]);
    push_terrain_vertex(primitive, top_left, normal_top_left, uv[3]);
    push_terrain_vertex(primitive, bottom_left, normal_bottom_left, uv[4]);
    Ok(())
}

#[must_use]
pub(super) fn wld_height(value: u8) -> f32 {
    f32::from(WLD_HEIGHT_TABLE[usize::from(value & 0x7F)]) / ENGINE_UNIT_SCALE
}

#[must_use]
pub(super) fn triangle_normal(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> [f32; 3] {
    let (a, b, c) = (
        glam::Vec3::from(a),
        glam::Vec3::from(b),
        glam::Vec3::from(c),
    );
    (b - a).cross(c - a).to_array()
}

#[must_use]
pub(super) fn normalize(n: [f32; 3]) -> [f32; 3] {
    glam::Vec3::from(n).normalize_or(glam::Vec3::Y).to_array()
}

#[allow(clippy::cast_possible_truncation)]
// GLTF indices are u32; terrain vertex counts are far below u32::MAX.
pub(super) fn push_terrain_vertex(
    primitive: &mut UnrolledPrimitive,
    pos: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
) {
    primitive.positions.push(pos);
    primitive.normals.push(normal);
    primitive.uvs.push(uv);
    primitive
        .indices
        .push((primitive.positions.len() - 1) as u32);
    primitive.min[0] = primitive.min[0].min(pos[0]);
    primitive.min[1] = primitive.min[1].min(pos[1]);
    primitive.min[2] = primitive.min[2].min(pos[2]);
    primitive.max[0] = primitive.max[0].max(pos[0]);
    primitive.max[1] = primitive.max[1].max(pos[1]);
    primitive.max[2] = primitive.max[2].max(pos[2]);
}

pub(crate) fn build_wld_unrolled_primitives(
    wld_file: &WldFile,
    texbsi_id: u16,
) -> Result<Vec<UnrolledPrimitive>> {
    let heightmap = wld_file.combined_map(0)?;
    let texturemap_raw = wld_file.combined_map(2)?;
    if heightmap.is_empty() || texturemap_raw.is_empty() || heightmap.len() != texturemap_raw.len()
    {
        return Ok(Vec::new());
    }

    let Some(map_side) = map_side_from_square_len(heightmap.len()) else {
        return Ok(Vec::new());
    };
    if map_side < 2 || map_side > usize::from(u16::MAX) {
        return Ok(Vec::new());
    }

    let cells = map_side - 1;
    let face_normals = build_face_normals(&heightmap, map_side, cells)?;
    let vertex_normals = build_vertex_normals(&face_normals, map_side, cells);

    let mut primitive_groups: BTreeMap<MaterialKey, UnrolledPrimitive> = BTreeMap::new();

    for y in 0..cells {
        for x in 0..cells {
            let idx = x + y * map_side;
            let tex_byte = texturemap_raw[idx];
            let tex_id = tex_byte & 0x3F;
            let tex_rot = usize::from((tex_byte & 0xC0) >> 6);
            let primitive = resolve_wld_material(&mut primitive_groups, texbsi_id, tex_id);
            append_wld_cell(
                primitive,
                &heightmap,
                map_side,
                &vertex_normals,
                x,
                y,
                UV_ROTATIONS[tex_rot.min(3)],
            )?;
        }
    }

    Ok(primitive_groups
        .into_values()
        .filter(|primitive| !primitive.indices.is_empty())
        .collect())
}
