use rgpre::import::{anm::parse_anm_file, lbm::parse_lbm_file, pcx::parse_pcx_file};
use std::{path::PathBuf, process::Command};

fn palette() -> Vec<u8> {
    (0..=255u8)
        .flat_map(|i| [i, i.wrapping_mul(3), 255 - i])
        .collect()
}

fn pcx_bytes(raw: bool) -> Vec<u8> {
    let mut bytes = vec![0; 128];
    bytes[0..4].copy_from_slice(&[10, 5, u8::from(!raw), 8]);
    bytes[8..10].copy_from_slice(&2u16.to_le_bytes());
    bytes[10..12].copy_from_slice(&1u16.to_le_bytes());
    bytes[65] = 1;
    bytes[66..68].copy_from_slice(&4u16.to_le_bytes());
    bytes.extend_from_slice(if raw {
        &[1, 2, 2, 192, 3, 3, 4, 0]
    } else {
        &[1, 194, 2, 193, 192, 194, 3, 4, 0]
    });
    bytes.push(12);
    bytes.extend_from_slice(&palette());
    bytes
}

fn lbm_bytes(raw: bool) -> Vec<u8> {
    let mut header = vec![0; 20];
    header[..2].copy_from_slice(&3u16.to_be_bytes());
    header[2..4].copy_from_slice(&2u16.to_be_bytes());
    header[8] = 8;
    header[10] = u8::from(!raw);
    let colors = palette()[..15].to_vec();
    let body = if raw {
        vec![1, 2, 2, 192, 3, 3, 4, 0]
    } else {
        vec![128, 0, 1, 255, 2, 0, 192, 255, 3, 1, 4, 0]
    };
    let mut bytes = b"FORM\0\0\0\0PBM ".to_vec();
    for (tag, payload) in [
        (b"BMHD", header),
        (b"CMAP", colors),
        (b"NOTE", vec![7, 8, 9]),
        (b"BODY", body),
    ] {
        bytes.extend_from_slice(tag);
        bytes.extend_from_slice(&u32::try_from(payload.len()).unwrap().to_be_bytes());
        bytes.extend_from_slice(&payload);
        if payload.len() % 2 != 0 {
            bytes.push(0);
        }
    }
    let size = u32::try_from(bytes.len() - 8).unwrap();
    bytes[4..8].copy_from_slice(&size.to_be_bytes());
    bytes
}

fn record(operations: &[u8]) -> Vec<u8> {
    let mut bytes = vec![b'B', 0, 1, 0];
    bytes.extend_from_slice(operations);
    bytes
}

fn anm_bytes(width: u16, records: &[Vec<u8>]) -> Vec<u8> {
    let mut bytes = vec![0; 2816 + 65536];
    bytes[..4].copy_from_slice(b"LPF ");
    bytes[4..6].copy_from_slice(&256u16.to_le_bytes());
    bytes[6..8].copy_from_slice(&2u16.to_le_bytes());
    bytes[8..12].copy_from_slice(&u32::try_from(records.len()).unwrap().to_le_bytes());
    bytes[12..14].copy_from_slice(&256u16.to_le_bytes());
    bytes[14..16].copy_from_slice(&1280u16.to_le_bytes());
    bytes[16..20].copy_from_slice(b"ANIM");
    bytes[20..22].copy_from_slice(&width.to_le_bytes());
    bytes[22..24].copy_from_slice(&1u16.to_le_bytes());
    bytes[25..32].copy_from_slice(&[1, 1, 1, 0, 1, 0, 1]);
    bytes[64..68].copy_from_slice(&u32::try_from(records.len()).unwrap().to_le_bytes());
    bytes[68..70].copy_from_slice(&10u16.to_le_bytes());
    for (i, rgb) in palette().as_chunks::<3>().0.iter().enumerate() {
        bytes[256 + i * 4..260 + i * 4].copy_from_slice(&[rgb[2], rgb[1], rgb[0], 0]);
    }
    let split = records.len() / 2;
    for (page, base, records) in [(0, split, &records[split..]), (1, 0, &records[..split])] {
        let mut descriptor = Vec::new();
        descriptor.extend_from_slice(&u16::try_from(base).unwrap().to_le_bytes());
        descriptor.extend_from_slice(&u16::try_from(records.len()).unwrap().to_le_bytes());
        let size: usize = records.iter().map(Vec::len).sum();
        descriptor.extend_from_slice(&u16::try_from(size).unwrap().to_le_bytes());
        bytes[1280 + page * 6..1286 + page * 6].copy_from_slice(&descriptor);
        let mut payload = descriptor;
        payload.extend_from_slice(&[0, 0]);
        for r in records {
            payload.extend_from_slice(&u16::try_from(r.len()).unwrap().to_le_bytes());
        }
        for r in records {
            payload.extend_from_slice(r);
        }
        let start = 2816 + page * 65536;
        bytes.resize(bytes.len().max(start + payload.len()), 0);
        bytes[start..start + payload.len()].copy_from_slice(&payload);
    }
    bytes
}

