//! Authored Windows Glide splash data, without executing the player's DLL.
//! The resource and playback contracts are documented in formats/GLIDE-SPLASH-DLL.md.

use super::glide_splash::{SplashFace, SplashMesh, SplashTexture, SplashVertex};
use crate::error::Error;

/// Installed Windows splash, with timed frames decoded from its command stream.
#[derive(Debug, Clone)]
pub struct WindowsGlideSplash {
    pub frames_per_second: u32,
    pub meshes: Vec<SplashMesh>,
    /// Geometry flag bit 3 selects sphere mapping.
    pub sphere_mapped: Vec<bool>,
    pub textures: Vec<SplashTexture>,
    pub frames: Vec<Vec<WindowsSplashDraw>>,
    /// Untimed final redraws before the fade-out command.
    pub final_draws: Vec<WindowsSplashDraw>,
}

/// Complete authored inputs for one draw. Matrices are column-major; FOV is radians.
#[derive(Debug, Clone)]
pub struct WindowsSplashDraw {
    pub mesh: usize,
    pub texture: usize,
    pub transform: [f32; 16],
    pub camera: [f32; 16],
    pub horizontal_fov: f32,
    pub light_color: [f32; 3],
    pub light_position: [f32; 3],
    pub alpha: u8,
}

fn invalid(message: &str) -> Error {
    Error::Parse(format!("Windows Glide splash: {message}"))
}

struct Reader<'a> {
    data: &'a [u8],
    position: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, length: usize) -> Result<&'a [u8], Error> {
        let end = self
            .position
            .checked_add(length)
            .ok_or_else(|| invalid("range overflow"))?;
        let result = self
            .data
            .get(self.position..end)
            .ok_or_else(|| invalid("truncated data"))?;
        self.position = end;
        Ok(result)
    }

    fn word(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn count(&mut self, size: usize) -> Result<usize, Error> {
        let count = self.word()? as usize;
        if count > self.data.len().saturating_sub(self.position) / size {
            return Err(invalid("count exceeds remaining resource"));
        }
        Ok(count)
    }

    fn floats<const N: usize>(&mut self) -> Result<[f32; N], Error> {
        let mut result = [0.0; N];
        for value in &mut result {
            *value = f32::from_bits(self.word()?);
            if !value.is_finite() {
                return Err(invalid("non-finite authored value"));
            }
        }
        Ok(result)
    }
}

fn at_word(data: &[u8], position: usize) -> Result<u32, Error> {
    Reader { data, position }.word()
}

/// Extracts the unique ANIM/101 resource from a PE32 DLL, through its section map.
pub fn parse_windows_glide_splash(data: &[u8]) -> Result<WindowsGlideSplash, Error> {
    if data.get(..2) != Some(b"MZ") {
        return Err(invalid("missing DOS header"));
    }
    let pe = at_word(data, 60)? as usize;
    if data.get(pe..pe.saturating_add(4)) != Some(b"PE\0\0") {
        return Err(invalid("missing PE header"));
    }
    let mut header = Reader {
        data,
        position: pe + 6,
    };
    let sections = u16::from_le_bytes(header.take(2)?.try_into().unwrap()) as usize;
    header.position = pe + 20;
    let optional_size = u16::from_le_bytes(header.take(2)?.try_into().unwrap()) as usize;
    let optional = pe + 24;
    if data.get(optional..optional + 2) != Some(&[0x0b, 0x01]) || optional_size < 120 {
        return Err(invalid("unsupported PE optional header"));
    }
    let table = optional + optional_size;
    let rva_to_file = |rva: u32, length: usize| -> Result<usize, Error> {
        for section in 0..sections {
            let entry = table + section * 40;
            let virtual_start = at_word(data, entry + 12)?;
            let raw_size = at_word(data, entry + 16)? as usize;
            let raw_start = at_word(data, entry + 20)? as usize;
            if let Some(delta) = rva.checked_sub(virtual_start)
                && (delta as usize)
                    .checked_add(length)
                    .is_some_and(|end| end <= raw_size)
            {
                let start = raw_start
                    .checked_add(delta as usize)
                    .ok_or_else(|| invalid("section overflow"))?;
                Reader {
                    data,
                    position: start,
                }
                .take(length)?;
                return Ok(start);
            }
        }
        Err(invalid("resource RVA is outside initialized sections"))
    };
    let resource_rva = at_word(data, optional + 112)?;
    let resource_size = at_word(data, optional + 116)? as usize;
    let start = rva_to_file(resource_rva, resource_size)?;
    let resource = &data[start..start + resource_size];
    let entries = |offset: usize| -> Result<Vec<(u32, u32)>, Error> {
        let mut r = Reader {
            data: resource,
            position: offset + 12,
        };
        let counts = r.take(4)?;
        let count = usize::from(u16::from_le_bytes(counts[..2].try_into().unwrap()))
            + usize::from(u16::from_le_bytes(counts[2..].try_into().unwrap()));
        r.take(count * 8)?
            .chunks_exact(8)
            .map(|entry| Ok((at_word(entry, 0)?, at_word(entry, 4)?)))
            .collect()
    };
    let mut matches = Vec::new();
    for (name, directory) in entries(0)? {
        if name & 0x80000000 == 0 {
            continue;
        }
        let mut name_reader = Reader {
            data: resource,
            position: (name & 0x7fffffff) as usize,
        };
        let length = u16::from_le_bytes(name_reader.take(2)?.try_into().unwrap()) as usize;
        let name = name_reader.take(length * 2)?;
        if name != [b'A', 0, b'N', 0, b'I', 0, b'M', 0] {
            continue;
        }
        if directory & 0x80000000 == 0 {
            return Err(invalid("ANIM is not a resource directory"));
        }
        for (id, languages) in entries((directory & 0x7fffffff) as usize)? {
            if id != 101 {
                continue;
            }
            if languages & 0x80000000 == 0 {
                return Err(invalid("ANIM/101 has no language directory"));
            }
            for (_, leaf) in entries((languages & 0x7fffffff) as usize)? {
                if leaf & 0x80000000 != 0 {
                    return Err(invalid("unexpected resource nesting"));
                }
                let rva = at_word(resource, leaf as usize)?;
                let length = at_word(resource, leaf as usize + 4)? as usize;
                let start = rva_to_file(rva, length)?;
                matches.push(&data[start..start + length]);
            }
        }
    }
    let [resource] = matches.as_slice() else {
        return Err(invalid("missing or ambiguous ANIM/101 resource"));
    };
    parse_animation(resource)
}

