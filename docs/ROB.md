# ROB File Format Documentation

There are 31 ROB files, found in the `fxart` directory.
They hold multiple 3D meshes.

## Format

The file is a binary file (little endian) composed of a header, a list of segments, and an optional footer.

### Header

The header is 20 bytes long.

| Address | Size | Data Type | Description |
|---------|------|------------------|-------------|
| 0x00    | 4    | `[u8; 4]`        | Magic number: `OARC` |
| 0x04    | 4    | `u32`            | Unknown |
| 0x08    | 4    | `u32`            | Number of segments in the file |
| 0x0C    | 4    | `[u8; 4]`        | Magic number: `OARD` |
| 0x10    | 4    | `u32`            | Unknown |

### Segments

Following the header, there are `num_segments` segment blocks. Each segment consists of an 80-byte header followed by variable-length data.

#### Segment Header

| Offset | Size | Data Type | Description |
|--------|------|------------------|-------------|
| 0x00   | 4    | `u32`            | Unknown |
| 0x04   | 8    | `[u8; 8]`        | Segment name (ASCII, null-padded) |
| 0x0C   | 4    | `u32`            | Segment type indicator. See notes below. |
| 0x10   | 60   | `[u32; 15]`      | Unknown data |
| 0x4C   | 4    | `u32`            | Size of the segment data that follows this header. |

#### Segment Data

The segment data immediately follows the segment header. Its length is specified by the `size` field.

The interpretation of the segment depends on the `unknown2` field in the segment header:
- If `unknown2 == 0` and `size > 0`, the segment's `data` contains embedded 3D model data (in 3DC format).
- If `unknown2 == 512`, the segment's `name` points to an external `.3DC` file, and the `data` block is empty.

### Footer

The file ends with a 4-byte marker: `END `.
