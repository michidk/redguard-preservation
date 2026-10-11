use rgpre::import::glide_splash_dll::parse_windows_glide_splash;

#[test]
fn rejects_truncated_and_out_of_range_pe_headers() {
    for length in 0..256 {
        let mut bytes = vec![0; length];
        if length >= 2 {
            bytes[..2].copy_from_slice(b"MZ");
        }
        if length >= 64 {
            bytes[60..64].copy_from_slice(&u32::MAX.to_le_bytes());
        }
        assert!(
            parse_windows_glide_splash(&bytes).is_err(),
            "length {length}"
        );
    }
}

#[test]
fn installed_dll_retains_swoosh_animation_and_rejects_damaged_resources() {
    let Some(path) = std::env::var_os("REDGUARD_SPLASH_DLL") else {
        eprintln!("skipped: set REDGUARD_SPLASH_DLL to the installed 3DfxSpl2.dll");
        return;
    };
    let bytes = std::fs::read(path).unwrap();
    let splash = parse_windows_glide_splash(&bytes).unwrap();
    assert_eq!(splash.frames_per_second, 30);
    assert_eq!(splash.frames.len(), 90);
    assert_eq!(splash.meshes.len(), 74);
    assert_eq!(
        splash
            .textures
            .iter()
            .map(|t| (t.width, t.height))
            .collect::<Vec<_>>(),
        [(128, 128), (256, 256)]
    );
    assert_eq!(
        splash
            .sphere_mapped
            .iter()
            .filter(|mapped| **mapped)
            .count(),
        14
    );
    assert_eq!(splash.frames[0].len(), 14);
    assert_eq!(splash.frames[89].len(), 16);
    assert_eq!(splash.final_draws.len(), 32);
    assert_eq!(splash.frames[89][0].mesh, 67);
    assert_eq!(splash.frames[89][0].texture, 1);
    assert_ne!(splash.frames[0][0].camera, splash.frames[89][0].camera);
    for length in [64, 256, bytes.len() / 2] {
        assert!(parse_windows_glide_splash(&bytes[..length]).is_err());
    }
    let version = bytes
        .windows(24)
        .rposition(|b| b == b"3Dfx Splash Version 1.2\n")
        .unwrap();
    let mut damaged = bytes.clone();
    damaged[version + 24..version + 28].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(parse_windows_glide_splash(&damaged).is_err());
}
