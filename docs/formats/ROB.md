# ROB File Format

Binary container format. Holds multiple 3D model segments — either embedded inline or as references to external `.3DC` files.

ROB stores model geometry buckets, not per-instance scene placement transforms. World/object placement comes from scene files (RGM), which reference these models.

## Overall Structure

```
[Header — 20 bytes]
[Segment 0 — 80-byte header + data]
[Segment 1 — 80-byte header + data]
...
[Segment N-1]
[Footer — 4 bytes: "END "]
```

Some fields use big-endian encoding (a remnant of the original Sega Saturn development), while most use little-endian. Endianness is noted per field.

## Header (20 bytes)

| Offset | Size | Type | Endian | Name | Description |
|---|---|---|---|---|---|
| 0x00 | 4 | `[u8; 4]` | — | magic | `"OARC"` — possibly "Object ARChive"; exact expansion unknown. |
| 0x04 | 4 | `u32` | BE | unused_04 | Always 4. Unused at runtime — the engine reads the file but never dereferences or tests this field. Likely a build-tool artifact. |
| 0x08 | 4 | `u32` | LE | num_segments | Number of segments. |
| 0x0C | 4 | `[u8; 4]` | — | magic2 | `"OARD"` — possibly "Object ARchive Data"; exact expansion unknown. |
| 0x10 | 4 | `u32` | BE | payload_size | File size minus 24 (= file size - 20-byte header - 4-byte footer). |

## Segment Header (80 bytes)

Each segment has a fixed 80-byte header followed by `data_size` bytes of payload.

