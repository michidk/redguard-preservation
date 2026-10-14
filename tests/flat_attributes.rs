use rgpre::import::rgm::{RgmSection, parser::parse_rgm_file};

#[test]
fn installed_flat_attributes_decode_every_map_and_candle_override() {
    let Some(root) = std::env::var_os("REDGUARD_DIR") else {
        eprintln!("SKIP installed flat attributes: REDGUARD_DIR is absent");
        return;
    };
    let maps = std::path::PathBuf::from(root).join("maps");
    let mut count = 0;
    for entry in std::fs::read_dir(maps).unwrap() {
        let path = entry.unwrap().path();
        if !path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("rgm"))
        {
            continue;
        }
        let bytes = std::fs::read(&path).unwrap();
        let (_, rgm) = parse_rgm_file(&bytes).unwrap();
        assert!(
            !rgm.sections
                .iter()
                .any(|section| matches!(section, RgmSection::Flat(_, _))),
            "{}: raw FLAT payload",
            path.display()
        );
        assert_eq!(
            rgm.sections
                .iter()
                .filter(|section| matches!(section, RgmSection::FlatParsed(_, _)))
                .count(),
            1,
            "{}: expected one typed FLAT section",
            path.display()
        );
        if path.file_stem().unwrap().eq_ignore_ascii_case("BELLTOWR") {
            let candle = rgm.flat_texture_attributes((352 << 7) | 1).unwrap();
            assert_eq!(candle.flags, 0);
            assert_eq!(candle.alpha(), 181);
        }
        count += 1;
    }
    assert_eq!(count, 27);
}
