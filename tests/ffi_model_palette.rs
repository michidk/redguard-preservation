use std::ffi::CString;

#[test]
fn model_glb_export_uses_explicit_palette_and_rejects_missing_palette() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let temp = std::env::temp_dir().join(format!("rgpre-palette-test-{}", std::process::id()));
    std::fs::create_dir_all(&temp).unwrap();
    let model_path = temp.join("triangle.3dc");
    let mut data = vec![0u8; 112];
    data[..4].copy_from_slice(b"v4.0");
    let words = [3u32, 1, 1, 1, 0, 3, 0, 0, 0, 0, 0, 64, 100, 3, 112];
    for (index, word) in words.into_iter().enumerate() {
        data[4 + index * 4..8 + index * 4].copy_from_slice(&word.to_le_bytes());
    }
    data[76..80].copy_from_slice(&5120i32.to_le_bytes());
    data[92..96].copy_from_slice(&5120i32.to_le_bytes());
    data[108..112].copy_from_slice(&256i32.to_le_bytes());
    data.extend_from_slice(&[3, 0]);
    data.extend_from_slice(&(4_000_000u32 << 8).to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    for vertex in 0..3u32 {
        data.extend_from_slice(&vertex.to_le_bytes());
        data.extend_from_slice(&[0; 4]);
    }
    std::fs::write(&model_path, data).unwrap();
    let mut bank = Vec::new();
    let mut header = [0u8; 26];
    header[4..6].copy_from_slice(&2i16.to_le_bytes());
    header[6..8].copy_from_slice(&1i16.to_le_bytes());
    header[14..16].copy_from_slice(&1i16.to_le_bytes());
    // No embedded palette: this bank would decode to grayscale without COL.
    for (tag, payload) in [
        (b"IFHD", vec![0; 44]),
        (b"BHDR", header.to_vec()),
        (b"DATA", vec![7, 7]),
    ] {
        bank.extend_from_slice(tag);
        bank.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        bank.extend_from_slice(&payload);
    }
    bank.extend_from_slice(b"END ");
    let bank_path = temp.join("TEXBSI.000");
    std::fs::write(&bank_path, bank).unwrap();
    let model = CString::new(model_path.to_str().unwrap()).unwrap();
    let root = CString::new(temp.to_str().unwrap()).unwrap();
    let palette = CString::new(fixture.join("palette.col").to_str().unwrap()).unwrap();
    let expected = rgpre::import::palette::Palette::parse(
        &std::fs::read(fixture.join("palette.col")).unwrap(),
    )
    .unwrap();

    // The fixture has no WORLD.INI: a standalone model cannot infer its palette.
    let result = unsafe {
        rgpre::ffi::rg_convert_model_from_path_with_palette(
            model.as_ptr(),
            root.as_ptr(),
            palette.as_ptr(),
        )
    };
    if result.is_null() {
        let error = unsafe { rgpre::ffi::rg_last_error() };
        let message =
            unsafe { std::slice::from_raw_parts((*error).ptr, (*error).length as usize) }.to_vec();
        unsafe { rgpre::ffi::rg_free_buffer(error) };
        panic!("{}", String::from_utf8_lossy(&message));
    }
    let bytes =
        unsafe { std::slice::from_raw_parts((*result).ptr, (*result).length as usize) }.to_vec();
    unsafe { rgpre::ffi::rg_free_buffer(result) };
    let glb = gltf::Gltf::from_slice(&bytes).unwrap();
    let blob = glb.blob.as_ref().unwrap();
    let mut found_palette_color = false;
    for image in glb.images() {
        let gltf::image::Source::View { view, .. } = image.source() else {
            panic!("embedded image expected")
        };
        let decoded = image::load_from_memory(&blob[view.offset()..view.offset() + view.length()])
            .unwrap()
            .to_rgb8();
        found_palette_color |= decoded
            .pixels()
            .any(|pixel| pixel[0] != pixel[1] && expected.colors.contains(&pixel.0));
    }
    assert!(
        found_palette_color,
        "export must embed colors from the selected palette"
    );
    let missing = CString::new(fixture.join("missing.col").to_str().unwrap()).unwrap();
    let result = unsafe {
        rgpre::ffi::rg_convert_model_from_path_with_palette(
            model.as_ptr(),
            root.as_ptr(),
            missing.as_ptr(),
        )
    };
    assert!(
        result.is_null(),
        "missing explicit palette must not silently export gray"
    );
    std::fs::remove_file(model_path).unwrap();
    std::fs::remove_file(bank_path).unwrap();
    std::fs::remove_dir(temp).unwrap();
}
