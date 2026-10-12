# PCX images

PCX files store palette-indexed raster images with scanline compression.

`SKY_61.PCX` is an 8-bit, single-plane image with a trailing 256-color palette.

## Overall Structure

```text
[PCX header — bounds, color depth, encoding, and scanline stride]
[Image data — raw or run-length encoded scanlines]
[Palette marker — 0C]
[Palette — 256 RGB triples]
```

Multibyte header integers are little-endian.

## Header

The header occupies 128 bytes.

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| `0x00` | 1 | `u8` | manufacturer | `0x0A` |
| `0x01` | 1 | `u8` | version | Format revision; 5 in `SKY_61.PCX` |
| `0x02` | 1 | `u8` | encoding | 0 for raw bytes, 1 for RLE |
| `0x03` | 1 | `u8` | bits_per_pixel | Bits per pixel per plane; 8 |
| `0x04` | 2 | `u16` | x_min | Left bound |
| `0x06` | 2 | `u16` | y_min | Top bound |
| `0x08` | 2 | `u16` | x_max | Inclusive right bound |
| `0x0A` | 2 | `u16` | y_max | Inclusive bottom bound |
| `0x0C` | 2 | `u16` | horizontal_dpi | Horizontal resolution |
| `0x0E` | 2 | `u16` | vertical_dpi | Vertical resolution |
| `0x10` | 48 | `[u8; 48]` | header_palette | Legacy 16-color palette |
| `0x40` | 1 | `u8` | reserved | Reserved byte |
| `0x41` | 1 | `u8` | planes | Number of color planes; 1 |
| `0x42` | 2 | `u16` | bytes_per_line | Decoded stride for each plane |
| `0x44` | 2 | `u16` | palette_info | Palette interpretation field |
| `0x46` | 2 | `u16` | horizontal_screen_size | Screen width field |
| `0x48` | 2 | `u16` | vertical_screen_size | Screen height field |
| `0x4A` | 54 | `[u8; 54]` | reserved_tail | Reserved bytes |

Width is `x_max - x_min + 1`; height is `y_max - y_min + 1`.
Each decoded scanline contains `bytes_per_line` bytes. Bytes beyond the visible
width are padding and do not produce pixels.

## Run-length encoding

A byte below `0xC0` is a literal pixel index. A byte with both high bits set
encodes a repeat count in its low six bits; the following byte is the repeated
index. Counts range from 1 through 63. A literal index of `0xC0` or above uses
a one-byte repeat. Runs end within the decoded scanline boundary.

## Palette

The final 769 bytes contain marker `0x0C` and 256 three-byte RGB entries.
Components use their full eight-bit range. Pixel indices select these colors;
index zero is opaque. The legacy header palette does not supply the colors
for this layout. See also [COL palettes](COL.md).

## External References

- [Pillow PCX header and palette reader](https://github.com/python-pillow/Pillow/blob/main/src/PIL/PcxImagePlugin.py)
