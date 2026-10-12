# ANM animations

ANM files store Deluxe Paint animations as indexed bitmap deltas in LPF pages.

This is the `LPF ` animation format, distinct from IFF `ANIM` containers.
See [LBM](LBM.md) for IFF bitmaps.

## Overall Structure

```text
[LPF header — page counts, dimensions, encoding, and frame rate]
[Color-cycle records — authored palette ranges]
[Palette — 256 BGRX entries]
[Page table — 256 page descriptors]
[Large pages — record sizes and RunSkipDump bitmap records]
```

All multibyte integers are little-endian.

## Header and palette

The header occupies 128 bytes.

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| `0x00` | 4 | `[u8; 4]` | signature | `LPF ` |
| `0x04` | 2 | `u16` | max_pages | Page-table capacity, 256 |
| `0x06` | 2 | `u16` | page_count | Number of active pages |
| `0x08` | 4 | `u32` | record_count | Total records, including the loop delta |
| `0x0C` | 2 | `u16` | max_records_per_page | Authored record capacity |
| `0x0E` | 2 | `u16` | page_table_offset | Absolute table position |
| `0x10` | 4 | `[u8; 4]` | content_type | `ANIM` |
| `0x14` | 2 | `u16` | width | Pixel width |
| `0x16` | 2 | `u16` | height | Pixel height |
| `0x18` | 1 | `u8` | variant | Bitmap variant, zero |
| `0x19` | 1 | `u8` | version | Format version |
| `0x1A` | 1 | `u8` | has_last_delta | Final record is a loop delta |
| `0x1B` | 1 | `u8` | last_delta_valid | Loop delta validity flag |
| `0x1C` | 1 | `u8` | pixel_type | Indexed eight-bit pixels, zero |
| `0x1D` | 1 | `u8` | compression | RunSkipDump, 1 |
| `0x1E` | 1 | `u8` | other_records_per_frame | Additional record types, zero |
| `0x1F` | 1 | `u8` | bitmap_type | Single bitmap, 1 |
| `0x20` | 32 | `[u8; 32]` | record_types | Record-type fields |
| `0x40` | 4 | `u32` | frame_count | Authored frame-count field |
| `0x44` | 2 | `u16` | frames_per_second | Playback rate |
| `0x46` | 58 | `[u8; 58]` | reserved | Reserved header bytes |

Sixteen eight-byte color-cycle records follow at `0x80`. The palette begins at
`0x100`, with 256 entries in blue, green, red, unused order. Pixel index zero
is opaque.

## Page table and large pages

Each page descriptor is six bytes:

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| `+0x00` | 2 | `u16` | base_record | First record number in the page |
| `+0x02` | 2 | `u16` | record_count | Low 14 bits contain the record count; high bits mark continuation records |
| `+0x04` | 2 | `u16` | data_size | Total record payload bytes |

Page `i` begins at `page_table_offset + 256 * 6 + i * 65536`. Its header repeats
the descriptor, followed by a reserved word, one `u16` size per record, and the
record payloads. The last physical page can end before its 64 KiB boundary.
Playback follows record numbers rather than physical page order.

## Bitmap records

A bitmap record begins with byte `0x42` (`B`), a zero padding flag, and a
two-byte `unknown_02` field. RunSkipDump operations follow. Each record starts
its output cursor at pixel zero; skipped pixels retain the preceding frame.

| Control | Operation |
|---|---|
| `0x01` through `0x7F` | Copy that many following pixel indices |
| `0x00` | Read an eight-bit count and index; repeat the index |
| `0x81` through `0xFF` | Skip the low-seven-bit count |
| `0x80` | Read a little-endian extended control word |

Extended words select these operations:

| Word | Operation |
|---|---|
| `0x0000` | End the record |
| `0x0001` through `0x3FFF` | Skip that many pixels |
| `0x4001` through `0x7FFF` | Skip the low-14-bit count plus `0x4000` pixels |
| `0x8001` through `0xBFFF` | Copy the low-14-bit count of following indices |
| `0xC001` through `0xFFFF` | Repeat the following index for the low-14-bit count |
| `0x4000`, `0xC000` | No operation |

`0x8000` is invalid. An eight-bit repeat count of zero is also a no-op, but
still consumes its index byte.

## Playback and looping

`gui.anm` plays at 10 frames per second. Its final record converts the last
display frame back to the first. This loop delta is excluded from the displayed
frame sequence. The authored frame-count field includes that record.

## External References

- [FFmpeg LPF header and page reader](https://github.com/FFmpeg/FFmpeg/blob/master/libavformat/anm.c)
- [FFmpeg RunSkipDump decoder](https://github.com/FFmpeg/FFmpeg/blob/master/libavcodec/anm.c)
