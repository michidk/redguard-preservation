use rgpre::import::{
    FileType,
    miles_driver::{DriverKind, IoParameters, parse_miles_driver},
};

fn image(signature: &[u8; 8], version: u32) -> Vec<u8> {
    let mut bytes = vec![0; 0x200];
    bytes[..8].copy_from_slice(signature);
    bytes[8..12].copy_from_slice(&version.to_le_bytes());
    bytes[0x34..0x36].copy_from_slice(&0x180_u16.to_le_bytes());
    bytes[0xba..0xc6].copy_from_slice(b"Test device\0");
    bytes[0x180] = 0xcf;
    bytes
}

#[test]
fn decodes_stored_fields_without_dereferencing_runtime_pointers() {
    let mut bytes = image(b"AIL3DIG\x1a", 0x114);
    bytes[0x0c..0x10].copy_from_slice(&0xabcd_1234_u32.to_le_bytes());
    bytes[0x10..0x12].copy_from_slice(&2_u16.to_le_bytes());
    bytes[0x12..0x16].copy_from_slice(&0x5678_9abc_u32.to_le_bytes());
    bytes[0x16..0x18].copy_from_slice(&0x220_i16.to_le_bytes());
    bytes[0x18..0x1a].copy_from_slice(&7_i16.to_le_bytes());
    bytes[0x1a..0x1c].copy_from_slice(&1_i16.to_le_bytes());
    bytes[0x1c..0x1e].copy_from_slice(&(-1_i16).to_le_bytes());
    bytes[0x1e..0x22].copy_from_slice(&(-123_i32).to_le_bytes());
    bytes[0x2e..0x30].copy_from_slice(&(-1_i16).to_le_bytes());
    bytes[0x30..0x32].copy_from_slice(&1_u16.to_le_bytes());
    bytes[0x32..0x34].copy_from_slice(&0xffff_u16.to_le_bytes());
    bytes[0x36..0x3a].copy_from_slice(&0x8765_4321_u32.to_le_bytes());
    let driver = parse_miles_driver(&bytes).unwrap();
    assert_eq!(driver.kind, DriverKind::Digital);
    assert_eq!(driver.driver_version, 0x114);
    assert_eq!(driver.common_io_configurations, 0xabcd_1234);
    assert_eq!(driver.num_io_configurations, 2);
    assert_eq!(driver.environment_string, 0x5678_9abc);
    assert_eq!(
        driver.io,
        IoParameters {
            io_address: 0x220,
            irq: 7,
            dma_8_bit: 1,
            dma_16_bit: -1,
            reserved: [-123, 0, 0, 0],
        }
    );
    assert_eq!(driver.service_rate, -1);
    assert_eq!(driver.busy, 1);
    assert_eq!(driver.driver_number, 0xffff);
    assert_eq!(driver.dispatcher_offset, 0x180);
    assert_eq!(driver.previous_isr, 0x8765_4321);
    assert_eq!(driver.device_name, Some("Test device"));
    assert_eq!(driver.image, bytes);
    let json = serde_json::to_value(&driver).unwrap();
    assert_eq!(json["kind"], "Digital");
    assert!(json.get("image").is_none());
}

#[test]
fn supports_older_headers_and_the_zero_revision_ess_variant() {
    for version in [0x100, 0x111] {
        let mut bytes = image(b"AIL3MDI\x1a", version);
        bytes.truncate(0xe3);
        bytes[0x34..0x36].copy_from_slice(&0xe2_u16.to_le_bytes());
        bytes[0xba..].fill(0xff);
        let driver = parse_miles_driver(&bytes).unwrap();
        assert_eq!(driver.kind, DriverKind::Midi);
        assert_eq!(driver.device_name, None);
    }
    let mut bytes = image(b"ail3dig\x1a", 0);
    assert_eq!(
        parse_miles_driver(&bytes).unwrap().device_name,
        Some("Test device")
    );
    bytes[0xba] = 0;
    assert_eq!(parse_miles_driver(&bytes).unwrap().device_name, Some(""));
}

