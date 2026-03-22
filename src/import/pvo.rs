use crate::{Result, error::Error};

/// Section header shared by all PVO sections: 4-byte ASCII tag + 4-byte BE data length.
#[derive(Debug, Clone, Copy)]
pub struct PvoSectionHeader {
    pub tag: [u8; 4],
    pub data_length: u32,
}

/// OCTH section — file header (52 bytes payload).
#[derive(Debug, Clone)]
pub struct PvoHeader {
    /// Maximum octree depth (always 10 in shipped files).
    pub depth: u32,
    /// Total octree node count. Equals `leaf_nodes + interior_nodes`.
    pub total_nodes: u32,
    /// Leaf node count.
    pub leaf_nodes: u32,
    /// Total entries in the MLST polygon index table.
    /// Invariant: `mlst_polygon_count * 2 == mlst section data_length`.
    pub mlst_polygon_count: u32,
    /// Always 0 in shipped files.
    pub reserved_18: u32,
    /// Root cell half-extent (power of 2: 16384 or 8192).
    pub cell_size: u32,
    /// Octree root center X coordinate.
    pub center_x: i32,
    /// Octree root center Y coordinate.
    pub center_y: i32,
    /// Octree root center Z coordinate.
    pub center_z: i32,
    /// Always zero (4 × u32) in shipped files.
    pub reserved_2c: [u32; 4],
}

impl PvoHeader {
    /// Number of interior-only nodes (nodes with children but no leaf data).
    #[must_use]
    pub const fn interior_nodes(&self) -> u32 {
        self.total_nodes.saturating_sub(self.leaf_nodes)
    }
}

/// Special value for `OctrNode::leaf_ref` indicating no leaf data (interior-only node).
pub const LEAF_REF_NONE: u32 = 0xFFFF_FFFF;

/// Special value for child references indicating an empty-child sentinel.
pub const CHILD_REF_EMPTY: u32 = 0xFFFF_FFFE;

/// OCTR node record. Variable-length: `5 + popcount(child_mask) * 4` bytes.
///
/// Octant bit assignment:
/// - bit `0` (`1`) = `z > center_z`
/// - bit `1` (`2`) = `y > center_y`
/// - bit `2` (`4`) = `x > center_x`
#[derive(Debug, Clone)]
pub struct OctrNode {
    /// Position within the `OCTR` section data (used by `child_ref` lookups).
    pub byte_offset: usize,
    /// Bit *i* set = octant *i* child present.
    pub child_mask: u8,
    /// Byte offset into PLST section. `0xFFFFFFFF` = no leaf data.
    pub leaf_ref: u32,
    /// `(octant_index, byte_offset_into_OCTR)` per set bit, low bit first.
    /// `0xFFFFFFFE` byte offset = empty-child sentinel.
    pub child_refs: Vec<(u8, u32)>,
}

impl OctrNode {
    /// Returns `true` when this node has no PLST leaf reference.
    #[must_use]
    pub const fn is_interior(&self) -> bool {
        self.leaf_ref == LEAF_REF_NONE
    }

    /// Returns `true` when this node has no children.
    #[must_use]
    pub const fn is_leaf(&self) -> bool {
        self.child_mask == 0
    }

    /// Returns the number of present children from `child_mask` bits.
    #[must_use]
    pub const fn child_count(&self) -> u32 {
        self.child_mask.count_ones()
    }

    /// Returns the encoded byte size of this OCTR node record.
    #[must_use]
    pub fn record_size(&self) -> usize {
        5 + usize::try_from(self.child_mask.count_ones()).unwrap_or(0) * 4
    }
}

#[derive(Debug, Clone)]
/// One PLST entry referencing a run of MLST polygon indices.
pub struct PlstEntry {
    /// Number of polygon indices in this sub-list.
    pub count: u16,
    /// Starting index into the MLST array.
    pub mlst_start: u32,
}

#[derive(Debug, Clone)]
/// One PLST leaf record containing one or more `PlstEntry` values.
pub struct PlstLeaf {
    /// Position within the `PLST` section data (used by `leaf_ref` lookups).
    pub byte_offset: usize,
    pub entries: Vec<PlstEntry>,
}

#[derive(Debug, Clone)]
/// Parsed PVO file with octree nodes, leaf lists, and MLST indices.
pub struct PvoFile {
    pub header: PvoHeader,
    pub octr_nodes: Vec<OctrNode>,
    pub plst_leaves: Vec<PlstLeaf>,
    pub mlst_indices: Vec<u16>,
}

