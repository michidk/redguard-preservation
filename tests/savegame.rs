use rgpre::import::savegame::{
    SaveSection, encode_savegame_file, load_save_slot, parse_savegame_file,
};
use std::path::PathBuf;

fn chunk(bytes: &mut Vec<u8>, name: &[u8; 4], data: &[u8]) {
    bytes.extend_from_slice(name);
    bytes.extend_from_slice(&u32::try_from(data.len()).unwrap().to_be_bytes());
    bytes.extend_from_slice(data);
}

fn save_bytes() -> Vec<u8> {
    let mut metadata = [0u8; 272];
    metadata[..4].copy_from_slice(b"1.31");
    metadata[4..8].copy_from_slice(&1u32.to_le_bytes());
    metadata[8..12].copy_from_slice(&0x1020_3040u32.to_le_bytes());
    metadata[12..23].copy_from_slice(b"TEST ISLAND");
    metadata[140..144].copy_from_slice(&0x0102_0304u32.to_le_bytes());
    metadata[144..162].copy_from_slice(b"system\\powerup.gxa");
    let mut bytes = Vec::new();
    chunk(&mut bytes, b"SVGM", &metadata);
    chunk(&mut bytes, b"THMB", &[]);
    chunk(&mut bytes, b"NEW!", &[0xff, 1, 2]);
    chunk(&mut bytes, b"NEW!", &[3]);
    bytes.extend_from_slice(b"END ");
    bytes
}

#[test]
fn decodes_metadata_and_retains_empty_unknown_and_duplicate_chunks() {
    let bytes = save_bytes();
    let game = parse_savegame_file(&bytes).unwrap();
    assert_eq!(game.metadata.version, *b"1.31");
    assert_eq!(game.metadata.unknown_04, 1);
    assert_eq!(game.metadata.unknown_08, 0x1020_3040);
    assert_eq!(game.metadata.description, "TEST ISLAND");
    assert_eq!(game.metadata.world_id, 0x0102_0304);
    assert_eq!(game.metadata.bitmap_filename, "system\\powerup.gxa");
    assert_eq!(game.sections.len(), 4);
    assert_eq!(game.sections[0].data, bytes[8..280]);
    assert!(game.sections[1].data.is_empty());
    assert_eq!(game.sections[2].name, *b"NEW!");
    assert_eq!(game.sections[2].data, [0xff, 1, 2]);
    assert_eq!(game.sections[3].data, [3]);
}

#[test]
fn rejects_truncation_overflow_bad_metadata_and_trailing_bytes() {
    let bytes = save_bytes();
    for length in 0..bytes.len() {
        assert!(
            parse_savegame_file(&bytes[..length]).is_err(),
            "length {length}"
        );
    }
    let mut huge = bytes.clone();
    huge[4..8].copy_from_slice(&u32::MAX.to_be_bytes());
    assert!(parse_savegame_file(&huge).is_err());
    let mut wrong_version = bytes.clone();
    wrong_version[8..12].copy_from_slice(b"9.99");
    assert!(parse_savegame_file(&wrong_version).is_err());
    let mut duplicate = bytes[..280].to_vec();
    duplicate.extend_from_slice(&bytes);
    assert!(parse_savegame_file(&duplicate).is_err());
    for length in [0, 271, 273] {
        let mut wrong_size = Vec::new();
        chunk(&mut wrong_size, b"SVGM", &vec![0; length]);
        wrong_size.extend_from_slice(b"END ");
        assert!(parse_savegame_file(&wrong_size).is_err());
    }
    assert!(parse_savegame_file(b"END ").is_err());
    assert!(parse_savegame_file(&bytes[280..]).is_err());
    let mut trailing = bytes;
    trailing.push(0);
    assert!(parse_savegame_file(&trailing).is_err());
}

#[test]
fn display_decoding_keeps_original_non_utf8_bytes_in_svgm() {
    let mut bytes = save_bytes();
    bytes[20] = 0x82;
    let game = parse_savegame_file(&bytes).unwrap();
    assert_eq!(game.metadata.description, "\u{fffd}EST ISLAND");
    assert_eq!(game.sections[0].data[12], 0x82);
}

struct SlotDir(PathBuf);

