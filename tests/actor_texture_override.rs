use rgpre::{
    import::rgm::apply_texture_override,
    model3d::{FaceData, TextureData, parse_3d_file},
};

#[test]
fn actor_skins_preserve_solids_reserved_banks_and_image_indices() {
    for set in [2, 445, 454] {
        let mut model = parse_3d_file(include_bytes!("fixtures/model_v40.3dc")).unwrap();
        let face = FaceData {
            vertex_count: 3,
            tex_hi: 173,
            texture_data: TextureData::SolidColor(0),
            face_vertices: (0..3)
                .map(|vertex_index| rgpre::model3d::FaceVertex {
                    vertex_index,
                    u: 17,
                    v: -33,
                })
                .collect(),
        };
        model.face_data = [
            TextureData::SolidColor(0),
            TextureData::SolidColor(173),
            TextureData::Texture {
                texture_id: 0,
                image_id: 7,
            },
            TextureData::Texture {
                texture_id: 1,
                image_id: 9,
            },
            TextureData::Texture {
                texture_id: 2,
                image_id: 11,
            },
            TextureData::Texture {
                texture_id: 453,
                image_id: 13,
            },
        ]
        .into_iter()
        .map(|texture_data| FaceData {
            texture_data,
            ..face.clone()
        })
        .collect();
        let before = serde_json::to_value(&model).unwrap();
        apply_texture_override(&mut model, set);
        let mut expected = before;
        for index in [4, 5] {
            expected["face_data"][index]["texture_data"]["Texture"]["texture_id"] = set.into();
        }
        assert_eq!(
            serde_json::to_value(&model).unwrap(),
            expected,
            "skin {set}"
        );
    }
}

#[test]
fn installed_thug_skin_retains_all_solid_faces() {
    let Some(root) = std::env::var_os("REDGUARD_DIR") else {
        eprintln!("skipped: REDGUARD_DIR absent for installed actor skin");
        return;
    };
    let bytes = std::fs::read(std::path::Path::new(&root).join("fxart/THUGA001.3DC")).unwrap();
    let source = parse_3d_file(&bytes).unwrap();
    let solid = |model: &rgpre::model3d::Model3DFile| {
        model
            .face_data
            .iter()
            .filter_map(|face| match face.texture_data {
                TextureData::SolidColor(index) => Some(index),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let expected = solid(&source);
    assert_eq!(expected.len(), 8);
    for set in [2, 445, 454] {
        let mut model = source.clone();
        apply_texture_override(&mut model, set);
        assert_eq!(solid(&model), expected, "skin {set}");
    }
}
