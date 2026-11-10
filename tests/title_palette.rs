use rgpre::import::{gxa::parse_gxa_file, palette::Palette};

#[test]
fn installed_title_retains_the_palette_used_by_low_font_tinting() {
    let Some(root) = std::env::var_os("REDGUARD_DIR") else {
        eprintln!("skipped: no install (title palette)");
        return;
    };
    let root = std::path::PathBuf::from(root);
    let directory = std::fs::read_dir(&root)
        .unwrap()
        .map(Result::unwrap)
        .find(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .eq_ignore_ascii_case("system")
        })
        .unwrap()
        .path();
    let path = std::fs::read_dir(directory)
        .unwrap()
        .map(Result::unwrap)
        .find(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .eq_ignore_ascii_case("startup2.gxa")
        })
        .unwrap()
        .path();
    let archive = parse_gxa_file(&std::fs::read(path).unwrap()).unwrap();
    let frame = &archive.frames[0];
    let palette = Palette {
        colors: frame.palette.map(|rgb| rgb.map(|c| c >> 2)),
    };
    assert_eq!(palette.match_vga([32; 3]), 89);
    assert_eq!(palette.match_vga([50, 0, 0]), 136);
    assert_eq!(frame.palette[161], [204, 124, 64]);
    assert_eq!(frame.palette[89], [132, 136, 120]);
    assert_eq!(frame.palette[136], [184, 8, 40]);
}
