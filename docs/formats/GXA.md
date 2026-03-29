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
| 0x06 | 4 | `[i16; 2]` | unknown_06 | Two unknown frame fields. |
| 0x0A | 2 | `i16` | compression | Compression type (`0` = raw, `1` = RLE, `2` = LZHUF). |
| 0x0C | 6 | `[i16; 3]` | unknown_0c | Three unknown frame fields. |

### Pixel Decode Rules

- Palette index `0` is transparent (`RGBA = 0,0,0,0`).
- Non-zero indices map to `BPAL` RGB and use alpha `255`.
- Stored rows are vertically flipped in shipped assets; decoders flip Y during RGBA expansion.

### Compression Type 0 - Raw

Pixel data is `width * height` uncompressed bytes starting at offset `0x12`.

### Compression Type 1 - RLE

| Offset | Size | Type | Name | Description |
|---|---:|---|---|---|
| 0x12 | 4 | `u32` | compressed_size | Size of RLE-compressed pixel data. |
| 0x16 | compressed_size | `[u8; N]` | compressed_data | RLE-encoded pixel stream. |

RLE control byte behavior:

- `0x80..=0xFF`: repeat the next byte `(ctrl & 0x7F) + 1` times.
- `0x00..=0x7F`: copy the next `ctrl + 1` literal bytes.

### Compression Type 2 - LZHUF

| Offset | Size | Type | Name | Description |
|---|---:|---|---|---|
| 0x12 | 4 | `u32` | compressed_size | Size of LZHUF-compressed pixel data. |
| 0x16 | 4 | `u32` | uncompressed_size | Expected decompressed size (`width * height`). |
| 0x1A | compressed_size | `[u8; N]` | compressed_data | LZHUF bitstream (Okumura LZSS + adaptive Huffman, 4 KB window). |

LZHUF uses a 4096-byte sliding window initialized to `0x20`, with adaptive Huffman coding for both character and position codes. This matches the classic Okumura `lzhuf.c` algorithm, equivalent to LHA method `lh1`.

### Section Length Quirk

Some files store BBMP `data_length_be` as `file_size - 4` instead of the actual BBMP payload size. Parsers should clamp the BBMP section length to available data.

## External References

- [RGUnity `RGGXAFile.cs`](https://github.com/RGUnity/redguard-unity/blob/master/Assets/Scripts/RGFileImport/RGGFXImport/RGGXAFile.cs)
- [RGUnity `GraphicsConverter.cs`](https://github.com/RGUnity/redguard-unity/blob/master/Assets/Scripts/RGFileImport/RGGFXImport/GraphicsConverter.cs)
- [WORLD.INI config keys for GXA usage](../config/world-ini.md)