fn parse_animation(data: &[u8]) -> Result<WindowsGlideSplash, Error> {
    let mut r = Reader { data, position: 0 };
    if r.take(24)? != b"3Dfx Splash Version 1.2\n" {
        return Err(invalid("unsupported ANIM version"));
    }
    let frame_count = r.word()? as usize;
    let frames_per_second = r.word()?;
    if frame_count != 90 || frames_per_second != 30 {
        return Err(invalid("unsupported animation cadence"));
    }
    let mut textures = Vec::new();
    for _ in 0..r.count(24)? {
        let small = r.word()?;
        let large = r.word()?;
        let aspect = r.word()?;
        let format = r.word()?;
        r.word()?;
        if small != 8 || large > 8 || aspect != 3 || format != 10 {
            return Err(invalid("unsupported texture descriptor"));
        }
        let side = 256u32 >> large;
        let length = r.count(1)?;
        let pixels = r.take(length)?;
        if length < (side * side * 2) as usize {
            return Err(invalid("truncated texture mip chain"));
        }
        let rgba = pixels[..(side * side * 2) as usize]
            .chunks_exact(2)
            .flat_map(|p| {
                let p = u16::from_le_bytes(p.try_into().unwrap());
                [
                    ((p >> 11) * 255 / 31) as u8,
                    (((p >> 5) & 63) * 255 / 63) as u8,
                    ((p & 31) * 255 / 31) as u8,
                    255,
                ]
            })
            .collect();
        textures.push(SplashTexture {
            width: side,
            height: side,
            rgba,
        });
    }
    let mut materials = Vec::new();
    for _ in 0..r.count(4)? {
        if r.word()? != 1 {
            return Err(invalid("unsupported material group"));
        }
        r.floats::<6>()?;
        let texture = r.word()? as usize;
        if texture >= textures.len() {
            return Err(invalid("material texture index is out of range"));
        }
        materials.push(texture);
    }
    let slots = r.count(4)?;
    let mut meshes = vec![None; slots];
    let mut sphere_mapped = vec![false; slots];
    loop {
        let slot = r.word()?;
        if slot == u32::MAX {
            break;
        }
        let slot = slot as usize;
        if slot >= slots || meshes[slot].is_some() {
            return Err(invalid("invalid or repeated geometry slot"));
        }
        let flags = r.word()?;
        if flags & !8 != 0 {
            return Err(invalid("unsupported geometry flags"));
        }
        let mut faces = Vec::new();
        for _ in 0..r.count(12)? {
            faces.push(SplashFace {
                vertices: [r.word()? as usize, r.word()? as usize, r.word()? as usize],
                material: 0,
                antialiased_edges: 7,
            });
        }
        let mut vertices = Vec::new();
        for _ in 0..r.count(24)? {
            let v = r.floats::<6>()?;
            if v[3..].iter().all(|value| *value == 0.0) {
                return Err(invalid("zero vertex normal"));
            }
            vertices.push(SplashVertex {
                position: v[..3].try_into().unwrap(),
                normal: v[3..].try_into().unwrap(),
                uv: [0.0; 2],
            });
        }
        if faces
            .iter()
            .any(|face| face.vertices.iter().any(|index| *index >= vertices.len()))
        {
            return Err(invalid("triangle vertex index is out of range"));
        }
        for index in 0..r.count(8)? {
            let uv = r.floats()?;
            if let Some(vertex) = vertices.get_mut(index) {
                vertex.uv = uv;
            }
        }
        meshes[slot] = Some(SplashMesh { vertices, faces });
        sphere_mapped[slot] = flags & 8 != 0;
    }
    let meshes = meshes
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| invalid("unfilled geometry slot"))?;
    let mut frames = Vec::new();
    let mut draws = Vec::new();
    let mut camera = None;
    let mut fov = 0.0;
    let mut texture = None;
    let mut light_color = [0.0; 3];
    let mut light_position = [0.0; 3];
    let mut alpha = 255;
    let mut finished = false;
    while r.position < data.len() {
        match r.take(1)?[0] {
            2 => frames.push(std::mem::take(&mut draws)),
            3 => {
                let mesh = r.word()? as usize;
                r.word()?;
                let transform = r.floats()?;
                if mesh >= meshes.len() {
                    return Err(invalid("draw geometry index is out of range"));
                }
                draws.push(WindowsSplashDraw {
                    mesh,
                    texture: texture.ok_or_else(|| invalid("draw has no material"))?,
                    transform,
                    camera: camera.ok_or_else(|| invalid("draw has no camera"))?,
                    horizontal_fov: fov,
                    light_color,
                    light_position,
                    alpha,
                });
            }
            4 => {
                fov = r.floats::<1>()?[0];
                if !(0.0..std::f32::consts::PI).contains(&fov) || fov == 0.0 {
                    return Err(invalid("invalid camera field of view"));
                }
                camera = Some(r.floats()?);
            }
            5 => {
                finished = true;
                break;
            }
            6 => {
                let group = r.word()? as usize;
                texture = Some(
                    *materials
                        .get(group)
                        .ok_or_else(|| invalid("material group index is out of range"))?,
                );
            }
            7 => {
                let v = r.floats::<7>()?;
                if v[..3] != [0.0; 3] {
                    return Err(invalid("unsupported directional light color"));
                }
            }
            8 => {
                if r.take(1)? != [1] {
                    return Err(invalid("unsupported auxiliary toggle"));
                }
            }
            9 => {
                let value = r.word()?;
                alpha = if value == 0 || value == 0xff000000 {
                    255
                } else {
                    (value >> 24) as u8
                };
            }
            11 | 12 | 13 => {}
            14 => {
                let v = r.floats::<7>()?;
                light_color = v[..3].try_into().unwrap();
                light_position = v[4..].try_into().unwrap();
            }
            _ => return Err(invalid("unsupported animation command")),
        }
    }
    if !finished || r.position != data.len() || frames.len() != frame_count {
        return Err(invalid("incomplete animation command stream"));
    }
    Ok(WindowsGlideSplash {
        frames_per_second,
        meshes,
        sphere_mapped,
        textures,
        frames,
        final_draws: draws,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn animation() -> Vec<u8> {
        let mut bytes = b"3Dfx Splash Version 1.2\n".to_vec();
        for value in [90u32, 30, 1, 8, 8, 3, 10, 0, 2] {
            bytes.extend(value.to_le_bytes());
        }
        bytes.extend(0xf800u16.to_le_bytes());
        for value in [1u32, 1] {
            bytes.extend(value.to_le_bytes());
        }
        for value in [1.0f32; 6] {
            bytes.extend(value.to_le_bytes());
        }
        for value in [0u32, 1, 0, 0, 1, 0, 1, 2, 3] {
            bytes.extend(value.to_le_bytes());
        }
        for position in [[0.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]] {
            for value in position.into_iter().chain([0.0, 0.0, 1.0]) {
                bytes.extend(value.to_le_bytes());
            }
        }
        bytes.extend(3u32.to_le_bytes());
        for value in [0.0f32, 0.0, 1.0, 0.0, 0.0, 1.0] {
            bytes.extend(value.to_le_bytes());
        }
        bytes.extend(u32::MAX.to_le_bytes());
        bytes.push(4);
        bytes.extend(1.0f32.to_le_bytes());
        for value in [
            1.0f32, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ] {
            bytes.extend(value.to_le_bytes());
        }
        bytes.push(6);
        bytes.extend(0u32.to_le_bytes());
        bytes.push(3);
        bytes.extend([0u8; 8]);
        for value in [
            1.0f32, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ] {
            bytes.extend(value.to_le_bytes());
        }
        bytes.extend([2u8; 90]);
        bytes.push(5);
        bytes
    }

    #[test]
    fn decodes_authored_triangle_texture_and_draw() {
        let splash = parse_animation(&animation()).unwrap();
        assert_eq!(splash.textures[0].rgba, [255, 0, 0, 255]);
        assert_eq!(splash.meshes[0].faces[0].vertices, [0, 1, 2]);
        assert_eq!(splash.meshes[0].vertices[2].position, [0.0, 1.0, 0.0]);
        assert_eq!(splash.meshes[0].vertices[2].uv, [0.0, 1.0]);
        assert_eq!(splash.frames[0][0].mesh, 0);
        assert_eq!(splash.frames[0][0].horizontal_fov, 1.0);
        assert_eq!(splash.frames.len(), 90);
    }

    #[test]
    fn rejects_every_truncation_of_an_animation() {
        let bytes = animation();
        for length in 0..bytes.len() {
            assert!(
                parse_animation(&bytes[..length]).is_err(),
                "length {length}"
            );
        }
    }
}
