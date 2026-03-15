# PVO Parser — Pseudocode

Reference parser for the PVO (Pre-computed Visibility Octree) file format. See [PVO Format](format.md) for format specification.

All field-level details below are validated against all 5 shipped PVO files with byte-exact section boundary matches.

## Data Types

```
u8       — unsigned 8-bit
u16_le   — unsigned 16-bit, little-endian
u32_le   — unsigned 32-bit, little-endian
i32_le   — signed 32-bit, little-endian
u32_be   — unsigned 32-bit, big-endian
tag      — 4 ASCII bytes (e.g. "OCTH")
```

## Structures

```
struct PvoFile {
    header:     OcthHeader,
    nodes:      Vec<OctrNode>,      // OCTR section, indexed by byte offset
    leaves:     Vec<PlstLeaf>,      // PLST section, indexed by byte offset
    mlst:       Vec<u16_le>,        // MLST section, flat polygon index table
}

struct OcthHeader {
    depth:              u32,        // always 10 — max octree depth
    total_nodes:        u32,        // len(nodes)
    leaf_nodes:         u32,        // nodes with leaf_ref != 0xFFFFFFFF
    mlst_polygon_count: u32,        // len(mlst)
    reserved:           u32,        // always 0
    cell_size:          u32,        // root half-extent (16384 or 8192)
    center_x:           i32,        // octree root center
    center_y:           i32,
    center_z:           i32,
    _pad:               [u32; 4],   // always zero
}

struct OctrNode {
    byte_offset:  usize,            // position within OCTR data (for child_ref lookups)
    child_mask:   u8,               // bit i set → child i present (octants 0..7)
    leaf_ref:     u32,              // byte offset into PLST, or 0xFFFFFFFF (no leaf)
    child_refs:   Vec<(u8, u32)>,   // (octant_index, byte_offset into OCTR) per set bit
}

struct PlstLeaf {
    byte_offset:  usize,            // position within PLST data (for leaf_ref lookups)
    entries:      Vec<PlstEntry>,
}

struct PlstEntry {
    count:        u16,              // number of polygon indices in this sub-list
    mlst_start:   u32,              // starting index into the MLST array
}
```

## Section Framing

Every section uses identical framing. Parse sections sequentially until `END `.

```
fn read_section_header(reader) -> (tag: [u8; 4], data_length: u32):
    tag         = reader.read_bytes(4)
    data_length = reader.read_u32_be()
    return (tag, data_length)
```

## Top-Level Parser

```
fn parse_pvo(reader) -> PvoFile:
    // --- OCTH ---
    (tag, data_length) = read_section_header(reader)
    assert tag == "OCTH"
    assert data_length == 52
    header = parse_octh(reader)

    // --- OCTR ---
    (tag, data_length) = read_section_header(reader)
    assert tag == "OCTR"
    nodes = parse_octr(reader, data_length)

    // --- PLST ---
    (tag, data_length) = read_section_header(reader)
    assert tag == "PLST"
    leaves = parse_plst(reader, data_length)

    // --- MLST ---
    (tag, data_length) = read_section_header(reader)
    assert tag == "MLST"
    assert data_length == header.mlst_polygon_count * 2
    mlst = parse_mlst(reader, data_length)

    // --- END ---
    (tag, data_length) = read_section_header(reader)
    assert tag == "END "
    assert data_length == 0

    // --- Validation ---
    assert len(nodes) == header.total_nodes
    leaf_count = count(n for n in nodes if n.leaf_ref != 0xFFFFFFFF)
    assert leaf_count == header.leaf_nodes

    return PvoFile { header, nodes, leaves, mlst }
```

## OCTH Parser

```
fn parse_octh(reader) -> OcthHeader:
    header = OcthHeader {
        depth:              reader.read_u32_le(),   // 0x08
        total_nodes:        reader.read_u32_le(),   // 0x0C
        leaf_nodes:         reader.read_u32_le(),   // 0x10
        mlst_polygon_count: reader.read_u32_le(),   // 0x14
        reserved:           reader.read_u32_le(),   // 0x18 — always 0
        cell_size:          reader.read_u32_le(),   // 0x1C
        center_x:           reader.read_i32_le(),   // 0x20
        center_y:           reader.read_i32_le(),   // 0x24
        center_z:           reader.read_i32_le(),   // 0x28
        _pad:               reader.read_bytes(16),  // 0x2C — always zero
    }
    return header
```

## OCTR Parser

Octree nodes are serialized as a flat sequence of variable-length records. Each record describes one octree node. The records are addressed by byte offset within the OCTR data section.

```
fn parse_octr(reader, data_length: u32) -> Vec<OctrNode>:
    nodes = []
    bytes_read = 0

    while bytes_read < data_length:
        node_offset = bytes_read

        child_mask = reader.read_u8()
        leaf_ref   = reader.read_u32_le()
        bytes_read += 5

        // Read one child_ref per set bit in child_mask (low bit first)
        child_refs = []
        for octant in 0..8:
            if child_mask & (1 << octant) != 0:
                ref = reader.read_u32_le()
                child_refs.push((octant, ref))
                bytes_read += 4

        nodes.push(OctrNode {
            byte_offset: node_offset,
            child_mask,
            leaf_ref,
            child_refs,
        })

    assert bytes_read == data_length
    return nodes
```

### Node record binary layout

