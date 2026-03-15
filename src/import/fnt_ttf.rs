use crate::error::Error;
use crate::import::fnt::FntFile;
use kurbo::BezPath;
use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};
use write_fonts::OffsetMarker;
use write_fonts::tables::cmap::Cmap;
use write_fonts::tables::glyf::Bbox;
use write_fonts::tables::glyf::{GlyfLocaBuilder, Glyph, SimpleGlyph};
use write_fonts::tables::head::{Flags, Head, MacStyle};
use write_fonts::tables::hhea::Hhea;
use write_fonts::tables::hmtx::Hmtx;
use write_fonts::tables::loca::LocaFormat;
use write_fonts::tables::maxp::Maxp;
use write_fonts::tables::name::{Name, NameRecord};
use write_fonts::tables::os2::{Os2, SelectionFlags};
use write_fonts::tables::post::Post;
use write_fonts::tables::vmtx::LongMetric;
use write_fonts::types::{FWord, Fixed, GlyphId, LongDateTime, NameId, Tag, UfWord};

const SCALE: u32 = 64;

fn push_rect(path: &mut BezPath, x0: f64, y0: f64, x1: f64, y1: f64) {
    path.move_to((x0, y0));
    path.line_to((x1, y0));
    path.line_to((x1, y1));
    path.line_to((x0, y1));
    path.close_path();
}

fn make_notdef_glyph(units: u32) -> Result<SimpleGlyph, Error> {
    let mut path = BezPath::new();
    let s = f64::from(units.max(SCALE));
    push_rect(&mut path, 0.0, 0.0, s, s);
    SimpleGlyph::from_bezpath(&path)
        .map_err(|_| Error::Conversion("failed to build .notdef glyph".to_string()))
}

fn sanitize_postscript_name(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            out.push(ch);
        }
        if out.len() >= 63 {
            break;
        }
    }

    if out.is_empty() {
        "RedguardFnt-Regular".to_string()
    } else {
        out
    }
}

fn collect_pixel_rects(
    width: u32,
    height: u32,
    pixels: &[u8],
    merge_rects: bool,
) -> Vec<(u32, u32, u32, u32)> {
    if !merge_rects {
        let mut rects = Vec::new();
        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) as usize;
                if pixels.get(idx).copied().unwrap_or(0) != 0 {
                    rects.push((x, y, 1, 1));
                }
            }
        }
        return rects;
    }

    let mut active: HashMap<(u32, u32), (u32, u32)> = HashMap::new();
    let mut rects = Vec::new();

    for y in 0..height {
        let mut runs = Vec::new();
        let mut x = 0_u32;
        while x < width {
            let idx = (y * width + x) as usize;
            if pixels.get(idx).copied().unwrap_or(0) == 0 {
                x += 1;
                continue;
            }

            let start = x;
            x += 1;
            while x < width {
                let idx = (y * width + x) as usize;
                if pixels.get(idx).copied().unwrap_or(0) == 0 {
                    break;
                }
                x += 1;
            }
            runs.push((start, x - start));
        }

        let mut seen = HashSet::new();
        for run in runs {
            seen.insert(run);
            if let Some((_, run_h)) = active.get_mut(&run) {
                *run_h += 1;
            } else {
                active.insert(run, (y, 1));
            }
        }

        let stale = active
            .keys()
            .filter(|key| !seen.contains(*key))
            .copied()
            .collect::<Vec<_>>();
        for key in stale {
            if let Some((start_y, run_h)) = active.remove(&key) {
                rects.push((key.0, start_y, key.1, run_h));
            }
        }
    }

    for (key, (start_y, run_h)) in active {
        rects.push((key.0, start_y, key.1, run_h));
    }

    rects
}