impl PvoFile {
    /// Counts OCTR nodes marked as interior-only nodes.
    #[must_use]
    pub fn count_interior_nodes(&self) -> usize {
        self.octr_nodes.iter().filter(|n| n.is_interior()).count()
    }

    /// Counts OCTR nodes that have no child nodes.
    #[must_use]
    pub fn count_leaf_nodes(&self) -> usize {
        self.octr_nodes.iter().filter(|n| n.is_leaf()).count()
    }

    /// Map an `MLST` index to an `MPSO` record index.
    /// Indices `0..mpso_count-1` map directly to `MPSO` records.
    /// Indices `>= mpso_count` belong to a secondary table and return `None`.
    #[must_use]
    pub const fn mlst_index_to_mpso(index: u16, mpso_count: u16) -> Option<u16> {
        if index < mpso_count {
            Some(index)
        } else {
            None
        }
    }
}

fn read_u32_le(data: &[u8], offset: usize) -> Result<u32> {
    if offset + 4 > data.len() {
        return Err(Error::Parse(format!(
            "PVO read out of bounds at offset 0x{offset:X}"
        )));
    }
    Ok(u32::from_le_bytes(
        data[offset..offset + 4]
            .try_into()
            .map_err(|_| Error::Parse(format!("PVO failed to parse LE u32 at 0x{offset:X}")))?,
    ))
}

fn read_u32_be(data: &[u8], offset: usize) -> Result<u32> {
    if offset + 4 > data.len() {
        return Err(Error::Parse(format!(
            "PVO read out of bounds at offset 0x{offset:X}"
        )));
    }
    Ok(u32::from_be_bytes(
        data[offset..offset + 4]
            .try_into()
            .map_err(|_| Error::Parse(format!("PVO failed to parse BE u32 at 0x{offset:X}")))?,
    ))
}

fn read_u16_le(data: &[u8], offset: usize) -> Result<u16> {
    if offset + 2 > data.len() {
        return Err(Error::Parse(format!(
            "PVO read out of bounds at offset 0x{offset:X}"
        )));
    }
    Ok(u16::from_le_bytes(
        data[offset..offset + 2]
            .try_into()
            .map_err(|_| Error::Parse(format!("PVO failed to parse LE u16 at 0x{offset:X}")))?,
    ))
}

fn read_i32_le(data: &[u8], offset: usize) -> Result<i32> {
    let value = read_u32_le(data, offset)?;
    Ok(i32::from_le_bytes(value.to_le_bytes()))
}

fn read_tag(data: &[u8], offset: usize) -> Result<[u8; 4]> {
    if offset + 4 > data.len() {
        return Err(Error::Parse(format!(
            "PVO read out of bounds at offset 0x{offset:X} for tag"
        )));
    }
    let mut tag = [0u8; 4];
    tag.copy_from_slice(&data[offset..offset + 4]);
    Ok(tag)
}

fn read_section_header(data: &[u8], offset: usize) -> Result<PvoSectionHeader> {
    let tag = read_tag(data, offset)?;
    let data_length = read_u32_be(data, offset + 4)?;
    Ok(PvoSectionHeader { tag, data_length })
}

fn parse_header_payload(data: &[u8], offset: usize) -> Result<PvoHeader> {
    if offset + 52 > data.len() {
        return Err(Error::Parse(format!(
            "PVO OCTH payload too short at 0x{offset:X}"
        )));
    }

    Ok(PvoHeader {
        depth: read_u32_le(data, offset)?,
        total_nodes: read_u32_le(data, offset + 0x04)?,
        leaf_nodes: read_u32_le(data, offset + 0x08)?,
        mlst_polygon_count: read_u32_le(data, offset + 0x0C)?,
        reserved_18: read_u32_le(data, offset + 0x10)?,
        cell_size: read_u32_le(data, offset + 0x14)?,
        center_x: read_i32_le(data, offset + 0x18)?,
        center_y: read_i32_le(data, offset + 0x1C)?,
        center_z: read_i32_le(data, offset + 0x20)?,
        reserved_2c: [
            read_u32_le(data, offset + 0x24)?,
            read_u32_le(data, offset + 0x28)?,
            read_u32_le(data, offset + 0x2C)?,
            read_u32_le(data, offset + 0x30)?,
        ],
    })
}

