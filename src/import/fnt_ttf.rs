use crate::error::Error;
use crate::import::fnt::FntFile;
use bitmap2ttf::{BitmapGlyph, FontConfig};

fn fnt_to_bitmap_glyphs(fnt: &FntFile) -> Vec<BitmapGlyph> {
    let char_start = u32::from(fnt.header.character_start);

    fnt.glyphs
        .iter()
        .enumerate()
        .map(|(idx, glyph)| {
            let codepoint = char_start.saturating_add(idx as u32);
            let pixels = if glyph.enabled != 0 {
                glyph.pixels.clone()
            } else {
                vec![0; glyph.pixels.len()]
            };

            BitmapGlyph {
                codepoint,
                width: glyph.width,
                height: glyph.height,
                offset_x: glyph.offset_left,
                offset_y: glyph.offset_top,
                advance_width: None,
                pixels,
            }
        })
        .collect()
}

/// Builds a TrueType font file from parsed Redguard FNT glyph data.
#[allow(clippy::missing_errors_doc)]
pub fn build_ttf_from_fnt(fnt: &FntFile, family_name: &str) -> Result<Vec<u8>, Error> {
    let glyphs = fnt_to_bitmap_glyphs(fnt);

    let config = FontConfig {
        family_name: family_name.to_string(),
        line_height: fnt.header.line_height.max(1),
        ..FontConfig::default()
    };

    bitmap2ttf::build_ttf(&glyphs, &config).map_err(|e| Error::Conversion(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import::fnt::{FntFile, FntGlyph, FntHeader, FntPalette};

    fn sample_fnt() -> FntFile {
        let header = FntHeader {
            description_raw: [0; 32],
            description_text: "TEST".to_string(),
            unknown_24: 0,
            has_rdat: 0,
            unknown_28: 0,
            unknown_2a: 0,
            unknown_2c: 0,
            max_width: 2,
            line_height: 8,
            character_start: 32,
            character_count: 1,
            unknown_36: 0,
            unknown_38: 0,
            has_palette: 1,
        };

        let palette = FntPalette {
            tag: *b"BPAL",
            colors: vec![[0, 0, 0], [63, 63, 63]],
        };

        let glyph = FntGlyph {
            enabled: 1,
            offset_left: 0,
            offset_top: 0,
            width: 2,
            height: 2,
            pixels: vec![1, 1, 1, 1],
        };

        FntFile {
            chunk_order: vec![
                "FNHD".to_string(),
                "BPAL".to_string(),
                "FBMP".to_string(),
                "END".to_string(),
            ],
            header,
            palette,
            glyphs: vec![glyph],
            rdat: None,
            trailing_padding: vec![],
        }
    }

    #[test]
    fn default_profile_generates_parseable_ttf() {
        let bytes = build_ttf_from_fnt(&sample_fnt(), "UnitTest Font")
            .expect("ttf generation should succeed");

        let face = ttf_parser::Face::parse(&bytes, 0).expect("ttf should parse");
        assert!(face.number_of_glyphs() >= 2);
        assert!(face.units_per_em() > 0);
    }
}
