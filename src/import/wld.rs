use crate::{Result, error::Error};
use image::{GrayImage, Luma};
use std::path::{Path, PathBuf};

pub const WLD_HEADER_BYTES: usize = 1184;
pub const WLD_HEADER_DWORDS: usize = WLD_HEADER_BYTES / 4;
pub const WLD_SECTION_COUNT: usize = 4;
pub const WLD_SECTION_HEADER_BYTES: usize = 22;
pub const WLD_MAP_SIDE: usize = 128;
pub const WLD_MAP_BYTES: usize = WLD_MAP_SIDE * WLD_MAP_SIDE;
pub const WLD_SECTION_BYTES: usize = WLD_SECTION_HEADER_BYTES + (4 * WLD_MAP_BYTES);
pub const WLD_FOOTER_BYTES: usize = 16;

#[derive(Debug, Clone)]
/// Parsed WLD header stored as raw 32-bit fields.
pub struct WldHeader {
    pub fields: [u32; WLD_HEADER_DWORDS],
}

impl WldHeader {
    /// Returns the four section offsets from header fields 36..39.
    pub fn section_offsets(&self) -> [u32; WLD_SECTION_COUNT] {
        [
            self.fields[36],
            self.fields[37],
            self.fields[38],
            self.fields[39],
        ]
    }
}

#[derive(Debug, Clone)]
/// Parsed WLD section containing a header blob and four 128x128 maps.
pub struct WldSection {
    pub header: [u8; WLD_SECTION_HEADER_BYTES],
    pub maps: [[u8; WLD_MAP_BYTES]; 4],
}

#[derive(Debug, Clone)]
/// Parsed WLD file with header, four sections, and footer bytes.
pub struct WldFile {
    pub header: WldHeader,
    pub sections: [WldSection; WLD_SECTION_COUNT],
    pub footer: [u8; WLD_FOOTER_BYTES],
}

fn read_u32_le(data: &[u8], offset: usize) -> Result<u32> {
    if offset + 4 > data.len() {
        return Err(Error::Parse(format!(
            "WLD read out of bounds at offset {offset}"
        )));
    }
    Ok(u32::from_le_bytes(
        data[offset..offset + 4]
            .try_into()
            .map_err(|_| Error::Parse(format!("WLD failed to parse LE u32 at offset {offset}")))?,
    ))
}

fn read_fixed<const N: usize>(data: &[u8], offset: usize) -> Result<[u8; N]> {
    if offset + N > data.len() {
        return Err(Error::Parse(format!(
            "WLD read out of bounds at offset {offset} for {N} bytes"
        )));
    }

    data[offset..offset + N].try_into().map_err(|_| {
        Error::Parse(format!(
            "WLD failed to read fixed buffer at offset {offset}"
        ))
    })
}

/// Parses a WLD file from bytes and validates core section layout.
pub fn parse_wld_file(data: &[u8]) -> Result<WldFile> {
    let minimum_size = WLD_HEADER_BYTES + WLD_FOOTER_BYTES;
    if data.len() < minimum_size {
        return Err(Error::Parse(format!(
            "WLD file too small: {} bytes (minimum {minimum_size})",
            data.len()
        )));
    }

    let mut fields = [0_u32; WLD_HEADER_DWORDS];
    for (idx, field) in fields.iter_mut().enumerate() {
        *field = read_u32_le(data, idx * 4)?;
    }
    let header = WldHeader { fields };

    let offsets = header.section_offsets();
    if offsets[0] as usize != WLD_HEADER_BYTES {
        return Err(Error::Parse(format!(
            "WLD section0 offset mismatch: expected {WLD_HEADER_BYTES}, found {}",
            offsets[0]
        )));
    }
    if header.fields[6] as usize != WLD_SECTION_HEADER_BYTES {
        return Err(Error::Parse(format!(
            "WLD section-header-size mismatch: expected {WLD_SECTION_HEADER_BYTES}, found {}",
            header.fields[6]
        )));
    }
    for (idx, off) in offsets.iter().enumerate() {
        let next_off = if idx + 1 < WLD_SECTION_COUNT {
            offsets[idx + 1] as usize
        } else {
            data.len().saturating_sub(WLD_FOOTER_BYTES)
        };

        let current = *off as usize;
        if current >= data.len() {
            return Err(Error::Parse(format!(
                "WLD section {idx} offset out of range: {current}"
            )));
        }
        if current >= next_off {
            return Err(Error::Parse(format!(
                "WLD section offsets are not strictly increasing at index {idx}: {current} >= {next_off}"
            )));
        }
        let span = next_off - current;
        if span < WLD_SECTION_BYTES {
            return Err(Error::Parse(format!(
                "WLD section {idx} too small: {span} bytes (minimum {WLD_SECTION_BYTES})"
            )));
        }
    }

    let mut parsed_sections = Vec::with_capacity(WLD_SECTION_COUNT);
    for off in offsets {
        let start = off as usize;
        let header_bytes = read_fixed::<WLD_SECTION_HEADER_BYTES>(data, start)?;
        let mut cursor = start + WLD_SECTION_HEADER_BYTES;
        let mut maps = [[0_u8; WLD_MAP_BYTES]; 4];
        for map in &mut maps {
            *map = read_fixed::<WLD_MAP_BYTES>(data, cursor)?;
            cursor += WLD_MAP_BYTES;
        }
        parsed_sections.push(WldSection {
            header: header_bytes,
            maps,
        });
    }

    let sections: [WldSection; WLD_SECTION_COUNT] = parsed_sections.try_into().map_err(|_| {
        Error::Parse("WLD internal section conversion failed (expected 4 sections)".to_string())
    })?;

    let footer = read_fixed::<WLD_FOOTER_BYTES>(data, data.len() - WLD_FOOTER_BYTES)?;

    Ok(WldFile {
        header,
        sections,
        footer,
    })
}

