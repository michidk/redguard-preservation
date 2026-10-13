use rgpre::import::software_texture::{
    PaletteLookupTable, SoftwareTextureEntry, parse_texture_bank,
};

fn bank(width: u16, height: u16, frames: u16, flags: u16, data: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0; 74];
    bytes[..2].copy_from_slice(&1u16.to_le_bytes());
    bytes[28..32].copy_from_slice(&46u32.to_le_bytes());
    for (offset, value) in [(50, width), (52, height), (54, flags), (66, frames)] {
        bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }
    bytes[60..64].copy_from_slice(&28u32.to_le_bytes());
    bytes.extend_from_slice(data);
    bytes
}

#[test]
fn packed_and_padded_rows_preserve_pixels() {
    for (flags, data) in [
        (0, vec![1, 2, 3, 4]),
        (0x800, {
            let mut v = vec![99; 258];
            v[..2].copy_from_slice(&[1, 2]);
            v[256..].copy_from_slice(&[3, 4]);
            v
        }),
    ] {
        let SoftwareTextureEntry::Image(image) = parse_texture_bank(&bank(2, 2, 1, flags, &data))
            .unwrap()
            .entries
            .remove(0)
        else {
            panic!()
        };
        assert_eq!(image.frames, vec![vec![1, 2, 3, 4]]);
    }
}

#[test]
fn animated_runs_and_corruption() {
    let data = [
        8, 0, 0, 0, 16, 0, 0, 0, 3, 0, 1, 0, 1, 2, 7, 8, 3, 0, 1, 0, 0, 3, 4, 5, 6,
    ];
    let bytes = bank(3, 1, 2, 0x900, &data);
    let SoftwareTextureEntry::Image(image) = parse_texture_bank(&bytes).unwrap().entries.remove(0)
    else {
        panic!()
    };
    assert_eq!(image.frames, vec![vec![0, 7, 8], vec![4, 5, 6]]);
    for end in 0..bytes.len() {
        assert!(
            parse_texture_bank(&bytes[..end]).is_err(),
            "truncation {end}"
        );
    }
    let mut invalid = bytes.clone();
    invalid[86] = 0;
    invalid[87] = 0;
    assert!(parse_texture_bank(&invalid).is_err());
}

#[test]
fn solid_colors_empty_records_and_lookup_tables() {
    let mut bytes = vec![0; 46];
    bytes[0] = 1;
    bytes[27] = 191;
    assert_eq!(
        parse_texture_bank(&bytes).unwrap().entries,
        vec![SoftwareTextureEntry::SolidColor(191)]
    );
    let SoftwareTextureEntry::Image(image) = parse_texture_bank(&bank(6, 0, 0, 0x900, &[]))
        .unwrap()
        .entries
        .remove(0)
    else {
        panic!()
    };
    assert!(image.frames.is_empty());
    let mut data = vec![0; 16384];
    data[5 * 256 + 12] = 99;
    let table = PaletteLookupTable::parse(&data).unwrap();
    assert_eq!(table.map(5, 12), Some(99));
    assert_eq!(table.map(64, 12), None);
    assert!(PaletteLookupTable::parse(&data[..16383]).is_err());
    data.push(0);
    assert!(PaletteLookupTable::parse(&data).is_err());
}

#[test]
fn software_model_deltas_add_to_nonzero_reference_and_keep_normal_component_order() {
    use rgpre::model3d;
    let mut bytes = vec![0; 206];
    bytes[..4].copy_from_slice(b"v2.7");
    let words = [3u32, 1, 0, 2, 64, 0, 0, 0, 0, 0, 1, 162, 198, 0, 112];
    for (i, v) in words.into_iter().enumerate() {
        bytes[4 + i * 4..8 + i * 4].copy_from_slice(&v.to_le_bytes());
    }
    bytes[64..68].copy_from_slice(&88u32.to_le_bytes());
    bytes[76..78].copy_from_slice(&1u16.to_le_bytes());
    bytes[78..80].copy_from_slice(&1u16.to_le_bytes());
    bytes[87] = 2;
    for (i, (v, n)) in [(144u32, 198u32), (162, 202)].into_iter().enumerate() {
        let o = 88 + i * 12;
        bytes[o..o + 4].copy_from_slice(&v.to_le_bytes());
        bytes[o + 4..o + 8].copy_from_slice(&n.to_le_bytes());
    }
    bytes[112] = 3;
    for vertex in 0..3 {
        for (axis, v) in [-64i16, 128, 256].into_iter().enumerate() {
            let o = 144 + vertex * 6 + axis * 2;
            bytes[o..o + 2].copy_from_slice(&v.to_le_bytes());
        }
        for (axis, v) in [256i32, 512, 768].into_iter().enumerate() {
            let o = 162 + vertex * 12 + axis * 4;
            bytes[o..o + 4].copy_from_slice(&v.to_le_bytes());
        }
    }
    for o in [198, 202] {
        bytes[o..o + 4].copy_from_slice(&((256u32 << 20) | (896u32 << 10) | 64).to_le_bytes());
    }
    let model = model3d::parse_3d_file(&bytes).unwrap();
    let p = model.vertex_coords[0];
    assert_eq!([p.x, p.y, p.z], [0., 4., 7.]);
    let p = model.at_frame(1).unwrap().vertex_coords[0];
    assert_eq!([p.x, p.y, p.z], [1., 2., 3.]);
    let n = &model.face_normals[0];
    assert_eq!([n.x, n.y, n.z], [1., -0.5, 0.25]);
    for end in 0..bytes.len() {
        assert!(
            model3d::parse_3d_file(&bytes[..end]).is_err(),
            "truncation {end}"
        );
    }
    bytes[78..80].copy_from_slice(&2u16.to_le_bytes());
    assert!(model3d::parse_3d_file(&bytes).is_err());
}

