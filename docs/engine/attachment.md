# Item Attachment System

How the engine positions held items (swords, shields) on animated characters at runtime. This is a vertex-tracking system — no skeleton or bone hierarchy exists in any Redguard format.

## Overview

Character models (3D/3DC) are flat polygon meshes with per-frame vertex animation. There are no bones, joints, or named attachment points in the model data. Instead, the engine tracks a specific **vertex index** from the character's animation, reads that vertex's world position each frame, and places the held item there.

The tracked vertex index is encoded **per-frame** in **packed 3-byte animation commands** within the RGM `RAGR` section (or the equivalent `AIAN` section in standalone `.AI` files). Each animation frame can specify which vertex to track, allowing the attachment point to change as the animation progresses.

## Locating the Vertex Index in a File

To find an actor's attachment vertex index in an RGM file:

```
RAHD record (165 bytes per actor, at payload offset 8 + i × 165)
  └─ offset 0x31: ragr_offset (u32 LE)
        │
        ▼
RAGR section payload + ragr_offset
  └─ read u16 entry_size (0 = end, else payload bytes follow)
  └─ animation group entry:
       +0x02: group_index (u16)
       +0x04: anim_id     (u16)
       +0x06: flag         (u16, low byte only)
       +0x08: frame_count  (u16)
       +0x0A: commands     (frame_count × 3 bytes)
  └─ next entry at: current_position + 2 + entry_size
                │
                ▼
       Per-frame command (3 bytes, packed LE):
         byte0 & 0x0F = opcode
         If opcode is 0, 4, or 10:
           vertex_index = (byte1 >> 6) | (byte2 << 2)
           Sign-extend from 10 bits: if value & 0x200, subtract 0x400
```

In `ISLAND.RGM`, Cyrus has 152 animation groups, 58 with attachment commands. Key vertex indices are **1** (hand/sword grip) and **−10** (scabbard/hip).

