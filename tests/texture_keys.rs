use rgpre::{
    gltf::TextureCache,
    import::{bsi::parse_bsi_file, palette::Palette},
};
use std::collections::HashMap;

fn texture() -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut header = [0; 26];
    header[4..6].copy_from_slice(&2_i16.to_le_bytes());
    header[6..8].copy_from_slice(&1_i16.to_le_bytes());
    header[14..16].copy_from_slice(&1_i16.to_le_bytes());
    let mut palette = vec![0; 768];
    palette[..6].copy_from_slice(&[17, 31, 63, 20, 40, 80]);
    for (tag, payload) in [
        (b"IFHD", vec![0; 44]),
        (b"BHDR", header.to_vec()),
        (b"CMAP", palette),
        (b"DATA", vec![0, 1]),
    ] {
        bytes.extend(tag);
        bytes.extend(u32::try_from(payload.len()).unwrap().to_be_bytes());
        bytes.extend(payload);
    }
    bytes.extend(b"END ");
    bytes
}

#[test]
fn texture_export_preserves_palette_zero_rgb_and_key_with_external_palette_precedence() {
    let mut colors = [[0; 3]; 256];
    colors[0] = [7, 19, 43];
    colors[1] = [80, 90, 100];
    for (palette, expected) in [
        (None, [17, 31, 63, 0, 20, 40, 80, 255]),
        (Some(Palette { colors }), [7, 19, 43, 0, 80, 90, 100, 255]),
    ] {
        let image = parse_bsi_file(&texture()).unwrap().images.remove(0);
        assert_eq!(image.decode_rgba(palette.as_ref()), expected);
        let mut animated = image;
        animated.extra_frames.push(vec![1, 0]);
        assert_eq!(
            animated.decode_frame_rgba(1, palette.as_ref()).unwrap(),
            [expected[4..8].to_vec(), expected[0..4].to_vec()].concat()
        );
        let mut cache = TextureCache::from_data(HashMap::from([(12, texture())]), palette);
        let mut model =
            rgpre::model3d::parse_3d_file(include_bytes!("fixtures/model_v40.3dc")).unwrap();
        model.vertex_coords = [[0.0, 0.0, 0.0], [20.0, 0.0, 0.0], [0.0, 20.0, 0.0]]
            .map(|[x, y, z]| rgpre::model3d::VertexCoord { x, y, z })
            .to_vec();
        model.face_data = vec![rgpre::model3d::FaceData {
            vertex_count: 3,
            tex_hi: 0,
            texture_data: rgpre::model3d::TextureData::SolidColor(0),
            face_vertices: (0..3)
                .map(|vertex_index| rgpre::model3d::FaceVertex {
                    vertex_index,
                    u: 0,
                    v: 0,
                })
                .collect(),
        }];
        for face in &mut model.face_data {
            face.texture_data = rgpre::model3d::TextureData::Texture {
                texture_id: 12,
                image_id: 0,
            };
        }
        let (document, buffer) =
            rgpre::convert_models_to_gltf(&[model], None, Some(&mut cache), false).unwrap();
        let material = &document.materials[0];
        let extra: serde_json::Value =
            serde_json::from_str(material.extras.as_ref().unwrap().get()).unwrap();
        assert_eq!(
            extra["rgpre_texture"],
            serde_json::json!({"version":1,"chroma_key_rgb":&expected[..3]})
        );
        let view = &document.buffer_views[document.images[0].buffer_view.unwrap().value()];
        let offset = view.byte_offset.unwrap_or_default().0 as usize;
        let png = &buffer[offset..offset + view.byte_length.0 as usize];
        assert_eq!(
            image::load_from_memory(png).unwrap().to_rgba8().as_raw(),
            &expected
        );
    }
}