impl SlotDir {
    fn new() -> Self {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("rgpre-save-{}-{unique}", std::process::id()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn write(&self, name: &str, bytes: &[u8]) {
        std::fs::write(self.0.join(name), bytes).unwrap();
    }
}

impl Drop for SlotDir {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn loads_case_insensitive_resources_and_preserves_logbook_without_writing() {
    let dir = SlotDir::new();
    let bytes = save_bytes();
    dir.write("savegame.sav", &bytes);
    dir.write("IsLaNd.TsG", &bytes);
    dir.write("logbook.txt", b"Entry \x82\r\n\r\n");
    dir.write("fxart.txt", b"ignored report");
    let slot = load_save_slot(&dir.0).unwrap();
    assert_eq!(slot.game.metadata.description, "TEST ISLAND");
    assert_eq!(slot.maps.len(), 1);
    assert_eq!(slot.maps["ISLAND.TSG"].metadata.world_id, 0x0102_0304);
    assert_eq!(
        slot.logbook.as_deref(),
        Some(b"Entry \x82\r\n\r\n".as_slice())
    );
    assert_eq!(std::fs::read(dir.0.join("savegame.sav")).unwrap(), bytes);
    assert_eq!(std::fs::read(dir.0.join("IsLaNd.TsG")).unwrap(), bytes);
    assert_eq!(
        std::fs::read(dir.0.join("logbook.txt")).unwrap(),
        b"Entry \x82\r\n\r\n"
    );
    assert_eq!(std::fs::read_dir(&dir.0).unwrap().count(), 4);
}

#[test]
fn optional_resources_can_be_absent_and_errors_identify_the_resource() {
    let dir = SlotDir::new();
    assert!(
        load_save_slot(&dir.0)
            .unwrap_err()
            .to_string()
            .contains("missing SAVEGAME.SAV")
    );
    dir.write("SAVEGAME.SAV", &save_bytes());
    let slot = load_save_slot(&dir.0).unwrap();
    assert!(slot.maps.is_empty());
    assert!(slot.logbook.is_none());
    dir.write("BAD.TSG", b"bad");
    assert!(
        load_save_slot(&dir.0)
            .unwrap_err()
            .to_string()
            .contains("BAD.TSG")
    );
}

#[test]
fn rejects_case_collisions_when_filesystem_preserves_distinct_names() {
    let dir = SlotDir::new();
    dir.write("SAVEGAME.SAV", &save_bytes());
    dir.write("savegame.sav", &save_bytes());
    if std::fs::read_dir(&dir.0).unwrap().count() == 2 {
        assert!(
            load_save_slot(&dir.0)
                .unwrap_err()
                .to_string()
                .contains("duplicate")
        );
    } else {
        assert!(load_save_slot(&dir.0).is_ok());
    }
}

#[cfg(unix)]
#[test]
fn rejects_directories_and_symlink_resources() {
    let dir = SlotDir::new();
    dir.write("SAVEGAME.SAV", &save_bytes());
    std::fs::create_dir(dir.0.join("BAD.TSG")).unwrap();
    assert!(
        load_save_slot(&dir.0)
            .unwrap_err()
            .to_string()
            .contains("regular file")
    );
    std::fs::remove_dir(dir.0.join("BAD.TSG")).unwrap();
    std::os::unix::fs::symlink("SAVEGAME.SAV", dir.0.join("BAD.TSG")).unwrap();
    assert!(
        load_save_slot(&dir.0)
            .unwrap_err()
            .to_string()
            .contains("regular file")
    );
}

#[test]
fn installed_save_slot() {
    let Some(path) = std::env::var_os("REDGUARD_SAVE_SLOT") else {
        eprintln!("SKIP installed save slot: REDGUARD_SAVE_SLOT is absent");
        return;
    };
    let slot = load_save_slot(&path).unwrap();
    for entry in std::fs::read_dir(&path).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().to_ascii_uppercase();
        if name == "SAVEGAME.SAV" || name.ends_with(".TSG") {
            let bytes = std::fs::read(entry.path()).unwrap();
            let parsed = parse_savegame_file(&bytes).unwrap();
            assert_eq!(
                encode_savegame_file(&parsed.sections).unwrap(),
                bytes,
                "{name}"
            );
        }
    }
    assert_eq!(slot.game.metadata.version, *b"1.31");
    if let Some(count) = std::env::var_os("REDGUARD_SAVED_ITEM_COUNT") {
        use rgpre::import::savegame::{encode_save_inventory, parse_save_inventory};
        let count = count.to_str().unwrap().parse::<usize>().unwrap();
        let section = slot
            .game
            .sections
            .iter()
            .find(|s| s.name == *b"SVIT")
            .unwrap();
        let inventory = parse_save_inventory(&section.data, count).unwrap();
        assert_eq!(encode_save_inventory(&inventory).unwrap(), section.data);
    }
    assert!(
        slot.game
            .sections
            .iter()
            .any(|s| s.name == *b"SMOB" && !s.data.is_empty())
    );
    for map in slot.maps.values() {
        assert_eq!(map.metadata.version, *b"1.31");
        assert!(
            map.sections
                .iter()
                .any(|s| s.name == *b"SMOB" && !s.data.is_empty())
        );
    }
}

#[test]
fn inventory_decodes_literal_fields_and_encodes_the_original_bytes() {
    use rgpre::import::savegame::{
        SaveInventory, SaveItem, encode_save_inventory, parse_save_inventory,
    };
    let bytes = [
        1, 0xa5, 4, 3, 2, 1, 9, 0, 0, 0, 0xfe, 0xff, 0xff, 0xff, 0x44, 0x33, 0x22, 0x11, 0x88,
        0x77, 0x66, 0x55, 0x78, 0x56, 0x34, 0x12,
    ];
    let expected = SaveInventory {
        items: vec![SaveItem {
            hide: 1,
            unknown_01: 0xa5,
            total: 0x01020304,
            player_max: 9,
            player_total: -2,
            unknown_0e: 0x11223344,
            unknown_12: 0x55667788,
        }],
        selected_item: 0x12345678,
    };
    assert_eq!(parse_save_inventory(&bytes, 1).unwrap(), expected);
    assert_eq!(encode_save_inventory(&expected).unwrap(), bytes);
    assert_eq!(
        parse_save_inventory(&[255; 4], 0).unwrap(),
        SaveInventory {
            items: vec![],
            selected_item: -1,
        }
    );
    for count in [0, 2, usize::MAX] {
        assert!(
            parse_save_inventory(&bytes, count)
                .unwrap_err()
                .to_string()
                .contains("SVIT")
        );
    }
    for size in 0..bytes.len() {
        assert!(parse_save_inventory(&bytes[..size], 1).is_err());
    }
    let mut extra = bytes.to_vec();
    extra.push(0);
    assert!(parse_save_inventory(&extra, 1).is_err());
}

#[test]
fn lossless_encoding_preserves_metadata_padding_and_non_utf8_text() {
    let mut bytes = save_bytes();
    bytes[20] = 0x82;
    bytes[52] = 0xa5;
    let game = parse_savegame_file(&bytes).unwrap();
    assert_eq!(encode_savegame_file(&game.sections).unwrap(), bytes);
    assert_eq!(
        parse_savegame_file(&encode_savegame_file(&game.sections).unwrap()).unwrap(),
        game
    );
}

#[test]
fn encoding_rejects_invalid_structure_and_metadata() {
    let game = parse_savegame_file(&save_bytes()).unwrap();
    assert!(encode_savegame_file(&[]).is_err());
    assert!(encode_savegame_file(&game.sections[1..]).is_err());
    for extra in [
        game.sections[0].clone(),
        SaveSection {
            name: *b"END ",
            data: vec![],
        },
    ] {
        let mut sections = game.sections.clone();
        sections.push(extra);
        assert!(encode_savegame_file(&sections).is_err());
    }
    let mut sections = game.sections.clone();
    sections[0].data[0] = b'9';
    assert!(encode_savegame_file(&sections).is_err());
    sections[0].data.pop();
    assert!(encode_savegame_file(&sections).is_err());
}

#[test]
fn encoding_keeps_edited_payloads_and_recomputes_chunk_lengths() {
    let mut game = parse_savegame_file(&save_bytes()).unwrap();
    game.sections[2].data = vec![7, 8, 9, 10];
    let encoded = encode_savegame_file(&game.sections).unwrap();
    let original = save_bytes();
    let mut expected = original[..288].to_vec();
    expected.extend_from_slice(b"NEW!\0\0\0\x04\x07\x08\x09\x0a");
    expected.extend_from_slice(&original[299..]);
    assert_eq!(encoded, expected);
}

#[test]
fn flag_payload_has_signed_values_then_states_without_interleaving() {
    use rgpre::import::savegame::{SaveFlags, encode_save_flags, parse_save_flags};
    let bytes = [
        2, 0, 0, 0, 0xff, 0xff, 0xff, 0xff, 0x78, 0x56, 0x34, 0x12, 1, 0x80,
    ];
    let flags = parse_save_flags(&bytes).unwrap();
    assert_eq!(
        flags,
        SaveFlags {
            values: vec![-1, 0x1234_5678],
            states: vec![1, 0x80]
        }
    );
    assert_eq!(encode_save_flags(&flags).unwrap(), bytes);
    assert_eq!(
        parse_save_flags(&[0; 4]).unwrap(),
        SaveFlags {
            values: vec![],
            states: vec![]
        }
    );
    for end in 0..bytes.len() {
        assert!(parse_save_flags(&bytes[..end]).is_err());
    }
    let mut trailing = bytes.to_vec();
    trailing.push(0);
    assert!(parse_save_flags(&trailing).is_err());
    assert!(parse_save_flags(&[0xff; 4]).is_err());
    assert!(
        encode_save_flags(&SaveFlags {
            values: vec![1],
            states: vec![]
        })
        .is_err()
    );
}
