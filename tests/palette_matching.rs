use rgpre::import::palette::Palette;

#[test]
fn vga_matching_skips_zero_prefers_exact_and_breaks_distance_ties_last() {
    let mut palette = Palette {
        colors: [[63; 3]; 256],
    };
    palette.colors[0] = [32; 3];
    palette.colors[4] = [31, 32, 32];
    palette.colors[5] = [31, 32, 32];
    assert_eq!(palette.match_vga([32; 3]), 5);
    palette.colors[8] = [32; 3];
    palette.colors[9] = [32; 3];
    assert_eq!(palette.match_vga([32; 3]), 8);
}

#[test]
fn vga_matching_masks_requests_and_excludes_white_from_approximation() {
    let mut palette = Palette {
        colors: [[63; 3]; 256],
    };
    palette.colors[2] = [0, 0, 0];
    assert_eq!(palette.match_vga([64; 3]), 2);
    assert_eq!(palette.match_vga([62; 3]), 2);
    assert_eq!(palette.match_vga([63; 3]), 1);
}

#[test]
fn close_brightness_can_win_before_the_channel_distance_fallback() {
    let mut palette = Palette {
        colors: [[63; 3]; 256],
    };
    palette.colors[2] = [31, 32, 32];
    palette.colors[3] = [31, 34, 24];
    assert_eq!(palette.match_vga([32; 3]), 3);
}
