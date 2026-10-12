use rgpre::import::savegame::{load_save_slot, parse_savegame_file};
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

#[cfg(unix)]
#[test]
fn rejects_case_collisions_directories_and_symlink_resources() {
    let dir = SlotDir::new();
    dir.write("SAVEGAME.SAV", &save_bytes());
    dir.write("savegame.sav", &save_bytes());
    assert!(
        load_save_slot(&dir.0)
            .unwrap_err()
            .to_string()
            .contains("duplicate")
    );
    std::fs::remove_file(dir.0.join("savegame.sav")).unwrap();
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
    let slot = load_save_slot(path).unwrap();
    assert_eq!(slot.game.metadata.version, *b"1.31");
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