fn animation() -> Vec<u8> {
    anm_bytes(
        4,
        &[
            record(&[4, 1, 2, 3, 4, 128, 0, 0]),
            record(&[129, 0, 2, 9, 129, 128, 0, 0]),
            record(&[128, 2, 128, 7, 8, 128, 2, 192, 6, 128, 0, 0]),
            record(&[4, 1, 2, 3, 4, 128, 0, 0]),
        ],
    )
}

#[test]
fn pcx_decodes_raw_and_rle_rows_with_padding() {
    for raw in [false, true] {
        let bytes = pcx_bytes(raw);
        let file = parse_pcx_file(&bytes).unwrap();
        assert_eq!((file.image.width, file.image.height), (3, 2));
        assert_eq!(file.image.pixels, [1, 2, 2, 3, 3, 4]);
        assert_eq!(&file.image.rgba()[..4], [1, 3, 254, 255]);
        for len in 0..bytes.len() {
            assert!(parse_pcx_file(&bytes[..len]).is_err(), "length {len}");
        }
    }
    let original = pcx_bytes(false);
    for (offset, value) in [(0, 0), (3, 4), (65, 3), (66, 2), (128, 196)] {
        let mut bytes = original.clone();
        bytes[offset] = value;
        assert!(parse_pcx_file(&bytes).is_err());
    }
    let mut bytes = original.clone();
    let marker = bytes.len() - 769;
    bytes[marker] = 0;
    assert!(parse_pcx_file(&bytes).is_err());
}

#[test]
fn pbm_decodes_byterun_rows_and_preserves_odd_chunks() {
    for raw in [false, true] {
        let bytes = lbm_bytes(raw);
        let file = parse_lbm_file(&bytes).unwrap();
        assert_eq!((file.image.width, file.image.height), (3, 2));
        assert_eq!(file.image.pixels, [1, 2, 2, 3, 3, 4]);
        assert_eq!(file.chunks[2].tag, *b"NOTE");
        assert_eq!(file.chunks[2].data, [7, 8, 9]);
        for len in 0..bytes.len() {
            assert!(parse_lbm_file(&bytes[..len]).is_err(), "length {len}");
        }
    }
    let mut bytes = lbm_bytes(false);
    bytes[29] = 2;
    bytes[33] = 1;
    assert_eq!(parse_lbm_file(&bytes).unwrap().image.rgba()[3], 0);
    for (offset, value) in [(8, b'I'), (28, 4), (29, 1), (30, 2), (16, 255)] {
        let mut bytes = lbm_bytes(false);
        bytes[offset] = value;
        assert!(parse_lbm_file(&bytes).is_err());
    }
    let mut bytes = lbm_bytes(false);
    let body = bytes.windows(4).position(|w| w == b"BODY").unwrap() + 8;
    bytes[body] = 4;
    assert!(parse_lbm_file(&bytes).is_err());
}

#[test]
fn anm_orders_pages_applies_deltas_and_omits_loop_record() {
    let bytes = animation();
    let file = parse_anm_file(&bytes).unwrap();
    assert_eq!(file.pages[0].base_record, 2);
    assert_eq!(file.records.len(), 4);
    assert_eq!(file.frames.len(), 3);
    assert_eq!(file.frames_per_second, 10);
    assert_eq!(file.frames[0].pixels, [1, 2, 3, 4]);
    assert_eq!(file.frames[1].pixels, [1, 9, 9, 4]);
    assert_eq!(file.frames[2].pixels, [7, 8, 6, 6]);
    assert_eq!(file.loop_delta, Some(file.records[3]));
    assert_eq!(file.frames[0].palette[1], [1, 3, 254]);
    for len in [0, 127, 255, 1279, 1280, 2815, 2816, bytes.len() - 1] {
        assert!(parse_anm_file(&bytes[..len]).is_err());
    }
    for (offset, value) in [(5, 0), (29, 2), (30, 1), (68, 0), (1283, 128), (2816, 0)] {
        let mut bytes = bytes.clone();
        bytes[offset] = value;
        assert!(parse_anm_file(&bytes).is_err(), "offset {offset}");
    }
    let broken = anm_bytes(
        4,
        &[record(&[5, 1, 2, 3, 4, 5, 128, 0, 0]), record(&[128, 0, 0])],
    );
    assert!(parse_anm_file(&broken).is_err());
    let broken = anm_bytes(
        4,
        &[record(&[128, 0, 128, 128, 0, 0]), record(&[128, 0, 0])],
    );
    assert!(parse_anm_file(&broken).is_err());
}