fn parse_octr_nodes(payload: &[u8], expected_count: u32) -> Result<Vec<OctrNode>> {
    let mut nodes = Vec::with_capacity(expected_count as usize);
    let mut cursor = 0usize;

    while cursor < payload.len() {
        let node_offset = cursor;

        if cursor + 5 > payload.len() {
            return Err(Error::Parse(format!(
                "PVO OCTR truncated node at offset 0x{cursor:X} \
                 (need 5 bytes, have {})",
                payload.len() - cursor
            )));
        }

        let child_mask = payload[cursor];
        cursor += 1;

        let leaf_ref = u32::from_le_bytes(
            payload[cursor..cursor + 4]
                .try_into()
                .map_err(|_| Error::Parse("PVO OCTR leaf_ref slice".to_string()))?,
        );
        cursor += 4;

        let n_children = child_mask.count_ones() as usize;
        let children_bytes = n_children * 4;
        if cursor + children_bytes > payload.len() {
            return Err(Error::Parse(format!(
                "PVO OCTR truncated child_refs at offset 0x{cursor:X} \
                 (need {children_bytes} bytes for {n_children} children, have {})",
                payload.len() - cursor
            )));
        }

        let mut child_refs = Vec::with_capacity(n_children);
        for octant in 0u8..8 {
            if child_mask & (1 << octant) != 0 {
                let ref_offset = u32::from_le_bytes(
                    payload[cursor..cursor + 4]
                        .try_into()
                        .map_err(|_| Error::Parse("PVO OCTR child_ref slice".to_string()))?,
                );
                child_refs.push((octant, ref_offset));
                cursor += 4;
            }
        }

        nodes.push(OctrNode {
            byte_offset: node_offset,
            child_mask,
            leaf_ref,
            child_refs,
        });
    }

    if nodes.len() != expected_count as usize {
        return Err(Error::Parse(format!(
            "PVO OCTR node count mismatch: parsed {} nodes, header says {}",
            nodes.len(),
            expected_count
        )));
    }

    Ok(nodes)
}

fn parse_plst_leaves(payload: &[u8]) -> Result<Vec<PlstLeaf>> {
    let mut leaves = Vec::new();
    let mut cursor = 0usize;

    while cursor < payload.len() {
        let leaf_offset = cursor;

        if cursor + 1 > payload.len() {
            return Err(Error::Parse(format!(
                "PVO PLST truncated entry_count at offset 0x{cursor:X}"
            )));
        }
        let entry_count = payload[cursor] as usize;
        cursor += 1;

        let entry_bytes = entry_count * 6;
        if cursor + entry_bytes > payload.len() {
            return Err(Error::Parse(format!(
                "PVO PLST truncated entries at offset 0x{cursor:X} \
                 (need {entry_bytes} bytes for {entry_count} entries, have {})",
                payload.len() - cursor
            )));
        }

        let mut entries = Vec::with_capacity(entry_count);
        for _ in 0..entry_count {
            let count = read_u16_le(payload, cursor)?;
            let mlst_start = read_u32_le(payload, cursor + 2)?;
            entries.push(PlstEntry { count, mlst_start });
            cursor += 6;
        }

        leaves.push(PlstLeaf {
            byte_offset: leaf_offset,
            entries,
        });
    }

    Ok(leaves)
}

fn parse_mlst_indices(payload: &[u8]) -> Result<Vec<u16>> {
    if !payload.len().is_multiple_of(2) {
        return Err(Error::Parse(format!(
            "PVO MLST payload size {} is not a multiple of 2",
            payload.len()
        )));
    }
    let count = payload.len() / 2;
    let mut indices = Vec::with_capacity(count);
    for i in 0..count {
        indices.push(read_u16_le(payload, i * 2)?);
    }
    Ok(indices)
}

fn read_section_payload<'a>(
    data: &'a [u8],
    cursor: &mut usize,
    expected_tag: [u8; 4],
) -> Result<&'a [u8]> {
    let section_header = read_section_header(data, *cursor)?;
    if section_header.tag != expected_tag {
        return Err(Error::Parse(format!(
            "PVO expected {} section, got '{}'",
            String::from_utf8_lossy(&expected_tag),
            String::from_utf8_lossy(&section_header.tag)
        )));
    }

    *cursor = cursor.saturating_add(8);
    let data_length = usize::try_from(section_header.data_length)
        .map_err(|e| Error::Parse(format!("PVO section length does not fit usize: {e}")))?;
    let payload_end = (*cursor)
        .checked_add(data_length)
        .ok_or_else(|| Error::Parse("PVO section payload end overflow".to_string()))?;
    if payload_end > data.len() {
        return Err(Error::Parse(format!(
            "PVO {} section extends past end of file (section ends at 0x{payload_end:X}, file is 0x{:X} bytes)",
            String::from_utf8_lossy(&expected_tag),
            data.len()
        )));
    }

    let payload = &data[*cursor..payload_end];
    *cursor = payload_end;
    Ok(payload)
}

