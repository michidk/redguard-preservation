# PVO File Format

Pre-computed Visibility Octree. Binary spatial data format located in the `/maps` directory alongside `.RGM` (scene) and `.WLD` (terrain) files.

5 PVO files exist, each corresponding to a game level: CATACOMB, CAVERNS, DRINT, ISLAND, PALACE.

For a complete reference parser with pseudocode, see [PVO Parser](parser.md).

## Purpose

PVO files store pre-computed visibility data used for geometry culling at runtime. Instead of calculating which polygons are visible from the camera every frame, the game looks up the camera position in the octree and retrieves a pre-built list of visible group IDs.

The runtime lookup works as follows: take the camera's world-space position, walk the octree from root to leaf by comparing coordinates against each node's center, then collect the polygon indices stored in that leaf's PLST entries (which reference ranges in the MLST table). Only those polygons are submitted for rendering — everything else is skipped.

The data is generated offline by placing a virtual camera at every point on a uniform 256-unit grid across the level, running a visibility query at each point, and baking the results into the octree. See [Generation Process](#generation-process) for details.

## Overall Structure

The file uses the same section-framing as RGM: 4-byte ASCII tag + 4-byte big-endian data length.

```
[OCTH — header]
[OCTR — octree node records]
[PLST — leaf polygon-list records]
[MLST — master polygon index table]
[END  — footer]
```

Each section is framed as:

| Offset | Size | Type | Endian | Name | Description |
|---|---|---|---|---|---|
| 0x00 | 4 | `[u8; 4]` | — | tag | ASCII section tag |
| 0x04 | 4 | `u32` | **BE** | data_length | Payload size in bytes (0 for `END `) |

### Section layout

| File | OCTH | OCTR | PLST | MLST | END | Total |
|---|---|---|---|---|---|---|
| CATACOMB | @0x00 (52) | @0x3C (256,091) | @0x3E89F (827,861) | @0x108A7C (110,788) | @0x123B48 (0) | 1,194,832 |
| CAVERNS | @0x00 (52) | @0x3C (271,546) | @0x424FE (900,844) | @0x11E3F2 (33,630) | @0x126758 (0) | 1,206,112 |
| DRINT | @0x00 (52) | @0x3C (300,688) | @0x496D4 (994,987) | @0x13C587 (81,074) | @0x150241 (0) | 1,376,841 |
| ISLAND | @0x00 (52) | @0x3C (213,134) | @0x340D2 (1,053,725) | @0x1354F7 (216,378) | @0x16A239 (0) | 1,483,329 |
| PALACE | @0x00 (52) | @0x3C (48,125) | @0xBC41 (224,647) | @0x429D0 (108,922) | @0x5D352 (0) | 381,786 |

A sixth section tag `PTCH` exists in the executable's string table but is not present in shipped files. The engine writes and loads PTCH sections through dedicated save/load paths (the `pvopatchsave` console command triggers a write).

Best-effort runtime characterization from engine behavior:

- `PTCH` section length is serialized as `patch_count * 6` bytes.
- The loader allocates and reads 6-byte records from `PTCH`.
- Patch application expands record data into MLST object-id lists used by PVO visibility checks.

Per-record layout (resolved from add/remove/apply behavior):

| Offset | Size | Type | Endian | Name | Description |
|---|---|---|---|---|---|
| 0x00 | 4 | `u32` | LE | octr_node_index | OCTR-node index (`(node_ptr - octr_base) / 5`) identifying which octree node receives the patch object id. |
| 0x04 | 2 | `u16` | LE | object_index | Object index appended to the runtime-visible MLST id list for that node. |

Engine behavior:

- Add patch: writes `octr_node_index` at `record+0` and `object_index` at `record+4`.
- Delete patch: matches/removes records by the same pair (`octr_node_index`, `object_index`).
- Apply patch: scans records matching current `octr_node_index` and appends `object_index` values into the runtime visibility-id buffer.

## OCTH Section — Header (52 bytes payload)

| Offset | Size | Type | Endian | Name | Description |
|---|---|---|---|---|---|
| 0x00 | 4 | `[u8; 4]` | — | magic | `OCTH` |
| 0x04 | 4 | `u32` | **BE** | header_data_size | Always 52 (0x34). |
| 0x08 | 4 | `u32` | LE | depth | Always 10. Maximum octree depth. |
| 0x0C | 4 | `u32` | LE | total_nodes | Total octree node count. Equals `leaf_nodes + interior_nodes`. |
| 0x10 | 4 | `u32` | LE | leaf_nodes | Leaf node count. Equals `total_nodes - interior_nodes`. |
| 0x14 | 4 | `u32` | LE | mlst_polygon_count | Total entries in the MLST polygon index table. Invariant: `mlst_polygon_count * 2 == MLST data_length`. |
| 0x18 | 4 | `u32` | LE | reserved | Always 0. |
| 0x1C | 4 | `u32` | LE | cell_size | Root cell half-extent. Power of 2: 16384 (4 files) or 8192 (PALACE). |
| 0x20 | 4 | `i32` | LE | center_x | Octree root center X coordinate. |
| 0x24 | 4 | `i32` | LE | center_y | Octree root center Y coordinate. |
| 0x28 | 4 | `i32` | LE | center_z | Octree root center Z coordinate. |
| 0x2C | 16 | — | — | reserved | Always zero (4 × u32). |

### Node count relationship

`total_nodes = leaf_nodes + interior_nodes` where `interior_nodes` equals the number of `0xFFFFFFFF` leaf_ref values in the OCTR section:

| File | total_nodes | leaf_nodes | interior_nodes |
|---|---|---|---|
| CATACOMB | 23,287 | 16,787 | 6,500 |
| CAVERNS | 25,694 | 19,618 | 6,076 |
| DRINT | 29,112 | 22,717 | 6,395 |
| ISLAND | 23,154 | 18,971 | 4,183 |
| PALACE | 5,113 | 4,105 | 1,008 |

### mlst_polygon_count confirmation

| File | mlst_polygon_count | MLST data_length | count × 2 == length |
|---|---|---|---|
| CATACOMB | 55,394 | 110,788 | yes |
| CAVERNS | 16,815 | 33,630 | yes |
| DRINT | 40,537 | 81,074 | yes |
| ISLAND | 108,189 | 216,378 | yes |
| PALACE | 54,461 | 108,922 | yes |

### Center coordinates and extents

| File | center_x | center_y | center_z | cell_size |
|---|---|---|---|---|
| CATACOMB | 35,584 | -11,520 | 29,952 | 16,384 |
| CAVERNS | 27,392 | -9,984 | 21,504 | 16,384 |
| DRINT | 23,808 | -13,056 | 33,024 | 16,384 |
| ISLAND | 35,584 | -16,384 | 36,608 | 16,384 |
| PALACE | 31,488 | -6,144 | 30,720 | 8,192 |

The octree root spans `[center - cell_size, center + cell_size]` on each axis.

## OCTR Section — Octree Node Records

Serialized octree nodes written sequentially. Each node is a variable-length record addressed by byte offset within the section.

### Node record format

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| 0 | 1 | `u8` | child_mask | Bit field. Bit *i* set = child *i* is present (octants 0..7). |
| 1 | 4 | `u32` | leaf_ref | Byte offset into the PLST section. `0xFFFFFFFF` = no leaf data (interior-only node). |
| 5 | 4 × *n* | `u32[n]` | child_refs | One entry per set bit in `child_mask`, low bit first. Each is a byte offset into the OCTR section pointing to a child node. `0xFFFFFFFE` = uninitialized-child sentinel (see below). |

Record size = `5 + popcount(child_mask) * 4`

Possible sizes: 5, 9, 13, 17, 21, 25, 29, 33, 37 bytes.

### Octant assignment

The 3-bit octant index encodes spatial position relative to the node center:

```
bit 0 (value 1) = z > center_z
bit 1 (value 2) = y > center_y
bit 2 (value 4) = x > center_x

Octant 0 = (x-, y-, z-)    Octant 4 = (x+, y-, z-)
Octant 1 = (x-, y-, z+)    Octant 5 = (x+, y-, z+)
Octant 2 = (x-, y+, z-)    Octant 6 = (x+, y+, z-)
Octant 3 = (x-, y+, z+)    Octant 7 = (x+, y+, z+)
```

### Common child_mask patterns

| Pattern | Binary | Meaning |
|---|---|---|
| `0x00` | `00000000` | Leaf node, no children |
| `0x33` | `00110011` | Children in octants 0,1,4,5 (one face) |
| `0xCC` | `11001100` | Children in octants 2,3,6,7 (opposite face) |
| `0xAA` | `10101010` | Children in octants 1,3,5,7 (axis-aligned half) |
| `0x55` | `01010101` | Children in octants 0,2,4,6 (other half) |
| `0xFF` | `11111111` | All 8 children present |

### Child and leaf sentinel values

Two sentinel values appear in octree records:

- **`0xFFFFFFFF`** in the `leaf_ref` field marks interior-only nodes — nodes with children but no directly associated polygon list. The count of these values equals `interior_nodes` from the header. In runtime traversal code, `0xFFFFFFFF` also serves as the null terminator that ends octree walks.
- **`0xFFFFFFFE`** in `child_refs` marks an uninitialized/placeholder child node. During runtime octree traversal, this value indicates an unpopulated slot; when traversal visits it, the slot is overwritten with the current position. This is distinct from a null child (`0xFFFFFFFF`) and from a valid child offset.

## PLST Section — Leaf Polygon-List Records

Serialized leaf data written sequentially. This is the largest section in every file. Each leaf describes which polygon groups are visible from an octree cell.

### Leaf record format

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| 0 | 1 | `u8` | entry_count | Number of entries in this leaf. |
| 1 | 6 × *n* | — | entries | Array of `entry_count` entries (see below). |

Record size = `1 + 6 * entry_count`

Each entry:

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| 0 | 2 | `u16` | count | Number of polygon indices in this sub-list. |
| 2 | 4 | `u32` | mlst_start | Starting index into the MLST array. |

Each entry references a contiguous slice: `mlst[mlst_start .. mlst_start + count]`.

Constraint: `mlst_start + count <= mlst_polygon_count`.

Entries within a leaf represent distinct polygon groups. The full visible set for a leaf is the union of all its entry slices. Multiple leaves may share the same MLST ranges.

`leaf_ref` values in OCTR are byte offsets into this section, pointing to the start of a leaf record.

## MLST Section — Master Polygon Index Table

A flat array of `u16` visibility group indices.

- Length: `mlst_polygon_count * 2` bytes.

This table is the master list of visibility groups referenced by the octree. PLST entries reference contiguous ranges within this table.

### Index semantics

Each `u16` value is a **placed-object visibility ID**, not an individual face index. The indices form a dense, zero-based sequential range with no gaps. The visibility check at runtime uses two lookup paths based on the index value:

- **Indices 0 .. MPSO_count-1**: direct MPSO record index. The runtime multiplies the index by 66 (0x42 = MPSO record size) and adds the MPSO array base to get the placed-object record.
- **Indices MPSO_count .. N-1**: secondary table index. The runtime subtracts MPSO_count and uses the result as an index into a separate pointer table. This table holds additional static objects loaded at runtime (e.g. via `LoadStatic` script commands, MPRP rope chains, or other non-MPSO visibility targets).

| File | Total IDs | MPSO range | Secondary range | MPSO objects | Secondary count |
|---|---|---|---|---|---|
| CATACOMB | 591 | 0–500 | 501–590 | 501 | 90 |
| CAVERNS | 233 | 0–203 | 204–232 | 204 | 29 |
| DRINT | 284 | 0–257 | 258–283 | 258 | 26 |
| ISLAND | 1,704 | 0–1,690 | 1,691–1,703 | 1,691 | 13 |
| PALACE | 289 | 0–262 | 263–288 | 263 | 26 |

The MPSO record size (66 bytes = 0x42) appears as the multiplier in visibility lookups.

## END Section — Footer

8 bytes: `END ` (4 ASCII bytes) followed by `0x00000000` (4 zero bytes). Data length is 0.

## Generation Process

PVO files are generated by iterating a uniform 3D grid over the world bounding box:

1. Compute world bounding box from level geometry.
2. Iterate a uniform 3D grid at 256-unit spacing.
3. At each grid point, run a visibility query to determine which polygons are visible.
4. Insert the visible polygon set as a leaf into the octree.
5. Prune single-child branches, then write the file.

### Debug console commands

| Command | Description |
|---|---|
| `pvoi` / `pvotreeinfo` | Display PVO tree statistics |
| `pvoa` / `pvoaddpatch` | Add object to PVO visibility patch |
| `pvod` / `pvodeletepatch` | Remove object from PVO patch |
| `pvoonoff` | Toggle PVO visibility system on/off |
| `pvos` / `pvopatchsave` | Save PVO patches to PTCH section |
| `pvol` / `pvotreeload` | Load PVO tree from file |

## Secondary Visibility Table

MLST indices `>= MPSO_count` reference a secondary pointer table built at runtime. This table is populated by iterating the placed object list and collecting objects whose visibility flag (offset `+0x7a` in the runtime object struct) is non-zero.

The visibility flag is set by SOUP script functions during object initialization. Objects that receive this flag — such as dynamically loaded static models — become trackable by the PVO system alongside the primary MPSO-based objects.

| Step | Description |
|---|---|
| 1 | Count placed objects with visibility flag set → `secondary_count` |
| 2 | Allocate `secondary_count × 4` bytes for pointer array |
| 3 | Iterate placed objects; for each with flag `+0x7a != 0`, append pointer to array |
| 4 | At runtime, MLST index `- MPSO_count` indexes into this array |

## External References

- [UESP: Mod:Redguard File Formats](https://en.uesp.net/wiki/Mod:Redguard_File_Formats)
- [UESP: User:Daveh/Redguard File Formats](https://en.uesp.net/wiki/User:Daveh/Redguard_File_Formats)
- [RGUnity/redguard-unity `RGFileImport/RGGFXImport/`](https://github.com/RGUnity/redguard-unity/tree/master/Assets/Scripts/RGFileImport/RGGFXImport)
