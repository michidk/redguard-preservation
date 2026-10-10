use rgpre::import::glide_splash::parse_glide_splash;

#[test]
fn rejects_missing_and_truncated_executable_headers() {
    for length in 0..256 {
        let mut data = vec![0; length];
        if length >= 2 {
            data[..2].copy_from_slice(b"MZ");
        }
        if length >= 64 {
            data[60..64].copy_from_slice(&u32::MAX.to_le_bytes());
        }
        assert!(parse_glide_splash(&data).is_err(), "length {length}");
    }
}

#[test]
fn installed_splash_has_complete_animation_and_textures() {
    let Some(path) = std::env::var_os("REDGUARD_GLIDE_OVL") else {
        eprintln!("skipped: set REDGUARD_GLIDE_OVL to the GOG DOSBOX/glide2x_emu.ovl");
        return;
    };
    let data = std::fs::read(path).unwrap();
    let splash = parse_glide_splash(&data).unwrap();
    assert_eq!(
        splash.meshes.each_ref().map(|mesh| mesh.vertices.len()),
        [68, 1694, 34]
    );
    assert_eq!(
        splash.meshes.each_ref().map(|mesh| mesh.faces.len()),
        [100, 1606, 32]
    );
    assert_eq!(splash.frames.len(), 75);
    assert_eq!(
        splash
            .textures
            .each_ref()
            .map(|texture| (texture.width, texture.height)),
        [(64, 64), (32, 32), (64, 32)]
    );
    for texture in &splash.textures {
        assert_eq!(
            texture.rgba.len(),
            (texture.width * texture.height * 4) as usize
        );
        assert!(
            texture
                .rgba
                .as_chunks::<4>()
                .0
                .iter()
                .any(|pixel| pixel != &texture.rgba[..4])
        );
    }
    for texture in &splash.textures[1..] {
        assert!(
            texture
                .rgba
                .as_chunks::<4>()
                .0
                .iter()
                .all(|pixel| pixel[0] == pixel[1] && pixel[1] == pixel[2] && pixel[3] == 255)
        );
    }
    assert!(
        splash
            .frames
            .windows(2)
            .any(|frames| frames[0] != frames[1])
    );
    for length in [0, 64, 256, data.len() / 2, data.len() - 1] {
        assert!(
            parse_glide_splash(&data[..length]).is_err(),
            "length {length}"
        );
    }
}
