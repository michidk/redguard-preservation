# Save-game containers

`SAVEGAME.SAV` and per-map `.TSG` files store saved engine state in tagged chunks.

## Overall Structure

```text
[SVGM — save metadata]
[Image and state chunks — length-prefixed payloads]
[END — four-byte terminator]
```

Chunk lengths are big-endian. Numeric fields within the `SVGM` payload are
little-endian. Chunks have no alignment padding.

| Offset | Size | Type | Endian | Name | Description |
|---|---|---|---|---|---|
| `0x00` | 4 | `[u8; 4]` | — | name | Four-byte chunk identifier. |
| `0x04` | 4 | `u32` | BE | data_length | Payload size, excluding the eight-byte header. |
| `0x08` | data_length | `[u8]` | — | data | Chunk payload. |

The terminal identifier is `END `, including its trailing space. It has no
length field or payload. An empty chunk still has an eight-byte header with
a zero length.

## SVGM metadata

Version `1.31` uses a 272-byte metadata payload. Offsets below are relative to
the payload, after the chunk header.

| Offset | Size | Type | Endian | Name | Description |
|---|---|---|---|---|---|
| `0x00` | 4 | `[u8; 4]` | — | version | Save-format version text, `1.31`. |
| `0x04` | 4 | `u32` | LE | unknown_04 | Engine compatibility value. |
| `0x08` | 4 | `u32` | LE | unknown_08 | Engine compatibility value. |
| `0x0C` | 128 | `[u8; 128]` | — | description | Slot display text, ending at the first NUL or the field boundary. |
| `0x8C` | 4 | `u32` | LE | world_id | Current world identifier, corresponding to [WORLD.INI](../config/world-ini.md). |
| `0x90` | 128 | `[u8; 128]` | — | bitmap_filename | Stored bitmap filename, ending at the first NUL or the field boundary. |

The engine checks the version and compatibility values before restoring a save.
The stored bitmap filename is separate from the `THMB` thumbnail and `SCR3`
screen-image chunks.

## State chunks

Save containers include global state and per-world state. Object and script
records depend on the corresponding [RGM](RGM.md) scene and runtime definitions.
They are not replacement scene assets.

`SMOB`, `SMRP`, `SMCO`, and `SMSH` hold object, rope, code, and shell state,
respectively. `SVCM` holds camera records, and `SMFM` holds family references.
Other state chunks include `SVGF`, `SMGR`, `SVRG`, `SVGV`, `SVG2`, `SVIT`,
`SVCB`, `SVMV`, and `SVMD`.

Temporary `.TSG` snapshots use the same container and metadata layout.
Their image chunks can be empty. The engine can restore these snapshots during
a world reload after initializing the player.

## Global flags

The `SVGF` payload stores the flag values and state bytes in installed global
flag order. Its payload length is `4 + 5 × count`. Numeric fields are
little-endian.

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| `0` | 4 | `u32` | count | Number of global flags. |
| `4` | `4 × count` | `[i32; count]` | values | Current global flag values. |
| `4 + 4 × count` | count | `[u8; count]` | states | Native flag kind and latch state. |

## General state and clock

Version `1.31` uses a 1,542-byte `SVGV` payload. It concatenates the retained
global fields without alignment padding. The clock fields below are four-byte
little-endian words. Offsets are relative to the payload.

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| `0x094` | 4 | `u32` | measurement_baseline | Last FPS measurement baseline in BIOS ticks. |
| `0x098` | 4 | `i32` | measurement_remainder | BIOS ticks retained toward the next measurement window. |
| `0x09C` | 4 | `u32` | measured_fps | Measured frame rate. |
| `0x0A0` | 4 | `u32` | next_scale | Next movement scale, with eight fractional bits. |
| `0x0A4` | 4 | `u32` | nominal_increment | Last computed whole nominal-tick increment. |
| `0x0A8` | 4 | `u32` | fine_increment | Last computed finer-clock increment; eight units per nominal tick. |
| `0x0AC` | 4 | `u32` | warmup | Remaining startup invocations. |
| `0x0B4` | 4 | `u32` | bios_sample | Last sampled BIOS tick. |
| `0x0B8` | 4 | `u32` | measurement_frames | Frames counted toward the next FPS measurement. |
| `0x0C0` | 4 | `u32` | smoothed_fps | Retained smoothed frame rate. |
| `0x0C4` | 4 | `u32` | frame_scale | Currently exposed movement scale, with eight fractional bits. |
| `0x0C8` | 4 | `u32` | exposed_nominal_ticks | Currently exposed whole nominal-tick increment. |
| `0x0CC` | 4 | `u32` | nominal_total | Whole nominal-tick accumulator. |
| `0x0D0` | 4 | `u32` | nominal_scaled_total | Nominal-tick accumulator with eight fractional bits. |
| `0x0D4` | 4 | `u32` | nominal_scaled_increment | Last change in the scaled nominal accumulator. |
| `0x0D8` | 4 | `u32` | exposed_fine_ticks | Currently exposed finer-clock increment; eight units per nominal tick. |
| `0x0DC` | 4 | `u32` | fine_total | Whole finer-clock accumulator. |
| `0x0E0` | 4 | `u32` | fine_scaled_total | Fine-tick accumulator with five fractional bits. |
| `0x0E4` | 4 | `u32` | fine_scaled_increment | Last change in the scaled fine accumulator. |
| `0x0E8` | 4 | `u32` | min_frame_rate | Retained minimum FPS setting. |
| `0x0EC` | 4 | `u32` | max_frame_rate | Retained maximum FPS setting. |
| `0x0F0` | 4 | `u32` | use_smooth_fps | Retained smoothed-FPS selection switch. |
| `0x0F4` | 4 | `u32` | use_smooth_divisor | Retained movement-scale smoothing switch. |
| `0x0F8` | 4 | `u32` | normal_rate_scaled | Retained normal frame rate multiplied by 256. |

The two scaled totals advance by `next_scale`; their low eight and five bits
retain the fractional remainders. Exposed timing can differ from the last
computed timing. Save restoration copies these fields directly, then captures
the partial clock checkpoint used during subsequent resource preparation.
The saved clock therefore includes fields omitted by a menu checkpoint,
including smoothed FPS, the measurement remainder and startup countdown.
It also retains the FPS settings rather than storing only their derived scale.
See [FPS-derived frame scale](../engine/player.md#fps-derived-frame-scale)
for measurement and checkpoint behavior. Other `SVGV` fields are retained state
and must be preserved when editing a save.

## Save-slot directories

```text
[SAVEGAME.NNN — slot directory]
  [SAVEGAME.SAV — current saved state]
  [Map-name.TSG files — other per-world snapshots]
  [LOGBOOK.TXT — journal content, when present]
```

The main save identifies the current world. The accompanying `.TSG` files
retain state for other worlds. `LOGBOOK.TXT` contains game journal content;
it is separate from the historical TXT tool reports described in
[Bundled support resources](BUNDLED-RESOURCES.md).

## External References

- [UESP: Redguard file formats](https://en.uesp.net/wiki/Mod:Redguard_File_Formats)
