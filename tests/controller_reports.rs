use rgpre::import::{FileType, more::parse_more_file, sidewinder::parse_sidewinder_file};
use std::{path::PathBuf, process::Command};

fn profile_bytes() -> Vec<u8> {
    let mut bytes = vec![0; 1510];
    bytes[..4].copy_from_slice(&1510u32.to_le_bytes());
    bytes[4..8].copy_from_slice(&3u32.to_le_bytes());
    bytes[12..22].copy_from_slice(b"SideWinder");
    bytes[22..38].copy_from_slice(&[
        0x84, 0xce, 0x48, 0xc9, 0x26, 0x90, 0xd0, 0x11, 0x9f, 0xfe, 0, 0xa0, 0xc9, 0x11, 0xf5, 0xaf,
    ]);
    bytes[142..152].copy_from_slice(b"Use/Attack");
    bytes[206..210].copy_from_slice(&1u32.to_le_bytes());
    bytes[222..228].copy_from_slice(&[1, 0x1d, 0, 0, 0, 0]);
    bytes[890..894].copy_from_slice(&256u32.to_le_bytes());
    bytes[1509] = 0xa5;
    bytes
}

const REPORT: &[u8] = b"Hugh's Marvelous 3D Information Program, Version 2.2\r\n(C)1998 Bog Off Productions, All Rights Reserved.\r\n\r\nXBUCKETE.3D\tv2.7\r\nXWANTED.3D\tv4.0\r\n\r\n2 files found.\r\n";

#[test]
fn profile_preserves_labels_empty_controls_and_opaque_bytes() {
    let bytes = profile_bytes();
    let file = parse_sidewinder_file(&bytes).unwrap();
    assert_eq!(file.version, 3);
    assert_eq!(file.device_id[0], 0x84);
    assert_eq!(file.controls.len(), 2);
    assert_eq!(file.controls[0].name, "Use/Attack");
    assert_eq!(file.controls[0].field_40, 1);
    assert_eq!(file.controls[1].name, "");
    assert_eq!(file.controls[1].field_40, 256);
    assert_eq!(file.controls[1].data.last(), Some(&0xa5));
    let reconstructed: Vec<u8> = file
        .header
        .iter()
        .copied()
        .chain(file.controls.iter().flat_map(|c| c.data.iter().copied()))
        .collect();
    assert_eq!(reconstructed, bytes);
    for len in 0..bytes.len() {
        assert!(
            parse_sidewinder_file(&bytes[..len]).is_err(),
            "length {len}"
        );
    }
}

#[test]
fn profile_rejects_bad_size_signature_version_and_partial_record() {
    for (offset, value) in [(0, 0), (4, 4), (12, b'X')] {
        let mut bytes = profile_bytes();
        bytes[offset] = value;
        assert!(parse_sidewinder_file(&bytes).is_err());
    }
    let mut bytes = profile_bytes();
    bytes.push(0);
    bytes[..4].copy_from_slice(&1511u32.to_le_bytes());
    assert!(parse_sidewinder_file(&bytes).is_err());
    let mut bytes = profile_bytes();
    bytes[142..206].fill(b'A');
    assert_eq!(
        parse_sidewinder_file(&bytes).unwrap().controls[0].name,
        "A".repeat(64)
    );
}