#[test]
fn rejects_truncation_invalid_names_signatures_and_dispatchers() {
    let bytes = image(b"AIL3DIG\x1a", 0x112);
    for length in 0..=0x180 {
        assert!(
            parse_miles_driver(&bytes[..length]).is_err(),
            "length {length:#x}"
        );
    }
    for offset in [0, 0xba, 0x109, 0x200, 0xffff] {
        let mut bad = bytes.clone();
        bad[0x34..0x36].copy_from_slice(&u16::try_from(offset).unwrap().to_le_bytes());
        assert!(parse_miles_driver(&bad).is_err(), "offset {offset:#x}");
    }
    for (offset, value) in [(0, b'X'), (7, 0), (0xba, 0xff)] {
        let mut bad = bytes.clone();
        bad[offset] = value;
        assert!(parse_miles_driver(&bad).is_err());
    }
    let mut bad = bytes;
    bad[0xba..0x10a].fill(b'A');
    assert!(parse_miles_driver(&bad).is_err());
}

#[test]
fn cli_and_registry_keep_digital_and_midi_images_with_the_same_stem() {
    let root = std::env::temp_dir().join(format!(
        "rgpre-miles-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    for (name, signature, kind) in [
        ("PAIR.DIG", b"AIL3DIG\x1a", "Digital"),
        ("PAIR.MDI", b"AIL3MDI\x1a", "Midi"),
    ] {
        let file = root.join(name);
        std::fs::write(&file, image(signature, 0x112)).unwrap();
        let read = std::process::Command::new(env!("CARGO_BIN_EXE_rgpre"))
            .arg("read")
            .arg(&file)
            .output()
            .unwrap();
        assert!(
            read.status.success(),
            "{}",
            String::from_utf8_lossy(&read.stderr)
        );
        assert!(String::from_utf8_lossy(&read.stderr).contains("Test device"));
        let mut convert = std::process::Command::new(env!("CARGO_BIN_EXE_rgpre"));
        convert.arg("convert");
        if kind == "Digital" {
            convert.arg("driver");
        }
        let result = convert.arg(&file).output().unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let metadata: serde_json::Value =
            serde_json::from_slice(&std::fs::read(root.join(format!("{name}.json"))).unwrap())
                .unwrap();
        assert_eq!(metadata["kind"], kind);
        assert_eq!(metadata["device_name"], "Test device");
        assert!(metadata.get("image").is_none());
        assert_eq!(std::fs::read(&file).unwrap(), image(signature, 0x112));
    }
    let registry = rgpre::import::registry::scan_dir(&root).unwrap();
    assert_eq!(registry.files.len(), 2);
    let scan = std::process::Command::new(env!("CARGO_BIN_EXE_rgpre"))
        .arg("scan")
        .arg(&root)
        .output()
        .unwrap();
    assert!(scan.status.success());
    assert!(String::from_utf8_lossy(&scan.stdout).contains("2 files"));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn installed_drivers_include_all_pictured_variants() {
    let Some(root) = std::env::var_os("REDGUARD_DIR") else {
        eprintln!("SKIP installed Miles drivers: REDGUARD_DIR absent");
        return;
    };
    let sound = std::path::PathBuf::from(root).join("sound");
    let mut parsed = std::collections::BTreeMap::new();
    for entry in std::fs::read_dir(sound).unwrap() {
        let path = entry.unwrap().path();
        if !matches!(
            FileType::from_path(&path),
            Some(FileType::Dig | FileType::Mdi)
        ) {
            continue;
        }
        let bytes = std::fs::read(&path).unwrap();
        let driver =
            parse_miles_driver(&bytes).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let name = path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_ascii_lowercase();
        assert_eq!(
            driver.kind,
            if name.ends_with(".dig") {
                DriverKind::Digital
            } else {
                DriverKind::Midi
            }
        );
        parsed.insert(
            name,
            (driver.driver_version, driver.device_name.map(str::to_owned)),
        );
    }
    assert_eq!(parsed.len(), 33);
    for (name, version, description) in [
        (
            "esfm.mdi",
            0x112,
            Some("ESFM Driver (C) 1995 ESS Technology Inc."),
        ),
        ("iwav.dig", 0x112, Some("InterWave(TM) Audio Products")),
        ("iwav.mdi", 0x112, Some("InterWave(TM) General MIDI")),
        ("jammer.dig", 0x111, None),
        (
            "mdimaxi.mdi",
            0x112,
            Some("Maxi Sound 64 (or 100% compatible)"),
        ),
        (
            "audiodrv.dig",
            0,
            Some("ESS AudioDrive (C) 1994,1996 ESS Technology Inc."),
        ),
    ] {
        assert_eq!(
            parsed[name],
            (version, description.map(str::to_owned)),
            "{name}"
        );
    }
}
