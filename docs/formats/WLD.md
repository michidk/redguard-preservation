# WLD World Geometry File Format

Terrain/world-grid container with a fixed 4-section layout; each section stores four 128x128 byte maps.

4 WLD files exist in `/maps`: `EXTPALAC.WLD`, `HIDEOUT.WLD`, `ISLAND.WLD`, `NECRISLE.WLD`.

## Overall Structure

All WLD files are exactly 263,432 bytes.

```
[Header — 1184 bytes]
[Section 0 — 65558 bytes]
[Section 1 — 65558 bytes]
[Section 2 — 65558 bytes]
[Section 3 — 65558 bytes]
[Footer — 16 bytes]
```

Each section (65,558 bytes) is:

```
[Section Header — 22 bytes]
[Map 1 — 128×128 bytes (heightmap)]
[Map 2 — 128×128 bytes (unused, zero-filled)]
[Map 3 — 128×128 bytes (texture/material)]
[Map 4 — 128×128 bytes (unused, zero-filled)]
```

## Header (1184 bytes)

The file header is 296 dwords (`u32[296]`). Most are zero; 12 are non-zero.

Logical field groups within the 296-dword header:

- `unknown1[6]` (`u32[6]`) at `0x00..0x17`
- `sec_hdr_size` (`u32`) at `0x18`
- `file_size` (`u32`) at `0x1C`
- `unknown2[28]` (`u32[28]`) at `0x20..0x8F`
- `sec_ofs[4]` (`u32[4]`) at `0x90..0x9F`
- `unknown3[256]` (`u32[256]`) at `0xA0..0x49F`

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| 0x00 | 4 | `u32` | unknown1_0 | Always `16`.
| 0x04 | 4 | `u32` | section_cols | Always `2`.
| 0x08 | 4 | `u32` | section_rows | Always `2`.
| 0x0C | 4 | `u32` | reserved_0c | Always `0`.
| 0x10 | 4 | `u32` | unknown1_4 | Always `160` (`0xA0`).
| 0x14 | 4 | `u32` | unknown1_5 | Always `1`.
| 0x18 | 4 | `u32` | section_header_size | Always `22`.
| 0x1C | 4 | `u32` | file_size_field | Always `263416` (`file_size - 16`).
| 0x90 | 4 | `u32` | section0_offset | Always `1184` (`0x4A0`).
| 0x94 | 4 | `u32` | section1_offset | Always `66742` (`0x104B6`).
| 0x98 | 4 | `u32` | section2_offset | Always `132300` (`0x204CC`).
| 0x9C | 4 | `u32` | section3_offset | Always `197858` (`0x304E2`).
| 0xA0 | 4 | `u32` | unknown3_0 | Always `2135957017` (`0x7F501E19`); first element of `unknown3[256]`.

`0xA0..0x49F` is a contiguous `u32[256]` block (`unknown3`), not a single field. Only `unknown3[0]` is non-zero; the remaining 255 dwords are zero.

The header is byte-identical across all 4 shipped WLD files. All remaining header dwords not listed above are zero.

The loader reads the first `0x90` bytes, validates `0x14 == 1` and `0x18 == 22`, and uses only `section_cols`, `section_rows`, and `section_header_size` at runtime. Fields beyond `0x1C` — including `sec_ofs[4]` and `unknown3[256]` — are not read by the terrain loader.

## Section Header (22 bytes)

Each section starts with 11 little-endian words (`u16[11]`):

| Offset (in section) | Size | Type | Name | Description |
|---|---|---|---|---|
| `+0x00` | 6 | `u16[3]` | unknown1 | Section-local unknown values.
| `+0x06` | 2 | `u16` | texbsi_file | Section-declared texture archive id (`TEXBSI.%03d`). In the original engine's terrain path, texture-bank loading is hard-wired to `texbsi.302` (see notes below).
| `+0x08` | 2 | `u16` | map_size | Always `256` (`2 x 128`).
| `+0x0A` | 12 | `u16[6]` | unknown2 | Always 0.

Section headers are identical across all 4 shipped WLD files. `unknown1[0]` varies by section index: section0=`2152`, section1=`568`, section2=`1308`, section3=`10`. `texbsi_file` is always `302`. `map_size` is always `256`.

The loader reads each 22-byte section header but does not decode or reference any fields — it proceeds directly to map-plane reads. The engine hard-wires `texbsi.302` for terrain textures regardless of the per-section `texbsi_file` value.

`unknown1[0]` cannot be a TEXBSI id (values like `568` and `1308` have no matching files). It appears to be build-tooling metadata fixed per section slot.

Section headers (identical across all files):

| Section | Header bytes (hex) |
|---|---|
| 0 | `68 08 00 00 00 00 2E 01 00 01 00 00 00 00 00 00 00 00 00 00 00 00` |
| 1 | `38 02 00 00 00 00 2E 01 00 01 00 00 00 00 00 00 00 00 00 00 00 00` |
| 2 | `1C 05 00 00 00 00 2E 01 00 01 00 00 00 00 00 00 00 00 00 00 00 00` |
| 3 | `0A 00 00 00 00 00 2E 01 00 01 00 00 00 00 00 00 00 00 00 00 00 00` |