#[test]
fn anm_decodes_extended_skips_and_noops() {
    let bytes = anm_bytes(
        16388,
        &[
            record(&[
                128, 0, 64, 128, 0, 192, 0, 0, 7, 128, 2, 64, 2, 5, 6, 128, 0, 0,
            ]),
            record(&[128, 0, 0]),
        ],
    );
    let file = parse_anm_file(&bytes).unwrap();
    assert_eq!(&file.frames[0].pixels[16386..], [5, 6]);
    assert!(file.frames[0].pixels[..16386].iter().all(|&b| b == 0));
}

#[test]
fn cli_exports_pngs_and_a_timed_gif() {
    use image::AnimationDecoder;
    let root = std::env::temp_dir().join(format!("rgpre-legacy-images-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    for (extension, bytes) in [
        ("pcx", pcx_bytes(false)),
        ("lbm", lbm_bytes(false)),
        ("anm", animation()),
    ] {
        let input = root.join(format!("test.{extension}"));
        std::fs::write(&input, bytes).unwrap();
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
        for explicit in [false, true] {
            let out = root.join(format!("{extension}-{explicit}"));
            let mut command = Command::new(env!("CARGO_BIN_EXE_rgpre"));
            command.arg("convert");
            if explicit {
                command.arg(extension);
            }
            let result = command.arg(&input).arg("-o").arg(&out).output().unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
            if extension == "anm" {
                let decoder = image::codecs::gif::GifDecoder::new(std::io::BufReader::new(
                    std::fs::File::open(out.join("animation.gif")).unwrap(),
                ))
                .unwrap();
                let frames = decoder.into_frames().collect_frames().unwrap();
                assert_eq!(frames.len(), 3);
                assert_eq!(frames[0].delay().numer_denom_ms(), (100, 1));
                assert_eq!(frames[0].buffer().dimensions(), (4, 1));
                assert_eq!(frames[0].buffer().get_pixel(0, 0).0, [1, 3, 254, 255]);
                assert_eq!(frames[1].buffer().get_pixel(1, 0).0, [9, 27, 246, 255]);
                assert_eq!(frames[2].buffer().get_pixel(0, 0).0, [7, 21, 248, 255]);
                let metadata: serde_json::Value =
                    serde_json::from_slice(&std::fs::read(out.join("index.json")).unwrap())
                        .unwrap();
                assert_eq!(metadata["frame_count"], 3);
                assert_eq!(metadata["record_count"], 4);
                assert_eq!(metadata["looping"], true);
            } else {
                let image = image::load_from_memory(&std::fs::read(out).unwrap())
                    .unwrap()
                    .to_rgba8();
                assert_eq!(image.dimensions(), (3, 2));
                assert_eq!(image.get_pixel(0, 0).0, [1, 3, 254, 255]);
            }
        }
        std::fs::write(&input, []).unwrap();
        let out = root.join(format!("invalid-{extension}"));
        let result = Command::new(env!("CARGO_BIN_EXE_rgpre"))
            .arg("convert")
            .arg(&input)
            .arg("-o")
            .arg(&out)
            .output()
            .unwrap();
        assert!(!result.status.success());
        assert!(!out.exists());
    }
    std::fs::remove_dir_all(root).unwrap();
}

fn fingerprint(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325u64, |hash, &b| {
        (hash ^ u64::from(b)).wrapping_mul(0x100000001b3)
    })
}

#[test]
fn installed_legacy_images_match_reference_decodes() {
    let Some(root) = std::env::var_os("REDGUARD_DIR") else {
        eprintln!("SKIP installed legacy images: REDGUARD_DIR absent");
        return;
    };
    let system = PathBuf::from(root).join("system");
    let bytes = std::fs::read(system.join("SKY_61.PCX")).unwrap();
    let pcx = parse_pcx_file(&bytes).unwrap();
    assert_eq!((pcx.image.width, pcx.image.height), (512, 512));
    // ImageMagick's decoded RGBA bytes.
    assert_eq!(fingerprint(&pcx.image.rgba()), 0x0eb9315350d4c49d);
    let bytes = std::fs::read(system.join("gui.lbm")).unwrap();
    let lbm = parse_lbm_file(&bytes).unwrap();
    assert_eq!((lbm.image.width, lbm.image.height), (640, 480));
    assert_eq!(fingerprint(&lbm.image.pixels), 0x6d48367899e31515);
    assert_eq!(lbm.chunks.iter().filter(|c| &c.tag == b"CRNG").count(), 16);
    let bytes = std::fs::read(system.join("gui.anm")).unwrap();
    let anm = parse_anm_file(&bytes).unwrap();
    assert_eq!(anm.frames.len(), 25);
    assert_eq!(anm.frames_per_second, 10);
    assert_eq!(fingerprint(&anm.frames[0].pixels), 0xd9a8e25b9f205e3b);
    assert_eq!(fingerprint(&anm.frames[24].pixels), 0x8c63c2950b94f301);
}
