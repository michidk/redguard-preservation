# LBM packed bitmaps

LBM files store IFF bitmap forms containing a header, palette, and pixel data.

`gui.lbm` uses the `PBM ` form type. Its pixels are packed bytes rather than
the separate bitplanes used by `ILBM`.

## Overall Structure

```text
[FORM header — container size and PBM form type]
[BMHD chunk — dimensions and pixel encoding]
[CMAP chunk — RGB palette]
[Additional chunks — application settings, color ranges, and thumbnail]
[BODY chunk — scanline pixel data]
```

All multibyte integers are big-endian.

## IFF container

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| `0x00` | 4 | `[u8; 4]` | tag | `FORM` |
| `0x04` | 4 | `u32` | size | Bytes following this size field |
| `0x08` | 4 | `[u8; 4]` | form_type | `PBM `, including the trailing space |

Each child chunk begins with a four-byte tag and a big-endian `u32` payload
size. Odd-sized payloads have one padding byte. The size excludes this padding.
Unknown chunks remain independent of pixel decoding.

## BMHD bitmap header

The header payload is 20 bytes.

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| `+0x00` | 2 | `u16` | width | Visible pixel width |
| `+0x02` | 2 | `u16` | height | Visible pixel height |
| `+0x04` | 2 | `i16` | x | Authored X position |
| `+0x06` | 2 | `i16` | y | Authored Y position |
| `+0x08` | 1 | `u8` | depth | Bits per pixel; 8 |
| `+0x09` | 1 | `u8` | masking | 0 for opaque pixels, 2 for a transparent palette index |
| `+0x0A` | 1 | `u8` | compression | 0 for raw rows, 1 for ByteRun1 |
| `+0x0B` | 1 | `u8` | reserved | Reserved byte |
| `+0x0C` | 2 | `u16` | transparent_color | Transparent index when masking is 2 |
| `+0x0E` | 1 | `u8` | x_aspect | Horizontal pixel aspect |
| `+0x0F` | 1 | `u8` | y_aspect | Vertical pixel aspect |
| `+0x10` | 2 | `i16` | page_width | Authored page width |
| `+0x12` | 2 | `i16` | page_height | Authored page height |

## Palette and pixel data

`CMAP` contains three-byte RGB entries. Each component uses its full eight-bit
range. `BODY` stores rows from top to bottom. Each row is padded to an even
byte count; padding is excluded from the visible image.

ByteRun1 interprets each control byte as signed:

| Control | Operation |
|---|---|
| 0 through 127 | Copy the next `control + 1` bytes |
| -127 through -1 | Repeat the next byte `1 - control` times |
| -128 | No operation |

Each row decodes independently. With masking zero, palette index zero is opaque.

## Additional chunks

`gui.lbm` also contains `DPPS`, `CRNG`, and `TINY` chunks. They do not change the
stored `BODY` pixels. A `CRNG` payload is eight bytes:

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| `+0x00` | 2 | `u16` | reserved | Reserved word |
| `+0x02` | 2 | `u16` | rate | Color-cycle rate |
| `+0x04` | 2 | `u16` | flags | Color-cycle flags |
| `+0x06` | 1 | `u8` | low | First palette index in the range |
| `+0x07` | 1 | `u8` | high | Last palette index in the range |

The static image uses the stored palette. See also [PCX images](PCX.md) and
[COL palettes](COL.md).

## External References

- [LBM and PBM container layout](https://moddingwiki.shikadi.net/wiki/LBM_Format)
- [Amiga IFF bitmap header and ByteRun1 encoding](https://wiki.amigaos.net/wiki/ILBM_IFF_Interleaved_Bitmap)
