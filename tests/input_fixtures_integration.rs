use rgpre::import::{FileType, bsi, fnt, gxa, model3d, palette::Palette, pvo, rgm, rob, wld};
use std::path::{Path, PathBuf};

fn with_large_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(f)
        .expect("failed to spawn large-stack test thread")
        .join()
        .expect("large-stack test thread panicked")
}

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn fixture_bytes(name: &str) -> Vec<u8> {
    let path = fixture_path(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("failed to read '{}': {e}", path.display()))
}

fn minimal_gxa_bytes() -> Vec<u8> {
    let mut bytes = Vec::new();

    let mut bmhd = vec![0u8; 34];
    bmhd[..4].copy_from_slice(b"TEST");
    bmhd[32..34].copy_from_slice(&1i16.to_le_bytes());

    let mut bpal = vec![0u8; 256 * 3];
    for i in 0..256usize {
        let o = i * 3;
        bpal[o] = u8::try_from(i).unwrap_or_default();
        bpal[o + 1] = u8::try_from(255usize.saturating_sub(i)).unwrap_or_default();
        bpal[o + 2] = 128;
    }

    let mut bbmp = Vec::new();
    bbmp.extend_from_slice(&0i16.to_le_bytes());
    bbmp.extend_from_slice(&2i16.to_le_bytes());
    bbmp.extend_from_slice(&2i16.to_le_bytes());
    bbmp.extend_from_slice(&[0u8; 12]);
    bbmp.extend_from_slice(&[0u8, 1u8, 2u8, 3u8]);

    bytes.extend_from_slice(b"BMHD");
    bytes.extend_from_slice(&(u32::try_from(bmhd.len()).unwrap_or_default()).to_be_bytes());
    bytes.extend_from_slice(&bmhd);

    bytes.extend_from_slice(b"BPAL");
    bytes.extend_from_slice(&(u32::try_from(bpal.len()).unwrap_or_default()).to_be_bytes());
    bytes.extend_from_slice(&bpal);

    bytes.extend_from_slice(b"BBMP");
    bytes.extend_from_slice(&(u32::try_from(bbmp.len()).unwrap_or_default()).to_be_bytes());
    bytes.extend_from_slice(&bbmp);

    bytes.extend_from_slice(b"END ");
    bytes
}

#[test]
fn filetype_resolution_covers_all_variants() {
    let cases = [
        ("texture.bsi", "bsi", FileType::Bsi),
        ("palette.col", "col", FileType::Col),
        ("model.3d", "3d", FileType::Model3d),
        ("model.3dc", "3dc", FileType::Model3dc),
        ("scene.rob", "rob", FileType::Rob),
        ("scene.rgm", "rgm", FileType::Rgm),
        ("octree.pvo", "pvo", FileType::Pvo),
        ("world.wld", "wld", FileType::Wld),
        ("font.fnt", "fnt", FileType::Fnt),
        ("startup.gxa", "gxa", FileType::Gxa),
    ];

    for (path, token, expected) in cases {
        let resolved_from_path = FileType::from_path(path)
            .unwrap_or_else(|| panic!("expected extension resolution for '{path}'"));
        assert_eq!(
            resolved_from_path, expected,
            "FileType::from_path mismatch for '{path}'"
        );

        let resolved_from_str: FileType = token
            .parse()
            .unwrap_or_else(|e| panic!("expected parse for '{token}': {e}"));
        assert_eq!(
            resolved_from_str, expected,
            "FileType::from_str mismatch for '{token}'"
        );
    }
}

#[test]
fn integration_covers_each_filetype_variant() {
    let bsi_data = fixture_bytes("texture.bsi");
    let col_data = fixture_bytes("palette.col");
    let model_3d_data = fixture_bytes("model_v26.3d");
    let model_3dc_data = fixture_bytes("model_v40.3dc");
    let rob_data = fixture_bytes("scene.rob");
    let rgm_data = fixture_bytes("scene.rgm");
    let pvo_data = fixture_bytes("world.pvo");
    let wld_data = fixture_bytes("world.wld");
    let fnt_data = fixture_bytes("font.fnt");
    let gxa_data = minimal_gxa_bytes();

    for filetype in [
        FileType::Bsi,
        FileType::Col,
        FileType::Model3d,
        FileType::Model3dc,
        FileType::Rob,
        FileType::Rgm,
        FileType::Pvo,
        FileType::Wld,
        FileType::Fnt,
        FileType::Gxa,
    ] {
        match filetype {
            FileType::Bsi => {
                let file = bsi::parse_bsi_file(&bsi_data)
                    .unwrap_or_else(|e| panic!("failed to parse BSI fixture: {e}"));
                assert!(!file.images.is_empty(), "expected BSI image records");
            }
            FileType::Col => {
                let palette = Palette::parse(&col_data)
                    .unwrap_or_else(|e| panic!("failed to parse COL fixture: {e}"));
                assert_eq!(
                    palette.colors.len(),
                    256,
                    "expected 256 COL palette entries"
                );
            }
            FileType::Model3d => {
                let model = model3d::parse_3d_file(&model_3d_data)
                    .unwrap_or_else(|e| panic!("failed to parse 3D fixture: {e}"));
                assert_eq!(
                    model.header.version_string(),
                    "v2.6",
                    "expected v2.6 header"
                );
            }
            FileType::Model3dc => {
                let model = model3d::parse_3d_file(&model_3dc_data)
                    .unwrap_or_else(|e| panic!("failed to parse 3DC fixture: {e}"));
                assert_eq!(
                    model.header.version_string(),
                    "v4.0",
                    "expected v4.0 header"
                );
            }
            FileType::Rob => {
                let (file, _models) = rob::parse_rob_with_models(&rob_data)
                    .unwrap_or_else(|e| panic!("failed to parse ROB fixture: {e}"));
                assert!(!file.segments.is_empty(), "expected ROB segments");
            }
            FileType::Rgm => {
                let file = rgm::parse_rgm_file(&rgm_data)
                    .unwrap_or_else(|e| panic!("failed to parse RGM fixture: {e}"));
                assert_eq!(file.sections.len(), 1, "expected single END section");
            }
            FileType::Pvo => {
                let file = pvo::parse_pvo_file(&pvo_data)
                    .unwrap_or_else(|e| panic!("failed to parse PVO fixture: {e}"));
                assert!(!file.octr_nodes.is_empty(), "expected PVO OCTR nodes");
            }
            FileType::Wld => {
                let wld_data = wld_data.clone();
                let section_count = with_large_stack(move || {
                    let file = wld::parse_wld_file(&wld_data)
                        .unwrap_or_else(|e| panic!("failed to parse WLD fixture: {e}"));
                    file.sections.len()
                });
                assert!(section_count > 0, "expected WLD sections");
            }
            FileType::Fnt => {
                let file = fnt::parse_fnt(&fnt_data)
                    .unwrap_or_else(|e| panic!("failed to parse FNT fixture: {e}"));
                assert!(!file.glyphs.is_empty(), "expected FNT glyphs");
            }
            FileType::Gxa => {
                let file = gxa::parse_gxa_file(&gxa_data)
                    .unwrap_or_else(|e| panic!("failed to parse GXA fixture: {e}"));
                assert_eq!(file.frames.len(), 1, "expected one GXA frame");
            }
            FileType::Sfx => {
                // No fixture for SFX; covered by unit tests in src/import/sfx.rs
            }
            FileType::Rtx => {
                // No fixture for RTX; covered by unit tests in src/import/rtx.rs
            }
            FileType::Cht => {
                // No fixture for CHT; trivial 256-byte format
            }
        }
    }
}
