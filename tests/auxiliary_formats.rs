use rgpre::import::{opl::parse_opl_file, pif::parse_pif_file, rst::parse_rst_file};
use std::{path::PathBuf, process::Command};

fn pif_bytes() -> Vec<u8> {
    let mut bytes = vec![0; 517];
    bytes[2..32].fill(b' ');
    bytes[2..6].copy_from_slice(b"Test");
    bytes[32..34].copy_from_slice(&640u16.to_le_bytes());
    bytes[36..44].copy_from_slice(b"TEST.EXE");
    bytes[165..170].copy_from_slice(b"/base");
    bytes[369..385].copy_from_slice(b"MICROSOFT PIFEX\0");
    bytes[385..387].copy_from_slice(&391u16.to_le_bytes());
    bytes[389..391].copy_from_slice(&369u16.to_le_bytes());
    bytes[391..407].copy_from_slice(b"WINDOWS 386 3.0\0");
    bytes[407..409].copy_from_slice(&0xffffu16.to_le_bytes());
    bytes[409..411].copy_from_slice(&413u16.to_le_bytes());
    bytes[411..413].copy_from_slice(&104u16.to_le_bytes());
    bytes[413..415].copy_from_slice(&512u16.to_le_bytes());
    bytes[453..457].copy_from_slice(b"/386");
    bytes
}

fn opl_bytes() -> Vec<u8> {
    vec![
        7, 0, 14, 0, 0, 0, 42, 127, 14, 0, 0, 0, 255, 255, 14, 0, 254, 1, 2, 3, 4, 5, 6, 7, 8, 9,
        10, 11,
    ]
}

#[test]
fn pif_reads_base_and_386_settings() {
    let bytes = pif_bytes();
    let pif = parse_pif_file(&bytes).unwrap();
    assert_eq!(pif.title, "Test");
    assert_eq!(pif.program, "TEST.EXE");
    assert_eq!(pif.memory_max_kb, 640);
    assert_eq!(pif.parameters, "/base");
    assert_eq!(pif.extensions.len(), 2);
    let settings = pif.extensions[1].settings_386.as_ref().unwrap();
    assert_eq!(settings.memory_max_kb, 512);
    assert_eq!(settings.parameters, "/386");
    assert_eq!(pif.extensions[0].data, &bytes[..369]);
    assert_eq!(pif.image, bytes);
    assert!(parse_pif_file(&bytes[..369]).unwrap().extensions.is_empty());
    for len in 0..bytes.len() {
        if len != 369 {
            assert!(parse_pif_file(&bytes[..len]).is_err(), "length {len}");
        }
    }
}

#[test]
fn pif_rejects_invalid_links_and_retains_unknown_extensions() {
    let original = pif_bytes();
    for (offset, replacement) in [
        (385, 0u16),
        (407, 369),
        (407, 391),
        (407, 0xfffe),
        (409, 0xffff),
        (411, 103),
        (389, 368),
    ] {
        let mut bytes = original.clone();
        bytes[offset..offset + 2].copy_from_slice(&replacement.to_le_bytes());
        assert!(
            parse_pif_file(&bytes).is_err(),
            "offset {offset}, value {replacement}"
        );
    }
    let mut bytes = original.clone();
    bytes[369] = b'X';
    assert!(parse_pif_file(&bytes).is_err());
    bytes = original;
    bytes[391..407].copy_from_slice(b"UNKNOWN BLOCK\0\0\0");
    let pif = parse_pif_file(&bytes).unwrap();
    assert_eq!(pif.extensions[1].signature, "UNKNOWN BLOCK");
    assert!(pif.extensions[1].settings_386.is_none());
    assert_eq!(pif.extensions[1].data, &bytes[413..]);
    bytes[36] = 0x82;
    assert_eq!(parse_pif_file(&bytes).unwrap().program, "\u{82}EST.EXE");
}

#[test]
fn opl_reads_shared_timbres_and_signed_pitch() {
    let bytes = opl_bytes();
    let opl = parse_opl_file(&bytes).unwrap();
    assert_eq!(opl.instruments.len(), 2);
    let instrument = &opl.instruments[1];
    assert_eq!(
        (instrument.patch, instrument.bank, instrument.offset),
        (42, 127, 14)
    );
    assert_eq!(instrument.transposition, -2);
    assert_eq!(instrument.modulator.reg_20, 1);
    assert_eq!(instrument.modulator.reg_e0, 5);
    assert_eq!(instrument.reg_c0, 6);
    assert_eq!(instrument.carrier.reg_40, 8);
    assert_eq!(instrument.carrier.reg_e0, 11);
    assert_eq!(opl.image, bytes);
    assert!(parse_opl_file(&[255, 255]).unwrap().instruments.is_empty());
    for len in 0..bytes.len() {
        assert!(parse_opl_file(&bytes[..len]).is_err(), "length {len}");
    }
}

#[test]
fn opl_rejects_invalid_offsets_and_record_sizes() {
    for offset in [0u32, 13, 15, u32::MAX] {
        let mut bytes = opl_bytes();
        bytes[2..6].copy_from_slice(&offset.to_le_bytes());
        assert!(parse_opl_file(&bytes).is_err());
    }
    let mut bytes = opl_bytes();
    bytes[14] = 13;
    assert!(parse_opl_file(&bytes).is_err());
    bytes = opl_bytes();
    bytes[12..14].fill(0);
    assert!(parse_opl_file(&bytes).is_err());
}

