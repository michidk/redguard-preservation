use redguard_preservation::import::{
    FileType, bsi, fnt, model3d, palette::Palette, pvo, rgm, rob, wld,
};
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
