# RGM Scene File Format

Scene/map container with sectioned records for placed objects, script metadata, and auxiliary world data. The `RA*`-prefixed sections (`RASC`, `RAHD`, `RAAT`, `RAHK`, etc.) form the per-map SOUP scripting layer — see [SOUP Scripting](../engine/SOUP.md) for a consolidated map of all script data sources and runtime boundaries.

## Section Framing

Each section starts with an 8-byte header. Some sections then include a 4-byte little-endian `record_count` word at the beginning of section data.

| Offset | Size | Type | Endian | Name | Description |
|---|---|---|---|---|---|
| 0x00 | 4 | `[u8; 4]` | — | section_name | ASCII section tag (for example `RAHD`, `MPOB`, `MPSO`, `END `) |
| 0x04 | 4 | `u32` | **BE** | data_length | Payload size in bytes (0 for `END `). Big-endian in section-framed formats (RGM, PVO, ROB, TEXBSI). |

For count-prefixed sections (`MPOB`, `MPSO`, `MPRP`, and several others), section payload begins with:

Little-endian.

| Relative Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| +0x00 | 4 | `u32` | record_count | Number of fixed-size records in this section |

Sections are parsed sequentially until `END `.

## MPOB (Object Instances)

`MPOB` starts with a little-endian object count, followed by 66-byte records.

All fields little-endian.

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| 0x00 | 4 | `u32` | id | Object id |
| 0x04 | 1 | `u8` | object_type | Object kind discriminator |
| 0x05 | 1 | `u8` | is_active | Activation flag |
| 0x06 | 9 | `[u8; 9]` | script_name | Script/object name |
| 0x0F | 9 | `[u8; 9]` | model_name | Model reference |
| 0x18 | 1 | `u8` | is_static | Static/dynamic flag |
| 0x19 | 2 | `i16` | reserved | Not read at runtime. |
| 0x1B | 3 | `i24` | pos_x | Position X (fixed scale) |
| 0x1E | 1 | `u8` | pad_x | Alignment byte |
| 0x1F | 3 | `i24` | pos_y | Position Y (fixed scale) |
| 0x22 | 1 | `u8` | pad_y | Alignment byte |
| 0x23 | 3 | `u24` | pos_z | Position Z (fixed scale) |
| 0x26 | 4 | `u32` | angle_x | Bethesda 2048-unit Euler angle |
| 0x2A | 4 | `u32` | angle_y | Bethesda 2048-unit Euler angle |
| 0x2E | 4 | `u32` | angle_z | Bethesda 2048-unit Euler angle |
| 0x32 | 2 | `i16` | texture_data | Packed texture id/image id |
| 0x34 | 2 | `i16` | intensity | Light/intensity-like field |
| 0x36 | 2 | `i16` | radius | Radius-like field |
| 0x38 | 2 | `i16` | model_id | Model index/id-like field |
| 0x3A | 2 | `i16` | world_id | World index/id-like field |
| 0x3C | 2 | `i16` | red | Color channel |
| 0x3E | 2 | `i16` | green | Color channel |
| 0x40 | 2 | `i16` | blue | Color channel |

Position decode used by current exporter:

- scale constant: `1 / 5120`
- `x = -(pos_x * 256) * scale`
- `y = -(pos_y * 256) * scale`
- `z = -(0x00FF_FFFF - (pos_z * 256)) * scale`

Bethesda 2048-unit Euler angles:

2048 discrete units represent a full 360° rotation — a power-of-two binary angle encoding. The raw u32 is reduced modulo 2048 (equivalently masked with `& 0x7FF`), giving a value in the range [0, 2047]. Each unit equals 180/1024 ≈ 0.176°.

```
degrees = (value % 2048) * (180.0 / 1024.0)
```

| Units | Degrees |
|---|---|
| 0 | 0° |
| 512 | 90° |
| 1024 | 180° |
| 1536 | 270° |
| 2048 | 360° (wraps to 0) |

MPOB model lookup behavior:

- Primary source is `model_name` (9 bytes, null-trimmed).
- If `model_name` is empty, exporter falls back to `RAAN` using `RAHD` script metadata (see [RAAN](#raan-animation-file-references)).
- If RAAN also yields no result, `script_name` is used as a last resort.
- Example: `script_name = FAVIS` resolves via RAAN to `FVPRA001`.

MPSO uses a 12-byte `model_name` field (no fallback chain — model name is always present).

## MPSO (Static Objects)

`MPSO` starts with a little-endian object count, followed by 66-byte records.

All fields little-endian.

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| 0x00 | 4 | `u32` | id | Object id |
| 0x04 | 12 | `[u8; 12]` | model_name | Model reference |
| 0x10 | 3 | `i24` | pos_x | Position X |
| 0x13 | 1 | `u8` | pad_x | Alignment byte |
| 0x14 | 3 | `i24` | pos_y | Position Y |
| 0x17 | 1 | `u8` | pad_y | Alignment byte |
| 0x18 | 3 | `u24` | pos_z | Position Z |
| 0x1B | 1 | `u8` | pad_z | Alignment byte |
| 0x1C | 36 | `i32[9]` | rotation_matrix | 3x3 Q4.28 rotation matrix |
| 0x40 | 2 | `u8[2]` | unused | Always 0. |

The exporter converts `rotation_matrix` from Q4.28 to float and emits a node matrix with translation.

Rotation parity note:

- MPSO `rotation_matrix` must be interpreted with transposed index mapping when building the scene rotation matrix:
  - row 0 = `[m0, m3, m6]`
  - row 1 = `[m1, m4, m7]`
  - row 2 = `[m2, m5, m8]`
- Earlier row-major mapping (`[m0,m1,m2]`, `[m3,m4,m5]`, `[m6,m7,m8]`) produced incorrect static-object orientation for cases like `TV_SEAT` and `BT_BOARD`.

## MPRP (Rope Chains)

`MPRP` starts with a little-endian record count, followed by 80-byte records.

All fields little-endian.

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| 0x00 | 4 | `u32` | id | Rope/object id |
| 0x04 | 1 | `u8` | reserved | Not read at runtime. |
| 0x05 | 3 | `i24` | pos_x | Base position X |
| 0x08 | 1 | `u8` | pad_x | Alignment byte |
| 0x09 | 3 | `i24` | pos_y | Base position Y |
| 0x0C | 1 | `u8` | pad_y | Alignment byte |
| 0x0D | 3 | `i24` | pos_z | Base position Z |
| 0x10 | 4 | `i32` | angle_y | Rope heading field |
| 0x14 | 4 | `i32` | type | Type/discriminator |
| 0x18 | 4 | `i32` | swing | Swing parameter |
| 0x1C | 4 | `i32` | speed | Speed parameter |
| 0x20 | 2 | `i16` | length | Number of rope links |
| 0x22 | 9 | `[u8; 9]` | static_model | Optional terminal model |
| 0x2B | 9 | `[u8; 9]` | rope_model | Link model name (for example `ROPELINK`) |
| 0x34 | 28 | `i32[7]` | reserved | Not read at runtime. |

Rope instancing behavior:

- Decode base translation with the same MPOB scale/sign rules.
- Spawn `length` copies of `rope_model`.
- For each link: subtract `0.8` from Y and place one instance.
- If `static_model` is present, place one additional instance after the chain.

Current parser behavior: `MPRP` is parsed into typed 80-byte records only when section payload is an exact fit for `record_count`; otherwise raw fallback is kept.

## RALC (Location Data)

`RALC` contains scripted coordinate offsets for objects (e.g. the Boatman's waypoints). Records are 12-byte entries.

All fields little-endian.

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| 0x00 | 4 | `s32` | offset_x | X coordinate offset (applied to MPOB translated position) |
| 0x04 | 4 | `s32` | offset_y | Y coordinate offset |
| 0x08 | 4 | `s32` | offset_z | Z coordinate offset |

Offsets are applied to the object's base MPOB position (pos × 256) by script commands `MoveToLocation` and `WanderToLocation`. Per-object RALC entry counts and offsets are stored in the corresponding RAHD record.

## RAVC (VCollide)

`RAVC` uses 9-byte entries and appears only in a subset of maps (CATACOMB and DRINT — collision data for the dragon and golem).

All fields little-endian.

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| 0x00 | 1 | `i8` | offset_x | Local collision offset X |
| 0x01 | 1 | `i8` | offset_y | Local collision offset Y |
| 0x02 | 1 | `i8` | offset_z | Local collision offset Z |
| 0x03 | 2 | `u16` | vertex | Model vertex index used as reference point for the collision sphere |
| 0x05 | 4 | `u32` | radius | Collision sphere radius |

Current parser behavior: records are parsed as fixed 9-byte entries only when section payload is an exact fit; otherwise raw fallback is kept. RAVC is flat-out missing (not just empty) in RGM files without collision objects.

## WDNM (Walk Node Map)

`WDNM` defines walk node maps for AI pathfinding. Count-prefixed: record count is the number of walk-map blocks.

All fields little-endian.

### WalkMap Record

| Relative Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| +0x00 | 4 | `u32` | map_length | Total byte length of this walk-map |
| +0x04 | 4 | `u32` | node_count | Number of walk-nodes in this map |
| +0x08 | 4 | `u32` | node_count_dup | Duplicate of node_count |
| +0x0C | 3 | `s24` | map_pos_x | Map position X |
| +0x0F | 1 | `u8` | pad_x | Alignment byte |
| +0x10 | 3 | `s24` | map_pos_y | Map position Y |
| +0x13 | 1 | `u8` | pad_y | Alignment byte |
| +0x14 | 3 | `s24` | map_pos_z | Map position Z |
| +0x17 | 1 | `u8` | pad_z | Alignment byte |
| +0x18 | 4 | `u32` | radius | Map bounding radius |
| +0x1C | ... | variable | walk_nodes | `node_count` × WalkNode records |

### WalkNode Record

| Relative Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| +0x00 | 4 | `u32` | node_length | Total byte length of this walk-node |
| +0x04 | 2 | `u16` | node_pos_x | Local position X |
| +0x06 | 2 | `s16` | node_pos_y | Local position Y |
| +0x08 | 2 | `u16` | node_pos_z | Local position Z |
| +0x0A | 1 | `u8` | reserved | Not read at runtime. |
| +0x0B | 1 | `u8` | route_count | Number of routes from this node |
| +0x0C | ... | variable | routes | `route_count` × NodeRoute records |

### NodeRoute Record (4 bytes)

| Relative Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| +0x00 | 2 | `u16` | target_node_id | Destination walk-node index |
| +0x02 | 2 | `u16` | cost | Route traversal cost |

## RAHD (Actor Header)

RAHD is a count-prefixed section with 165-byte records. Each record provides per-actor metadata: script name, bytecode location in RASC, string/variable table pointers, animation references, collision data, and attribute hooks.

Section payload starts with a 4-byte LE record count, followed by 4 fixed bytes (`1B 80 37 00`), then `count × 165` bytes of records. The engine reads the count and prefix separately, then bulk-reads `count × 165` bytes as the record array. Offsets below are within each 165-byte record (starting at payload offset `8 + i × 165`). The Rust parser in this repo starts records 4 bytes earlier (at `4 + i × 165`) and adds 4 to all field offsets.

At load time, the engine converts most offset fields into absolute pointers by adding the corresponding section's data pointer (rebasing). Offset 0x00 is overwritten with a linked-list next-pointer at runtime.

All typed fields are little-endian.

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| 0x00 | 4 | — | field_00 | Overwritten at runtime (linked-list next-pointer) |
| 0x04 | 9 | `[u8; 9]` | script_name | Script/actor name, null-padded |
| 0x0D | 2 | `u16` | instances | Number of instances for this actor |
| 0x0F | 2 | — | padding | Always 0. |
| 0x11 | 4 | `i32` | instance_counter | Runtime instance counter (incremented during setup; advances `variable_offset` by `num_variables × 4` per instance) |
| 0x15 | 4 | `u8[4]` | anim_speed | Read as individual bytes. Byte 0x15: frame limit — animation advances only while `frame_counter < byte_0x15`. Byte 0x16: frame increment added per tick. |
| 0x19 | 4 | `u32` | ranm_offset | Byte offset into RANM section data (rebased to pointer at load) |
| 0x1D | 4 | `u32` | raat_offset | Byte offset into RAAT section data (rebased to pointer at load) |
| 0x21 | 4 | `i32` | raan_count | Number of RAAN entries for this actor |
| 0x25 | 4 | `u32` | raan_data_size | Total byte size of this actor's RAAN entries. Zero when `raan_count` = 0. |
| 0x29 | 4 | `i32` | raan_offset | Byte offset into RAAN section data (rebased to pointer at load) |
| 0x2D | 4 | — | anim_control_prefix | Low byte stored at animation control struct offset `+0x12` during RAGR loading. Remaining bytes are not decoded. |
| 0x31 | 4 | `u32` | ragr_offset | Byte offset into RAGR section data (rebased to pointer at load) |
| 0x35 | 8 | — | padding | Always 0. |
| 0x3D | 4 | `u32` | rafs_index | Index into RAFS data (vestigial — RAFS is 1 byte in all shipped files) |
| 0x41 | 4 | `u32` | num_strings | Number of strings used by this actor's script |
| 0x45 | 4 | — | padding | Always 0. |
| 0x49 | 4 | `u32` | string_offsets_index | Byte offset into RASB section data |
| 0x4D | 4 | `u32` | script_length | Byte length of this actor's bytecode block in RASC |
| 0x51 | 4 | `u32` | script_data_offset | Byte offset into RASC section data (rebased to pointer at load) |
| 0x55 | 4 | `u32` | script_pc | Execution start address; rebased at load to `script_data_offset + script_pc` (absolute pointer) |
| 0x59 | 4 | — | anim_buffer_swap | Read as byte at 0x59. Boolean: non-zero triggers animation frame buffer swap (copies between offsets +0xCB and +0x108 in actor struct). Zero uses primary buffer only. |
| 0x5D | 4 | `u32` | rahk_offset | Byte offset into RAHK section data (rebased to pointer at load) |
| 0x61 | 8 | — | dialogue_lock | Read as byte at 0x61. Set during dialogue initiation; prevents animation transitions while dialogue is active. Checked alongside actor-type and combat-state guards. |
| 0x69 | 4 | `u32` | ralc_offset | Byte offset into RALC section data (rebased: `ralc_data + (offset ÷ 12) × 12`) |
| 0x6D | 4 | `u8[4]` | actor_flags | Read as individual bytes. Byte 0x6D: animation state ID loaded into a global during dialogue setup, compared against hook data at +0x247 for state matching. Byte 0x6E: item/equipment flag (toggled at runtime). Byte 0x6F: passed to animation/sound function. |
| 0x71 | 4 | `u32` | raex_offset | Byte offset into RAEX section data (rebased to pointer at load) |
| 0x75 | 4 | `u32` | num_variables | Number of local variables for this actor |
| 0x79 | 4 | `u8[4]` | visibility_flags | Read as individual bytes. Byte 0x79: visibility test bypass (non-zero = always visible, skip LOD culling). Byte 0x7B: LOD culling mode (0 = fixed distance threshold, non-zero = dynamic distance threshold). |
| 0x7D | 4 | `u32` | variable_offset | Byte offset into RAVA section data (÷ 4 = variable array index) |
| 0x81 | 4 | `u32` | variable_offset_dup | Runtime copy of `variable_offset`; advanced by `num_variables × 4` per instance |
| 0x85 | 4 | `u32` | anim_frame_data | Animation frame count or group index. Upper 16 bits used as count (× 11 bytes per frame for allocation). |
| 0x89 | 4 | `i32` | soup_func_primary | SOUP386 function table index (primary). Multiplied by 49 to index into function table. -1 = disabled. |
| 0x8D | 4 | `i32` | soup_func_secondary | SOUP386 function table index (secondary). Same indexing. -1 = disabled. |
| 0x91 | 4 | `i32` | soup_func_tertiary | SOUP386 function table index (tertiary). -1 = disabled. |
| 0x95 | 2 | `i16` | combat_flag | Combat/state flag. |
| 0x97 | 2 | `i16` | raex_stat | Stored at actor state +0x97 after SOUP function lookup. -1 = disabled. |
| 0x99 | 2 | `i16` | reserved_99 | Always -1. Not read at runtime. |
| 0x9B | 2 | `i16` | reserved_9b | Always 0. Not read at runtime. |
| 0x9D | 4 | `i32` | ravc_offset | Byte offset into RAVC section data (rebased to pointer at load; -1 = none) |
| 0xA1 | 4 | `i32` | ravc_count | Number of RAVC collision entries for this actor |

Total record size: 165 bytes (0xA5).

## RAAN (Animation File References)

RAAN contains animation/model file path entries. Records are variable-length null-terminated strings with a 6-byte prefix.

Entry structure at a given byte offset (from RAHD `raan_offset`):

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| 0x00 | 4 | `u32` | reserved | Not read by any engine function. Skipped during entry iteration. |
| 0x04 | 1 | `u8` | frame_count | Used as loop count for animation handle table entries. Capped at 255. |
| 0x05 | 1 | `u8` | model_type | Type flag, converted to lowercase at load. Values: `0x63` (ASCII 'c') and `0x73` (ASCII 's'). |
| 0x06 | var | `[u8]` | file_path | Null-terminated file path string (e.g. `3dart\cyrsa001.3d`) |

The engine iterates RAAN entries by skipping the 6-byte prefix, then scanning forward to the null terminator of `file_path`. The 4-byte dword at offset 0x00 is NOT used for seeking — the next entry is found purely by string scan.

The model name is extracted by stripping directory separators, file extension, and uppercasing the stem.

### Model Fallback Resolution

When an MPOB record has an empty `model_name`, the exporter resolves a model via RAHD/RAAN:

1. Look up `script_name` in the RAHD index to get `(raan_offset, raan_count)`.
2. Parse the RAAN entry at `raan_offset` to extract the file path.
3. Strip the path to a bare filename stem (e.g. `fxart\FVPRA001.3DC` → `FVPRA001`).
4. Use the stem as the model name for asset lookup.
5. If no RAHD/RAAN match, fall back to using `script_name` as the model name.

## RAFS (Actor FSphere Data)

RAFS was intended to store pre-computed fsphere (combat/collision sphere) data for actors. The engine only loads this section when its size exceeds 10 bytes. In all 27 shipped RGM files, RAFS is exactly 1 byte — the section is vestigial and never loaded at runtime. FSpheres are instead computed from MPSZ bounding volume records indexed via RAHD.

## RAST (String Data)

RAST contains all script string literals as null-terminated strings concatenated into a single blob. No count prefix; it is a flat byte array. Individual strings are located by offsets stored in RASB.

During loading, RASB offsets are rebased by adding the RAST data pointer, converting relative offsets into direct pointers.

## RASB (String Offset Table)

RASB contains u32 LE offsets into RAST, one per string per actor. Each actor's portion starts at `string_offsets_index` (from RAHD) and contains `num_strings` entries.

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| +0x00 | 4 | `u32` | string_offset | Byte offset into RAST where the null-terminated string begins |

Total section length: sum of all actors' `num_strings × 4`.

## RAVA (Local Variables)

RAVA contains initial values for local script variables as a flat array of i32 LE integers. The first 4 bytes are always zero (sentinel).

Each actor's portion starts at byte offset `variable_offset` (from RAHD; divide by 4 for the array index) and contains `num_variables` entries. When an actor has multiple instances, variables are replicated `instances` times. Variables are addressed by index in script bytecode (opcode `0x0A`).

## RASC (Script Bytecode)

RASC contains compiled SOUP386 scripting bytecode as a contiguous byte blob. Per-actor instruction blocks are located via `RAHD` offsets (`script_data_offset`, `script_length`, `script_pc`) and executed by the SOUP virtual machine.

For VM architecture, opcode encoding, operator tables, and execution semantics, see [SOUP Scripting](../engine/SOUP.md). For the definition-file format, see [SOUP386.DEF](../engine/SOUPDEF.md).

### Section Layout

| Region | Size | Description |
|---|---|---|
| Preamble | `script_data_offset` bytes | Zero-padded address space used by rebased hook/offset references |
| Script blocks | remainder | Concatenated per-actor bytecode blocks in RAHD order |

Total payload = first actor `script_data_offset` + sum of all actors' `script_length` values. For each actor, execution starts at `script_pc` relative to that actor's block.

Other `RA*` sections (for example `RAHK`, `RALC`, `RAVC`) provide data referenced by `RASC`, often as `u32` offset arrays rebased to loaded script memory.

## RAHK (Hook Data)

RAHK contains hook offset tables. Entries are u32 LE offsets that are rebased against the RASC bytecode base address at load time, enabling scripts to register named entry points (hooks) that external events can invoke.

Per-actor hook counts and offsets are stored in the corresponding RAHD record. Entries are accessed at `base + index × 4 + 0x25` — the 0x25-byte region before the offset array is a section header. The upper 16 bits of each u32 entry are extracted separately (`>> 0x10`) as a secondary field. No additional per-entry structure beyond the u32 offset array exists.

## RAEX (Extra Data)

RAEX contains per-actor extra data with 30-byte fixed-size records (15 × i16 LE fields). The engine requires this section during loading. Record count is `section_data_length ÷ 30`. Per-actor RAEX offsets are stored in RAHD (`raex_offset`).

Field names `Grip0` through `RangeMax` are from the in-game debug console.

All fields little-endian.

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| 0x00 | 2 | `i16` | grip0 | Named from console. Used as animation frame offset during weapon transitions (consumed by the combat animation subsystem, not the attachment vertex system). |
| 0x02 | 2 | `i16` | grip1 | Same subsystem as grip0. |
| 0x04 | 2 | `i16` | scabbard0 | Named from console. Same subsystem as grip0. |
| 0x06 | 2 | `i16` | scabbard1 | Same subsystem as scabbard0. |
| 0x08 | 2 | `i16` | anim_frame_ref | Matches RAHD `anim_frame_data` at 0x85. Set on mobile actors only. |
| 0x0A | 2 | `u16` | texture_id | Texture override id for actor skin variants. |
| 0x0C | 2 | `i16` | v_vertex | Vertex-related field. |
| 0x0E | 2 | `i16` | v_size | Size-related field. |
| 0x10 | 2 | `i16` | taunt_id | First taunt animation id; additional taunts count up from this value. |
| 0x12 | 2 | `i16` | field_12 | Set only on large creatures (dragon, gremlin). |
| 0x14 | 2 | `i16` | field_14 | Source value for RAHD `raex_stat` at 0x97. Set on combat actors. |
| 0x16 | 2 | `i16` | field_16 | Set on some combat actors. |
| 0x18 | 2 | `i16` | range_min | Combat engagement minimum range. Multiply by 256 for world units. Only set on dragon, golem, serpent. |
| 0x1A | 2 | `i16` | range_ideal | Combat ideal range. Same scaling. |
| 0x1C | 2 | `i16` | range_max | Combat maximum range. Same scaling. |

Total record size: 30 bytes (0x1E). Record count is `section_data_length / 30`.

## RAAT (Attribute Data)

RAAT contains per-actor attribute tables. Each actor has a 256-byte attribute block, ordered sequentially (actor 0 at offset 0, actor 1 at offset 256, etc.).

Attribute names are defined in the `auto`...`endauto` section of `SOUP386.DEF` (see [SOUP386.DEF](../engine/SOUPDEF.md)). Each byte is a named attribute value; zero means unset. Attributes are read/written by the script functions `GetAttribute` and `SetAttribute`.

Total section length: `record_count × 256`.

## RAGR (Animation Groups)

RAGR contains animation group definitions that link actors to their animation data in RAAN. RAGR provides the RGM-embedded equivalent of the `AIAN` section found in standalone `.AI` files; the engine selects one or the other source based on a runtime mode flag.

Per-actor RAGR data is located via `RAHD.ragr_offset` (offset 0x31). Entries are size-prefixed: the first u16 is the entry payload size (excluding itself); a value of `0` terminates the list. Advance to the next entry: `current_position + 2 + entry_size`.

The prefix byte used by the AIAN (standalone `.AI`) path is NOT present in RGM RAGR — instead, that value comes from RAHD offset 0x2D.

### Animation Group Entry

All fields little-endian.

| Relative Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| +0x00 | 2 | `u16` | entry_size | Payload size in bytes after this field. `0` = end of groups. Should equal `8 + frame_count × 3`. |
| +0x02 | 2 | `u16` | group_index | Animation group slot (0–177; validated ≤ 0xB1 at load) |
| +0x04 | 2 | `u16` | anim_id | Animation identifier |
| +0x06 | 2 | `u16` | anim_type | Animation type (only low byte used). Values: `0` = interruptible (idle/panic), `1` = must complete (combat), `2` = no panic revert (ledge-hang loops). |
| +0x08 | 2 | `u16` | frame_count | Number of animation frames in this group |
| +0x0A | var | `[u8; frame_count × 3]` | commands | Packed 3-byte animation commands, one per frame |

In `ISLAND.RGM`, Cyrus (RAHD record 22, `ragr_offset=1318`) has 152 animation groups. 58 groups contain attachment commands (opcode 0/4/10 with non-zero vertex index). Vertex 1 = hand attachment (sword combat), vertex -10 = scabbard attachment.

### Animation Command (3 bytes, packed LE)

Each command is a 24-bit little-endian packed value. The low 4 bits select the opcode type, which determines how the remaining 20 bits are allocated to parameters.

**Opcode 0 (ShowFrame)** — the only opcode that sets the attachment vertex:

```
byte 0          byte 1          byte 2
7 6 5 4 3 2 1 0 7 6 5 4 3 2 1 0 7 6 5 4 3 2 1 0
├─hdl─┤ ├─op──┤ ├v┤ ├──handle─┤ ├───vertex────┤

opcode       = byte0 & 0x0F                        (4 bits)
handle_index = (byte0 >> 4) | ((byte1 & 0x3F) << 4)  (10-bit signed)
vertex_index = (byte1 >> 6) | (byte2 << 2)           (10-bit signed)
```

Both `handle_index` and `vertex_index` are 10-bit sign-extended values (range −512..+511). The `handle_index` is a relative index into the per-actor animation handle lookup table (built from RAAN entries at load time; patched to absolute runtime handles during loading). The `vertex_index` identifies which vertex to track for item attachment — see [Item Attachment System](../engine/attachment.md).

Opcodes 4 (PlaySound) and 10 (ChangeAnimGroup) share the same 10+10 bit layout but their parameters are NOT handle/vertex — they are sound params and animation jump targets respectively. See [attachment.md](../engine/attachment.md) for the full 16-opcode table with names, bit layouts, and playback behavior.

## RANM (Namespace)

RANM contains object namespace strings used for cross-script object references. Each actor's portion is located by offset and length fields stored in RAHD (within the undecoded gap at 0x5D). The extracted string provides the actor's canonical name for `ObjDot*` opcodes using selector byte 4 (named object from string table).

## MPSL (Lights)

`MPSL` starts with a little-endian record count, followed by 42-byte records.

All fields little-endian.

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| 0x00 | 3 | `u8[3]` | color_rgb | Color bytes (R, G, B). |
| 0x03 | 1 | `u8` | light_type | Light type. Values: 0, 130 (0x82), 131 (0x83), 132 (0x84). |
| 0x04 | 4 | `u32` | light_param | Zero for ambient lights; 28 for directional lights. |
| 0x08 | 3 | `i24` | pos_x | Position X |
| 0x0B | 1 | `u8` | pad_x | Alignment byte |
| 0x0C | 3 | `i24` | pos_y | Position Y |
| 0x0F | 1 | `u8` | pad_y | Alignment byte |
| 0x10 | 3 | `i24` | pos_z | Position Z |
| 0x13 | 1 | `u8` | pad_z | Alignment byte |
| 0x14 | 2 | `i16` | param0 | Intensity or range parameter |
| 0x16 | 2 | `i16` | param1 | Intensity or range parameter |
| 0x18 | 6 | `i16[3]` | direction | Direction/attenuation vector (3 × i16). Non-zero in active lights. |
| 0x1E | 8 | `u8[8]` | channel_map | Light channel enable. Always either `00 01 02 03 04 05 06 07` (active, identity mapping to 8 channels) or all zeros (inactive). |
| 0x26 | 4 | `u8[4]` | reserved_26 | Always 0. |

Position fields use the same i24+pad encoding as MPOB/MPSO.

## MPMK (Markers)

`MPMK` starts with a little-endian record count, followed by 13-byte records.

All fields little-endian.

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| 0x00 | 3 | `i24` | pos_x | Position X |
| 0x03 | 1 | `u8` | pad_x | Alignment byte |
| 0x04 | 3 | `i24` | pos_y | Position Y |
| 0x07 | 1 | `u8` | pad_y | Alignment byte |
| 0x08 | 3 | `i24` | pos_z | Position Z |
| 0x0B | 1 | `u8` | pad_z | Alignment byte |
| 0x0C | 1 | `u8` | reserved | Not read by the engine at runtime. Engine uses bytes 0x04 (type) and 0x05 (subtype) from the runtime marker struct for processing. |

Position fields use the same i24+pad encoding as MPOB/MPSO. No explicit record ID field. The engine branches on marker type (byte +0x04 in runtime struct) with values 0x02 and 0x06 triggering distinct paths.

## MPSZ (Bounding Volumes)

MPSZ is an array of 49-byte bounding volume records used to build per-actor **fspheres** (combat/collision bounding spheres) at runtime. Not count-prefixed — record count is `section_size / 49`. RAHD fields at +0x8D and +0x91 store per-actor indices into this table (−1 = no record).

### Record Layout (49 bytes)

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| 0x00 | 4 | `i32` | total_x | Total extent X (`neg_x + pos_x`) |
| 0x04 | 4 | `i32` | total_y | Total extent Y (`neg_y + pos_y`) |
| 0x08 | 4 | `i32` | total_z | Total extent Z (`neg_z + pos_z`) |
| 0x0C | 4 | `i32` | center_x | Center offset X (always 0 in shipped data) |
| 0x10 | 4 | `i32` | center_y | Center offset Y (always 0 in shipped data) |
| 0x14 | 4 | `i32` | center_z | Center offset Z (always 0 in shipped data) |
| 0x18 | 4 | `i32` | neg_x | Negative half-extent X |
| 0x1C | 4 | `i32` | neg_y | Negative half-extent Y |
| 0x20 | 4 | `i32` | neg_z | Negative half-extent Z |
| 0x24 | 4 | `i32` | pos_x | Positive half-extent X |
| 0x28 | 4 | `i32` | pos_y | Positive half-extent Y |
| 0x2C | 4 | `i32` | pos_z | Positive half-extent Z |
| 0x30 | 1 | `u8` | flags | Always 0 in shipped data |

Invariant: `total = neg + pos` for each axis. Center is always zero in shipped files. About 30% of records are symmetric (`neg == pos`); the rest have asymmetric bounds.

At runtime, the engine copies these 49 bytes and calls the fsphere builder to create a 3D bounding volume for the actor. RAHD provides two separate index fields per actor, allowing reference to different bounding records. Only a subset of actors have direct RAHD indices — other records may be referenced by MPOB objects or other runtime systems.

Present in all 27 shipped RGM files (5–144 records per map).

## MPSF (Flat Objects)

`MPSF` starts with a little-endian record count, followed by 24-byte records. Each record places a textured quad in the scene.

All fields little-endian.

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| 0x00 | 4 | `u32` | id | Object id |
| 0x04 | 4 | `i32` | reserved | Not read at runtime. |
| 0x08 | 3 | `i24` | pos_x | Position X |
| 0x0B | 1 | `u8` | pad_x | Alignment byte |
| 0x0C | 3 | `i24` | pos_y | Position Y |
| 0x0F | 1 | `u8` | pad_y | Alignment byte |
| 0x10 | 3 | `u24` | pos_z | Position Z |
| 0x13 | 1 | `u8` | pad_z | Alignment byte |
| 0x14 | 2 | `u16` | texture_data | Packed: `texture_id = data >> 7`, `image_id = data & 0x7F` |
| 0x16 | 2 | `i16` | reserved | Not read at runtime. |

Position decode uses the same MPOB scale/sign rules. MPSF items are flat quads with zero rotation.

## FLAT

`FLAT` appears in all 27 shipped RGM files as the last section before `END `. Fixed size: 1111 bytes in 26 files, 1108 in `HIDEOUT.RGM`. Mostly zero-filled with sparse non-zero values concentrated in two regions: small integer indices (0–4) at offsets 92–156 and 580–988, and a dense 87-byte block at offset 1024+ containing packed byte triplets. Internal structure is not decoded. Distinct from `MPSF` (flat billboard objects), which has its own section tag and parsed record format.

## External References

- [UESP: Mod:RGM File Format](https://en.uesp.net/wiki/Mod:RGM_File_Format)
- [UESP: Mod:Redguard File Formats](https://en.uesp.net/wiki/Mod:Redguard_File_Formats)
- [RGUnity/redguard-unity `RGRGMFile.cs`](https://github.com/RGUnity/redguard-unity/blob/master/Assets/Scripts/RGFileImport/RGGFXImport/RGRGMFile.cs) — RGM section parser
- [RGUnity/redguard-unity `RGRGMScriptStore.cs`](https://github.com/RGUnity/redguard-unity/blob/master/Assets/Scripts/RGFileImport/RGMData/RGRGMScriptStore.cs) — RASC bytecode interpreter with dispatch loop and flags table (369 entries)
- [RGUnity/redguard-unity `soupdeffcn_nimpl.cs`](https://github.com/RGUnity/redguard-unity/blob/master/Assets/Scripts/RGFileImport/RGMData/soupdeffcn_nimpl.cs) — SOUP function ID-to-name table (367 functions)
- [Dillonn241/redguard-mod-manager `ScriptReader.java`](https://github.com/Dillonn241/redguard-mod-manager/blob/main/src/redguard/ScriptReader.java) — RASC bytecode disassembler (bytecode to readable script text)
- [Dillonn241/redguard-mod-manager `ScriptParser.java`](https://github.com/Dillonn241/redguard-mod-manager/blob/main/src/redguard/ScriptParser.java) — RASC bytecode assembler (script text to bytecode)
- [Dillonn241/redguard-mod-manager `MapFile.java`](https://github.com/Dillonn241/redguard-mod-manager/blob/main/src/redguard/MapFile.java) — RGM section reader/writer with chunk tag list
- [Dillonn241/redguard-mod-manager `MapHeader.java`](https://github.com/Dillonn241/redguard-mod-manager/blob/main/src/redguard/MapHeader.java) — RAHD record parser with field offsets
- [Dillonn241/redguard-mod-manager `MapDatabase.java`](https://github.com/Dillonn241/redguard-mod-manager/blob/main/src/redguard/MapDatabase.java) — SOUP386.DEF parser (function, flag, reference, attribute definitions)