#[test]
fn software_cache_preserves_palette_zero_and_does_not_fold_bank_ids() {
    use rgpre::{gltf::TextureCache, import::palette::Palette};
    let mut palette = Palette {
        colors: [[8, 9, 10]; 256],
    };
    palette.colors[0] = [1, 2, 3];
    palette.colors[7] = [11, 12, 13];
    let mut cache = TextureCache::from_software_data(
        std::collections::HashMap::from([(2, bank(2, 2, 1, 0, &[0, 7, 7, 0]))]),
        Some(palette),
    )
    .unwrap();
    assert_eq!(
        cache.get_image_rgba(2, 0),
        Some((
            vec![11, 12, 13, 255, 1, 2, 3, 255, 1, 2, 3, 255, 11, 12, 13, 255],
            2,
            2
        ))
    );
    assert_eq!(cache.get_image_dimensions(514, 0), None);
    assert!(
        TextureCache::from_software_data(std::collections::HashMap::from([(2, vec![1])]), None)
            .is_err()
    );
    let mut cache = TextureCache::from_software_data(
        std::collections::HashMap::from([(2, bank(1, 1, 1, 0x100, &[0]))]),
        None,
    )
    .unwrap();
    assert_eq!(cache.get_image_rgba(2, 0), Some((vec![0, 0, 0, 0], 1, 1)));
}

#[test]
fn registry_keeps_numeric_banks_and_selects_one_art_tree() {
    use rgpre::import::{
        FileType,
        registry::{Registry, scan_dir_for_edition},
    };
    let mut registry = Registry::new("/test".into());
    registry.add_file("/test/3dart/TEXTURE.002", FileType::Texture);
    registry.add_file("/test/3dart/TEXTURE.003", FileType::Texture);
    assert!(registry.get_file_by_name("TEXTURE.002").is_some());
    assert!(registry.get_file_by_name("TEXTURE.003").is_some());
    let root = std::env::temp_dir().join(format!("rgpre-edition-{}", std::process::id()));
    std::fs::create_dir_all(root.join("fxart")).unwrap();
    std::fs::create_dir_all(root.join("3dart")).unwrap();
    std::fs::write(root.join("fxart/MODEL.3D"), []).unwrap();
    std::fs::write(root.join("3dart/MODEL.3D"), []).unwrap();
    for software in [false, true] {
        let registry = scan_dir_for_edition(&root, software).unwrap();
        assert_eq!(registry.files.len(), 1);
        assert!(
            registry
                .get_file_by_name("MODEL")
                .unwrap()
                .path
                .starts_with(root.join(if software { "3dart" } else { "fxart" }))
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn software_uncompressed_frames_use_full_positions_and_normals() {
    let mut bytes = vec![0; 176];
    bytes[..4].copy_from_slice(b"v2.6");
    for (i, value) in [1u32, 1, 0, 2, 64, 0, 0, 0, 0, 0, 1, 128, 140, 0, 120]
        .into_iter()
        .enumerate()
    {
        bytes[4 + i * 4..8 + i * 4].copy_from_slice(&value.to_le_bytes());
    }
    bytes[64..68].copy_from_slice(&88u32.to_le_bytes());
    bytes[76..78].copy_from_slice(&2u16.to_le_bytes());
    for (i, (v, n)) in [(128u32, 140u32), (152, 164)].into_iter().enumerate() {
        let o = 88 + i * 16;
        bytes[o..o + 4].copy_from_slice(&v.to_le_bytes());
        bytes[o + 4..o + 8].copy_from_slice(&n.to_le_bytes());
    }
    for (o, values) in [
        (128, [256i32, 512, 768]),
        (140, [0, 0, 256]),
        (152, [-512, 128, 1024]),
        (164, [0, -256, 0]),
    ] {
        for (axis, value) in values.into_iter().enumerate() {
            bytes[o + axis * 4..o + axis * 4 + 4].copy_from_slice(&value.to_le_bytes());
        }
    }
    let model = rgpre::model3d::parse_3d_file(&bytes).unwrap();
    let frame = model.at_frame(1).unwrap();
    let p = frame.vertex_coords[0];
    assert_eq!([p.x, p.y, p.z], [-2., 0.5, 4.]);
    let n = &frame.face_normals[0];
    assert_eq!([n.x, n.y, n.z], [0., -1., 0.]);
    assert!(rgpre::model3d::parse_3d_file(&bytes[..175]).is_err());
}

#[test]
fn software_cache_uses_the_engines_three_digit_filename() {
    let root = std::env::temp_dir().join(format!("rgpre-texture-name-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("TEXTURE.071"), bank(1, 1, 1, 0, &[9])).unwrap();
    std::fs::write(root.join("TEXTURE.71"), bank(2, 1, 1, 0, &[3, 4])).unwrap();
    let mut cache = rgpre::gltf::TextureCache::new(root.clone(), None);
    assert_eq!(
        cache.get_image_rgba(71, 0),
        Some((vec![9, 9, 9, 255], 1, 1))
    );
    std::fs::remove_dir_all(root).unwrap();
}
