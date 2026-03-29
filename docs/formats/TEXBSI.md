# TEXBSI Texture File Format

Container format for indexed-color texture images. Files are named `TEXBSI.###` where `###` is the texture bank number.

## Overall Structure

A TEXBSI file is a flat sequence of image records with no file-level header. The sequence ends when a 9-byte null sentinel is encountered.

```
[Image Record 0]
[Image Record 1]
...
[Image Record N]
[9 × 0x00]   ← end sentinel
```

## Image Record

Image-record envelope fields are little-endian.

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| 0 | 9 | `[u8; 9]` | image_name | Image name, null-padded. Format: `{type_char}{file_num:02d}{image_idx:03d}`. All-zero = end of file. |
| 9 | 4 | `u32` | subrecord_bytes | Total size of all subrecords that follow (excludes this 13-byte envelope). |
| 13 | ... | subrecords | subrecords | Tagged subrecords until `END `. |

### Image Name Encoding

The 9-byte name encodes the file number and sub-image index:

```
"E01005\0\0\0" → type 'E', file 01, image index 005
"A02003\0\0\0" → type 'A', file 02, image index 003
```

The image index (last 3 digits) is how 3D model faces reference sub-images via `image_id`.

Filename/name coupling across shipped `TEXBSI.###` files:

- `### % 100` matches the 2-digit file number in image names.
- `### / 100` maps to type-char family: `0->A`, `1->B`, `2->C`, `3->D`, `4->E`, `5->F`.

Examples:

- `TEXBSI.302` contains `D02xxx` images.
- `TEXBSI.114` contains `B14xxx` images.

Type characters range from A through F.

The type character has no semantic meaning — it is a deterministic artifact of the base-40 name encoding used internally. The game converts numeric texture IDs to 6-character strings using the alphabet `0123456789abcdefghijklmnopqrstuvwxyz~_#%`, then shifts the first character by subtracting `0x31`. The type letter is simply which base-40 digit range the texture ID falls into (A=digit 27, B=28, ..., F=32). The game never tests or filters by the type character — it round-trips through the numeric ID.

## Subrecord Structure

Every subrecord has an 8-byte header:

Subrecord size words are big-endian.

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| 0 | 4 | `[u8; 4]` | tag | Tag: `BSIF`, `IFHD`, `BHDR`, `CMAP`, `DATA`, or `END ` |
| 4 | 4 | `u32` | payload_size | Payload size in bytes (not including this 8-byte header) |
| 8 | ... | payload | payload | Tag-specific data |

Subrecords always appear in this order:

```
BSIF or IFHD  (mutually exclusive)
BHDR           (required)
CMAP           (optional, only with IFHD)
DATA           (required)
END            (terminates the record — no size field)
```

## Subrecord Payloads

### `BSIF` — Static Image Marker

Payload size: 0 bytes (empty). Marks a static (non-animated) image.

### `IFHD` — Animated Image Marker

Payload size: 44 bytes (always `01` followed by 43 `00` bytes). Marks an animated image.

When `IFHD` is present, the `DATA` subrecord uses the animated offset-table format.

### `BHDR` — Image Header (26 bytes)