#[test]
fn rst_preserves_sections_duplicates_and_text_values() {
    let bytes = b"; state\r\n[edit-]\r\nscreen=80 50\r\nsrch=\r\nfile=d:\\sound\\dig.ini 1 7\r\nfile=x=y;z \r\n[brief]\r\nfile=\x82\r\n";
    let rst = parse_rst_file(bytes).unwrap();
    assert_eq!(rst.sections.len(), 2);
    assert_eq!(rst.sections[0].name, "edit-");
    let entries = &rst.sections[0].entries;
    assert_eq!(entries.len(), 4);
    assert_eq!((&*entries[0].key, &*entries[0].value), ("screen", "80 50"));
    assert_eq!(entries[1].value, "");
    assert_eq!(entries[2].value, "d:\\sound\\dig.ini 1 7");
    assert_eq!(entries[3].value, "x=y;z ");
    assert_eq!(rst.sections[1].entries[0].value, "\u{82}");
    assert_eq!(rst.image, bytes);
    for invalid in [
        "",
        "x=1",
        "[]",
        "[broken",
        "[ok]\nx",
        "[ok]\n=1",
        "[ok]\nx=\0",
    ] {
        assert!(parse_rst_file(invalid.as_bytes()).is_err(), "{invalid:?}");
    }
}

#[test]
fn cli_reads_and_exports_auxiliary_formats() {
    let root = std::env::temp_dir().join(format!("rgpre-auxiliary-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    for (extension, bytes, key, pointer, expected) in [
        (
            "pif",
            pif_bytes(),
            "title",
            "/title",
            serde_json::json!("Test"),
        ),
        (
            "opl",
            opl_bytes(),
            "instruments",
            "/instruments/1/transposition",
            serde_json::json!(-2),
        ),
        (
            "rst",
            b"[edit-]\nscreen=80 50\n".to_vec(),
            "sections",
            "/sections/0/entries/0/value",
            serde_json::json!("80 50"),
        ),
    ] {
        let input = root.join(format!("TEST.{extension}"));
        std::fs::write(&input, &bytes).unwrap();
        let read = Command::new(env!("CARGO_BIN_EXE_rgpre"))
            .arg("read")
            .arg(&input)
            .output()
            .unwrap();
        assert!(
            read.status.success(),
            "{}",
            String::from_utf8_lossy(&read.stderr)
        );
        assert!(String::from_utf8_lossy(&read.stderr).contains(key));
        for explicit in [false, true] {
            let mut command = Command::new(env!("CARGO_BIN_EXE_rgpre"));
            command.arg("convert");
            if explicit {
                command.arg(extension);
            }
            let result = command.arg(&input).output().unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
            let output = input.with_extension(format!("{extension}.json"));
            let json: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&output).unwrap()).unwrap();
            assert_eq!(json.pointer(pointer), Some(&expected));
            std::fs::remove_file(output).unwrap();
        }
        assert_eq!(std::fs::read(&input).unwrap(), bytes);
        std::fs::write(&input, []).unwrap();
        let output = root.join(format!("invalid-{extension}.json"));
        let result = Command::new(env!("CARGO_BIN_EXE_rgpre"))
            .arg("convert")
            .arg(&input)
            .arg("-o")
            .arg(&output)
            .output()
            .unwrap();
        assert!(!result.status.success());
        assert!(!output.exists());
    }
    let scan = Command::new(env!("CARGO_BIN_EXE_rgpre"))
        .arg("scan")
        .arg(&root)
        .output()
        .unwrap();
    assert!(scan.status.success());
    let output = String::from_utf8_lossy(&scan.stdout);
    for name in [
        "Windows Program Settings",
        "FM Instrument Bank",
        "Editor State",
    ] {
        assert!(output.contains(name), "{output}");
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn installed_auxiliary_files() {
    let Some(root) = std::env::var_os("REDGUARD_DIR") else {
        eprintln!("SKIP installed auxiliary files: REDGUARD_DIR absent");
        return;
    };
    let sound = PathBuf::from(root).join("sound");
    for (name, program, parameters) in [
        ("DIGPLAY.PIF", "DigPlay.EXE", "R212.WAV"),
        ("SETSOUND.PIF", "SETSOUND.EXE", ""),
    ] {
        let bytes = std::fs::read(sound.join(name)).unwrap();
        let pif = parse_pif_file(&bytes).unwrap();
        assert_eq!(pif.program, program);
        assert_eq!(pif.parameters, parameters);
        assert_eq!(pif.extensions.len(), 3);
        assert_eq!(pif.extensions[2].signature, "WINDOWS VMM 4.0");
        assert_eq!(pif.extensions[2].data.len(), 428);
        assert_eq!(
            pif.extensions[1].settings_386.as_ref().unwrap().parameters,
            parameters
        );
    }
    for (name, count) in [("sample.opl", 168), ("Fatman.opl", 181)] {
        let bytes = std::fs::read(sound.join(name)).unwrap();
        let opl = parse_opl_file(&bytes).unwrap();
        assert_eq!(opl.instruments.len(), count);
        assert_eq!((opl.instruments[0].patch, opl.instruments[0].bank), (0, 0));
        assert_eq!(opl.instruments.last().unwrap().bank, 127);
    }
    let bytes = std::fs::read(sound.join("STATE.RST")).unwrap();
    let rst = parse_rst_file(&bytes).unwrap();
    assert_eq!(
        rst.sections
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>(),
        ["edit-", "brief", "shared-"]
    );
    assert_eq!(rst.sections[0].entries[0].value, "80 50");
    assert_eq!(
        rst.sections[2].entries[0].value,
        "d:\\redguard\\sound\\dig.ini 1 7"
    );
}