## Map Planes

After each 22-byte section header, four 128x128 byte maps follow.

- `Map 1`: heightmap plane; low 7 bits are height (0–127), high bit is a build-time flag stripped at load time. See [Map 1 High Bit](#map-1-high-bit-0x80).
- `Map 2`: unused — always zero-filled; skipped by the engine at load time.
- `Map 3`: texture/material plane; packed bits:
  - low 6 bits (`0..63`) = texture index
  - high 2 bits (`0..3`) = quarter-turn rotation
- `Map 4`: unused — always zero-filled; skipped by the engine at load time.

The engine only uses Map 1 and Map 3. Maps 2 and 4 are skipped during loading — their bytes are read to advance the file stream, but the data is never stored or used.

`texbsi_file = 302` matches on-disk texture archive `fxart/TEXBSI.302`.

In `TEXBSI.302`, image names follow `D02xxx`, which aligns with TEXBSI naming rules:

- filename low two digits (`02`) match image-name file number (`D02xxx`)
- filename hundreds digit (`3`) matches type-char group (`D`)

Notes:

- `Map 3` bit packing (index + rotation) is corroborated by UESP documentation.
- Original-engine terrain initialization hard-loads `texbsi.302` and precomputes a 64-entry terrain texture lookup table from that archive, matching Map 3's 0..63 index space.
- Engine string analysis confirms `texbsi.302` is referenced by the terrain initialization path; `texbsi.%s` is referenced by the generic TEXBSI loader path, not the terrain-table initialization path.
- Conclusion: per-section `texbsi_file` switching is not used by the original engine's terrain renderer; terrain textures come from `TEXBSI.302`.

Cross-check against shipped `fxart/TEXBSI.302`:

- `Map 3` index range `0..63` is fully covered by image ids in `TEXBSI.302` (name suffixes `00..63`).
- The previously "missing" ids (`00`, `05`, `06`, `07`, `30`, `31`, `32`, `52`) are present as alternate record forms in the TEXBSI stream (not just the main BSIF image-record form), matching UESP's note that index mapping is not a simple 1:1 record-order lookup.
- Those alternate-form ids are `IFHD` animated records in `TEXBSI.302`, consistent with lookup by image-id suffix rather than simple static-record order.
- Rotation bits are still populated when texture index is `0`; this suggests engine-side handling likely treats index `0` as dominant (rotation may be ignored for empty/default tiles).

Bank summary (`fxart/TEXBSI.*`, D-prefix terrain-style banks):

- Only `TEXBSI.302` has full `0..63` coverage (56 `BSIF` static records + 8 `IFHD` animated records). Other `D*` banks are partial/specialized.
- Combined with engine evidence (the engine hard-wires `texbsi.302`), the original engine's terrain path should be treated as fixed-bank (`302`) rather than generic per-bank Map 3 lookup behavior.

### Map 1 High Bit (`0x80`)

The game engine strips the high bit (`& 0x7F`) from every Map 1 byte at WLD load time, before storing height values in its runtime buffer. The high bit is discarded and never used for rendering, height lookup, texture selection, or any other runtime purpose.

The high bit tends to appear on outer-border cells and cells with large height deltas. It may be a build-time artifact (e.g., marking boundary or steep cells for the level editor) that the runtime engine does not consume.

Each section contributes one 128x128 tile per map plane. The four section tiles combine into a 2x2 world grid (256x256) as described by UESP.

## Terrain Rendering Pipeline

At runtime, Map 1 and Map 3 are loaded into separate buffers and processed independently — there is no interaction between the two during rendering.

### Grid Coordinate System

Each grid cell is 256 engine units wide. Terrain vertex positions are computed as:

```
world_x = grid_index_x × 256
world_z = grid_index_z × 256
world_y = -height_table[heightmap_byte & 0x7F]
```

No origin offset is applied to vertex positions. A separate world→grid reverse-lookup (used for camera cell detection) applies half-cell offsets (−0.5 on X, +0.5 on Z), but these do not affect terrain geometry.

### Height Values (Map 1)

Each Map 1 byte is masked to 7 bits (`& 0x7F`) at load time, producing height values in the range 0–127. These values index into a 128-entry float lookup table to produce world-space Y coordinates.

The engine stores a static source table of positive float values and negates them at initialization (`-ABS(source)`), so terrain heights are negative — the terrain surface sits below a reference plane. A second initialization mode computes `water_level - ABS(source)`, adjusting heights relative to a configurable water-level parameter.

#### Height Lookup Table

The 128-entry source table (values in engine units before negation):

```
   0:     0    40    40    40    80    80    80   120   120   120
  10:   160   160   160   200   200   200   240   240   240   280
  20:   280   320   320   320   360   360   400   400   400   440
  30:   440   480   480   480   520   520   560   560   600   600
  40:   600   640   640   680   680   720   720   760   760   800
  50:   800   840   840   880   880   920   920   960  1000  1000
  60:  1040  1040  1080  1120  1120  1160  1160  1200  1240  1240
  70:  1280  1320  1320  1360  1400  1440  1440  1480  1520  1560
  80:  1600  1600  1640  1680  1720  1760  1800  1840  1880  1920
  90:  1960  2000  2040  2080  2120  2200  2240  2280  2320  2400
 100:  2440  2520  2560  2640  2680  2760  2840  2920  3000  3080
 110:  3160  3240  3360  3440  3560  3680  3800  3960  4080  4280
 120:  4440  4680  4920  5200  5560  6040  6680  7760
```

The table uses a non-linear encoding with three regions:

- **Indices 0–52** (values 0–2120): near-linear with ~40-unit steps and repeated values, giving maximum height precision for flat and gently sloped terrain where the player spends most time.
- **Indices 53–69** (values 2120–3240): transition zone with gradually increasing step sizes.
- **Indices 70–127** (values 3240–7760): accelerating steps (80 → 120 → 240 → 1080), compressing tall cliffs and peaks into fewer index values.

This is a hand-tuned gamma-like curve that allocates more precision to common terrain heights (flat ground, gentle slopes) while still supporting the full elevation range with 7 bits of storage.

### Texture Selection (Map 3)

Each Map 3 byte encodes two fields:

| Bits | Mask | Field | Range |
|------|------|-------|-------|
| 5:0  | `& 0x3F` | texture index | 0–63 |
| 7:6  | `>> 6 & 3` | quarter-turn rotation | 0–3 |

In the original engine, the texture index selects one of 64 preloaded terrain texture entries sourced from `TEXBSI.302` (not a per-section runtime bank switch). The rotation selects one of four UV orientation states (0°, 90°, 180°, 270° counter-clockwise).

### Terrain Texture Blending (SURFACE.INI)

The engine loads a [`SURFACE.INI`](../config/surface-ini.md) configuration file (from the game directory) that defines per-texture-index blend behavior and surface-type sound remapping. This drives a pixel-level alpha-blending system for terrain tile transitions — it does not affect geometry, UVs, or material assignment.

### Water Tiles

The terrain renderer treats certain texture indices as water or special tiles. When all four corners of a grid cell have texture indices in the set {0, 5, 30, 31}, the cell is rendered as a water surface instead of normal terrain geometry. This applies water-plane rendering with wave animation effects. See [Water Waves](../engine/water.md) for the per-frame displacement formula and rendering pipeline.

### Terrain Normals

The engine computes smooth vertex normals for terrain in three passes:

1. **Face normals** — Each grid cell is split into two triangles (TL→BR→TR and BR→TL→BL). A cross-product normal is computed per triangle.
2. **Vertex averaging** — At each grid vertex, the face normals from all adjacent triangles are summed and normalized. Each interior vertex touches 6 triangles from 4 cells: both triangles of the cell to the upper-left and lower-right, plus one triangle each from the cells above and to the left.
3. **Rendering** — The averaged vertex normals are used for Gouraud-interpolated shading across each triangle.

This produces smooth terrain shading. The per-triangle face normals (pass 1) are retained as intermediate values but are not used directly for rendering.

## Footer (16 bytes)

The final 16 bytes are constant in all files:

```
54 55 4C 4F 28 C0 43 00 FF FF FF FF 35 37 04 00
```

Interpreted as four dwords (`u32`, little-endian):

1. `0x4F4C5554` (`"TULO"` bytes in file order)
2. `0x0043C028`
3. `0xFFFFFFFF`
4. `0x00043735`

The WLD loader does not read or parse this footer; it stops after reading the 4 section data blocks. The footer is a build-time artifact or reserved metadata ignored at runtime. Field-level semantics are unknown.

## Relationships to Other Formats

- [RGM](RGM.md) stores scene/object placement for the same levels.
- [PVO](pvo/format.md) stores octree-style spatial/visibility data for some of the same levels. WLD terrain is not included in PVO visibility culling. The engine's PVO visibility check operates exclusively on placed objects — it searches MLST entries against MPSO records and actor pointers, and is called only from the placed-object render loop. Terrain is rendered through a separate unconditional path (the engine's terrain surface subsystem). Default visibility is 1 (visible) when PVO data is absent.
- [TEXBSI](TEXBSI.md) supplies textures referenced by `Map 3` indices (Map 3 index range 0–63 is fully covered by `TEXBSI.302`).

## External References

- [UESP: Mod:World Files](https://en.uesp.net/wiki/Mod:World_Files)
- [UESP: Mod:Redguard File Formats](https://en.uesp.net/wiki/Mod:Redguard_File_Formats)
- [uesp/redguard-3dfiletest `3DFileTest/Common/`](https://github.com/uesp/redguard-3dfiletest/tree/master/3DFileTest/Common)
- [RGUnity/redguard-unity `RGWLDFile.cs`](https://github.com/RGUnity/redguard-unity/blob/master/Assets/Scripts/RGFileImport/RGGFXImport/RGWLDFile.cs)