impl WldFile {
    /// Combines one map layer across the four WLD sections into a 2x2 grid.
    pub fn combined_map(&self, map_index: usize) -> Result<Vec<u8>> {
        if map_index >= 4 {
            return Err(Error::Parse(format!(
                "WLD map index out of range: {map_index} (expected 0..3)"
            )));
        }

        let out_side = WLD_MAP_SIDE * 2;
        let mut out = vec![0_u8; out_side * out_side];

        for y in 0..WLD_MAP_SIDE {
            let src_row = y * WLD_MAP_SIDE;
            let dst_top = y * out_side;
            let dst_bottom = (y + WLD_MAP_SIDE) * out_side;

            out[dst_top..dst_top + WLD_MAP_SIDE].copy_from_slice(
                &self.sections[0].maps[map_index][src_row..src_row + WLD_MAP_SIDE],
            );
            out[dst_top + WLD_MAP_SIDE..dst_top + out_side].copy_from_slice(
                &self.sections[1].maps[map_index][src_row..src_row + WLD_MAP_SIDE],
            );
            out[dst_bottom..dst_bottom + WLD_MAP_SIDE].copy_from_slice(
                &self.sections[2].maps[map_index][src_row..src_row + WLD_MAP_SIDE],
            );
            out[dst_bottom + WLD_MAP_SIDE..dst_bottom + out_side].copy_from_slice(
                &self.sections[3].maps[map_index][src_row..src_row + WLD_MAP_SIDE],
            );
        }

        Ok(out)
    }

    /// Returns combined map 1 with the high bit cleared for heightmap luminance.
    pub fn combined_heightmap_luma(&self) -> Result<Vec<u8>> {
        let map1 = self.combined_map(0)?;
        Ok(map1.into_iter().map(|value| value & 0x7F).collect())
    }
}

fn ensure_parent_dir(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|e| Error::File {
            path: parent.to_path_buf(),
            message: format!("failed to create directory: {e}"),
        })?;
    }
    Ok(())
}

/// Exports combined WLD heightmap data to a PNG file.
pub fn export_wld_heightmap_png(input_path: &Path, output_path: &Path) -> Result<()> {
    let paths = export_wld_maps_pngs(input_path, output_path)?;
    if paths.map1_path != output_path {
        std::fs::copy(&paths.map1_path, output_path).map_err(|e| Error::File {
            path: output_path.to_path_buf(),
            message: format!("failed to copy from '{}': {e}", paths.map1_path.display()),
        })?;
    }
    Ok(())
}

#[derive(Debug, Clone)]
/// Output PNG paths generated when exporting all four WLD maps.
pub struct WldMapExportPaths {
    pub map1_path: PathBuf,
    pub map2_path: PathBuf,
    pub map3_path: PathBuf,
    pub map4_path: PathBuf,
}

