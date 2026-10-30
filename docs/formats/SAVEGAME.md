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

## Rope jump latch

The selected player's rope jump latch is a four-byte little-endian word at
`SVGV` payload offset `0x06C`. The global-state writer stores the latch directly,
and the global-state reader copies the saved word back before restoring the
per-world object and rope records. It is separate from the `SMRP` rope records.
See [rope control while attached](../engine/SOUP.md#rope-control-while-attached)
for its input behavior.

`SMOB` restores an actor's rope attachment by resolving the saved rope identity
and constructing a fresh attachment. The actor receives that fresh pointer;
the saved attachment pointer is not reused. The saved attachment node and motion
word are then restored into the fresh attachment.

When restoring `SMRP`, the rope's list references are resolved by rope identity.
The current link-array and end-model pointers are retained. If the selected
player has an attachment, the rope's current attachment-list head is retained
as well. If the selected player has no attachment, the current head is freed
and the restored head is null. The rope jump latch is restored through `SVGV`
independently of these attachment pointers.

## Engine globals

The main save (`SAVEGAME.SAV`, not the per-world `.TSG` files) also writes an `SVG2` chunk after
`SVGV`. Its payload is a fixed 1,666-byte concatenation of retained engine globals without padding or
per-field headers, in this order:

| Size | Contents |
|---|---|
| 4 | Retained player health, copied into both player records at load. |
| 4 | Health-candle feedback state. |
| 2 | Retained state word, written only by the save loader. |
| 12 | Retained state block. |
| 1600 | The journal tag table: 400 four-byte tags, zero marking an empty entry. |
| 4 | Selected-player switch. |
| 4 × 5 | Power-up slots: the two slots' frame values, their two retained frame indices (`-1` empty), then the slot alternation (see [power-up slots](../engine/hud.md#power-up-slots)). |
| 4 × 4 | Four retained world-state words written when a world loads. |
| 4 | Retained HUD state word. |

## Inventory

`SVIT` contains 22-byte records in initialized item-list order,
followed by a signed 32-bit selected-item ID. All numeric fields are little-endian.
The payload contains neither an item count nor per-record IDs. Its length is
`22 × saved_record_count + 4`. The save reader and writer traverse one fewer
record than item initialization creates. The final initialized item remains at
its initialized values when loading; it has no record in `SVIT`.

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| `0x00` | 1 | `u8` | hide | Current item visibility field. |
| `0x01` | 1 | `u8` | unknown_01 | Retained item-state byte. |
| `0x02` | 4 | `i32` | total | Current item total limit. |
| `0x06` | 4 | `i32` | player_max | Current player ownership limit. |
| `0x0A` | 4 | `i32` | player_total | Current owned count. |
| `0x0E` | 4 | `u32` | unknown_0e | Retained item-state word. |
| `0x12` | 4 | `u32` | unknown_12 | Retained item-state word. |

The selected-item value `-1` means no selection. Other values are looked up as
full item IDs after the records have been restored. An absent ID leaves no
selection. Restoration directly replaces the saved fields in initialized item
records. It does not invoke acquisition, requirements, drop lists or pickup effects.

See [ITEM.INI](../config/item-ini.md) for the initialized limits and acquisition
rules. These save records can replace limits as well as owned counts.

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
