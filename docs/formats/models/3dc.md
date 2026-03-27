# 3DC Animated Model File Format

Animated variant of the [3D model format](3d.md). Same binary layout — identical header and section structure — but with multiple animation frames.

## Differences from .3D

| Aspect | .3D | .3DC |
|---|---|---|
| Frames | Always 1 | 2+ (animated) |
| frame_type (frame 0) | 0 | 2, 4, or 8 |
| Animated frame data | None | Frames 1+ with compressed or full-precision geometry |
| Section4 (SubObject BVH) | Present (v5.0) | Always absent (offset=0, count=0) |
| Vertex-normal indirection table | Present | Always absent (offset=0) |
| Version | v4.0 or v5.0 | Always v4.0 |

All shared structures (header, face data, vertex coordinates, face normals, vertex normals, texture encoding) are documented in [3D.md](3d.md).

## Animation Frame Data

The frame data array at `offset_frame_data` contains `num_frames` × 16-byte records (see [3D.md — Frame Data](3d.md#frame-data) for the record layout).

Frame 0 always points to the base geometry (same as `.3D` files). Frames 1+ contain per-frame vertex positions and face normals in a compact encoding determined by `frame_type`:

### frame_type Values

| Value | Meaning | Frame 1+ vertex encoding | Frame 1+ normal encoding |
|---|---|---|---|
| 2 | Compressed animation | i16 × 3 (6 bytes/vertex) | 10-10-10-2 packed (4 bytes/face) |
| 4 | Full-precision animation | i32 × 3 (12 bytes/vertex) | 10-10-10-2 packed (4 bytes/face) |
| 8 | Static (single-frame `.3DC`) | N/A | N/A |

`frame_type` is only meaningful in frame 0's record. Frame 1+ records always have `frame_type = 0`.

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

The values (0 and 3) are likely packing artifacts — 3 (0b11) can result from sign extension during the build tool's encoding step. Parsers should mask or discard these bits.

## Section Layout

Same as [3D.md — Section Layout](3d.md#section-layout), but with animated frame data inserted and `.3D`-only sections absent:

1. **Face Data** — at 0x40
2. **Vertex Coordinates** — base frame positions
3. **Face Normals** — base frame normals
4. **Frame Data** — `num_frames` × 16-byte records
5. **Animated Frame Vertex/Normal Data** — frames 1+ geometry
6. **Vertex Normals** — per-vertex normals (f32 × 3)

## External References

- [UESP: Mod:Model Files](https://en.uesp.net/wiki/Mod:Model_Files) — baseline notes for shared `.3D`/`.3DC` structures and version differences.
- [RGUnity/redguard-unity `RG3DFile.cs`](https://github.com/RGUnity/redguard-unity/blob/master/Assets/Scripts/RGFileImport/RGGFXImport/RG3DFile.cs) — 3D/3DC importer with animated-frame decoding.
