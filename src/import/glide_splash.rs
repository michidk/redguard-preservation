//! Authored splash geometry, transforms, and textures in the GOG Glide overlay.
//! This reads data tables, without executing or translating executable code.

use crate::error::Error;

#[derive(Debug, Clone)]
pub struct SplashVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    /// Glide texture coordinates, measured on a 256-unit square.
    pub uv: [f32; 2],
}

#[derive(Debug, Clone)]
pub struct SplashFace {
    pub vertices: [usize; 3],
    pub material: u32,
    pub antialiased_edges: u32,
}

#[derive(Debug, Clone)]
pub struct SplashMesh {
    pub vertices: Vec<SplashVertex>,
    pub faces: Vec<SplashFace>,
}

#[derive(Debug, Clone)]
pub struct SplashTexture {
    pub width: u32,
    pub height: u32,
    /// Largest mip level, in row-major RGBA order. Intensity textures have equal RGB channels and opaque alpha.
    pub rgba: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct GlideSplash {
    /// Shield, logo, and outer shield, in that order.
    pub meshes: [SplashMesh; 3],
    /// Column-major affine matrices, one per mesh in each authored frame.
    pub frames: Vec<[[f32; 16]; 3]>,
    /// Logo color, highlight intensity, and shadow intensity textures.
    pub textures: [SplashTexture; 3],
}

fn invalid(message: &str) -> Error {
    Error::Parse(format!("Glide splash: {message}"))
}

fn bytes(data: &[u8], start: usize, length: usize) -> Result<&[u8], Error> {
    data.get(
        start
            ..start
                .checked_add(length)
                .ok_or_else(|| invalid("range overflow"))?,
    )
    .ok_or_else(|| invalid("truncated data"))
}

fn word(data: &[u8], start: usize) -> Result<u32, Error> {
    Ok(u32::from_le_bytes(
        bytes(data, start, 4)?.try_into().unwrap(),
    ))
}

fn floats<const N: usize>(data: &[u8], start: usize) -> Result<[f32; N], Error> {
    let mut result = [0.0; N];
    for (i, value) in result.iter_mut().enumerate() {
        *value = f32::from_bits(word(data, start + i * 4)?);
        if !value.is_finite() {
            return Err(invalid("non-finite geometry or transform"));
        }
    }
    Ok(result)
}

fn unique(data: &[u8], words: &[u32]) -> Result<usize, Error> {
    let pattern: Vec<_> = words.iter().flat_map(|value| value.to_le_bytes()).collect();
    let mut matches = data
        .windows(pattern.len())
        .enumerate()
        .filter(|(_, bytes)| *bytes == pattern);
    let first = matches
        .next()
        .ok_or_else(|| invalid("unsupported data-table layout"))?
        .0;
    if matches.next().is_some() {
        return Err(invalid("ambiguous data-table layout"));
    }
    Ok(first)
}

/// Extracts the splash in the GOG `DOSBOX/glide2x_emu.ovl` LE overlay.
/// Other overlay revisions return an unsupported-layout error. Pointer values
/// are interpreted relative to their LE data object, never as host pointers.
pub fn parse_glide_splash(data: &[u8]) -> Result<GlideSplash, Error> {
    if bytes(data, 0, 2)? != b"MZ" {
        return Err(invalid("missing DOS header"));
    }
    let le = word(data, 60)? as usize;
    if bytes(data, le, 4)? != b"LE\0\0" {
        return Err(invalid("unsupported executable header"));
    }
    let page_size = word(data, le + 0x28)? as usize;
    let object_table = le + word(data, le + 0x40)? as usize;
    let object_count = word(data, le + 0x44)? as usize;
    let page_table = le + word(data, le + 0x48)? as usize;
    let file_pages = word(data, le + 0x80)? as usize;
    if !(1..=64).contains(&object_count) || page_size != 4096 {
        return Err(invalid("unsupported LE object/page layout"));
    }
    let mut candidates = Vec::new();
    for object in 0..object_count {
        let entry = object_table + object * 24;
        let length = word(data, entry)? as usize;
        let flags = word(data, entry + 8)?;
        if flags & 2 == 0 {
            continue;
        }
        if length > 8 * 1024 * 1024 {
            return Err(invalid("data object is too large"));
        }
        let first_page = word(data, entry + 12)? as usize;
        let page_count = word(data, entry + 16)? as usize;
        if first_page == 0 || page_count > length.div_ceil(page_size) {
            return Err(invalid("invalid data-object page range"));
        }
        let mut image = vec![0; length];
        for page in 0..page_count {
            let map = bytes(data, page_table + (first_page - 1 + page) * 4, 4)?;
            if map[3] != 0 {
                return Err(invalid("unsupported encoded LE page"));
            }
            let number =
                (usize::from(map[0]) << 16) | (usize::from(map[1]) << 8) | usize::from(map[2]);
            if number == 0 {
                return Err(invalid("invalid LE file-page number"));
            }
            let size = if number == word(data, le + 0x14)? as usize {
                word(data, le + 0x2c)? as usize
            } else {
                page_size
            };
            if size > page_size || page * page_size + size > image.len() {
                return Err(invalid("invalid LE page size"));
            }
            image[page * page_size..page * page_size + size].copy_from_slice(bytes(
                data,
                file_pages + (number - 1) * page_size,
                size,
            )?);
        }
        if let Ok(root) = unique(&image, &[68, 1694, 34]) {
            candidates.push((image, root));
        }
    }
    if candidates.len() != 1 {
        return Err(invalid("unsupported or ambiguous splash object"));
    }
    let (image, counts) = candidates.pop().unwrap();
    let root = counts
        .checked_sub(12)
        .ok_or_else(|| invalid("invalid mesh-table location"))?;
    if [
        word(&image, root + 36)?,
        word(&image, root + 40)?,
        word(&image, root + 44)?,
        word(&image, root + 48)?,
    ] != [100, 1606, 32, 75]
    {
        return Err(invalid("unsupported splash mesh/frame counts"));
    }
    let mut meshes = Vec::new();
    for mesh in 0..3 {
        let vertex_start = word(&image, root + mesh * 4)? as usize;
        let vertex_count = word(&image, root + 12 + mesh * 4)? as usize;
        let face_start = word(&image, root + 24 + mesh * 4)? as usize;
        let face_count = word(&image, root + 36 + mesh * 4)? as usize;
        let vertices = (0..vertex_count)
            .map(|i| {
                let values = floats::<8>(&image, vertex_start + i * 32)?;
                Ok(SplashVertex {
                    position: values[..3].try_into().unwrap(),
                    normal: values[3..6].try_into().unwrap(),
                    uv: values[6..].try_into().unwrap(),
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let faces = (0..face_count)
            .map(|i| {
                let start = face_start + i * 20;
                let vertices = [
                    word(&image, start)? as usize,
                    word(&image, start + 4)? as usize,
                    word(&image, start + 8)? as usize,
                ];
                let material = word(&image, start + 12)?;
                let antialiased_edges = word(&image, start + 16)?;
                if vertices.iter().any(|index| *index >= vertex_count)
                    || material > 4
                    || antialiased_edges > 7
                {
                    return Err(invalid("invalid face index, material, or edge flags"));
                }
                Ok(SplashFace {
                    vertices,
                    material,
                    antialiased_edges,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        meshes.push(SplashMesh { vertices, faces });
    }
    let frames = (0..75)
        .map(|frame| {
            let start = root + 52 + frame * 192;
            Ok([
                floats(&image, start)?,
                floats(&image, start + 64)?,
                floats(&image, start + 128)?,
            ])
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let textures = [
        texture(&image, [64, 64, 8, 2, 3, 1])?,
        texture(&image, [32, 32, 8, 3, 3, 3])?,
        texture(&image, [64, 32, 8, 2, 2, 3])?,
    ];
    Ok(GlideSplash {
        meshes: meshes.try_into().unwrap(),
        frames,
        textures,
    })
}

fn texture(data: &[u8], header: [u32; 6]) -> Result<SplashTexture, Error> {
    let start = unique(data, &header)?;
    let [width, height, _, _, _, format] = header;
    let mip_size = word(data, start + 1052)? as usize;
    if mip_size < (width * height) as usize || mip_size > 64 * 1024 {
        return Err(invalid("invalid texture mip-chain size"));
    }
    let pixels = bytes(data, start + 1056, mip_size)?;
    let table = bytes(data, start + 24, 64)?;
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for &pixel in &pixels[..(width * height) as usize] {
        if format == 3 {
            rgba.extend([pixel, pixel, pixel, 255]);
        } else {
            for channel in 0..3 {
                let i = 16 + (usize::from((pixel >> 2) & 3) * 3 + channel) * 2;
                let q = 40 + (usize::from(pixel & 3) * 3 + channel) * 2;
                let component = |offset| {
                    let bits = u16::from_le_bytes(table[offset..offset + 2].try_into().unwrap());
                    i32::from(bits & 255) - i32::from(bits & 256)
                };
                let value = i32::from(table[usize::from(pixel >> 4)]) + component(i) + component(q);
                rgba.push(value.clamp(0, 255) as u8);
            }
            rgba.push(255);
        }
    }
    Ok(SplashTexture {
        width,
        height,
        rgba,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yiq_sign_extends_nine_bit_color_components() {
        let header = [64, 64, 8, 2, 3, 1];
        let mut data = vec![0; 1056 + 4096];
        for (i, value) in header.into_iter().enumerate() {
            data[i * 4..i * 4 + 4].copy_from_slice(&u32::to_le_bytes(value));
        }
        data[24 + 8] = 128;
        for (start, vector) in [
            (24 + 16 + 6, [502u16, 20, 30]),
            (24 + 40 + 12, [5, 507, 492]),
        ] {
            for (channel, value) in vector.into_iter().enumerate() {
                data[start + channel * 2..start + channel * 2 + 2]
                    .copy_from_slice(&value.to_le_bytes());
            }
        }
        data[1052..1056].copy_from_slice(&4096u32.to_le_bytes());
        data[1056] = (8 << 4) | (1 << 2) | 2;
        assert_eq!(
            &texture(&data, header).unwrap().rgba[..4],
            &[123, 143, 138, 255]
        );
    }

    #[test]
    fn intensity_is_opaque_grayscale() {
        let header = [32, 32, 8, 3, 3, 3];
        let mut data = vec![0; 1056 + 1024];
        for (i, value) in header.into_iter().enumerate() {
            data[i * 4..i * 4 + 4].copy_from_slice(&u32::to_le_bytes(value));
        }
        data[1052..1056].copy_from_slice(&1024u32.to_le_bytes());
        data[1056..1060].copy_from_slice(&[0, 64, 128, 255]);
        let decoded = texture(&data, header).unwrap();
        assert_eq!(
            &decoded.rgba[..16],
            &[
                0, 0, 0, 255, 64, 64, 64, 255, 128, 128, 128, 255, 255, 255, 255, 255
            ]
        );
    }
}
