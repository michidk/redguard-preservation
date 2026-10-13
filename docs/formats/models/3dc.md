# 3DC Animated Model File Format

Animated variant of the [3D model format](3d.md). The fixed header is shared with 3D models; animation directories depend on the renderer generation.

## Differences from .3D

| Aspect | .3D | .3DC |
|---|---|---|
| Frames | 0 in software static files, 1 in Glide static files | Multiple animation frames, or a single static frame |
| frame_type (frame 0) | 0 | 2, 4, or 8 |
| Animated frame data | None | Frames 1+ with compressed or full-precision geometry |
| Section4 (SubObject BVH) | Present in v5.0 | Absent in Glide animations; software uses a different auxiliary layout |
| Vertex-normal indirection table | Present | Always absent (offset=0) |
| Version | v4.0 or v5.0 | v4.0 (`fxart/`), v2.6 or v2.7 (`3dart/`) |

All shared structures (header, face data, vertex coordinates, face normals, vertex normals, texture encoding) are documented in [3D.md](3d.md).

## Glide animation frame data

The frame data array at `offset_frame_data` contains `num_frames` × 16-byte records (see [3D.md — Frame Data](3d.md#frame-data) for the record layout).

Frame 0 always points to the base geometry (same as `.3D` files). Frames 1+ contain encoded vertex data and face normals. Compressed vertices are deltas from frame zero, not absolute positions.

### frame_type Values

| Value | Meaning | Frame 1+ vertex encoding | Frame 1+ normal encoding |
|---|---|---|---|
| 2 | Compressed animation | i16 × 3 (6 bytes/vertex) | 10-10-10-2 packed (4 bytes/face) |
| 4 | Full-precision animation | i32 × 3 (12 bytes/vertex) | 10-10-10-2 packed (4 bytes/face) |
| 8 | Static (single-frame `.3DC`) | N/A | N/A |

`frame_type` is only meaningful in frame 0's record. Frame 1+ records always have `frame_type = 0`.

### Compressed vertex reconstruction

The low four bits of frame zero's type select the vertex encoding. For type 2,
each later signed i16 component is subtracted from the corresponding decoded
frame-zero component. When type flag `0x10` is clear, divide the signed delta by
256 before subtraction. When it is set, subtract the signed delta in whole model
units. Store the resulting absolute coordinate at single precision. Each frame
uses frame zero as its base; deltas are not accumulated between frames.

Type 1 uses signed byte deltas subtracted in whole model units. Type 4 stores
absolute signed i32 components scaled by 1/256. Frame zero itself uses decoded
base geometry regardless of the later-frame encoding.

### 10-10-10-2 Packed Normal Format

Used for face normals in frames 1+:

```
Bits  0- 9: nx (10-bit signed, subtract 1024 if >= 512)
Bits 10-19: ny (10-bit signed, subtract 1024 if >= 512)
Bits 20-29: nz (10-bit signed, subtract 1024 if >= 512)
Bits 30-31: unused (values: 0 and 3)
```

Each component is divided by 256.0 to produce the final normal vector.

The engine's normal decoder extracts the three 10-bit signed components via sign-extending shifts and discards bits 30–31:

```c
nx = (float)((packed << 22) >> 22) * scale;   // bits  0–9, sign-extended
ny = (float)((packed << 12) >> 22) * scale;   // bits 10–19, sign-extended
nz = (float)((packed <<  2) >> 22) * scale;   // bits 20–29, sign-extended
// bits 30–31 are shifted out by << 2 — never read
```

The values are 0 and 3. Parsers should mask or discard these bits.

## Section Layout (v4.0)

Same as [3D.md — Section Layout](3d.md#section-layout), but with animated frame data inserted and `.3D`-only sections absent:

1. **Face Data** — at 0x40
2. **Vertex Coordinates** — base frame positions
3. **Face Normals** — base frame normals
4. **Frame Data** — `num_frames` × 16-byte records
5. **Animated Frame Vertex/Normal Data** — frames 1+ geometry
6. **Vertex Normals** — per-vertex normals (f32 × 3)

## Software animation directory

In v2.6/v2.7, `offset_frame_data` points to a 24-byte animation header. It is not the first frame record. Values are little-endian.

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| 0x00 | 4 | `u32` | table_offset | Absolute file offset of the frame directory |
| 0x04 | 4 | `u32` | face_data_size | Byte length of shared face records |
| 0x08 | 4 | `u32` | unknown_08 | Auxiliary field |
| 0x0C | 2 | `u16` | flags | Bit 0 selects compressed frame data |
| 0x0E | 2 | `u16` | reference_frame | Frame with full i32 positions when compressed |
| 0x10 | 4 | `u32` | unknown_10 | Auxiliary field |
| 0x14 | 2 | `u16` | unknown_14 | Auxiliary field |
| 0x16 | 1 | `u8` | unknown_16 | Auxiliary field |
| 0x17 | 1 | `u8` | delta_shift | Shift applied to compressed position deltas |

The directory contains `num_frames` records. Compressed records occupy 12 bytes; uncompressed records occupy 16 bytes.

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| 0x00 | 4 | `u32` | vertex_offset | Absolute file offset of frame positions or deltas |
| 0x04 | 4 | `u32` | normal_offset | Absolute file offset of face normals |
| 0x08 | 4 | `u32` | auxiliary_offset | Absolute file offset of per-face auxiliary data |
| 0x0C | 4 | `u32` | unknown_0c | Present only in uncompressed records |

Uncompressed positions and face normals contain three signed i32 components, each scaled by 1/256.

In compressed animations, the reference frame contains full signed i32 positions. Other frames contain three signed i16 deltas per vertex. Reconstruct each raw component with wrapping i32 arithmetic:

```text
position = reference_position + (signed_delta << (delta_shift & 31))
```

Divide the reconstructed raw component by 256. Every frame uses the same reference frame. Deltas are added, not subtracted or accumulated from the preceding frame. Frame zero may itself be compressed.

Every compressed frame's face normals use signed 10-bit components in this order:

```text
Bits 20-29: x
Bits 10-19: y
Bits  0- 9: z
Bits 30-31: discarded
```

Sign-extend each component and divide it by 256. This component order differs from Glide's packed normals. Use decoded frame-zero positions and normals as the model's base geometry.

## External References

- [UESP: Mod:Model Files](https://en.uesp.net/wiki/Mod:Model_Files) — baseline notes for shared `.3D`/`.3DC` structures and version differences.
- [RGUnity/redguard-unity `RG3DFile.cs`](https://github.com/RGUnity/redguard-unity/blob/master/Assets/Scripts/RGFileImport/RGGFXImport/RG3DFile.cs) — 3D/3DC importer with animated-frame decoding.