fn write_luma_png(output_path: &Path, luma: &[u8]) -> Result<()> {
    let expected = (WLD_MAP_SIDE * 2) * (WLD_MAP_SIDE * 2);
    if luma.len() != expected {
        return Err(Error::Conversion(format!(
            "invalid luma buffer size: got {}, expected {}",
            luma.len(),
            expected
        )));
    }

    let mut image = GrayImage::new((WLD_MAP_SIDE * 2) as u32, (WLD_MAP_SIDE * 2) as u32);
    for (idx, px) in luma.iter().enumerate() {
        let x = (idx % (WLD_MAP_SIDE * 2)) as u32;
        let y = (idx / (WLD_MAP_SIDE * 2)) as u32;
        image.put_pixel(x, y, Luma([*px]));
    }

    ensure_parent_dir(output_path)?;
    image.save(output_path).map_err(|e| {
        Error::Conversion(format!(
            "failed to write PNG '{}': {e}",
            output_path.display()
        ))
    })
}

fn map_output_paths(output_path: &Path) -> WldMapExportPaths {
    let mut base = output_path.to_path_buf();
    base.set_extension("");

    let stem = base
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("wld_map");
    let parent = output_path.parent().unwrap_or_else(|| Path::new(""));

    WldMapExportPaths {
        map1_path: parent.join(format!("{stem}_map1.png")),
        map2_path: parent.join(format!("{stem}_map2.png")),
        map3_path: parent.join(format!("{stem}_map3.png")),
        map4_path: parent.join(format!("{stem}_map4.png")),
    }
}

/// Exports all four combined WLD maps as grayscale PNG files.
pub fn export_wld_maps_pngs(input_path: &Path, output_path: &Path) -> Result<WldMapExportPaths> {
    let bytes = std::fs::read(input_path)?;
    let wld = parse_wld_file(&bytes)?;
    let paths = map_output_paths(output_path);

    write_luma_png(&paths.map1_path, &wld.combined_heightmap_luma()?)?;
    write_luma_png(&paths.map2_path, &wld.combined_map(1)?)?;
    write_luma_png(&paths.map3_path, &wld.combined_map(2)?)?;
    write_luma_png(&paths.map4_path, &wld.combined_map(3)?)?;

    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combine_map_stitches_sections_as_2x2_grid() {
        let section = |fill: u8| WldSection {
            header: [0_u8; WLD_SECTION_HEADER_BYTES],
            maps: [
                [fill; WLD_MAP_BYTES],
                [0; WLD_MAP_BYTES],
                [0; WLD_MAP_BYTES],
                [0; WLD_MAP_BYTES],
            ],
        };

        let file = WldFile {
            header: WldHeader {
                fields: [0; WLD_HEADER_DWORDS],
            },
            sections: [section(10), section(20), section(30), section(40)],
            footer: [0; WLD_FOOTER_BYTES],
        };

        let stitched = file.combined_map(0).expect("combined map");
        let side = WLD_MAP_SIDE * 2;
        assert_eq!(stitched.len(), side * side);

        assert_eq!(stitched[0], 10);
        assert_eq!(stitched[WLD_MAP_SIDE], 20);
        assert_eq!(stitched[WLD_MAP_SIDE * side], 30);
        assert_eq!(stitched[(WLD_MAP_SIDE * side) + WLD_MAP_SIDE], 40);
    }

    #[test]
    fn parse_rejects_too_small_file() {
        let bytes = vec![0_u8; 32];
        let err = parse_wld_file(&bytes).expect_err("expected parse error");
        assert!(matches!(err, Error::Parse(_)));
    }

    #[test]
    fn combined_heightmap_strips_high_bit_without_scaling() {
        let mut section = WldSection {
            header: [0_u8; WLD_SECTION_HEADER_BYTES],
            maps: [[0; WLD_MAP_BYTES]; 4],
        };
        section.maps[0][0] = 0xFF;

        let file = WldFile {
            header: WldHeader {
                fields: [0; WLD_HEADER_DWORDS],
            },
            sections: [section.clone(), section.clone(), section.clone(), section],
            footer: [0; WLD_FOOTER_BYTES],
        };

        let luma = file
            .combined_heightmap_luma()
            .expect("combined heightmap luma");
        assert_eq!(luma[0], 0x7F);
    }

    #[test]
    fn map_output_paths_append_map_suffixes() {
        let paths = map_output_paths(Path::new("out/island_height.png"));
        assert_eq!(paths.map1_path, PathBuf::from("out/island_height_map1.png"));
        assert_eq!(paths.map2_path, PathBuf::from("out/island_height_map2.png"));
        assert_eq!(paths.map3_path, PathBuf::from("out/island_height_map3.png"));
        assert_eq!(paths.map4_path, PathBuf::from("out/island_height_map4.png"));
    }
}
