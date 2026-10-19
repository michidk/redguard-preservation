# Item Attachment System

How the engine positions held items (swords, shields) on animated characters at runtime. This is a vertex-tracking system — no skeleton or bone hierarchy exists in any Redguard format.

## Overview

Character models (3D/3DC) are flat polygon meshes with per-frame vertex animation. There are no bones, joints, or named attachment points in the model data. Instead, the engine tracks a specific **vertex index** from the character's animation, reads that vertex's world position each frame, and places the held item there.

Animation commands select the character's model and frame. Attachment vertex
selection is separate from that frame selection. File `ShowFrame` commands
must not be interpreted as vertex indices. RAEX retains the grip and scabbard
vertex inputs.

## Animation command stream

Groups contain packed three-byte commands. File `ShowFrame` commands use a
twenty-bit reference into the concatenated RAAN model/frame table. The loader
patches them into a runtime model handle and frame index. See
[animation playback](animation.md) and [RGM](../formats/RGM.md#ragr-animation-groups).

### Complete Opcode Table

Names follow UESP where available.

| Opcode | UESP Name | Bit Layout | Parameters | Playback Behavior |
|---|---|---|---|---|
| **0** | ShowFrame | 20-bit file reference | frame_reference | Select a model/frame through RAAN; runtime representation is a patched 10 + 10 model handle and frame index. |
| **1** | EndAnimation | 20-bit | (unused, always 0) | Set animation handle to −1; stop associated sound; call playback recursively for next animation. |
| **2** | GoToPrevious | 20-bit | target_frame | Jump backward to an earlier frame in this group. Used for walk/run loops. |
| **3** | GoToFuture | 20-bit | target_frame | Jump forward to a later frame. Conditional — checks animation state flags and pending transitions. |
| **4** | PlaySound | 10 + 10 | effect, rate_offset | Play SFX positioned at the animated object. Both fields are signed. `effect` is the effect index after [SURFACE.INI](../config/surface-ini.md#sound-remap-records) remapping; `rate_offset × 64` Hz is added to the effect's sample rate (result clamped to 512..44000 Hz). Volume and pan come from the [positional sound](sound.md#animation-playsound) rules, not from this command. Same bit layout as opcode 0 but params are NOT handle/vertex. |
| **5** | BreakPoint | 20-bit | (unused, always 0) | Set the animation breakpoint flag. Often the target of GoToFuture jumps. |
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

Opcodes 6–10 and 12–14 are implemented in the engine but do not appear in any
of the 27 shipped RGM files.

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

### Frame reference patching

The loader resolves the entire twenty-bit file reference through RAAN. It
rewrites the command with a runtime model handle and frame index. Neither
field is an attachment vertex index.

## Vertex Position Lookup

At each frame, the engine reads the tracked vertex position through this call chain:

1. **Entry point** — resolves animation handle and reads the tracked vertex.
2. **3D object manager** — looks up handle in a table. For "virtual" animations (type `0x02`), follows a parent handle chain recursively.
3. **Frame builder** — reads the requested vertices from the selected model frame. Frame encoding is described in [3DC](../formats/models/3dc.md).
4. Result is scaled by a global constant and rounded to integer world coordinates.

### Actor model-point conversion

The runtime model-frame vertex query reads frame zero directly from its stored
floating-point base vertices. Later frames with encoding nibble zero convert
signed 32-bit coordinates to floating point with a factor of 1/256. Encoding
nibble one subtracts signed byte deltas from the base vertex. Encoding nibble
two subtracts signed 16-bit deltas from the base vertex, using a factor of
1/256 unless runtime flag `0x10` selects unscaled deltas. Each resulting
coordinate is stored as single precision.

The animation-vertex query's disable gate is the dummy 3D manager state.
Initialization reads `[3dmanager] dummy_manager` from SYSTEM.INI, while the
`dm` command-line option forces that state to one. Static-angle conversion,
selected by `[debug] convert_static_angles` or the `convert` option, also
forces dummy-manager state to one when enabled. The query's disable comparison
accepts exactly one as disabled.

A successful animation-frame vertex lookup supplies floating-point coordinates.
The animation-vertex wrapper multiplies each coordinate by 256 and stores each
product as single precision before converting the stored values to integers
using the active floating-point rounding mode. With nearest-even rounding,
half-integer ties select the even integer. It forwards the selected frame and
vertex index to the model lookup. A nonpositive animation handle or an animation
query-disable value equal to one returns the zero integer vector without model
lookup.

The actor model-point query reads an integer vertex from the selected animation
frame or the actor's static model. It converts each coordinate to single
precision before applying the actor matrix. For each transformed coordinate,
it adds the actor's local integer position before storing the sum as single
precision. The integer position is not rounded to single precision separately
before addition. The stored sums then convert to integer coordinates using the
active floating-point rounding mode. With nearest-even rounding, half-integer
ties select the even integer.

## Item Data (from INVENTRY.ROB)

Items are loaded from a ROB file keyed as `"ITEMS"` (`INVENTRY.ROB`). The item initialization function iterates all items and populates per-item runtime fields:

| Item Struct Offset | Source | Description |
|---|---|---|
| `+0x10` | Item type | Type discriminator: `1` = weapon/hand-object, `3` = general item |
| `+0x4a` | ROB handle | 3D model handle for the item |
| `+0x77` | ROB handle (type 1 only) | **Hand model** — the 3D model shown when weapon is drawn |
| `+0x7b` | ROB handle (type 1 only) | **Hilt model** — the 3D model shown when weapon is sheathed |
| `+0x7f` | ROB segment data | **Length/offset** — read from the ROB segment's internal metadata. Used to offset the weapon collision sphere along the item axis. |

For weapon items, `hand_object_file` selects the hand model and
`hilt_object_file` selects the stowed model. The hand model's ROB segment
`bbox_extent_y` supplies weapon length. The runtime reads that word as a signed
32-bit value before preparing the weapon's collision spheres. The X and Z
extents do not supply this length.

## Attachment Transform

The drawn-weapon routine queries the signed RAEX grip vertex pair from the
actor's current animation handle and frame. It adds the supplied local offsets
to each vertex with signed 32-bit wrapping, then transforms both points through
the actor matrix. The transformed points supply the direction calculation.

The two-point direction subtracts the second point from the first with signed
32-bit wrapping. It computes horizontal distance from the X/Z difference using
the integer distance query. With the signed bearing convention whose positive
X axis is 512 and positive Z axis is zero, direction pitch is the bearing of
`(horizontal_distance, delta_y)` minus 512. Direction heading is 1024 minus the
bearing of `(delta_x, delta_z)`. Both results are masked to 2047; direction roll
is zero. Coincident points produce pitch 1536 and heading 1024.

The attachment matrix starts as identity. Its successive rotations use the
negated direction heading, one quarter-turn minus the direction pitch, and the
negated actor roll. Each angle is masked to 2047. The actor retains pitch plus
one quarter-turn, heading, and roll, each masked to the same range.

For each attachment-position component, subtract the first transformed vertex
from the second with signed 32-bit wrapping, shift the difference arithmetically
right one bit, add the first vertex with wrapping, and shift that result
arithmetically right eight bits. Add this integer midpoint to the actor's
floating position and store the sum as single precision. Separately, the actor
retains the first transformed vertex plus its floating position multiplied by
256, converted to a signed integer by truncation.

If attachment allocation fails, the routine performs no vertex queries or
transform publication.

The stowed-weapon routine uses the signed RAEX scabbard vertex pair with the same
local-offset, actor-matrix and direction calculation. It builds the same matrix
and retains the same angle fields. Its attachment position uses the first
transformed vertex rather than the pair's midpoint. Add that vertex to the
actor's floating position multiplied by 256 and truncate to a signed integer.
Shift this integer arithmetically right eight bits and store it as single
precision. An out-of-range integer conversion produces the signed minimum
value before the shift.

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

## Held-item assignment

`HandItem(id)` is separate from inventory selection and weapon drawing. It rejects
assignment while the receiver is in deep water, on an edge, in the air, or holding
a weapon. It also requires an open item system, a receiver, and execution outside
the shell context. Deep water means stored surface 2 or 4; edge motion states are
10 through 19. The in-air predicate includes nonzero falling, sliding, or queued
motion, and motion states 27 through 29.

After admission, the requested ID must resolve to an item record. No owned-count,
visibility, or item-type check is made. Success stores the low eight bits of the
ID in the actor's held-item field. It does not change the selected inventory item,
assign the weapon attribute, or immediately position or draw an attachment. The
function returns zero both after assignment and after a rejected admission.

## Data Flow Summary

Animation playback resolves the model and frame first. Item attachment then
reads selected vertices from that model frame and computes the item transform.
The frame reference and the vertex selection are distinct inputs.

## External References

- [UESP: Mod:RGM File Format § RAEX](https://en.uesp.net/wiki/Mod:RGM_File_Format#RAEX:_Extra_data) — RAEX field names from in-game console
- [RGUnity/redguard-unity `RGRGMFile.cs`](https://github.com/RGUnity/redguard-unity/blob/master/Assets/Scripts/RGFileImport/RGGFXImport/RGRGMFile.cs) — RGMRAEXItem struct definition
