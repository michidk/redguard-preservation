use rgpre::import::surface_ini::{SURFACE_NAMES, SurfaceIni};

#[test]
fn independent_rows_default_to_identity_and_preserve_all_class_codes() {
    let mut text = String::from("[surfaces]\n");
    for (code, name) in SURFACE_NAMES.iter().enumerate() {
        text.push_str(&format!("1,{code},{name}\n"));
    }
    let ini = SurfaceIni::parse(&text).unwrap();
    for code in 0..12 {
        assert_eq!(ini.assignments.surface(128 | u16::from(code)), code);
        let row = ini.sound_table(code).unwrap();
        for source in 0..=255u8 {
            assert_eq!(row[usize::from(source)], u16::from(source));
            assert_eq!(ini.animation_sound(code, source), Some(u16::from(source)));
        }
    }
    assert_eq!(ini.sound_table(12), None);
    assert_eq!(ini.animation_sound(12, 1), None);
    assert_eq!(ini.animation_sound(255, 1), None);
}

#[test]
fn texture_first_match_and_sound_last_match_have_different_precedence() {
    let ini = SurfaceIni::parse(
        "[WOOD]\n17=401\n[surfaces]\n8,2,wood\n8,2,rock\n0,0,sand\n9,1,tile\n\
         [wood]\n17 = 402\n17 = 403\n[rock]\n17 = 404\n\
         [unknown]\n17 = 405\n[unrelated]\nignored\n[wood]\n18 = 406\n",
    )
    .unwrap();
    assert_eq!(ini.assignments.surface((8 << 7) | 2), 7);
    assert_eq!(ini.assignments.surface((9 << 7) | 1), 0);
    assert_eq!(ini.animation_sound(7, 17), Some(403));
    assert_eq!(ini.animation_sound(7, 18), Some(406));
    assert_eq!(ini.animation_sound(10, 17), Some(404));
    assert_eq!(ini.animation_sound(6, 17), Some(17));
    assert_eq!(ini.sound_table(0).unwrap()[17], 405);
    assert_eq!(ini.animation_sound(0, 17), Some(17));
}

#[test]
fn decimal_prefixes_optional_equals_and_word_storage_match_loader() {
    let ini = SurfaceIni::parse(
        "; synthetic\r\n[surfaces]\r\n[TiLe]\r\n0 = 65536 ; wraps\r\n\
         255suffix\t65537suffix\r\n17 = 2147483647 trailing tokens\r\n18 = 0\r\n",
    )
    .unwrap();
    assert_eq!(ini.animation_sound(8, 0), Some(0));
    assert_eq!(ini.animation_sound(8, 255), Some(1));
    assert_eq!(ini.animation_sound(8, 17), Some(65535));
    assert_eq!(ini.animation_sound(8, 18), Some(0));
}

#[test]
fn malformed_sound_records_have_file_section_line_and_key() {
    for record in [
        "256 = 1",
        "-1 = 1",
        "+1 = 1",
        "x = 1",
        "1 = -1",
        "1 = +1",
        "1 = x",
        "1 =",
        "1",
        "1 == 2",
        "1 = 2147483648",
        "999999999999 = 1",
    ] {
        let error = SurfaceIni::parse(&format!("[surfaces]\n[wood]\n{record}\n")).unwrap_err();
        assert!(error.contains("SURFACE.INI [wood] line 3, key"), "{error}");
    }
    assert!(
        SurfaceIni::parse("[surfaces]\n[wood")
            .unwrap_err()
            .contains("line 2")
    );
    assert!(
        SurfaceIni::parse("[wood]\n1=2")
            .unwrap_err()
            .contains("missing [surfaces]")
    );
    assert!(
        SurfaceIni::parse("[surfaces]\n512,0,wood")
            .unwrap_err()
            .contains("line 2")
    );
}