| Offset | Size | Type | Endian | Name | Description |
|---|---|---|---|---|---|
| 0x00 | 4 | `u32` | LE | total_size | Data size + 80 (total segment size including header). |
| 0x04 | 8 | `[u8; 8]` | — | name | Segment name, ASCII null-padded. Model name or external `.3DC` filename stem. |
| 0x0C | 2 | `u16` | LE | segment_type | 0 = embedded 3D data, 512 = external `.3DC` reference. See below. |
| 0x0E | 2 | `u16` | LE | segment_flags | Render mode flags. Only the **high byte** (at file offset 0x0F) is used at runtime — it becomes a render mode selector. A value of 0 defaults to 0xFF (normal rendering). See [Segment Flags](#segment-flags-0x0e) below. |
| 0x10 | 1 | `u8` | — | segment_attribs | Per-segment attribute flags. Only this single byte is read at runtime. Bit 1 (0x02) triggers texture pre-loading. Value 0x40 marks special objects (inventory items, shop objects). See [Segment Attributes](#segment-attributes-0x10) below. |
| 0x11 | 3 | — | — | face_count_low | Build tool artifact: the last byte (0x13) equals `face_count mod 256`. Not read at runtime. |
| 0x14 | 4 | `u32` | BE | unused_14 | Unused — never read at runtime. Most commonly 1 (3,690 segments), but other values exist. |
| 0x18 | 4 | `u32` | — | reserved_18 | Always 0. |
| 0x1C | 4 | `u32` | LE | bbox_extent_x | Bounding box total X extent. |
| 0x20 | 4 | `u32` | LE | bbox_extent_y | Bounding box total Y extent. |
| 0x24 | 4 | `u32` | LE | bbox_extent_z | Bounding box total Z extent. |
| 0x28 | 4 | `u32` | — | reserved_28 | Always 0. |
| 0x2C | 4 | `u32` | — | reserved_2C | Always 0. |
| 0x30 | 4 | `u32` | — | reserved_30 | Always 0. |
| 0x34 | 4 | `u32` | LE | bbox_positive_x | Positive X extent from center. |
| 0x38 | 4 | `u32` | LE | bbox_positive_y | Positive Y extent from center. |
| 0x3C | 4 | `u32` | LE | bbox_positive_z | Positive Z extent from center. |
| 0x40 | 4 | `u32` | LE | bbox_negative_x | Negative X extent from center. |
| 0x44 | 4 | `u32` | LE | bbox_negative_y | Negative Y extent from center. |
| 0x48 | 4 | `u32` | LE | bbox_negative_z | Negative Z extent from center. |
| 0x4C | 4 | `u32` | LE | data_size | Byte count of the data payload that follows. 0 for external references. |

### Bounding Box Invariant

`bbox_extent == bbox_positive + bbox_negative` for all three axes.

For symmetric models: `bbox_positive == bbox_negative` (center at origin). For asymmetric models, the difference encodes the center offset.

### Segment Types

| Type | Description |
|---|---|
| 0 | Embedded 3D model data. Payload is a complete [3D file](models/3d.md) (v5.0 format). |
| 256 | Embedded 3D model data (menu-specific). Only in `MENU.ROB`. Structurally identical to type 0 — payload is a complete [3D file](models/3d.md). Uses versions v4.0 and v4.02 (v4.02 is unique to these segments). |
| 512 | External reference. `name` is the `.3DC` filename stem (e.g. `"CYRSA001"` → `CYRSA001.3DC`). `data_size` is 0. |

### Segment Flags (0x0E)

The engine reads only the **high byte** (file offset 0x0F) of this u16 field. The low byte is always 0x00 and is ignored. The high byte is stored as a render mode selector in the model's internal data — a value of 0 defaults to 0xFF (standard rendering).

| Value | High byte | Names | Render mode |
|---|---|---|---|
| 0x0000 | 0x00 → 0xFF | (all normal) | Standard (default) |
| 0x8C00 | 0x8C | DR_WA01, DR_WA02, LH_MIRR | Transparency / mirror |
| 0xC800 | 0xC8 | WATERWAT | Water |
| 0x8000 | 0x80 | VR_OHT | Stored as render-mode metadata |
| 0x5A00 | 0x5A | BEAMA001 | Beam / light effect |

### Segment Attributes (0x10)

A single byte of per-segment attribute flags.

| Value | Segments | Meaning |
|---|---|---|
| 0x00 | (all normal) | No special attributes |
| 0x02 | PALMTR01–04 (CAVERNS, EXTPALAC, ISLAND) | Texture pre-load trigger (bit 1). Engine calls a texture pre-caching function for all face textures in this segment. |
| 0x40 | SS_OBJ01–06, IGRING, IWATER1 (shop items, inventory) | Special object flag (bit 6). Stored as model metadata. |

## Segment Data

For `segment_type == 0` (embedded): the data payload is a complete [3D model file](models/3d.md) starting with its own 64-byte header. Parse with the standard 3D parser.

For `segment_type == 256` (menu-specific embedded): same as type 0 — payload is a complete 3D model file. Parse identically. See [MENU.ROB Segments](#menurob-segments) below.

For `segment_type == 512` (external reference): no data payload. Load the referenced `.3DC` file from the asset directory using `name` as the filename stem. External references are exclusively `.3DC` (animated models) — static `.3D` geometry is always embedded inline as type 0 segments, never referenced externally.

### MENU.ROB Segments

`MENU.ROB` is the only ROB file containing type 256 segments. It holds 3D models used for the game's menu screens.

| # | Name | Type | Size | Version | Notes |
|---|---|---|---|---|---|
| 0 | MENUA001 | 256 | 79,328 | v4.0 | Menu character model (also exists as standalone `MENUA001.3DC` in `/fxart`) |
| 1 | MB_TABLE | 0 | 886 | v5.0 | Small prop |
| 2 | MB_PG01 | 256 | 42,208 | v4.02 | Menu page model |
| 3 | MB_PG02 | 256 | 42,208 | v4.02 | Menu page model (same size as PG01) |
| 4 | MB_PG03 | 256 | 42,208 | v4.02 | Menu page model (same size as PG01) |
| 5 | SCROLL | 0 | 8,136 | v5.0 | Scroll decoration prop |

Version `v4.02` appears only in these three MB_PG segments — it is not found in any other ROB file or standalone 3D/3DC file.

## Footer

4-byte ASCII marker: `"END "` (with trailing space). Always present.

## External References

- [UESP: Mod:ROB File Format](https://en.uesp.net/wiki/Mod:ROB_File_Format) — high-level ROB structure and historical notes.
- [RGUnity/redguard-unity `RGROBFile.cs`](https://github.com/RGUnity/redguard-unity/blob/master/Assets/Scripts/RGFileImport/RGGFXImport/RGROBFile.cs) — ROB segment parsing.