```
 Byte 0       Bytes 1..4        Bytes 5..5+4n
┌──────────┬─────────────────┬────────────────────────────────┐
│child_mask│   leaf_ref      │ child_ref[0] .. child_ref[n-1] │
│  (u8)    │   (u32_le)      │ (u32_le each)                  │
└──────────┴─────────────────┴────────────────────────────────┘
 n = popcount(child_mask)
 record_size = 5 + 4n
```

### Octant numbering

The 3-bit octant index encodes spatial position relative to the node center:

```
 bit 0 (1) = z > center_z
 bit 1 (2) = y > center_y
 bit 2 (4) = x > center_x

 Octant 0 = (x-, y-, z-)    Octant 4 = (x+, y-, z-)
 Octant 1 = (x-, y-, z+)    Octant 5 = (x+, y-, z+)
 Octant 2 = (x-, y+, z-)    Octant 6 = (x+, y+, z-)
 Octant 3 = (x-, y+, z+)    Octant 7 = (x+, y+, z+)
```

### Interpreting references

- `leaf_ref`: byte offset into the PLST section data. `0xFFFFFFFF` = no leaf (interior-only node).
- `child_refs`: byte offset into the OCTR section data. Use to look up child nodes by their `byte_offset` field.

Reference consistency in shipped files:

- 100% of `child_refs` match a node `byte_offset`.
- 100% of `leaf_refs` fall within PLST bounds.

## PLST Parser

Leaf records describe which polygon groups are visible from an octree cell. Each leaf contains a list of (count, mlst_start) entries that reference ranges within the MLST polygon index table. Multiple leaves may share the same MLST ranges.

```
fn parse_plst(reader, data_length: u32) -> Vec<PlstLeaf>:
    leaves = []
    bytes_read = 0

    while bytes_read < data_length:
        leaf_offset = bytes_read

        entry_count = reader.read_u8()
        bytes_read += 1

        entries = []
        for _ in 0..entry_count:
            count      = reader.read_u16_le()
            mlst_start = reader.read_u32_le()
            entries.push(PlstEntry { count, mlst_start })
            bytes_read += 6

        leaves.push(PlstLeaf {
            byte_offset: leaf_offset,
            entries,
        })

    assert bytes_read == data_length
    return leaves
```

### Leaf record binary layout

```
 Byte 0         Bytes 1..1+6n
┌────────────┬──────────────────────────────────────────┐
│entry_count │ entry[0]          .. entry[n-1]          │
│  (u8)      │ [count:u16][mlst_start:u32] each         │
└────────────┴──────────────────────────────────────────┘
 record_size = 1 + 6 * entry_count
```

### Entry semantics

Each entry references a contiguous slice of the MLST array:

```
 polygons = mlst[mlst_start .. mlst_start + count]
```

Constraint: `mlst_start + count <= header.mlst_polygon_count`.

Entries within a leaf represent distinct polygon groups (e.g. different model faces, terrain sections). The full set of visible polygons for a leaf is the union of all its entry slices.

## MLST Parser

Flat array of `u16_le` placed-object visibility IDs. Each entry is an MPSO record index identifying a placed object for visibility determination, not an individual face/polygon index.

```
fn parse_mlst(reader, data_length: u32) -> Vec<u16>:
    count = data_length / 2
    mlst = []
    for _ in 0..count:
        mlst.push(reader.read_u16_le())
    return mlst
```

## Octree Reconstruction

To build the tree in memory from the flat node list:

```
fn build_tree(nodes: Vec<OctrNode>) -> OctrNode:
    // Build lookup: byte_offset -> node index
    offset_to_idx = {}
    for (i, node) in nodes.enumerate():
        offset_to_idx[node.byte_offset] = i

    // The root is always the first node (byte_offset 0)
    root = nodes[0]

    // Recursively link children
    fn link(node_idx, nodes, offset_to_idx):
        node = nodes[node_idx]
        for (octant, child_offset) in node.child_refs:
            child_idx = offset_to_idx[child_offset]
            node.children[octant] = link(child_idx, nodes, offset_to_idx)
        return node

    return link(0, nodes, offset_to_idx)
```

## Visibility Query

Given a world-space point, traverse the octree to find which polygons are visible:

```
fn query_visible(tree: OctrNode, header: OcthHeader, leaves: Vec<PlstLeaf>,
                 mlst: Vec<u16>, point_x: i32, point_y: i32, point_z: i32) -> Set<u16>:

    node = tree
    cx, cy, cz = header.center_x, header.center_y, header.center_z
    half = header.cell_size

    // Walk from root to leaf
    while true:
        // Determine octant for query point
        octant = 0
        if point_z > cz: octant |= 1
        if point_y > cy: octant |= 2
        if point_x > cx: octant |= 4

        // Descend into child
        if node.child_mask & (1 << octant) == 0:
            break   // no child in this octant

        child_offset = node.child_ref_for_octant(octant)
        node = tree.lookup(child_offset)

        // Update center and half-extent for child cell
        half = half / 2
        if point_x > cx: cx += half  else: cx -= half
        if point_y > cy: cy += half  else: cy -= half
        if point_z > cz: cz += half  else: cz -= half

    // Collect visible polygons from the leaf
    visible = Set()
    if node.leaf_ref != 0xFFFFFFFF:
        leaf = leaves.lookup(node.leaf_ref)
        for entry in leaf.entries:
            for i in entry.mlst_start .. entry.mlst_start + entry.count:
                visible.add(mlst[i])

    return visible
```
