use rgpre::{
    gltf::TextureCache,
    model3d::{FaceData, FaceVertex, TextureData, VertexCoord},
};
use std::collections::HashMap;

fn texture_bank(pixels: [u8; 4]) -> Vec<u8> {
    let mut chunks = Vec::new();
    let mut header = [0; 26];
    header[4..6].copy_from_slice(&1_u16.to_le_bytes());
    header[6..8].copy_from_slice(&2_u16.to_le_bytes());
    header[14..16].copy_from_slice(&2_u16.to_le_bytes());
    header[16..18].copy_from_slice(&500_i16.to_le_bytes());
    header[24..26].copy_from_slice(&4_i16.to_le_bytes());
    let mut rows: Vec<_> = [16_u32, 17, 18, 19]
        .into_iter()
        .flat_map(u32::to_le_bytes)
        .collect();
    rows.extend(pixels);
    for (tag, data) in [
        (b"IFHD", vec![0; 44]),
        (b"BHDR", header.to_vec()),
        (b"DATA", rows),
    ] {
        chunks.extend(tag);
        chunks.extend(u32::try_from(data.len()).unwrap().to_be_bytes());
        chunks.extend(data);
    }
    chunks.extend(b"END ");
    let mut bytes = b"A01000\0\0\0".to_vec();
    bytes.extend(u32::try_from(chunks.len()).unwrap().to_le_bytes());
    bytes.extend(chunks);
    bytes
}

#[test]
fn animated_material_exports_ordered_frames_delay_and_later_transparency() {
    let bytes = texture_bank([1, 2, 0, 4]);
    let mut cache = TextureCache::from_data(HashMap::from([(1, bytes)]), None);
    let mut model =
        rgpre::model3d::parse_3d_file(include_bytes!("fixtures/model_v40.3dc")).unwrap();
    model.vertex_coords = [[0.0, 0.0, 0.0], [20.0, 0.0, 0.0], [0.0, 20.0, 0.0]]
        .into_iter()
        .map(|[x, y, z]| VertexCoord { x, y, z })
        .collect();
    model.face_data = vec![
        FaceData {
            vertex_count: 3,
            tex_hi: 0,
            texture_data: TextureData::Texture {
                texture_id: 1,
                image_id: 0
            },
            face_vertices: (0..3)
                .map(|vertex_index| FaceVertex {
                    vertex_index,
                    u: 0,
                    v: 0
                })
                .collect(),
        };
        2
    ];
    let (root, buffer) =
        rgpre::convert_models_to_gltf(&[model], None, Some(&mut cache), false).unwrap();
    let glb = rgpre::to_glb(&root, &buffer).unwrap();
    let parsed = gltf::Gltf::from_slice(&glb).unwrap();
    let material = parsed.materials().next().unwrap();
    let extras: serde_json::Value =
        serde_json::from_str(material.extras().as_ref().unwrap().get()).unwrap();
    let animation = &extras["rgpre_texture_animation"];
    assert_eq!(animation["version"], 1);
    assert_eq!(animation["image_name"], "A01000");
    assert_eq!(animation["delay_ms"], 500);
    assert_eq!(animation["textures"], serde_json::json!([0, 1]));
    assert_eq!(material.alpha_mode(), gltf::material::AlphaMode::Mask);
    assert_eq!(parsed.materials().count(), 1);
    let mut images = parsed.images();
    for expected in [[2, 2, 2, 255, 1, 1, 1, 255], [4, 4, 4, 255, 0, 0, 0, 0]] {
        let gltf::image::Source::View { view, .. } = images.next().unwrap().source() else {
            panic!("embedded PNG expected")
        };
        let png = &buffer[view.offset()..view.offset() + view.length()];
        let decoded = image::load_from_memory(png).unwrap().into_rgba8();
        assert_eq!(decoded.dimensions(), (1, 2));
        assert_eq!(decoded.as_raw(), &expected);
    }
    assert!(images.next().is_none());
}

#[test]
fn terrain_frames_keep_opaque_palette_zero_without_changing_model_decoding() {
    use rgpre::import::{palette::Palette, wld::parse_wld_file};
    let mut palette = Palette {
        colors: [[0; 3]; 256],
    };
    palette.colors[0] = [19, 37, 83];
    palette.colors[2] = [101, 103, 107];
    palette.colors[4] = [211, 223, 227];
    let mut cache = TextureCache::from_data(
        HashMap::from([(1, texture_bank([0, 2, 0, 4]))]),
        Some(Palette {
            colors: palette.colors,
        }),
    );
    let mut world = parse_wld_file(include_bytes!("fixtures/world.wld")).unwrap();
    for section in world.sections.iter_mut() {
        section.maps[2].fill(0);
    }
    let (root, buffer) =
        rgpre::convert_wld_scene_to_gltf(&world, 1, &[], Some(&palette), Some(&mut cache), false)
            .unwrap();
    let glb = rgpre::to_glb(&root, &buffer).unwrap();
    let parsed = gltf::Gltf::from_slice(&glb).unwrap();
    let material = parsed.materials().next().unwrap();
    assert_eq!(parsed.materials().count(), 1);
    assert_eq!(material.alpha_mode(), gltf::material::AlphaMode::Opaque);
    let extras: serde_json::Value =
        serde_json::from_str(material.extras().as_ref().unwrap().get()).unwrap();
    assert_eq!(
        extras["rgpre_texture_animation"]["textures"],
        serde_json::json!([0, 1])
    );
    let mut images = parsed.images();
    for expected in [
        [101, 103, 107, 255, 19, 37, 83, 255],
        [211, 223, 227, 255, 19, 37, 83, 255],
    ] {
        let gltf::image::Source::View { view, .. } = images.next().unwrap().source() else {
            panic!("embedded PNG expected")
        };
        let png = &buffer[view.offset()..view.offset() + view.length()];
        assert_eq!(
            image::load_from_memory(png).unwrap().into_rgba8().as_raw(),
            &expected
        );
    }
    assert!(images.next().is_none());
    assert_eq!(
        cache.get_image_rgba(1, 0).unwrap().0,
        [101, 103, 107, 255, 19, 37, 83, 0]
    );
}