The full animation command format is documented in [RGM § RAGR](../formats/RGM.md#ragr-animation-groups).

## Animation Command Stream

Animation group data consists of a 10-byte entry header followed by `frame_count` × 3-byte packed commands (see [RGM § RAGR](../formats/RGM.md#ragr-animation-groups) for the entry layout).

### Animation Command (3 bytes, packed)

Each command is a 24-bit packed value. The low 4 bits select the opcode type, which determines how remaining bits are allocated to parameters.

#### Opcode 0 — ShowFrame (Set Handle + Vertex)

The **only** opcode that sets the attachment vertex. Encoding: 10-bit handle + 10-bit vertex.

```
byte 0          byte 1          byte 2
7 6 5 4 3 2 1 0 7 6 5 4 3 2 1 0 7 6 5 4 3 2 1 0
├─hdl─┤ ├─op──┤ ├v┤ ├──handle─┤ ├───vertex────┤

opcode       = byte0 & 0x0F                          (4 bits)
handle_index = (byte0 >> 4) | ((byte1 & 0x3F) << 4)  (10-bit signed)
vertex_index = (byte1 >> 6) | (byte2 << 2)            (10-bit signed)
```

During animation playback, the decoded values are written to the actor struct:
- `handle_index` → actor animation handle (which 3D object to read from)
- `vertex_index` → actor tracked vertex (which vertex to read position of)

In `ISLAND.RGM`, Cyrus uses vertex 1 (hand grip) in 664 commands across combat animations.

### Complete Opcode Table

Semantics from engine analysis. Names from UESP where available.

| Opcode | UESP Name | Bit Layout | Parameters | Playback Behavior |
|---|---|---|---|---|
| **0** | ShowFrame | 10 + 10 | handle_index, vertex_index | Advance frame; set attachment handle + vertex. The only opcode that drives item positioning. |
| **1** | EndAnimation | 20-bit | (unused, always 0) | Set animation handle to −1; stop associated sound; call playback recursively for next animation. |
| **2** | GoToPrevious | 20-bit | target_frame | Jump backward to an earlier frame in this group. Used for walk/run loops. |
| **3** | GoToFuture | 20-bit | target_frame | Jump forward to a later frame. Conditional — checks animation state flags and pending transitions. |
| **4** | PlaySound | 10 + 10 | sound_param, volume_shift | Play SFX. Calls sound system with `sound_param` as setup and `volume_shift << 6` as volume. Same bit layout as opcode 0 but params are NOT handle/vertex. |
| **5** | BreakPoint | 20-bit | (unused, always 0) | Set vertex-enable flag at anim control +0x0B. Often the target of GoToFuture jumps. |
| **6** | SetRotationXYZ | 6 + 6 + 6 | rot_x, rot_y, rot_z | Set 3-axis rotation (each param × 256). Actor orientation override. |
| **7** | SetRotationAxis | 2 + 18 | axis (0=X, 1=Y, 2=Z), value | Set rotation on a single axis. Finer precision than opcode 6. |
| **8** | SetPositionXYZ | 6 + 6 + 6 | pos_x, pos_y, pos_z | Set 3-axis position offset (each param × 8). |
| **9** | SetPositionAxis | 2 + 18 | axis (0=X, 1=Y, 2=Z), value | Set position offset on a single axis. |
| **10** | ChangeAnimGroup | 10 + 10 | target_group, target_frame | Jump to a different animation group and frame. Same bit layout as opcode 0, but writes to anim control fields, not attachment. |
| **11** | Rumble/SFX | 20-bit | effect_bitmask | 5-bit mask combining sounds + screen shake. Bit 0 → SFX 0x2580 if actor state=5; bit 1/2 → SFX 0x2500 if state=1; bit 3 → SFX 0x2700 if state=6; **bit 4 → screen shake** (camera pitch oscillation with exponential decay, ±0x20 units). Cyrus has param=0 (placeholder, no effect). Golem in DRINT has param=16 (bit 4 only = screen shake on attack). |
| **12** | DelayCounter | 20-bit | counter_value | Set frame delay counter; pause animation until counter expires. |
| **13** | ConditionalDelay | 20-bit | counter_value | Set conditional delay; direction-dependent counter. |
| **14** | LoopControl | 20-bit | target_frame | Decrement loop counter; jump to target frame if counter > 0. |
| **15** | Transition | 6 + 7 + 7 | trigger_mask, start_frame, target_group | Mid-animation transition to another group. trigger_mask bits: 0=jump right, 1=jump left, 2=anim trigger (0x2500), 3=anim trigger (0x2700), 4=counter increment, 5=unused. Used by 8 combat actors. |

Opcodes 6–10 and 12–14 are implemented in the engine but **never appear in any of the 27 shipped RGM files**. They may exist in standalone `.AI` files or be entirely vestigial.

### Opcode Usage Census (all 27 shipped maps)

| Opcode | Total Cmds | Maps | Actors | Notes |
|---|---|---|---|---|
| 0 (ShowFrame) | 93,750 | 27 | 280 | Every animated actor |
| 1 (EndAnimation) | 8,513 | 27 | 280 | Every animated actor |
| 2 (GoToPrevious) | 3,258 | 27 | 275 | Walk/run loops |
| 3 (GoToFuture) | 7,121 | 27 | 276 | Conditional jumps |
| 4 (PlaySound) | 6,913 | 27 | 98 | Combat actors |
| 5 (BreakPoint) | 13,167 | 27 | 105 | Combat actors |
| 11 (ActorSound) | 164 | 27 | 2 | Cyrus + Golem only |
| 15 (Transition) | 461 | 27 | 22 | Guards only |
| 6–10, 12–14 | 0 | 0 | 0 | Dead code in shipped game |

### Bit Layout Summary

| Layout | Opcodes | Extraction |
|---|---|---|
| 10 + 10 | 0, 4, 10 | `(packed >> 4) & 0x3FF`, `(packed >> 14) & 0x3FF` (both sign-extended) |
| 6 + 6 + 6 | 6, 8 | 6-bit signed at positions 4, 10, 16 |
| 2 + 18 | 7, 9 | 2-bit selector at 4, 18-bit signed at 6 |
| 6 + 7 + 7 | 15 | 6-bit at 4, 7-bit signed at 10, 7-bit signed at 17 |
| 20-bit | 1, 2, 3, 5, 11, 12, 13, 14 | 20-bit signed at 4 |

### Handle Index Patching

During loading, commands with opcode type 0 are post-processed. The `handle_index` field contains a **relative index** into a per-actor animation handle lookup table. The engine rewrites the packed command in-place to replace the relative index with the resolved runtime animation handle. This table is built from RAAN entries loaded for the actor.

## Vertex Position Lookup

At each frame, the engine reads the tracked vertex position through this call chain:

1. **Entry point** — resolves animation handle and reads the tracked vertex.
2. **3D object manager** — looks up handle in a table. For "virtual" animations (type `0x02`), follows a parent handle chain recursively.
3. **Frame builder** — reads `(x, y, z)` float position of the given vertex from the current animation frame. For frame 0: reads `base_vertices[vertex_index × 12]`. For animated frames: applies delta-compressed offsets (i8×3 or i16×3) from the base frame. Returns `float[3]`.
4. Result is scaled by a global constant and rounded to integer world coordinates.

## Item Data (from INVENTRY.ROB)

Items are loaded from a ROB file keyed as `"ITEMS"` (`INVENTRY.ROB`). The item initialization function iterates all items and populates per-item runtime fields:

| Item Struct Offset | Source | Description |
|---|---|---|
| `+0x10` | Item type | Type discriminator: `1` = weapon/hand-object, `3` = general item |
| `+0x4a` | ROB handle | 3D model handle for the item |
| `+0x77` | ROB handle (type 1 only) | **Hand model** — the 3D model shown when weapon is drawn |
| `+0x7b` | ROB handle (type 1 only) | **Hilt model** — the 3D model shown when weapon is sheathed |
| `+0x7f` | ROB segment data | **Length/offset** — read from the ROB segment's internal metadata. Used to offset the weapon collision sphere along the item axis. |

## Attachment Transform

Two nearly-identical functions compute the held item's world transform. Both:

1. Read two vertex positions from the actor's current animation frame (via the tracked vertex index).
2. Add world position offsets.
3. Rotate by the actor's orientation matrix (actor struct `+0x51`).
4. Compute heading and pitch from the direction between the two points.
5. Build item rotation from the computed direction + actor roll.
6. Set item world position = vertex position + (actor radius × scale factor).

The two routines differ in scale factor, corresponding to the "in-hand" and "on-hip/scabbard" attachment positions.

## Weapon State Machine

The weapon state machine selects which attachment routine and which model (hand vs hilt) to use based on the actor's weapon state:

| State (actor `+0x1b4`) | Condition | Action |
|---|---|---|
| `0x14` (drawing sword) | Frame < draw threshold | Position hilt model at scabbard |
| `0x14` (drawing sword) | Frame ≥ draw threshold | Position hand model at hand; set drawn flag |
| `0x15` (sheathing sword) | Frame < sheath threshold | Position hand model at hand |
| `0x15` (sheathing sword) | Frame ≥ sheath threshold | Position hilt model at scabbard; clear drawn flag |
| `0x00` (idle, sheathed) | — | Position hilt model at scabbard |
| `0x00` (idle, drawn) | — | Position hand model at hand |

The draw/sheath frame thresholds are read from the actor's attribute data (offsets `+0x22` and `+0x23` from an attribute block pointer at actor `+0x272`).

The collision sphere tip is offset from the grip point by `item.length × -0x100`, positioning it along the weapon axis for combat hit detection.

## SOUP Script Interface

Scripts drive weapon state transitions through these SOUP functions:

| Function | Purpose |
|---|---|
| `handitem` | Assign a held item to an actor |
| `displayhandmodel` | Show the hand (drawn) model |
| `displayhanditem` | Show the item in the hand |
| `displayhiltmodel` | Show the hilt (sheathed) model |
| `displayhiltitem` | Show the item at the hilt position |
| `drawsword` | Trigger draw animation/state transition |
| `sheathsword` | Trigger sheath animation/state transition |
| `isholdingweapon` | Query: is actor holding a weapon? |
| `iscarryingweapon` | Query: is actor carrying (has) a weapon? |
| `isdrawingsword` | Query: is actor in draw animation? |
| `issheathingsword` | Query: is actor in sheath animation? |

Additional runtime state tracked per actor: `hand_pos.vx/vy/vz`, `hand_angle.vx/vy/vz`, `hand_type`, `hand_length`, `weapon_drawn`, `hand_item`.

## Data Flow Summary

```
File data (RGM):
  RAHD record → ragr_offset (offset 0x31)
       │
       ▼
  RAGR section payload + ragr_offset
       → size-prefixed entries (u16 entry_size; 0 = end):
            +0x02 group_index, +0x04 anim_id, +0x06 flag,
            +0x08 frame_count, +0x0A commands (frame_count × 3 bytes)
            command bits 14–23 = vertex_index (for opcode 0/4/10)

Map load:
  RAAN entries → load .3DC animation files → get runtime handles
  RAGR         → load animation command streams
                 → patch handle_index from relative to absolute

Runtime (per frame):
  Animation playback → decode 3-byte command for current frame
                     → extract vertex_index + handle_index
                     → store in actor struct (+0x24f, +0x251)

  Item attachment    → read vertex position from current anim frame
                       using stored handle + vertex index
                     → compute world transform (position + orientation)
                     → place item model at computed transform
```

## External References

- [UESP: Mod:RGM File Format § RAEX](https://en.uesp.net/wiki/Mod:RGM_File_Format#RAEX:_Extra_data) — RAEX field names from in-game console
- [RGUnity/redguard-unity `RGRGMFile.cs`](https://github.com/RGUnity/redguard-unity/blob/master/Assets/Scripts/RGFileImport/RGGFXImport/RGRGMFile.cs) — RGMRAEXItem struct definition
