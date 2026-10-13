# Software texture banks

`TEXTURE.###` files store indexed textures for the software renderer.

## Overall Structure

```text
[Header — entry count and bank name]
[Directory — 20-byte records]
[Image records — metadata and pixels]
```

The engine selects banks by a three-digit decimal suffix. Values are little-endian. Colors come from the world's [COL palette](COL.md).

## Header

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| 0x00 | 2 | `u16` | entry_count | Number of directory entries |
| 0x02 | 24 | `[u8; 24]` | name | NUL-terminated bank name, padded to 24 bytes |

## Directory

Each entry occupies 20 bytes, beginning at byte 26.

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| 0x00 | 1 | `u8` | unknown_00 | Auxiliary field |
| 0x01 | 1 | `u8` | color | Palette index when the image offset is zero |
| 0x02 | 4 | `u32` | image_offset | Absolute file offset of the image record; zero for solid colors |
| 0x06 | 14 | `[u8; 14]` | auxiliary | Auxiliary fields |

Banks 0 and 1 contain solid-color entries for the two halves of the palette. Image IDs are directory indices.

## Image record

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| 0x00 | 2 | `i16` | x_offset | Image X offset |
| 0x02 | 2 | `i16` | y_offset | Image Y offset |
| 0x04 | 2 | `u16` | width | Image width |
| 0x06 | 2 | `u16` | height | Image height |
| 0x08 | 2 | `u16` | flags | Bit 8 enables transparent palette index zero; bit 11 selects a 256-byte row stride for single-frame data |
| 0x0A | 4 | `u32` | record_size | Record size field |
| 0x0E | 4 | `u32` | data_offset | Pixel data offset relative to the image record |
| 0x12 | 2 | `u16` | unknown_12 | Auxiliary field |
| 0x14 | 2 | `u16` | frame_count | Number of frames; zero represents an empty record |
| 0x16 | 2 | `u16` | frame_delay | Animation frame duration in milliseconds |
| 0x18 | 2 | `i16` | scale_x | Stored X scale field |
| 0x1A | 2 | `i16` | scale_y | Stored Y scale field |

Data offsets may address shared pixel storage beyond the record-size field. Resolve them against the complete bank.

## Pixel data

Single-frame images contain raw palette indices. Rows have a stride of 256 bytes when flag bit 11 is set, otherwise the image width. Only `width` pixels from each row belong to the image.

Animated images begin with `frame_count` little-endian u32 offsets, each relative to the start of the pixel data. Each frame begins with its u16 width and height, followed by rows of run pairs:

```text
[Transparent count — u8]
[Literal count — u8]
[Literal palette indices — literal count bytes]
```

Repeat pairs until exactly the frame width has been emitted, then start the next row. Transparent runs emit index zero. A pair must advance the row and must not overrun its width.

Software models select these banks through their [v2.6/v2.7 texture fields](models/3d.md#v26--v27-face-record). The layout differs from [TEXBSI](TEXBSI.md).

## External References

- [Daggerfall Unity TextureFile.cs](https://github.com/Interkarma/daggerfall-unity/blob/master/Assets/Scripts/API/TextureFile.cs): related XnGine texture archive structures.
