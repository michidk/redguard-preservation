# COL Palette File Format

256-color RGB palette format.

Each file maps color indices (0–255) to RGB values. Models reference palette entries via `color_index` in solid-color face data (see [3D.md](models/3d.md#texture-decoding-v40--v50)).

## Overall Structure

All COL files are exactly 776 bytes.

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| 0x00 | 4 | `u32` | file_size | Always 776. |
| 0x04 | 4 | `u32` | magic | Always 0x0000B123. |
| 0x08 | 768 | `[u8; 768]` | palette | 256 × RGB triplets (3 bytes each, values 0–255). |

Palette entry N is at offset `8 + N × 3`, yielding bytes `(R, G, B)`.

## Usage

COL files are per-scene, not per-model. Different levels use different palettes (e.g. `ISLAND.COL`, `CATACOMB.COL`). The same `color_index` in a model's face data produces different colors depending on which palette is active.

Entry 0 is always (0, 0, 0) — black.

## Related Formats

- [TEXBSI](TEXBSI.md) — the `CMAP` subrecord in animated TEXBSI images uses the identical 256 × RGB triplet layout (768 bytes). CMAP palettes are embedded per-image; COL palettes are per-scene.
- [FNT](FNT.md) — font files embed their own `BPAL`/`FPAL` palettes (same 768-byte layout), independent of scene COL palettes.

## Redguard Preservation CLI

The `convert` command exports a COL file as two companion files:

- **Swatch PNG** — 256×256 image with a 16×16 grid of color swatches (16 px per cell). Index 0 is top-left, 255 is bottom-right, row-major order.
- **Palette JSON** — structured metadata with per-entry `index`, `r`, `g`, `b` (0–255), and `hex` fields. Versioned as `redguard-col-v1`.

The output path determines the primary filename; the companion file shares the same stem with the other extension. Passing `-o ISLAND.png` produces `ISLAND.png` + `ISLAND.json`; passing `-o ISLAND.json` produces the same pair.

## External References

- [UESP: Mod:Palette Files](https://en.uesp.net/wiki/Mod:Palette_Files) — dedicated Redguard COL page with the 8-byte header + 768-byte palette layout.
- [UESP: User:Daveh/Redguard File Formats](https://en.uesp.net/wiki/User:Daveh/Redguard_File_Formats) — COL size and palette usage notes.
