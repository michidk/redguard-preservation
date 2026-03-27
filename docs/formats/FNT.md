# FNT Bitmap Font File Format

Chunked bitmap-font format with per-file palette and per-glyph indexed image data.

`.FNT` stores UI/dialog font glyphs as palette-indexed bitmaps. Each file embeds its own palette (not scene `COL` palettes), then stores glyph records in ASCII order. For palette structure background, see [COL.md](COL.md).

## Top-Level Layout

The file is a sequence of named chunks, followed by an end marker:

1. `FNHD` (always present)
2. `BPAL` or `FPAL` (always present)
3. `FBMP` (always present)
4. `RDAT` (optional)
5. `END ` marker

Common chunk orders:

- `FNHD -> BPAL -> FBMP -> END`
- `FNHD -> BPAL -> FBMP -> RDAT -> END`
- `FNHD -> FPAL -> FBMP -> END` (`ARIALVS.FNT` only)

## Chunk Header

Each chunk begins with an 8-byte header:

Chunk-length fields are big-endian.

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| 0x00 | 4 | `[u8; 4]` | tag | Chunk name (`FNHD`, `BPAL`, `FPAL`, `FBMP`, `RDAT`) |
| 0x04 | 4 | `u32` | length | Chunk payload size in bytes |

`END ` is a 4-byte marker tag with no payload. `ARIALVS.FNT` has 4 additional trailing zero bytes after `END `.

## FNHD Chunk (56 bytes)

`FNHD` payload is always 56 bytes.

Numeric fields are little-endian.

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| 0x00 | 32 | `[u8; 32]` | description | Font/tool description string; may contain NUL padding or multiple NUL-terminated fragments |
| 0x20 | 2 | `u16` | unknown_24 | Not read at runtime — overwritten during glyph loading. Known values: 0, 1, 3. Export-tool metadata. |
| 0x22 | 2 | `u16` | has_rdat | 1 if `RDAT` chunk present; 0 otherwise. Not checked at runtime — the engine never searches for `RDAT`. |
| 0x24 | 2 | `u16` | reserved_28 | Always 0. Not read at runtime. |
| 0x26 | 2 | `u16` | reserved_2a | Always 0. Not read at runtime. |
| 0x28 | 2 | `u16` | reserved_2c | Always 0. Not read at runtime. |
| 0x2A | 2 | `u16` | max_width | Export-tool hint (range 11–23). Not read at runtime — overwritten with the width of glyph 'W' during loading. |
| 0x2C | 2 | `u16` | line_height | Used by the engine for text layout and baseline positioning. Values: 9, 10, 12, 14, 16, 22, 25, 26. |
| 0x2E | 2 | `u16` | character_start | First encoded codepoint; always 32 (`0x20`, space). Not read at runtime — engine assumes fixed start. |
| 0x30 | 2 | `u16` | character_count | Used by the engine as the glyph loop bound (clamped to 256). Number of glyph records in `FBMP`. Values: 95, 97, 98, 104, 112, 208. |
| 0x32 | 2 | `u16` | reserved_36 | Always 0. Overwritten to 0xFF during glyph loading. |
| 0x34 | 2 | `u16` | reserved_38 | Always 0. Overwritten to 0xFF during glyph loading. |
| 0x36 | 2 | `u16` | has_palette | Used by the engine to control palette loading. Non-zero = allocate 768-byte palette and search for `FPAL`/`BPAL` chunk. Always 1. |

## BPAL / FPAL Chunk (Palette)

Palette payload is always 768 bytes (256 RGB triplets).

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| 0x00 | 768 | `[u8; 768]` | rgb_triplets | 256 entries x 3 bytes (R, G, B), palette indices referenced by `FBMP` pixel bytes |

Notes:

- `BPAL` is the normal tag.
- `FPAL` appears in `ARIALVS.FNT` with the same 768-byte payload shape.
- These palettes are local to each font file, independent of scene palettes in [COL.md](COL.md).

## FBMP Chunk (Glyph Records)

`FBMP` payload contains `character_count` glyph records in sequential codepoint order starting at `character_start`.

Each glyph record:

Numeric fields are little-endian.

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| 0x00 | 2 | `u16` | enabled | 0 = disabled/unrendered glyph; non-zero = active glyph |
| 0x02 | 2 | `i16` | offset_left | Horizontal draw offset in pixels |
| 0x04 | 2 | `i16` | offset_top | Vertical draw offset in pixels |
| 0x06 | 2 | `u16` | width | Glyph bitmap width in pixels |
| 0x08 | 2 | `u16` | height | Glyph bitmap height in pixels |
| 0x0A | `width*height` | `[u8]` | pixels | Row-major palette indices |

Value ranges:

- `offset_left`: 0..5
- `offset_top`: 0..19
- `width`: 1..22
- `height`: 1..25

`FBMP` payload length equals the sum of all glyph record sizes (10-byte header + `width * height` pixels each).

## RDAT Chunk (Optional, 173 bytes)

`RDAT` is optional and always 173 bytes when present.

Layout (partially decoded):

Numeric fields are little-endian.

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| 0x00 | 136 | `[u8; 136]` | source_name | NUL-padded source/tool string |
| 0x88 | 4 | `u32` | unknown_90 | Non-zero metadata field |
| 0x8C | 4 | `u32` | unknown_94 | Non-zero metadata field |
| 0x90 | 4 | `u32` | unknown_98 | Always 0 |
| 0x94 | 4 | `u32` | unknown_9c | Usually 0; value 2 in `ARIALBG.FNT` |
| 0x98 | 4 | `u32` | unknown_a0 | Near `max_width`-like values |
| 0x9C | 4 | `u32` | unknown_a4 | Near `line_height`-like values |
| 0xA0 | 4 | `u32` | unknown_a8 | Small enum-like values (1..3) |
| 0xA4 | 4 | `u32` | unknown_ac | Small enum-like values (1..2) |
| 0xA8 | 4 | `u32` | unknown_b0 | Always 0 |
| 0xAC | 1 | `u8` | unknown_b4 | Always 0 |

`RDAT` is metadata. The font loader uses `FNHD`, `FPAL`/`BPAL`, and `FBMP`; it does not parse `RDAT` payload data.

## External References

- [UESP: Mod:Font Files](https://en.uesp.net/wiki/Mod:Font_Files) — primary external reference for `FNHD`/`BPAL`/`FBMP`/`RDAT` chunk semantics.
- [UESP: Redguard:Glide Differences](https://en.uesp.net/wiki/Redguard:Glide_Differences) — renderer-specific font usage differences (non-structure behavior).
- [UESP: Mod:RedguardFNTImporter](https://en.uesp.net/wiki/Mod:RedguardFNTImporter) — FNT import/export tooling notes.
