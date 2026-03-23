# GXA bitmap archive format for UI and sky textures.

## Overall Structure

```
[Section header — 4-byte tag + BE u32 payload length]
[Section payload — tag-specific]
...
[END section — tag `END ` with no payload length field]
```

Sections are chunked and can appear in this observed order:

```
[BMHD — bitmap header (title + frame count)]
[BPAL — 256-color RGB palette]
[BBMP — bitmap frame records]
[END  — terminator]
```

## Section Header

| Offset | Size | Type | Name | Description |
|---|---:|---|---|---|
| 0x00 | 4 | `[u8; 4]` | tag | Chunk name (`BMHD`, `BPAL`, `BBMP`, `END `). |
| 0x04 | 4 | `u32` | data_length_be | Big-endian payload size. Not present for `END `. |

## BMHD Section

| Offset | Size | Type | Name | Description |
|---|---:|---|---|---|
| 0x00 | 22 | `[u8; 22]` | title | Null-terminated title/label. |
| 0x16 | 10 | `[u8; 10]` | unknown_16 | Unknown header bytes. |
| 0x20 | 2 | `i16` | num_images | Number of bitmap frames in `BBMP`. |

## BPAL Section

| Offset | Size | Type | Name | Description |
|---|---:|---|---|---|
| 0x00 | 768 | `[u8; 768]` | colors | 256 palette entries, each RGB triplet (`r,g,b`). |

## BBMP Section

`BBMP` stores `num_images` frame records back-to-back.

### Frame Record Header

| Offset | Size | Type | Name | Description |
|---|---:|---|---|---|
| 0x00 | 2 | `i16` | unknown_00 | Unknown frame field. |
| 0x02 | 2 | `i16` | width | Frame width in pixels. |
| 0x04 | 2 | `i16` | height | Frame height in pixels. |
| 0x06 | 12 | `[u8; 12]` | unknown_06 | Six unknown `i16` values. |
| 0x12 | `width*height` | `[u8; N]` | pixels | Indexed-color pixel data. |

### Pixel Decode Rules

- Palette index `0` is transparent (`RGBA = 0,0,0,0`).
- Non-zero indices map to `BPAL` RGB and use alpha `255`.
- Stored rows are vertically flipped in shipped assets; decoders flip Y during RGBA expansion.

## Redguard Preservation CLI

- `rgpre convert SYSTEM/STARTUP2.GXA -o out/startup_gxa/` extracts all frames to PNG and writes `metadata.json`.
- `rgpre read SYSTEM/STARTUP2.GXA` prints parsed title/frame-count summary.
- FFI: `rg_decode_gxa(data, len, frame)` returns one decoded RGBA frame (`width`, `height`, `frame_count=1`, `rgba_size`, `rgba`).

## External References

- [RGUnity `RGGXAFile.cs`](https://github.com/RGUnity/redguard-unity/blob/master/Assets/Scripts/RGFileImport/RGGFXImport/RGGXAFile.cs)
- [RGUnity `GraphicsConverter.cs`](https://github.com/RGUnity/redguard-unity/blob/master/Assets/Scripts/RGFileImport/RGGFXImport/GraphicsConverter.cs)
- [WORLD.INI config keys for GXA usage](../config/world-ini.md)