All fields are little-endian.

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| 0 | 2 | `i16` | x_offset | X position hint (placement on virtual canvas) |
| 2 | 2 | `i16` | y_offset | Y position hint |
| 4 | 2 | `i16` | width | Image width in pixels |
| 6 | 2 | `i16` | height | Image height in pixels |
| 8 | 1 | `u8` | has_cmap | Export-tool flag, not read at runtime. Set to 1 when the image has an embedded CMAP palette (always co-occurs with IFHD animated images). Per UESP: "images that have 1 are all animated effects such as fire and water." |
| 9 | 1 | `u8` | export_flags | Export-tool metadata. Values: 0, 1, or 9. Packed into the same u16 as `has_cmap` during export. Purpose within the build pipeline unknown. |
| 10 | 4 | — | reserved | Always 0 |
| 14 | 2 | `i16` | frame_count | 1 = static, 2–16 = animated |
| 16 | 2 | `i16` | anim_delay | **Read at runtime.** Animation frame duration in milliseconds. Converted to DOS PIT timer ticks via `round(anim_delay × 18.2 / 1000)`; clamped to minimum 1. Typical value 71 → 1 tick (~55 ms); value 500 → 9 ticks (~495 ms). Range: 0–500. |
| 18 | 4 | — | reserved | Always 0 |
| 22 | 2 | `u16` | tex_scale | **Read at runtime as a single LE `u16`.** 8.8 fixed-point texture coordinate scale factor: `scale = tex_scale / 256.0`. Default `0x0100` (= 1.0, neutral) is substituted when zero. Referenced during polygon rendering but does **not** participate in UV coordinate normalization (UVs are normalized by `16 × texture_dimension` alone). Exact runtime effect is unverified. Known values: `0` (defaulted to 1.0), `128` (scale 0.5), `163` (scale ~0.637), `256` (scale 1.0), `512` (scale 2.0). Previously documented as two separate bytes (`effect_id` / `effect_param`); they are the low and high bytes of this single fixed-point field. |
| 24 | 2 | `i16` | data_encoding | Pixel data encoding mode. Selects which compression method the DATA subrecord uses. Known values: 0 = raw uncompressed, 4 = animated offset table. Values 1–3 are engine-supported but unused. |

### `CMAP` — Embedded Palette (768 bytes)

256 × RGB triplets (3 bytes each, values 0–255). Same layout as [COL files](COL.md).

Optional — always co-occurs with `IFHD` (animated images). When absent, the image uses an external `.COL` palette file.

### `DATA` — Pixel Data

**Static images** (`BSIF` present, `frame_count == 1`):

Payload is `width × height` bytes of 8-bit indexed color, row-major, top-to-bottom. Each byte is a palette index (0–255). **Index 0 = transparent.**

**Animated images** (`IFHD` present, `frame_count > 1`):

Payload starts with an offset table of `height × frame_count` LE u32 entries. Each entry is a byte offset from the start of the DATA payload to the first byte of that row. Rows can be shared across frames (identical rows point to the same data).

```
offset_table[frame * height + row] → byte offset to row data (width bytes)
```

### `END` — Record Terminator

4-byte tag `"END "` (with trailing space). No size field. Followed by 4 zero bytes.

## Pixel Decoding

```
for each pixel byte:
  if byte == 0:    → transparent (alpha = 0)
  else:            → palette[byte] as RGB (alpha = 255)
```

Palette values are raw 8-bit RGB (0–255). No gamma correction needed.

## Palette Selection

When decoding pixel data, palette lookup order is:

1. **External `.COL` palette** — the scene-level palette file passed to the converter (e.g. `ISLAND.COL`). Which COL file to use is determined per-scene by the game engine.
2. **Embedded `CMAP`** — the 256-color palette stored inside the image record (only present in animated `IFHD` images).
3. **Grayscale fallback** — if neither is available, each index maps to a gray value.

## How 3D Models Reference Textures

Each face in a [3D model](models/3d.md) encodes a `texture_id` and `image_id`:

- `texture_id` → selects the TEXBSI.### file number
- `image_id` → selects the sub-image within that file (0-indexed, matching the 3-digit suffix in the image name)

See [3D.md — Texture Decoding](models/3d.md#texture-decoding-v40--v50) for the BCD encoding.

## External References

- [UESP: Mod:TEXBSI File Format](https://en.uesp.net/wiki/Mod:TEXBSI_File_Format) — community byte-layout notes for TEXBSI records and subrecords.
- [uesp/redguard-3dfiletest `RedguardTexBsiFile.cpp`](https://github.com/uesp/redguard-3dfiletest/blob/master/3DFileTest/Common/RedguardTexBsiFile.cpp)
- [RGUnity/redguard-unity `RGTEXBSIFile.cs`](https://github.com/RGUnity/redguard-unity/blob/master/Assets/Scripts/RGFileImport/RGGFXImport/RGTEXBSIFile.cs)
- [RGUnity/redguard-unity `RGBSIFile.cs`](https://github.com/RGUnity/redguard-unity/blob/master/Assets/Scripts/RGFileImport/RGGFXImport/RGBSIFile.cs)
