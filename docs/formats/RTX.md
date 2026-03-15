# RTX Dialogue Audio Format

Container format used for dialogue strings and audio assets (for example `ENGLISH.RTX`).

## Overview

`RTX` stores two payload kinds under a common chunk/index system:

- **String-only entries** (ASCII text payload)
- **Audio entries** (string metadata + fixed audio header + audio bytes)

The file uses:

- per-chunk on-disk headers (`tag` + big-endian payload size)
- a footer that points to a central index table

All values below are validated against `ENGLISH.RTX`.

## File Layout

```
[chunk records ...]
[index table]
[footer]
```

### Footer (12 bytes, file end)

| Offset (from EOF) | Size | Type | Name | Description |
|---|---|---|---|---|
| -12 | 4 | ASCII | footer_tag | Always `RNAV` |
| -8  | 4 | u32 LE | index_offset | Absolute file offset of index table |
| -4  | 4 | u32 LE | index_count | Number of index entries |

For `ENGLISH.RTX`:

- `footer_tag = RNAV`
- `index_offset = 184629473`
- `index_count = 4866`

## Chunk Record (on disk)

Every payload in the data region has an 8-byte chunk header before it:

| Offset | Size | Type | Endian | Name | Description |
|---|---|---|---|---|---|
| 0x00 | 4 | ASCII | — | tag | 4-character chunk label |
| 0x04 | 4 | u32 | BE | payload_size | Size of payload bytes that follow |
| 0x08 | var | `[u8]` | — | payload | Entry payload |

## Index Table

The index is an array of 12-byte entries. Each entry points to one chunk payload.

| Offset | Size | Type | Endian | Name | Description |
|---|---|---|---|---|---|
| 0x00 | 4 | ASCII | — | tag | Same tag as the chunk header |
| 0x04 | 4 | u32 | LE | payload_offset | Absolute file offset of payload (not header) |
| 0x08 | 4 | u32 | LE | payload_size | Payload byte size |

Validation notes (all 4866 entries):

- `payload_offset - 8` points to a chunk header with matching `tag`
- header `payload_size` (big-endian) matches indexed `payload_size`
- all indexed ranges are in bounds (`offset + size <= file_size`)
- entries are ordered by descending payload offset

## Payload Types

Payload type is identified by byte `payload[1]`:

- `0` = string-only entry
- `1` = audio entry

### String-Only Payload (`payload[1] = 0`)

| Offset | Size | Type | Endian | Name | Description |
|---|---|---|---|---|---|
| 0x00 | 1 | u8 | — | kind | Always `0` |
| 0x01 | 1 | u8 | — | subtype | Always `0` for text entries |
| 0x02 | 2 | u16 | LE | string_len | Byte length of ASCII text |
| 0x04 | 2 | u16 | LE | reserved | Always `0` |
| 0x06 | var | `[u8]` | — | text | ASCII text bytes, no terminator |

Payload size rule:

`payload_size = 6 + string_len`

### Audio Payload (`payload[1] = 1`)

| Offset | Size | Type | Endian | Name | Description |
|---|---|---|---|---|---|
| 0x00 | 1 | u8 | — | kind | Always `0` |
| 0x01 | 1 | u8 | — | subtype | Always `1` for voice entries |
| 0x02 | 2 | u16 | LE | string_len | Byte length of ASCII label |
| 0x04 | 2 | u16 | LE | reserved | Always `0` |
| 0x06 | var | `[u8]` | — | label | ASCII label bytes, no terminator |
| 0x06+N | 27 | struct | — | audio_header | Audio metadata (below), `N = string_len` |
| 0x21+N | var | `[u8]` | — | audio_data | Raw PCM audio bytes |

Audio payload size rule:

`payload_size = 6 + string_len + 27 + audio_length`

#### Audio Header (27 bytes)

All fields little-endian unless noted.

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| 0x00 | 4 | u32 | type_id | `0` = 8-bit mono, `1` = 16-bit mono |
| 0x04 | 4 | u32 | bit_depth | `0` = 8-bit, `1` = 16-bit |
| 0x08 | 4 | u32 | sample_rate | `11025` or `22050` |
| 0x0C | 1 | u8 | level_0c | Always `100` |
| 0x0D | 1 | i8 | loop_flag | Always `0` |
| 0x0E | 4 | u32 | loop_offset | Always `0` |
| 0x12 | 4 | u32 | loop_end | Always `0xFFFFFFFF` |
| 0x16 | 4 | u32 | audio_length | Byte length of `audio_data` |
| 0x1A | 1 | u8 | reserved_1a | Always `0` |

## Notes

- Tags are 4-byte IDs and are not globally reused in `ENGLISH.RTX`.
- A few tags include punctuation (for example `#bon`, `?vql`).
- The index points to payload starts; on-disk chunk headers are always 8 bytes earlier.

## Related Formats

- [SFX](SFX.md) — sound effects container. Uses the same 27-byte audio header structure (offsets 0x00–0x1A) as RTX audio entries. RTX stores voice clips; SFX stores sound effects.

## Redguard Preservation CLI

### Read

`cargo run -- read ENGLISH.RTX` parses the file and prints a per-entry summary: tag, type (TEXT or AUDIO), audio format, sample rate, duration, and a label preview.

### Convert

`cargo run -- convert ENGLISH.RTX -o output_dir/` extracts all audio entries as individual `.wav` files (named by 4-character tag, e.g. `zbza.wav`) and writes an `index.json` sidecar containing metadata for all 4866 entries (both text-only and audio).

Validated against `ENGLISH.RTX`: 3933 `.wav` files + 933 text entries in `index.json`.

## External References

- [RGUnity/redguard-unity `RGRTXFile.cs`](https://github.com/RGUnity/redguard-unity/blob/ab09f557050a4a52591bedbb3445f0cbd25ae1af/Assets/Scripts/RGFileImport/RGGFXImport/RGRTXFile.cs) | RTX container reader
- [Dillonn241/redguard-mod-manager `RtxDatabase.java`](https://github.com/Dillonn241/redguard-mod-manager/blob/08dab1c413a73433a9c04b2e9e6a5bfb92b7bc3f/src/redguard/RtxDatabase.java) | RTX read/write with round-trip support
- [Dillonn241/redguard-mod-manager `RtxEntry.java`](https://github.com/Dillonn241/redguard-mod-manager/blob/08dab1c413a73433a9c04b2e9e6a5bfb92b7bc3f/src/redguard/RtxEntry.java) | RTX entry structure and audio format definitions
- [UESP: Redguard Console](https://en.uesp.net/wiki/Redguard:Console) (script command context)
