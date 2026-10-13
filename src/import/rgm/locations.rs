//! Actor-local authored locations from the RAHD count and RALC offset.
use super::{RgmFile, RgmSection, shared::read_i32_le};

/// Read an actor header's authored Q8 offsets, excluding runtime home location 0.
/// Header indices use authored RAHD order. The original rebases nonzero offsets
/// to a 12-byte boundary by signed division; invalid counts or ranges are errors.
/// See `docs/formats/RGM.md#ralc-location-data`.
pub fn actor_location_offsets(rgm: &RgmFile, header_index: usize) -> Result<Vec<[i32; 3]>, String> {
    let context = || format!("RAHD actor {header_index} RALC locations");
    let headers = rgm
        .sections
        .iter()
        .find_map(|section| match section {
            RgmSection::Rahd(_, data) => Some(data.as_slice()),
            _ => None,
        })
        .ok_or_else(|| format!("{}: RAHD section missing", context()))?;
    let count =
        read_i32_le(headers, 0).ok_or_else(|| format!("{}: RAHD count missing", context()))?;
    if count < 0 || header_index >= usize::try_from(count).unwrap_or(0) {
        return Err(format!(
            "{}: header index outside RAHD count {count}",
            context()
        ));
    }
    let start = header_index
        .checked_mul(165)
        .and_then(|v| v.checked_add(8))
        .ok_or_else(|| format!("{}: header offset overflow", context()))?;
    let record = headers
        .get(start..start.saturating_add(165))
        .ok_or_else(|| format!("{}: truncated RAHD record", context()))?;
    let count = read_i32_le(record, 0x61)
        .ok_or_else(|| format!("{}: location count missing", context()))?;
    if count == 0 {
        return Ok(Vec::new());
    }
    let count = usize::try_from(count)
        .map_err(|_| format!("{}: negative location count {count}", context()))?;
    let offset = read_i32_le(record, 0x69)
        .ok_or_else(|| format!("{}: location offset missing", context()))?;
    if offset <= 0 {
        return Err(format!(
            "{}: nonpositive location offset {offset}",
            context()
        ));
    }
    let offset = usize::try_from(offset / 12)
        .map_err(|_| format!("{}: location offset invalid", context()))?
        * 12;
    let end = count
        .checked_mul(12)
        .and_then(|length| offset.checked_add(length))
        .ok_or_else(|| format!("{}: location range overflow", context()))?;
    let locations = rgm
        .sections
        .iter()
        .find_map(|section| match section {
            RgmSection::Ralc(_, data) => Some(data.as_slice()),
            _ => None,
        })
        .ok_or_else(|| format!("{}: RALC section missing", context()))?;
    let data = locations.get(offset..end).ok_or_else(|| {
        format!(
            "{}: range {offset}..{end} exceeds RALC size {}",
            context(),
            locations.len()
        )
    })?;
    Ok(data
        .as_chunks::<12>()
        .0
        .iter()
        .map(|entry| {
            std::array::from_fn(|axis| {
                let at = axis * 4;
                i32::from_le_bytes([entry[at], entry[at + 1], entry[at + 2], entry[at + 3]])
            })
        })
        .collect())
}
