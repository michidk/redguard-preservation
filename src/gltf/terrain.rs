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

pub(super) fn wld_height(value: u8) -> f32 {
    WLD_HEIGHT_TABLE[(value & 0x7F) as usize] as f32 / ENGINE_UNIT_SCALE
}

pub(super) fn triangle_normal(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> [f32; 3] {
    let (a, b, c) = (
        glam::Vec3::from(a),
        glam::Vec3::from(b),
        glam::Vec3::from(c),
    );
    (b - a).cross(c - a).to_array()
}

pub(super) fn normalize(n: [f32; 3]) -> [f32; 3] {
    glam::Vec3::from(n).normalize_or(glam::Vec3::Y).to_array()
}

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

pub(super) fn build_wld_unrolled_primitives(
    wld_file: &WldFile,
    texbsi_id: u16,
) -> Result<Vec<UnrolledPrimitive>> {
    let heightmap = wld_file.combined_map(0)?;
    let texturemap_raw = wld_file.combined_map(2)?;
    if heightmap.is_empty() || texturemap_raw.is_empty() || heightmap.len() != texturemap_raw.len()
    {
        return Ok(Vec::new());
    }

    let map_side = (heightmap.len() as f64).sqrt() as usize;
    if map_side < 2 || map_side * map_side != heightmap.len() {
        return Ok(Vec::new());
    }

    let uv_rotations: [[[f32; 2]; 6]; 4] = [
        [
            [1.0, 1.0],
            [1.0, 0.0],
            [0.0, 0.0],
            [0.0, 0.0],
            [0.0, 1.0],
            [1.0, 1.0],
        ],
        [
            [0.0, 1.0],
            [1.0, 1.0],
            [1.0, 0.0],
            [1.0, 0.0],
            [0.0, 0.0],
            [0.0, 1.0],
        ],
        [
            [0.0, 0.0],
            [0.0, 1.0],
            [1.0, 1.0],
            [1.0, 1.0],
            [1.0, 0.0],
            [0.0, 0.0],
        ],
        [
            [1.0, 0.0],
            [0.0, 0.0],
            [0.0, 1.0],
            [0.0, 1.0],
            [1.0, 1.0],
            [1.0, 0.0],
        ],
    ];

    let cells = map_side - 1;
    let h = |x: usize, y: usize| wld_height(heightmap[x + y * map_side]);
    let pos = |x: usize, y: usize| -> [f32; 3] {
        [
            -(x as f32 * WLD_SIZE_SCALE),
            h(x, y),
            -(y as f32 * WLD_SIZE_SCALE),
        ]
    };

    let mut face_normals: Vec<([f32; 3], [f32; 3])> = Vec::with_capacity(cells * cells);
    for y in 0..cells {
        for x in 0..cells {
            let tl = pos(x, y);
            let tr = pos(x + 1, y);
            let bl = pos(x, y + 1);
            let br = pos(x + 1, y + 1);
            face_normals.push((triangle_normal(tl, br, tr), triangle_normal(br, tl, bl)));
        }
    }

    let mut vertex_normals: Vec<[f32; 3]> = Vec::with_capacity(map_side * map_side);
    for gy in 0..map_side {
        for gx in 0..map_side {
            let mut acc = [0.0f32; 3];
            let cell =
                |cx: usize, cy: usize| -> &([f32; 3], [f32; 3]) { &face_normals[cx + cy * cells] };
            if gx > 0 && gy > 0 {
                let (t1, t2) = cell(gx - 1, gy - 1);
                acc[0] += t1[0] + t2[0];
                acc[1] += t1[1] + t2[1];
                acc[2] += t1[2] + t2[2];
            }
            if gx < cells && gy > 0 {
                let (_, t2) = cell(gx, gy - 1);
                acc[0] += t2[0];
                acc[1] += t2[1];
                acc[2] += t2[2];
            }
            if gx > 0 && gy < cells {
                let (t1, _) = cell(gx - 1, gy);
                acc[0] += t1[0];
                acc[1] += t1[1];
                acc[2] += t1[2];
            }
            if gx < cells && gy < cells {
                let (t1, t2) = cell(gx, gy);
                acc[0] += t1[0] + t2[0];
                acc[1] += t1[1] + t2[1];
                acc[2] += t1[2] + t2[2];
            }
            vertex_normals.push(normalize(acc));
        }
    }

    let mut primitive_groups: BTreeMap<MaterialKey, UnrolledPrimitive> = BTreeMap::new();
    let vn = |x: usize, y: usize| vertex_normals[x + y * map_side];

    for y in 0..cells {
        for x in 0..cells {
            let idx = x + y * map_side;
            let tex_byte = texturemap_raw[idx];
            let tex_id = tex_byte & 0x3F;
            let tex_rot = ((tex_byte & 0xC0) >> 6) as usize;
            let material_key = MaterialKey::Textured(texbsi_id, tex_id);

            let primitive =
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
                    });

            let tl = pos(x, y);
            let tr = pos(x + 1, y);
            let bl = pos(x, y + 1);
            let br = pos(x + 1, y + 1);
            let n_tl = vn(x, y);
            let n_tr = vn(x + 1, y);
            let n_bl = vn(x, y + 1);
            let n_br = vn(x + 1, y + 1);

            let uv = uv_rotations[tex_rot.min(3)];

            push_terrain_vertex(primitive, tl, n_tl, uv[2]);
            push_terrain_vertex(primitive, br, n_br, uv[0]);
            push_terrain_vertex(primitive, tr, n_tr, uv[1]);
            push_terrain_vertex(primitive, br, n_br, uv[5]);
            push_terrain_vertex(primitive, tl, n_tl, uv[3]);
            push_terrain_vertex(primitive, bl, n_bl, uv[4]);
        }
    }

    Ok(primitive_groups
        .into_values()
        .filter(|primitive| !primitive.indices.is_empty())
        .collect())
}