#[test]
fn report_retains_order_versions_duplicates_and_checks_the_footer() {
    let file = parse_more_file(REPORT).unwrap();
    assert_eq!(file.tool_version, "2.2");
    assert_eq!(file.declared_count, 2);
    assert_eq!(file.entries[0].filename, "XBUCKETE.3D");
    assert_eq!(file.entries[0].version, "v2.7");
    assert_eq!(file.entries[1].filename, "XWANTED.3D");
    assert_eq!(file.image, REPORT);
    let lf = String::from_utf8(REPORT.to_vec())
        .unwrap()
        .replace("\r\n", "\n")
        .replace("XWANTED.3D", "XBUCKETE.3D");
    assert_eq!(
        parse_more_file(lf.as_bytes()).unwrap().entries[1].filename,
        "XBUCKETE.3D"
    );
    let text = std::str::from_utf8(REPORT).unwrap();
    for broken in [
        text.replace("2 files found.", "3 files found."),
        text.replace("2 files found.\r\n", ""),
        text.replace("\tv2.7", " v2.7"),
        text.replace("v2.7", "v2.x"),
        text.replace("XBUCKETE.3D", "../XBUCKETE.3D"),
        text.replace("XBUCKETE.3D", "XBUCKETE.ROB"),
        text.replace("Hugh's", "Unknown"),
        format!("{text}extra\n"),
        format!("{text}\0"),
    ] {
        assert!(parse_more_file(broken.as_bytes()).is_err(), "{broken:?}");
    }
    assert!(parse_more_file(&[0xff]).is_err());
    assert_eq!(FileType::from_path("fxart/MORE"), Some(FileType::More));
    assert_eq!(FileType::from_path("more.txt"), None);
    assert_eq!(FileType::from_path("more.3d"), Some(FileType::Model3d));
}

#[test]
fn cli_reads_scans_and_exports_profiles_and_extensionless_reports() {
    let root =
        std::env::temp_dir().join(format!("rgpre-controller-reports-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    for (name, subcommand, bytes, expected) in [
        (
            "REDGUARD.SWG",
            "sidewinder",
            profile_bytes(),
            serde_json::json!("Use/Attack"),
        ),
        (
            "redguard.swx",
            "sidewinder",
            profile_bytes(),
            serde_json::json!("Use/Attack"),
        ),
        (
            "more",
            "more",
            REPORT.to_vec(),
            serde_json::json!("XBUCKETE.3D"),
        ),
    ] {
        let input = root.join(name);
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
        let output = if name == "more" {
            root.join("more.json")
        } else {
            root.join(format!("{name}.json"))
        };
        for explicit in [false, true] {
            let mut cmd = Command::new(env!("CARGO_BIN_EXE_rgpre"));
            cmd.arg("convert");
            if explicit {
                cmd.arg(subcommand);
            }
            let result = cmd.arg(&input).output().unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
            let json: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&output).unwrap()).unwrap();
            let pointer = if name == "more" {
                "/entries/0/filename"
            } else {
                "/controls/0/name"
            };
            assert_eq!(json.pointer(pointer), Some(&expected));
            std::fs::remove_file(&output).unwrap();
        }
        assert_eq!(std::fs::read(&input).unwrap(), bytes);
        std::fs::write(&input, []).unwrap();
        let result = Command::new(env!("CARGO_BIN_EXE_rgpre"))
            .arg("convert")
            .arg(&input)
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
    let text = String::from_utf8_lossy(&scan.stdout);
    assert!(text.contains("SideWinder Profile"), "{text}");
    assert!(text.contains("3D Information Report"), "{text}");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn installed_profiles_and_report() {
    let Some(root) = std::env::var_os("REDGUARD_DIR") else {
        eprintln!("SKIP installed profiles and report: REDGUARD_DIR absent");
        return;
    };
    let root = PathBuf::from(root);
    for (name, label, device) in [
        ("REDGUARD.SWG", "Use/Attack", 0x84),
        ("redguard.swx", "Button A", 0x86),
    ] {
        let bytes = std::fs::read(root.join(name)).unwrap();
        let file = parse_sidewinder_file(&bytes).unwrap();
        assert_eq!(file.controls.len(), 27);
        assert_eq!(file.controls[0].name, label);
        assert_eq!(file.device_id[0], device);
        assert_eq!(file.controls[19].name, "Up + Right");
        assert_eq!(file.controls[26].name, "");
    }
    let bytes = std::fs::read(root.join("fxart/more")).unwrap();
    let file = parse_more_file(&bytes).unwrap();
    assert_eq!(file.entries.len(), 65);
    assert_eq!(file.entries[0].filename, "XBUCKETE.3D");
    assert_eq!(file.entries[0].version, "v2.7");
    assert_eq!(file.entries[64].filename, "CRAK0019.3D");
}