/// Builds a TrueType font file from parsed Redguard FNT glyph data.
pub fn build_ttf_from_fnt(fnt: &FntFile, family_name: &str) -> Result<Vec<u8>, Error> {
    let line_height = u32::from(fnt.header.line_height.max(1));
    let baseline = ((line_height as i32) / 5).max(1) as i16;

    let mut builder = write_fonts::FontBuilder::new();
    let mut glyf_builder = GlyfLocaBuilder::new();

    let mut glyph_metrics: Vec<(u32, i16)> = Vec::new();
    let mut glyph_names: Vec<String> = Vec::new();
    let mut cmap_entries: Vec<(u16, u16)> = Vec::new();
    let mut cmap_mappings: Vec<(char, GlyphId)> = Vec::new();
    let mut glyph_bboxes: Vec<(i16, i16, i16, i16)> = Vec::new();

    let mut max_points: u16 = 0;
    let mut max_contours: u16 = 0;

    let notdef = make_notdef_glyph(line_height * SCALE)?;
    glyf_builder
        .add_glyph(&notdef)
        .map_err(|e| Error::Conversion(e.to_string()))?;
    glyph_metrics.push((line_height * SCALE, 0));
    glyph_names.push(".notdef".to_string());
    glyph_bboxes.push((
        notdef.bbox.x_min,
        notdef.bbox.y_min,
        notdef.bbox.x_max,
        notdef.bbox.y_max,
    ));

    for (idx, g) in fnt.glyphs.iter().enumerate() {
        let codepoint = u32::from(fnt.header.character_start) + idx as u32;
        if codepoint > u32::from(u16::MAX) {
            continue;
        }

        let glyph_id = (glyph_names.len()) as u16;
        cmap_entries.push((codepoint as u16, glyph_id));
        if codepoint != 0
            && let Some(ch) = char::from_u32(codepoint)
        {
            cmap_mappings.push((ch, GlyphId::new(glyph_id.into())));
        }
        glyph_names.push(format!("u{codepoint:04X}"));

        let width = u32::from(g.width);
        let height = u32::from(g.height);
        let xoff = i32::from(g.offset_left);
        let yoff = i32::from(g.offset_top);

        let mut path = BezPath::new();
        let mut contours = 0_u16;

        if g.enabled != 0 && width > 0 && height > 0 {
            for (rx, ry, rw, rh) in collect_pixel_rects(width, height, &g.pixels, true) {
                let left = (xoff + rx as i32) as f64 * f64::from(SCALE);
                let right = (xoff + (rx + rw) as i32) as f64 * f64::from(SCALE);

                let y_top_px = (line_height as i32) - yoff - (ry as i32);
                let y_bottom_px = y_top_px - rh as i32;
                let top = y_top_px as f64 * f64::from(SCALE);
                let bottom = y_bottom_px as f64 * f64::from(SCALE);

                push_rect(&mut path, left, bottom, right, top);
                contours = contours.saturating_add(1);
            }
        }

        let glyph = if contours == 0 {
            glyph_bboxes.push((0, 0, 0, 0));
            Glyph::Simple(SimpleGlyph::default())
        } else {
            let mut sg = SimpleGlyph::from_bezpath(&path)
                .map_err(|_| Error::Conversion("failed to build glyph from path".to_string()))?;
            if sg.bbox == Bbox::default() {
                sg.recompute_bounding_box();
            }
            glyph_bboxes.push((sg.bbox.x_min, sg.bbox.y_min, sg.bbox.x_max, sg.bbox.y_max));
            let points = sg.contours.iter().map(|c| c.len() as u16).sum::<u16>();
            max_points = max_points.max(points);
            max_contours = max_contours.max(sg.contours.len() as u16);
            Glyph::Simple(sg)
        };

        glyf_builder
            .add_glyph(&glyph)
            .map_err(|e| Error::Conversion(e.to_string()))?;

        let x_advance = width.saturating_add(1).saturating_mul(SCALE);
        glyph_metrics.push((x_advance.max(SCALE), (xoff * SCALE as i32) as i16));
    }

    if cmap_entries.is_empty() {
        return Err(Error::Conversion(
            "FNT has no mappable glyphs for TTF export".to_string(),
        ));
    }

    cmap_entries.sort_by_key(|(cp, _)| *cp);

    let units_per_em = (line_height * SCALE) as u16;
    let ascender = ((line_height as i32 - baseline as i32) * SCALE as i32) as i16;
    let descender = -(baseline * SCALE as i16);

    let mut head_x_min = 0_i16;
    let mut head_y_min = 0_i16;
    let mut head_x_max = 0_i16;
    let mut head_y_max = 0_i16;
    let mut bbox_inited = false;

    let mut advance_width_max = 0_u16;
    let mut min_lsb = i16::MAX;
    let mut min_rsb = i16::MAX;
    let mut x_max_extent = i16::MIN;

    for ((aw, lsb), (x_min, y_min, x_max, y_max)) in glyph_metrics.iter().zip(glyph_bboxes.iter()) {
        let aw_u16 = (*aw).min(u32::from(u16::MAX)) as u16;
        advance_width_max = advance_width_max.max(aw_u16);

        let glyph_width = i32::from(*x_max) - i32::from(*x_min);
        let rsb_i32 = i32::from(aw_u16) - (i32::from(*lsb) + glyph_width);
        let rsb = rsb_i32.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16;

        min_lsb = min_lsb.min(*lsb);
        min_rsb = min_rsb.min(rsb);

        let extent_i32 = i32::from(*lsb) + glyph_width;
        let extent = extent_i32.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16;
        x_max_extent = x_max_extent.max(extent);

        if *x_min != 0 || *y_min != 0 || *x_max != 0 || *y_max != 0 || !bbox_inited {
            if !bbox_inited {
                head_x_min = *x_min;
                head_y_min = *y_min;
                head_x_max = *x_max;
                head_y_max = *y_max;
                bbox_inited = true;
            } else {
                head_x_min = head_x_min.min(*x_min);
                head_y_min = head_y_min.min(*y_min);
                head_x_max = head_x_max.max(*x_max);
                head_y_max = head_y_max.max(*y_max);
            }
        }
    }

    let now_unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| Error::Conversion(e.to_string()))?
        .as_secs() as i64;
    let now_mac_epoch = now_unix.saturating_add(2_082_844_800);

    let mut head_flags = Flags::empty();
    head_flags.insert(Flags::BASELINE_AT_Y_0);
    let all_lsb_match_xmin = glyph_metrics
        .iter()
        .zip(glyph_bboxes.iter())
        .all(|((_, lsb), (x_min, _, _, _))| *lsb == *x_min);
    if all_lsb_match_xmin {
        head_flags.insert(Flags::LSB_AT_X_0);
    }

    let (glyf, loca, loca_format) = glyf_builder.build();
    let index_to_loc_format = match loca_format {
        LocaFormat::Short => 0,
        LocaFormat::Long => 1,
    };

    let head = Head::new(
        Fixed::from(0),
        0,
        head_flags,
        units_per_em,
        LongDateTime::new(now_mac_epoch),
        LongDateTime::new(now_mac_epoch),
        head_x_min,
        head_y_min,
        head_x_max,
        head_y_max,
        MacStyle::empty(),
        8,
        index_to_loc_format,
    );
    builder
        .add_table(&head)
        .map_err(|e| Error::Conversion(e.to_string()))?;

    let mut name_records = Vec::new();
    let platform_id = 3_u16;
    let encoding_id = 1_u16;
    let language_id = 0x0409_u16;
    let postscript_name = sanitize_postscript_name(&format!("{family_name}-Regular"));
    name_records.push(NameRecord {
        platform_id,
        encoding_id,
        language_id,
        name_id: NameId::from(1),
        string: OffsetMarker::new(family_name.to_string()),
    });
    name_records.push(NameRecord {
        platform_id,
        encoding_id,
        language_id,
        name_id: NameId::from(2),
        string: OffsetMarker::new("Regular".to_string()),
    });
    name_records.push(NameRecord {
        platform_id,
        encoding_id,
        language_id,
        name_id: NameId::from(3),
        string: OffsetMarker::new(format!("{postscript_name};Version 1.0")),
    });
    name_records.push(NameRecord {
        platform_id,
        encoding_id,
        language_id,
        name_id: NameId::from(4),
        string: OffsetMarker::new(format!("{family_name} Regular")),
    });
    name_records.push(NameRecord {
        platform_id,
        encoding_id,
        language_id,
        name_id: NameId::from(5),
        string: OffsetMarker::new("Version 1.0".to_string()),
    });
    name_records.push(NameRecord {
        platform_id,
        encoding_id,
        language_id,
        name_id: NameId::from(6),
        string: OffsetMarker::new(postscript_name),
    });
    builder
        .add_table(&Name::new(name_records))
        .map_err(|e| Error::Conversion(e.to_string()))?;

    let avg_advance = if glyph_metrics.is_empty() {
        (line_height * SCALE) as i16
    } else {
        let sum: u64 = glyph_metrics.iter().map(|(aw, _)| u64::from(*aw)).sum();
        (sum / glyph_metrics.len() as u64).min(i16::MAX as u64) as i16
    };

    let os2 = Os2 {
        x_avg_char_width: avg_advance,
        us_weight_class: 400,
        us_width_class: 5,
        fs_type: 0,
        y_subscript_x_size: (line_height * SCALE / 2) as i16,
        y_subscript_y_size: (line_height * SCALE / 2) as i16,
        y_subscript_x_offset: 0,
        y_subscript_y_offset: (line_height * SCALE / 4) as i16,
        y_superscript_x_size: (line_height * SCALE / 2) as i16,
        y_superscript_y_size: (line_height * SCALE / 2) as i16,
        y_superscript_x_offset: 0,
        y_superscript_y_offset: (line_height * SCALE / 2) as i16,
        y_strikeout_size: SCALE as i16,
        y_strikeout_position: (line_height * SCALE / 2) as i16,
        s_family_class: 0,
        panose_10: [0; 10],
        ul_unicode_range_1: 0,
        ul_unicode_range_2: 0,
        ul_unicode_range_3: 0,
        ul_unicode_range_4: 0,
        ach_vend_id: Tag::new(b"RGPR"),
        fs_selection: SelectionFlags::REGULAR,
        us_first_char_index: cmap_entries.first().map(|(c, _)| *c).unwrap_or(32),
        us_last_char_index: cmap_entries.last().map(|(c, _)| *c).unwrap_or(126),
        s_typo_ascender: ascender,
        s_typo_descender: descender,
        s_typo_line_gap: 0,
        us_win_ascent: head_y_max.max(0) as u16,
        us_win_descent: head_y_min.unsigned_abs(),
        ul_code_page_range_1: Default::default(),
        ul_code_page_range_2: Default::default(),
        sx_height: Default::default(),
        s_cap_height: Default::default(),
        us_default_char: Default::default(),
        us_break_char: Default::default(),
        us_max_context: Default::default(),
        us_lower_optical_point_size: Default::default(),
        us_upper_optical_point_size: Default::default(),
    };
    builder
        .add_table(&os2)
        .map_err(|e| Error::Conversion(e.to_string()))?;

    let num_glyphs = glyph_names.len() as u16;
    let maxp = Maxp {
        num_glyphs,
        max_points: Some(max_points.max(4)),
        max_contours: Some(max_contours.max(1)),
        max_composite_points: Some(0),
        max_composite_contours: Some(0),
        max_zones: Some(2),
        max_twilight_points: Some(0),
        max_storage: Some(1),
        max_function_defs: Some(1),
        max_instruction_defs: Some(0),
        max_stack_elements: Some(128),
        max_size_of_instructions: Some(0),
        max_component_elements: Some(0),
        max_component_depth: Some(0),
    };
    builder
        .add_table(&maxp)
        .map_err(|e| Error::Conversion(e.to_string()))?;

    let glyph_name_refs: Vec<&str> = glyph_names.iter().map(|s| s.as_str()).collect();
    let mut post = Post::new_v2(glyph_name_refs);
    post.underline_position = FWord::new(-(SCALE as i16));
    post.underline_thickness = FWord::new((SCALE / 2) as i16);
    post.is_fixed_pitch = 0;
    builder
        .add_table(&post)
        .map_err(|e| Error::Conversion(e.to_string()))?;

    let cmap = Cmap::from_mappings(cmap_mappings)
        .map_err(|e| Error::Conversion(format!("failed to build cmap: {e}")))?;
    builder
        .add_table(&cmap)
        .map_err(|e| Error::Conversion(e.to_string()))?;

    let hhea = Hhea::new(
        FWord::new(ascender),
        FWord::new(descender),
        FWord::new(0),
        UfWord::new(advance_width_max),
        FWord::new(min_lsb),
        FWord::new(min_rsb),
        FWord::new(x_max_extent),
        1,
        0,
        0,
        num_glyphs,
    );
    builder
        .add_table(&hhea)
        .map_err(|e| Error::Conversion(e.to_string()))?;

    let hmtx = Hmtx::new(
        glyph_metrics
            .iter()
            .map(|(aw, lsb)| LongMetric::new(*aw as u16, *lsb))
            .collect(),
        vec![],
    );
    builder
        .add_table(&hmtx)
        .map_err(|e| Error::Conversion(e.to_string()))?;

    builder
        .add_table(&glyf)
        .map_err(|e| Error::Conversion(e.to_string()))?;
    builder
        .add_table(&loca)
        .map_err(|e| Error::Conversion(e.to_string()))?;

    Ok(builder.build())
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
    fn merge_rects_reduces_contour_count() {
        let pixels = vec![1, 1, 1, 1, 1, 1];
        let per_pixel = collect_pixel_rects(3, 2, &pixels, false);
        let merged = collect_pixel_rects(3, 2, &pixels, true);
        assert_eq!(per_pixel.len(), 6);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0], (0, 0, 3, 2));
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