fn parse_octh_section(data: &[u8], cursor: &mut usize) -> Result<PvoHeader> {
    let section_header = read_section_header(data, *cursor)?;
    if &section_header.tag != b"OCTH" {
        return Err(Error::Parse(format!(
            "PVO expected OCTH section, got '{}'",
            String::from_utf8_lossy(&section_header.tag)
        )));
    }
    if section_header.data_length != 52 {
        return Err(Error::Parse(format!(
            "PVO OCTH data_length expected 52 (0x34), got {} (0x{:X})",
            section_header.data_length, section_header.data_length
        )));
    }

    *cursor = cursor.saturating_add(8);
    let header = parse_header_payload(data, *cursor)?;
    *cursor = cursor.saturating_add(52);
    Ok(header)
}

fn validate_end_section(data: &[u8], cursor: usize) -> Result<()> {
    let end_section = read_section_header(data, cursor)?;
    if &end_section.tag != b"END " {
        return Err(Error::Parse(format!(
            "PVO expected END section, got '{}'",
            String::from_utf8_lossy(&end_section.tag)
        )));
    }
    if end_section.data_length != 0 {
        return Err(Error::Parse(format!(
            "PVO END data_length expected 0, got {}",
            end_section.data_length
        )));
    }
    Ok(())
}

fn validate_leaf_count(header: &PvoHeader, octr_nodes: &[OctrNode]) -> Result<()> {
    let actual_leaf_count = octr_nodes.iter().filter(|node| !node.is_interior()).count();
    if actual_leaf_count != usize::try_from(header.leaf_nodes).unwrap_or(usize::MAX) {
        return Err(Error::Parse(format!(
            "PVO leaf_nodes mismatch: header says {}, OCTR has {} nodes with leaf data",
            header.leaf_nodes, actual_leaf_count
        )));
    }
    Ok(())
}

fn validate_node_refs(plst_size: usize, octr_nodes: &[OctrNode]) -> Result<()> {
    let node_offsets: std::collections::HashSet<usize> =
        octr_nodes.iter().map(|node| node.byte_offset).collect();

    for node in octr_nodes {
        for &(octant, ref_offset) in &node.child_refs {
            if ref_offset == CHILD_REF_EMPTY {
                continue;
            }

            let ref_offset_usize = usize::try_from(ref_offset)
                .map_err(|e| Error::Parse(format!("PVO child_ref conversion failed: {e}")))?;
            if !node_offsets.contains(&ref_offset_usize) {
                return Err(Error::Parse(format!(
                    "PVO OCTR node at 0x{:X} child octant {} references offset 0x{ref_offset:X} which is not a valid node byte_offset",
                    node.byte_offset, octant
                )));
            }
        }

        if node.leaf_ref != LEAF_REF_NONE {
            let leaf_ref_usize = usize::try_from(node.leaf_ref)
                .map_err(|e| Error::Parse(format!("PVO leaf_ref conversion failed: {e}")))?;
            if leaf_ref_usize >= plst_size {
                return Err(Error::Parse(format!(
                    "PVO OCTR node at 0x{:X} has leaf_ref 0x{:X} outside PLST range (0..0x{plst_size:X})",
                    node.byte_offset, node.leaf_ref
                )));
            }
        }
    }

    Ok(())
}

fn validate_plst_ranges(mlst_polygon_count: u32, plst_leaves: &[PlstLeaf]) -> Result<()> {
    let mlst_count_u64 = u64::from(mlst_polygon_count);
    for leaf in plst_leaves {
        for entry in &leaf.entries {
            let end = u64::from(entry.mlst_start) + u64::from(entry.count);
            if end > mlst_count_u64 {
                return Err(Error::Parse(format!(
                    "PVO PLST leaf at 0x{:X} entry overflows MLST: mlst_start={} + count={} > mlst_polygon_count={}",
                    leaf.byte_offset, entry.mlst_start, entry.count, mlst_polygon_count
                )));
            }
        }
    }
    Ok(())
}

