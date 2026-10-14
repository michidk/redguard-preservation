use rgpre::import::rgm::{RgmSection, parser::parse_rgm_file};

#[test]
fn installed_flat_attributes_decode_every_map_and_candle_override() {
    let Some(root) = std::env::var_os("REDGUARD_DIR") else {
        eprintln!("SKIP installed flat attributes: REDGUARD_DIR is absent");
        return;
    };
    let maps = std::path::PathBuf::from(root).join("maps");
    let mut count = 0;
    for entry in std::fs::read_dir(maps).unwrap() {
        let path = entry.unwrap().path();
        if !path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("rgm"))
        {
            continue;
        }
        let bytes = std::fs::read(&path).unwrap();
        let (_, rgm) = parse_rgm_file(&bytes).unwrap();
        assert!(
            !rgm.sections
                .iter()
                .any(|section| matches!(section, RgmSection::Flat(_, _))),
            "{}: raw FLAT payload",
            path.display()
        );
        assert_eq!(
            rgm.sections
                .iter()
                .filter(|section| matches!(section, RgmSection::FlatParsed(_, _)))
                .count(),
            1,
            "{}: expected one typed FLAT section",
            path.display()
        );
        if path.file_stem().unwrap().eq_ignore_ascii_case("BELLTOWR") {
            let candle = rgm.flat_texture_attributes((352 << 7) | 1).unwrap();
            assert_eq!(candle.flags, 0);
            assert_eq!(candle.alpha(), 181);
        }
        count += 1;
    }
    assert_eq!(count, 27);
}

fn flat_world(attributes: Option<&[u8]>) -> Vec<u8> {
    let mut record = vec![0; 24];
    record[8..12].copy_from_slice(&20i32.to_le_bytes());
    record[12..16].copy_from_slice(&(-40i32).to_le_bytes());
    record[16..20].copy_from_slice(&100u32.to_le_bytes());
    record[20..22].copy_from_slice(&(12u16 << 7).to_le_bytes());
    let mut payload = 1u32.to_le_bytes().to_vec();
    payload.extend(record);
    let mut bytes = Vec::new();
    for (tag, data) in
        std::iter::once((b"MPSF", payload.as_slice())).chain(attributes.map(|data| (b"FLAT", data)))
    {
        bytes.extend(tag);
        bytes.extend(u32::try_from(data.len()).unwrap().to_be_bytes());
        bytes.extend(data);
    }
    bytes.extend(b"END ");
    bytes.extend(0u32.to_be_bytes());
    bytes
}

fn flat_texture() -> Vec<u8> {
    let mut header = [0; 26];
    header[4..6].copy_from_slice(&9i16.to_le_bytes());
    header[6..8].copy_from_slice(&26i16.to_le_bytes());
    header[14..16].copy_from_slice(&1i16.to_le_bytes());
    header[22..24].copy_from_slice(&128u16.to_le_bytes());
    let mut bytes = Vec::new();
    for (tag, payload) in [
        (b"IFHD", vec![0; 44]),
        (b"BHDR", header.to_vec()),
        (b"CMAP", vec![255; 768]),
        (b"DATA", vec![1; 9 * 26]),
    ] {
        bytes.extend(tag);
        bytes.extend(u32::try_from(payload.len()).unwrap().to_be_bytes());
        bytes.extend(payload);
    }
    bytes.extend(b"END ");
    bytes
}

#[test]
fn flat_export_retains_texture_dimensions_scale_and_authored_alpha() {
    use rgpre::{
        gltf::TextureCache,
        import::{registry::Registry, rgm::parse_rgm_with_models},
    };
    let mut attributes = vec![0; 1024];
    attributes[24..26].copy_from_slice(&1u16.to_le_bytes());
    attributes.extend([0, 32, 180]);
    for (attributes, flags, alpha) in [(None, 1, 255), (Some(attributes.as_slice()), 33, 181)] {
        let (_, models, _) =
            parse_rgm_with_models(&flat_world(attributes), &Registry::new("unused".into()))
                .unwrap();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].flat.unwrap().texture_id, 12);
        assert_eq!(models[0].flat.unwrap().image_id, 0);
        let mut cache = TextureCache::from_data(
            std::collections::HashMap::from([(12, flat_texture())]),
            None,
        );
        let (document, _) = rgpre::gltf::convert_positioned_models_to_gltf(
            &models,
            &[],
            None,
            Some(&mut cache),
            false,
        )
        .unwrap();
        let extras: serde_json::Value =
            serde_json::from_str(document.nodes[0].extras.as_ref().unwrap().get()).unwrap();
        assert_eq!(
            extras["rgpre_flat"],
            serde_json::json!({"version":1,"texture_id":12,"image_id":0,"dimensions":[9,26],"texture_scale":0.5,"flags":flags,"alpha":alpha})
        );
        let (missing, _) =
            rgpre::gltf::convert_positioned_models_to_gltf(&models, &[], None, None, false)
                .unwrap();
        let extras: serde_json::Value =
            serde_json::from_str(missing.nodes[0].extras.as_ref().unwrap().get()).unwrap();
        assert!(extras["rgpre_flat"]["dimensions"].is_null());
        assert!(extras["rgpre_flat"]["texture_scale"].is_null());
    }
}

#[test]
fn malformed_flat_attributes_report_the_affected_placement() {
    use rgpre::import::{registry::Registry, rgm::parse_rgm_with_models_report};
    let (_, models, _, diagnostics) = parse_rgm_with_models_report(
        &flat_world(Some(&[0; 4])),
        &Registry::new("unused".into()),
        None,
    )
    .unwrap();
    assert!(models.is_empty());
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].field, "MPSF[0].texture_data");
    assert_eq!(diagnostics[0].model, "TEXBSI.012/0");
    assert_eq!(diagnostics[0].reason, "malformed FLAT texture attributes");
}

#[test]
fn installed_candle_export_identifies_sprite_size_and_opacity() {
    use rgpre::{
        gltf::TextureCache,
        import::{registry::Registry, rgm::parse_rgm_with_models},
    };
    let Some(root) = std::env::var_os("REDGUARD_DIR") else {
        eprintln!("SKIP installed candle export: REDGUARD_DIR is absent");
        return;
    };
    let root = std::path::PathBuf::from(root);
    let bytes = std::fs::read(root.join("maps/BELLTOWR.RGM")).unwrap();
    let (_, models, _) = parse_rgm_with_models(&bytes, &Registry::new(root.clone())).unwrap();
    let candle = models
        .into_iter()
        .find(|model| model.model_name == "F000_352/1")
        .unwrap();
    let mut cache = TextureCache::from_data(
        std::collections::HashMap::from([(
            352,
            std::fs::read(root.join("fxart/TEXBSI.352")).unwrap(),
        )]),
        None,
    );
    let (document, _) = rgpre::gltf::convert_positioned_models_to_gltf(
        &[candle],
        &[],
        None,
        Some(&mut cache),
        false,
    )
    .unwrap();
    let extras: serde_json::Value =
        serde_json::from_str(document.nodes[0].extras.as_ref().unwrap().get()).unwrap();
    assert_eq!(
        extras["rgpre_flat"],
        serde_json::json!({"version":1,"texture_id":352,"image_id":1,"dimensions":[9,26],"texture_scale":0.5,"flags":1,"alpha":181})
    );
}
