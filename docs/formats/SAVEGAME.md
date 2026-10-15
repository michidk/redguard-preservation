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