/// Section order: OCTH → OCTR → PLST → MLST → END.
#[allow(clippy::missing_errors_doc)]
pub fn parse_pvo_file(data: &[u8]) -> Result<PvoFile> {
    if data.len() < 16 {
        return Err(Error::Parse(format!(
            "PVO file too small: {} bytes",
            data.len()
        )));
    }

    let mut cursor = 0usize;

    let header = parse_octh_section(data, &mut cursor)?;
    let octr_payload = read_section_payload(data, &mut cursor, *b"OCTR")?;
    let octr_nodes = parse_octr_nodes(octr_payload, header.total_nodes)?;

    let plst_payload = read_section_payload(data, &mut cursor, *b"PLST")?;
    let plst_leaves = parse_plst_leaves(plst_payload)?;

    let mlst_payload = read_section_payload(data, &mut cursor, *b"MLST")?;
    let mlst_indices = parse_mlst_indices(mlst_payload)?;

    if usize::try_from(header.mlst_polygon_count).unwrap_or(usize::MAX) != mlst_indices.len() {
        return Err(Error::Parse(format!(
            "PVO header mlst_polygon_count ({}) != MLST entry count ({})",
            header.mlst_polygon_count,
            mlst_indices.len()
        )));
    }

    validate_end_section(data, cursor)?;
    validate_leaf_count(&header, &octr_nodes)?;
    validate_node_refs(plst_payload.len(), &octr_nodes)?;
    validate_plst_ranges(header.mlst_polygon_count, &plst_leaves)?;

    Ok(PvoFile {
        header,
        octr_nodes,
        plst_leaves,
        mlst_indices,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_minimal_pvo() -> Vec<u8> {
        let mut buf = Vec::new();

        // OCTH: 2 nodes, 1 leaf, 3 MLST polygons, cell_size=16384, center=(100,-200,300)
        buf.extend_from_slice(b"OCTH");
        buf.extend_from_slice(&52u32.to_be_bytes());
        buf.extend_from_slice(&10u32.to_le_bytes());
        buf.extend_from_slice(&2u32.to_le_bytes());
        buf.extend_from_slice(&1u32.to_le_bytes());
        buf.extend_from_slice(&3u32.to_le_bytes());
        buf.extend_from_slice(&0u32.to_le_bytes());
        buf.extend_from_slice(&16384u32.to_le_bytes());
        buf.extend_from_slice(&100i32.to_le_bytes());
        buf.extend_from_slice(&(-200i32).to_le_bytes());
        buf.extend_from_slice(&300i32.to_le_bytes());
        for _ in 0..4 {
            buf.extend_from_slice(&0u32.to_le_bytes());
        }

        // OCTR: node 0 (offset 0) = interior, 1 child in octant 0 pointing to node 1 (offset 9)
        //        node 1 (offset 9) = leaf, leaf_ref=0 into PLST
        let mut octr = Vec::new();
        octr.push(0x01);
        octr.extend_from_slice(&LEAF_REF_NONE.to_le_bytes());
        octr.extend_from_slice(&9u32.to_le_bytes());
        octr.push(0x00);
        octr.extend_from_slice(&0u32.to_le_bytes());
        buf.extend_from_slice(b"OCTR");
        let octr_len = u32::try_from(octr.len()).expect("test OCTR length fits u32");
        buf.extend_from_slice(&octr_len.to_be_bytes());
        buf.extend_from_slice(&octr);

        // PLST: 1 leaf with 1 entry: count=3, mlst_start=0
        let mut plst = Vec::new();
        plst.push(1u8);
        plst.extend_from_slice(&3u16.to_le_bytes());
        plst.extend_from_slice(&0u32.to_le_bytes());
        buf.extend_from_slice(b"PLST");
        let plst_len = u32::try_from(plst.len()).expect("test PLST length fits u32");
        buf.extend_from_slice(&plst_len.to_be_bytes());
        buf.extend_from_slice(&plst);

        // MLST: 3 polygon indices
        let mut mlst = Vec::new();
        mlst.extend_from_slice(&10u16.to_le_bytes());
        mlst.extend_from_slice(&20u16.to_le_bytes());
        mlst.extend_from_slice(&30u16.to_le_bytes());
        buf.extend_from_slice(b"MLST");
        let mlst_len = u32::try_from(mlst.len()).expect("test MLST length fits u32");
        buf.extend_from_slice(&mlst_len.to_be_bytes());
        buf.extend_from_slice(&mlst);

        buf.extend_from_slice(b"END ");
        buf.extend_from_slice(&0u32.to_be_bytes());

        buf
    }

    #[test]
    fn parse_minimal_pvo() {
        let data = build_minimal_pvo();
        let file = parse_pvo_file(&data).expect("should parse minimal PVO");

        assert_eq!(file.header.depth, 10);
        assert_eq!(file.header.total_nodes, 2);
        assert_eq!(file.header.leaf_nodes, 1);
        assert_eq!(file.header.mlst_polygon_count, 3);
        assert_eq!(file.header.reserved_18, 0);
        assert_eq!(file.header.cell_size, 16384);
        assert_eq!(file.header.center_x, 100);
        assert_eq!(file.header.center_y, -200);
        assert_eq!(file.header.center_z, 300);
        assert_eq!(file.header.reserved_2c, [0; 4]);
        assert_eq!(file.header.interior_nodes(), 1);

        assert_eq!(file.octr_nodes.len(), 2);

        let n0 = &file.octr_nodes[0];
        assert_eq!(n0.byte_offset, 0);
        assert_eq!(n0.child_mask, 0x01);
        assert!(n0.is_interior());
        assert!(!n0.is_leaf());
        assert_eq!(n0.child_count(), 1);
        assert_eq!(n0.child_refs, vec![(0, 9)]);
        assert_eq!(n0.record_size(), 9);

        let n1 = &file.octr_nodes[1];
        assert_eq!(n1.byte_offset, 9);
        assert_eq!(n1.child_mask, 0x00);
        assert!(!n1.is_interior());
        assert!(n1.is_leaf());
        assert_eq!(n1.child_count(), 0);
        assert!(n1.child_refs.is_empty());
        assert_eq!(n1.leaf_ref, 0);
        assert_eq!(n1.record_size(), 5);

        assert_eq!(file.plst_leaves.len(), 1);
        assert_eq!(file.plst_leaves[0].byte_offset, 0);
        assert_eq!(file.plst_leaves[0].entries.len(), 1);
        assert_eq!(file.plst_leaves[0].entries[0].count, 3);
        assert_eq!(file.plst_leaves[0].entries[0].mlst_start, 0);

        assert_eq!(file.mlst_indices, vec![10, 20, 30]);
    }

    #[test]
    fn parse_rejects_bad_magic() {
        let mut data = build_minimal_pvo();
        data[0..4].copy_from_slice(b"NOPE");
        let err = parse_pvo_file(&data).expect_err("expected parse error");
        assert!(matches!(err, Error::Parse(_)));
    }

    #[test]
    fn parse_rejects_too_small_file() {
        let data = vec![0u8; 8];
        let err = parse_pvo_file(&data).expect_err("expected parse error");
        assert!(matches!(err, Error::Parse(_)));
    }

    #[test]
    fn parse_rejects_mismatched_node_count() {
        let mut data = build_minimal_pvo();
        let total_nodes_offset = 8 + 4;
        data[total_nodes_offset..total_nodes_offset + 4].copy_from_slice(&99u32.to_le_bytes());
        let err = parse_pvo_file(&data).expect_err("expected node count mismatch");
        assert!(matches!(err, Error::Parse(_)));
    }

    #[test]
    fn parse_rejects_mismatched_mlst_count() {
        let mut data = build_minimal_pvo();
        let mlst_count_offset = 8 + 12;
        data[mlst_count_offset..mlst_count_offset + 4].copy_from_slice(&999u32.to_le_bytes());
        let err = parse_pvo_file(&data).expect_err("expected mlst count mismatch");
        assert!(matches!(err, Error::Parse(_)));
    }

    #[test]
    fn octr_node_all_children() {
        let mut payload = Vec::new();
        payload.push(0xFF);
        payload.extend_from_slice(&42u32.to_le_bytes());
        for i in 0u32..8 {
            payload.extend_from_slice(&(i * 10).to_le_bytes());
        }

        let nodes = parse_octr_nodes(&payload, 1).expect("should parse");
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].byte_offset, 0);
        assert_eq!(nodes[0].child_mask, 0xFF);
        assert_eq!(nodes[0].leaf_ref, 42);
        assert_eq!(nodes[0].child_count(), 8);
        assert_eq!(nodes[0].record_size(), 37);
        assert_eq!(
            nodes[0].child_refs,
            vec![
                (0, 0),
                (1, 10),
                (2, 20),
                (3, 30),
                (4, 40),
                (5, 50),
                (6, 60),
                (7, 70)
            ]
        );
    }
}
