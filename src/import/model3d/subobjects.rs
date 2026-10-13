//! Authored collision face subsets; see docs/formats/models/3d.md.
use super::parser::parse_3d_header;
use crate::{Result, error::Error};
use nom::{
    IResult, Parser,
    number::complete::{le_f32, le_i32, le_u16, le_u32},
};

/// One authored subobject. Integer center and radius retain their file units.
#[derive(serde::Serialize, Debug, PartialEq)]
pub struct Subobject {
    pub center: [i32; 3],
    pub radius: u32,
    /// v5.0 only; v4.0 has no extent fields.
    pub extents: Option<[f32; 3]>,
    /// Original zero-based face indices, preserving order and duplicates.
    pub faces: Vec<u16>,
}

/// Decode collision subsets from a complete v4.0/v5.0 model file.
/// Face references are checked against both the index and original record offset.
/// Earlier formats have no Section4 and return an empty list.
pub fn parse_subobjects(bytes: &[u8]) -> Result<Vec<Subobject>> {
    let (_, header) = parse_3d_header(bytes)?;
    if header.is_v27_or_earlier() || header.section4_count == 0 {
        return Ok(Vec::new());
    }
    let version5 = header.version == *b"v5.0";
    if !version5 && header.version != *b"v4.0" {
        return Err(Error::Parse("model subobjects: unsupported version".into()));
    }
    let invalid =
        || Error::Parse("model subobjects: invalid offset, count or face reference".into());
    let mut face_offsets = Vec::new();
    let mut offset = usize::try_from(header.offset_face_data).map_err(|_| invalid())?;
    for _ in 0..header.num_faces {
        let count = usize::from(*bytes.get(offset).ok_or_else(invalid)?);
        face_offsets.push(offset);
        offset = offset.checked_add(10 + count * 8).ok_or_else(invalid)?;
        if offset > bytes.len() {
            return Err(invalid());
        }
    }
    if header.offset_section4 == 0 {
        return Err(invalid());
    }
    let mut input = bytes
        .get(usize::try_from(header.offset_section4).map_err(|_| invalid())?..)
        .ok_or_else(invalid)?;
    let mut nodes = Vec::new();
    for _ in 0..header.section4_count {
        let (rest, (x, y, z, radius, count)) = node_header(input)?;
        input = rest;
        let extents = if version5 {
            let (rest, (x, y, z)) = extent_fields(input)?;
            input = rest;
            if ![x, y, z].iter().all(|v| v.is_finite()) {
                return Err(invalid());
            }
            Some([x, y, z])
        } else {
            None
        };
        let mut faces = Vec::new();
        for _ in 0..count {
            let (rest, (offset, index)) = face_reference(input)?;
            input = rest;
            if index % 4 != 0
                || face_offsets.get(usize::from(index / 4)).copied() != usize::try_from(offset).ok()
            {
                return Err(invalid());
            }
            faces.push(index / 4);
        }
        nodes.push(Subobject {
            center: [x, y, z],
            radius,
            extents,
            faces,
        });
    }
    Ok(nodes)
}

fn node_header(input: &[u8]) -> IResult<&[u8], (i32, i32, i32, u32, u16)> {
    (le_i32, le_i32, le_i32, le_u32, le_u16).parse(input)
}
fn extent_fields(input: &[u8]) -> IResult<&[u8], (f32, f32, f32)> {
    (le_f32, le_f32, le_f32).parse(input)
}
fn face_reference(input: &[u8]) -> IResult<&[u8], (u32, u16)> {
    (le_u32, le_u16).parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(version5: bool) -> Vec<u8> {
        let mut bytes = vec![0; 74];
        bytes[..4].copy_from_slice(if version5 { b"v5.0" } else { b"v4.0" });
        for (offset, value) in [(8, 1u32), (28, 74), (32, 1), (60, 64)] {
            bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
        for value in [1i32, -2, 3, 4] {
            bytes.extend(value.to_le_bytes());
        }
        bytes.extend(2u16.to_le_bytes());
        if version5 {
            for value in [1.0f32, 2.0, 3.0] {
                bytes.extend(value.to_le_bytes());
            }
        }
        for _ in 0..2 {
            bytes.extend(64u32.to_le_bytes());
            bytes.extend(0u16.to_le_bytes());
        }
        bytes
    }

    #[test]
    fn versions_preserve_duplicate_references_and_reject_corruption() {
        for version5 in [false, true] {
            let mut bytes = fixture(version5);
            assert_eq!(
                parse_subobjects(&bytes).unwrap(),
                vec![Subobject {
                    center: [1, -2, 3],
                    radius: 4,
                    extents: version5.then_some([1.0, 2.0, 3.0]),
                    faces: vec![0, 0],
                }]
            );
            let last = bytes.len() - 6;
            bytes[last] = 65;
            assert!(parse_subobjects(&bytes).is_err());
            bytes[last] = 64;
            bytes.pop();
            assert!(parse_subobjects(&bytes).is_err());
        }
    }
    #[test]
    fn installed_subobjects_preserve_authored_face_subsets() {
        let Some(root) = std::env::var_os("REDGUARD_DIR") else {
            eprintln!("skipping subobject install check: REDGUARD_DIR absent");
            return;
        };
        let directory = std::path::PathBuf::from(root).join("fxart");
        let mut models = 0;
        let mut nodes = 0;
        let mut references = 0;
        let mut omitted = 0;
        for entry in std::fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if !path
                .extension()
                .is_some_and(|v| v.eq_ignore_ascii_case("rob"))
            {
                continue;
            }
            let archive =
                crate::import::rob::parse_rob_file(&std::fs::read(&path).unwrap()).unwrap();
            for segment in archive.segments {
                if !matches!(segment.data.get(..4), Some(b"v4.0" | b"v5.0")) {
                    continue;
                }
                let parsed = parse_subobjects(&segment.data).unwrap_or_else(|error| {
                    panic!("{} {}: {error}", path.display(), segment.name())
                });
                if parsed.is_empty() {
                    continue;
                }
                let (_, header) = parse_3d_header(&segment.data).unwrap();
                let unique: std::collections::BTreeSet<_> = parsed
                    .iter()
                    .flat_map(|n| n.faces.iter().copied())
                    .collect();
                omitted += usize::from(unique.len() < header.num_faces as usize);
                models += 1;
                nodes += parsed.len();
                references += parsed.iter().map(|n| n.faces.len()).sum::<usize>();
            }
        }
        assert_eq!(
            (models, nodes, references, omitted),
            (3820, 22931, 222322, 9)
        );
    }
}
