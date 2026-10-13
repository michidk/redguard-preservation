use rgpre::import::{
    rgm::{
        RgmFile, RgmSection, RgmSectionHeader, export_rgm_metadata_json,
        script_program::{CallKind, Object, Value, decode_native_hook},
    },
    soup_def::{SoupDef, parse_soup_def},
};

fn definition() -> SoupDef {
    parse_soup_def("[functions]\nfunction Empty parms 0\ntask Pair parms 2").unwrap()
}

fn map(offset: i32, records: Option<Vec<u8>>) -> RgmFile {
    let mut header = vec![0; 8 + 165];
    header[..4].copy_from_slice(&1u32.to_le_bytes());
    header[12..17].copy_from_slice(b"ACTOR");
    header[8 + 0x5d..8 + 0x61].copy_from_slice(&offset.to_le_bytes());
    let section = |name, size| RgmSectionHeader {
        name,
        data_length: size,
        record_count: None,
    };
    let mut sections = vec![RgmSection::Rahd(
        section(*b"RAHD", header.len() as u32),
        header,
    )];
    if let Some(records) = records {
        sections.push(RgmSection::Rahk(
            section(*b"RAHK", records.len() as u32),
            records,
        ));
    }
    RgmFile { sections }
}

#[test]
fn zero_offset_is_absent_and_three_byte_final_record_is_a_real_hook() {
    assert_eq!(
        decode_native_hook(&map(0, None), 0, &definition()).unwrap(),
        None
    );
    let data = map(4, Some(vec![0, 0, 0, 0, 1, 0, 0]));
    let call = decode_native_hook(&data, 0, &definition())
        .unwrap()
        .unwrap();
    assert_eq!(call.function, "Empty");
    assert_eq!(call.function_id, 1);
    assert_eq!(call.kind, CallKind::Function);
    assert_eq!(call.target, Object::Me);
    assert!(call.arguments.is_empty());
    let metadata = export_rgm_metadata_json(&data, Some(&definition()));
    assert_eq!(metadata["actors"][0]["native_hook"]["function"], "Empty");
    assert!(metadata["actors"][0].get("rahk_hooks").is_none());
    let metadata = export_rgm_metadata_json(&data, None);
    assert_eq!(metadata["actors"][0]["native_hook_offset"], 4);
}

#[test]
fn hook_arguments_use_parameter_semantics_without_task_dispatch_or_adjacent_records() {
    let data = map(
        4,
        Some(vec![
            0, 0, 0, 0, 2, 0, 2, 7, 249, 255, 255, 255, 22, 3, 0, 255, 255, 1, 0, 0,
        ]),
    );
    let call = decode_native_hook(&data, 0, &definition())
        .unwrap()
        .unwrap();
    assert_eq!(call.function, "Pair");
    assert_eq!(call.kind, CallKind::Function);
    assert_eq!(
        call.arguments,
        [Value::Integer { value: -7 }, Value::Integer { value: 3 }]
    );
}

#[test]
fn invalid_hook_records_are_contextual_errors_instead_of_missing_hooks() {
    for data in [
        map(-1, Some(vec![0; 4])),
        map(4, None),
        map(4, Some(vec![0; 6])),
        map(4, Some(vec![0, 0, 0, 0, 9, 0, 0])),
        map(4, Some(vec![0, 0, 0, 0, 1, 0, 1, 7, 0, 0, 0, 0])),
        map(4, Some(vec![0, 0, 0, 0, 2, 0, 2, 7, 0, 0, 0, 0])),
        map(4, Some(vec![0, 0, 0, 0, 2, 0, 9])),
        map(
            4,
            Some(vec![0, 0, 0, 0, 2, 0, 2, 255, 0, 0, 0, 0, 7, 0, 0, 0, 0]),
        ),
    ] {
        let error = decode_native_hook(&data, 0, &definition()).unwrap_err();
        assert!(error.contains("actor header 0 native hook"), "{error}");
        let metadata = export_rgm_metadata_json(&data, Some(&definition()));
        assert_eq!(metadata["actors"][0]["native_hook_error"], error);
    }
    assert!(decode_native_hook(&map(0, None), usize::MAX, &definition()).is_err());
}

#[test]
fn installed_maps_preserve_every_native_hook_and_parameter() {
    let Some(root) = std::env::var_os("REDGUARD_DIR") else {
        eprintln!("SKIP installed native hooks: REDGUARD_DIR absent");
        return;
    };
    let root = std::path::PathBuf::from(root);
    let definition = rgpre::import::soup_def::load_soup_def(&root).unwrap();
    let mut counts = std::collections::BTreeMap::new();
    let mut maps = 0;
    let mut absent = 0;
    let mut parameters = 0;
    for entry in std::fs::read_dir(root.join("maps")).unwrap() {
        let path = entry.unwrap().path();
        if !path
            .extension()
            .is_some_and(|value| value.eq_ignore_ascii_case("rgm"))
        {
            continue;
        }
        maps += 1;
        let map = rgpre::import::rgm::parse_rgm_file(&std::fs::read(&path).unwrap()).unwrap();
        let metadata = export_rgm_metadata_json(&map, Some(&definition));
        for (index, actor) in metadata["actors"].as_array().unwrap().iter().enumerate() {
            let hook = decode_native_hook(&map, index, &definition)
                .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            if let Some(hook) = hook {
                assert_eq!(hook.target, Object::Me);
                assert_eq!(hook.kind, CallKind::Function);
                assert_eq!(actor["native_hook"], serde_json::to_value(&hook).unwrap());
                assert!(
                    hook.arguments
                        .iter()
                        .all(|argument| matches!(argument, Value::Integer { .. }))
                );
                parameters += hook.arguments.len();
                *counts.entry(hook.function).or_insert(0) += 1;
            } else {
                absent += 1;
                assert!(actor.get("native_hook_error").is_none());
            }
        }
    }
    assert_eq!(maps, 27);
    assert_eq!(absent, 1542);
    assert_eq!(parameters, 82);
    assert_eq!(
        counts,
        std::collections::BTreeMap::from([
            ("PlayerMain".to_owned(), 54),
            ("cameraController".to_owned(), 27),
            ("Wander".to_owned(), 40),
            ("AtNodeMarker".to_owned(), 1),
        ])
    );
}
