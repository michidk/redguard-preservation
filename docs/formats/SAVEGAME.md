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
An incompatible format version prevents state restoration. A mismatch in the two
compatibility values displays a warning but does not prevent state restoration
when the format version matches.
The stored bitmap filename is separate from the `THMB` thumbnail and `SCR3`
screen-image chunks.

## Saved images

`SCR3` stores a 640 by 480 screen image and `THMB` stores a 256 by 192
book preview. Each pixel occupies one little-endian 16-bit framebuffer word,
with consecutive pixels forming rows. Neither payload has an image header or
an embedded palette. Their nonempty payload sizes are 614400 and 98304 bytes,
respectively. Empty image chunks contain no image.

The book Load path displays `SCR3` behind its progress bar instead of selecting
the destination world's loading artwork. The metadata bitmap filename does not
supply this screen.

The book Load handler disables destination artwork selection before starting the
saved-game restoration. That route configures progress over the saved screen
without drawing from the shared random stream for a loading image. The retained
world-artwork frame index is not a new selection when this policy is disabled.

The ordinary world-artwork route opens the configured GXA. A one-frame image
selects frame zero without consuming shared randomness. An image with multiple
frames consumes one value from the shared engine random stream through the
bounded selector, then stores the selected frame index. This selection belongs
to opening that loading presentation, rather than each progress update. Sky
opening has its own independent random-direction selection.

## State chunks

Save containers include global state and per-world state. Object and script
records depend on the corresponding [RGM](RGM.md) scene and runtime definitions.
They are not replacement scene assets.

The world loader constructs object, rope and code owners before applying their
saved records. The `SMOB`, `SMRP` and `SMCO` readers consume the number of records
established by that initialized world, rather than deriving a count from the
chunk length. `SMSH` includes a count, which must match the initialized shell
count.

Each `SMCO` record starts with a four-byte little-endian code-object identifier
followed by a 204-byte packed runtime body. The reader resolves that identifier
against the initialized code owners. Nonzero saved script references are offsets
rebased against the loaded world's code and native-hook bases. Live list links
and the code owner's runtime-only pointer are retained instead of accepting the
saved pointer values. Matching a world number alone therefore does not supply
the owners and script bases needed to interpret these records.

`SMOB`, `SMRP`, `SMCO`, and `SMSH` hold object, rope, code, and shell state,
respectively. `SVCM` holds camera records, and `SMFM` holds family references.
Other state chunks include `SVGF`, `SMGR`, `SVRG`, `SVGV`, `SVG2`, `SVIT`,
`SVCB`, `SVMV`, and `SVMD`.

Temporary `.TSG` snapshots use the same container and metadata layout.
Their image chunks can be empty. The engine can restore these snapshots during
a world reload after initializing the player.

## Camera records

`SVCM` is 1016 bytes: a little-endian `u32` record count of nine,
nine 112-byte records, and a little-endian `u32` selected-record slot.
The final slot identifies the selected record, independently of the camera's
separate selected-index word.

All numeric record fields are little-endian.

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| `0x00` | 12 | `[i32; 3]` | position_offset | Camera offset. |
| `0x0c` | 12 | `[i32; 3]` | angle_offset | Camera angle offset. |
| `0x18` | 12 | `[i32; 3]` | request | Requested position or displacement. |
| `0x24` | 24 | `[[i32; 3]; 2]` | basis | Retained camera basis vectors. |
| `0x3c` | 4 | `u32` | position_reference | Runtime pointer, reconstructed during load. |
| `0x40` | 12 | `[i32; 3]` | bound_position_history | Retained bound position. |
| `0x4c` | 4 | `u32` | angle_reference | Runtime pointer, reconstructed during load. |
| `0x50` | 4 | `i32` | owner | Saved actor identity; zero and minus one resolve to no actor. |
| `0x54` | 12 | `[i32; 3]` | target_offset | Target offset. |
| `0x60` | 4 | `i32` | distance | Distance before candidate-stage Q8 conversion. |
| `0x64` | 4 | `i32` | minimum_distance | Minimum distance. |
| `0x68` | 4 | `u32` | flags | Camera stages consume the low-byte flags. |
| `0x6c` | 4 | `u32` | maximum_movement | Movement limit. |

The loader resolves each owner against the initialized actors. For a resolved
owner, it reconstructs the position and angle references using that actor's
local position and local angles, and retains the initialized slot's angle offset
instead of the saved angle offset. Every other record byte is copied from the
save. The selected record must have a resolved owner.

The chunk does not contain the published logical pose, render pose, glide marks,
or separate selected-index word. The saved runtime pointer words do not provide
portable references to these values.

`SVGV` retains the previous position at `0x17c`, current position at `0x18c`,
current angles at `0x198`, previous angles at `0x1a4`, two transformed basis
vectors at `0x1b0` and `0x1bc`, scripted-camera active word at `0x22e`, nesting
count at `0x230`, and selected-index word at `0x24e`. Vectors contain three
signed 32-bit components; the active word is 16 bits and the nesting count is
32 bits. The render position at `0x5fa` contains three rounded signed map-unit
coordinates. The reader converts these coordinates to renderer floats.

After restoring the chunks, the session loader selects the record identified
by the low byte of the saved selected-index word, replacing the temporary SVCM
selection. It replaces that record's flags with the sign-extended 16-bit word
at `SVG2` offset `0x08`, clears the transition and screen-shake state, and marks
the next presentation as a cut. That first presentation copies the published
pose instead of interpolating from earlier renderer angles.

## Object-record framing

`SMOB` concatenates variable-length records. Each record begins with the
following header; numeric fields are little-endian. The length excludes its own
four-byte field. The next record therefore starts `4 + record_length` bytes
after the current record's start.

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| `0x00` | 4 | `u32` | record_length | Bytes remaining in this record, including its identity header and payload. |
| `0x04` | 4 | `i32` | object_id | Object identity; `-2` and `-3` select the two player objects, and zero creates an object. |
| `0x08` | 4 | `u32` | rope_id | Rope identity used to reconstruct the actor's attachment. |
| `0x0C` | 1 | `u8` | record_kind | Record-kind discriminator used by the restore path. |
| `0x0D` | variable | `[u8]` | payload | Packed object state and conditional trailing records. |

The full object restore reads a 783-byte packed object body before its
conditional trailing records. The body is not the complete record: additional
state includes an optional script-state body of 422 bytes, attachment node and
motion words, and other object-owned data. The reader seeks to the recorded end
after applying the supported fields. Record framing alone does not resolve the
saved references or establish a restorable actor.

### Conditional object tails

The full object body is followed by these fields in order. Presence offsets
refer to the 783-byte body, not the record header. Unnamed blocks remain opaque.

| Presence condition | Size | Contents |
|---|---|---|
| Byte `0xC1` equals one | 1 or 10 | Presence byte, then nine name bytes when nonzero. |
| Word `0x272` nonzero | 256 | Actor attributes. |
| Word `0x23F` nonzero | 41 | Retained script light controls, strength and modulation. |
| Word `0x243` nonzero | 20 | Retained flat sprite controls and playback, with regenerated texture binding. |
| Word `0x247` nonzero | 37 | Mutable animation header; installed animation allocation is retained. |
| Word `0x23B` differs from `0xFFFFFFFF` | 422 | Script state. Zero is a present identity. The loader additionally requires the initialized script allocation. |
| Record rope identity nonzero | 8 | Attachment node and motion words. |
| Always | 48 | Saved vector blocks of 36 and 12 bytes. |
| Word `0x21` equals one | 60 | Retained block with an object identity in its first word. |
| Word `0x1C` nonzero | `4 × byte at 0x20` | Private variable array. |

Position occupies three signed little-endian 24.8 words at body offsets
`0xCB`, `0xCF`, and `0xD3`. The retained orientation occupies three signed
little-endian words at `0x120`, `0x124`, and `0x128`: pitch, heading and roll.
Angles wrap modulo 2048; heading is the middle word. The saved object references at offsets zero, four,
eight, twelve, and `0x1D4` are object identities.
The unsigned two-byte field at `0x14` is the initialized runtime actor identity.
Ordinary actor records bind to initialized actors by that identity; the special
player record identities remain distinct. The signed word at `0x1D4` names the
supporting actor's SMOB record identity, or minus one when no actor supports the
object. The support record carries its own world position and orientation;
these are not offsets from the supported player. The 422-byte script state
starts with a separate installed actor identity followed by two object identities.
Its other fields include runtime bindings retained from the initialized owner.
Uninterpreted fields and trailing extensions must remain intact during a
lossless edit; they are not portable native pointers.

### Retained actor spatial and controller state

Full object restoration copies the saved local, attached, previous and initial
position and angle triples independently. Angles retain signed word values;
reading the save does not wrap them or rebuild a transform. The stable-position
validity byte and its stored Q8 coordinates are independent fields. A zero
validity byte disables recovery without erasing those coordinates; any nonzero
byte enables it. The separate water-rollback position is retained independently
of the update-entry and stable positions. An ordinary update replaces it when
the surface is not water or falling is active; otherwise it stays unchanged.
See [actor history properties](../engine/SOUPPROPS.md#position-and-angle-history-properties).

The embedded model-instance translation and orientation matrix are retained
single-precision values. They have a separate lifetime from logical position
and angles. Translation uses map units; it must not be replaced by the current
Q8 coordinates divided by 256. The saved matrix must not be regenerated from
the current angle triple. The redundant trailing vector blocks remain part of
the lossless record.

The actor's property-backed controller state is retained with the body,
including the movement counter, pending and accepted action words, motion and
AI selectors, timers, hand/ledge vectors, collision marks and stable recovery
state. The extinguished-torch timer is retained independently of the burning
counter: dousing saves the counter and clears burning; lighting uses the saved
counter, initializing it from the item configuration only when it is zero.
Loading these values does not request an animation, enter a motion or
invoke a native callback. Script writes can leave values outside the ordinary
engine-produced ranges; semantic views preserve their stored widths.

Parent, linked and supporting actors and the optional position-link owner use
saved object identities. Minus one denotes no owner. A consumer binds these
against its independently initialized actor allocations, preserving special
player identities separately from runtime identity words. Missing, ambiguous
or mismatched owners invalidate the complete binding population before native
state is replaced. The optional position-link record retains its allocation
even after its owner is cleared. Its signed owner word is followed by the Q8
offset triple, cached position triple and previous cached position triple.
Detachment clears the owner but preserves the allocation and both caches.

### Selected and previous model frames

The actor body retains five signed little-endian two-byte fields separately
from its optional animation header:

| Offset | Field |
|---|---|
| `0x24F` | Selected model-frame index. |
| `0x251` | Selected runtime model handle. |
| `0x253` | Previous model-frame index. |
| `0x255` | Previous runtime model handle. |
| `0x257` | Stored default animation group. |

Full object load copies these fields unchanged. A minus-one model handle means
no selected model; the accompanying frame word can be stale. Positive handles
bind to the initialized world's model allocations, including distinct texture
variants. They are not file-frame references. See
[save-load pose ownership](../engine/animation.md#save-load-pose-ownership).

### Shell state

`SMSH` starts with a little-endian four-byte shell count. Each shell has a
186-byte body followed by a private variable array of four-byte words. The
body's word at `0x28` gives the array count. The first eight bytes identify the
installed shell by name. Restoration clears the live shell list, constructs each
saved shell from its installed named definition, and retains that fresh shell's
runtime binding and variable storage while copying saved state. The saved shell
count must agree with the world's retained shell count before reconstruction.

Code and shell script references remain offsets in saved data. Nonzero native
hook references are rebased against the world's native-hook base; nonzero script
references are rebased against its script base. These offsets do not identify
instructions in a different interpreter without an explicit mapping.

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

The global clock fields are restored before the saved actor lists are
reconstructed. Observing saved clock values alone therefore does not establish
that actor restoration has completed.

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

## Actor combat restoration

The optional 422-byte allocation framed as `script_state` by the lossless
parser is the actor's combat record. SOUP continuation, task words and authored
locals have separate storage in the object body and `SVRG`.

Combat allocations are constructed from installed model and attribute data
before saved combat state is applied. Loading retains the initialized model
bindings, taunt-list storage, nine attack bindings and four defence bindings.
It resolves the next combat-list reference by allocation index and the owner
and target references by actor identity. The saved RTX-cache pointer is cleared,
and flag `0x40` is cleared. Other dynamic combat values are copied.

The first retained selector addresses the nine installed attack bindings; the
second addresses the four installed defence bindings. Current attack collision
and motion queries use the first selector together with the current motion
selector. The two selectors are independent: a populated first selector and an
absent second selector describe an attack without a pending defence. Executor
state one advances the nine-binding motion sequence; state two advances the
four-binding sequence. Loading preserves these selector values and executor
state rather than deriving them from the animation group or action selector.

All offsets below are relative to the combat allocation. Integer words are
little-endian.

| Offset | Type | Meaning |
|---|---|---|
| `0x00` | `u32` | Next combat allocation index; all-ones means absent. |
| `0x04`, `0x08` | `i32` | Owner and target actor identities. |
| `0x0C` | `i32` | Retained allocation index. |
| `0x28` | `[i32; 3]` | Retained target position in Q8 units. |
| `0x38` | `i32` | Combat action selector. |
| `0x3C` | `[i32; 3]` | Near, far and midpoint spacing ranges. |
| `0x48`, `0x4C` | `i32` | Attack and defence selectors. |
| `0x58`, `0x5C` | `i32` | Strength and armour. |
| `0x60`, `0x62` | `i16`, `i32` | Contact vertex and Q8 radius. |
| `0x139` | `i32` | Aggression budget. |
| `0x13D` | `i32` | Death-default animation group. |
| `0x141` | `i32` | Consecutive close-distance decision passes. |
| `0x145` | `i32` | Attacker coordination selection state. |
| `0x14D` | `i32` | Attack/defence executor state. |
| `0x151` | `i32` | Current move-table selector. |
| `0x159` | `u32` | Combat cooldown. |
| `0x161`, `0x165`, `0x169` | `i32` | Attack pacing, defence pacing and defence counter. |
| `0x179`, `0x17D` | `i32` | Taunt activity and dialogue-slot identity. |
| `0x181` | `u32` | RTX-cache pointer, cleared on load. |
| `0x185`, `0x189` | `i32` | Taunt and spacing periods. |
| `0x18D` | `u8` | Combat flags. |
| `0x18E`, `0x192`, `0x196` | `i32`, `u32`, `i32` | Wall-avoidance side, mask and hold ticks. |
| `0x19A`, `0x19E` | `i32` | Lost-sight ticks and routed-pursuit flag. |

## Actor light and flat records

The optional 41-byte light record stores lock, enable and inactive-tick bytes at
0–2; three signed 16-bit offsets at 3; starting and current intensity at 9 and
11; starting and current radius at 13 and 15; three 16-bit flicker words at 17;
three size-effect words at 23; current signed RGB strengths at 29; and starting
signed RGB strengths at 35. Each modulation triple holds amplitude, phase and
period. Loading copies these values without calling a light native.

The optional 20-byte flat sprite record stores enable and inactive-tick bytes
at 0 and 1; three 16-bit offsets at 2; a texture identifier at 8; a native
bitmap-cache slot at 10; last-frame, counter and current-frame words at 12, 14
and 16; and animation bytes at 18 and 19. Loading regenerates the cache slot
from the saved texture identifier while retaining the playback words.

## World spatial and family tables

`SMGR` starts with a little-endian `u32` list count. Each entry contains a
little-endian `u32` spatial-list identity followed by a signed `i32` actor
identity for the first serialized actor in that list. The entries are followed
by 16,384 little-endian `u16` cell-to-list identities, occupying 32,768 bytes.
Zero cells denote no list. The writer omits leading actors whose runtime
identity is zero when choosing the saved head. It can clear cells belonging to
an empty object-only list in the serialized copy without changing the live grid.

Full object load rebuilds ordinary spatial membership rather than retaining the
serialized cell-list links. For each restored object whose runtime update
exclusion byte is zero, it clears both list links and the cell-list identity,
then prepends the object to its restored position cell. Objects are processed
in `SMOB` record order. An attached object uses its restored attached position
for this insertion while retaining its local position. A bound position-link
cache takes precedence over that position during cell insertion. Consequently, actors
restored into the same cell appear in reverse `SMOB` record order. Regional
selection subsequently visits those cells in row-major order.

`SMFM` contains 256 consecutive 28-byte family slots. Each slot contains a
signed little-endian actor identity followed by its accumulated group position
in Q8 map units and accumulated group angles. Each vector has three signed
little-endian 32-bit components. All vector words are copied on load,
including vectors in slots with no actor. Actor identities zero and minus one
resolve to no actor; minus two and minus three select the two player owners.
Other identities resolve against initialized actors. These tables are independent
of each actor's linked-position record. Family updates overwrite entries for
masters they visit; absent entries retain their vectors. `FixSlaveAngle` reads
the retained group-angle entry directly, including a slot with no actor.

`SMRP` consists of 142-byte records: a four-byte rope identity followed by a
138-byte body. The first three words of the body are rope list references,
resolved by rope identity. The saved body also contains pointer fields that
must be replaced with initialized runtime owners as described under
[rope jump latch](#rope-jump-latch).
The retained oscillator fields are current amplitude at body offset `0x56`,
phase at `0x6A`, link step at `0x66`, pending push at `0x62`, direction byte at
`0x38`, and preceding link-step sum at `0x5E`. All except direction are signed
32-bit little-endian words. Restoring the body copies these values without
advancing the oscillator or invoking the amplitude service.


## Actor navigation restoration

The object body retains the actor's current walk-map identity and four signed
node indices. The save writer replaces the live map pointer with the map's
runtime identity; all-ones means no map, and zero is a valid map identity.
The reader resolves that identity against the initialized walk-map list before
copying the object body. An unknown identity is an error. It does not choose a
nearest map or node from the restored position.

| Object-body offset | Type | Meaning |
|---|---|---|
| `0x1E9` | `u32` | Walk-map identity, or all-ones for no map. |
| `0x1ED` | `i32` | Held current node. |
| `0x1F1` | `i32` | Held next node. |
| `0x1F5` | `i32` | Previous-next node history. |
| `0x1F9` | `i32` | Final destination node. |

The node indices are copied from the saved body, including minus-one sentinels
and stale values. They are not replaced by initialized placement navigation.
The navigation core performs its ordinary map/index validation when it runs.
These fields are independent of the optional script-state allocation and of
each task slot's four native invocation words.

After full save restoration, the main load path clears every walk-node
reservation byte. It does not reclaim reservations from the restored actors'
held indices. A retained next-node index can therefore coexist with an unowned
node after load and on resumed walking updates. Later navigation assignments,
releases and periodic maintenance apply the ordinary
[walk-map ownership rules](../engine/SOUP.md#navigation-map-validation).

## Actor script-state ownership

`SMOB` retains the actor's existing script-state pointer instead of restoring the
saved pointer value. When that actor has script state, its saved script-state
record is copied into the existing allocation. The main save reader restores
object and rope records before inventory, then reads `SVCB` into the selected
player's existing script state. These records depend on initialized actor script
owners; they do not allocate a replacement selected-player script owner.

The script-state copy preserves the live taunt-pointer array, all nine attack
references and all four defence references. Saved pointer values do not replace
these dependencies of the initialized owner.

## Engine globals

The main save (`SAVEGAME.SAV`, not the per-world `.TSG` files) also writes an `SVG2` chunk after
`SVGV`. Its payload is a fixed 1,666-byte concatenation of retained engine globals without padding or
per-field headers, in this order:

| Size | Contents |
|---|---|
| 4 | Retained player health, copied into both player records at load. |
| 4 | Compass display flag; nonzero draws the compass. |
| 2 | Retained state word, written only by the save loader. |
| 12 | Retained state block. |
| 1600 | The journal tag table: 400 four-byte tags, zero marking an empty entry. |
| 4 | Selected-player switch. |
| 4 × 5 | Power-up slots: the two slots' `ShowBitmap` IDs, their two retained atlas-frame indices (`-1` empty), then the next slot (zero or one; see [power-up slots](../engine/hud.md#power-up-slots)). |
| 4 × 4 | Four retained world-state words written when a world loads. |
| 4 | Health-candle feedback countdown. Zero is inactive; 50 pins the display; positive values below 50 decrease after drawing by the elapsed nominal ticks, stopping at zero. |

## Inventory

`SVIT` contains 22-byte records in initialized item-list order,
followed by a signed 32-bit selected-item ID. All numeric fields are little-endian.
The payload contains neither an item count nor per-record IDs. Its length is
`22 × saved_record_count + 4`. The save reader and writer traverse one fewer
record than item initialization creates. The reader leaves the final item
unchanged; it has no record in `SVIT`. Loading into an existing item list
therefore retains that item's current values.

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| `0x00` | 1 | `u8` | hide | Current item visibility field. |
| `0x01` | 1 | `u8` | pushed | Nonzero while a pushed value is retained. |
| `0x02` | 4 | `i32` | total | Current item total limit. |
| `0x06` | 4 | `i32` | player_max | Current player ownership limit. |
| `0x0A` | 4 | `i32` | player_total | Current owned count. |
| `0x0E` | 4 | `u32` | item_used | Wrapping use count returned unchanged by `ItemUsed`. |
| `0x12` | 4 | `u32` | saved_player_total | Count saved by the first push and restored by a pop. |

The pushed byte and saved count are restored directly, without normalizing the
byte or replacing the saved count with the current count.
The use count is also restored directly. A completed use increments it with
32-bit wrapping; refused attempts and torch dousing do not increment it.
[PushItem and PopItem](../engine/SOUP.md#pushitem-saved-value-replacement) use
these fields: the first push saves the current count, later pushes retain that
saved count, and a pop clears the pushed byte and restores the saved count.

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

## Actor task slots

The object body retains six SOUP task slots. Each slot combines the following
parallel fields; slot order is allocation and execution order.

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| `0x27F` | 24 | `[u32; 6]` | task callbacks | Six little-endian callback references. Zero is empty; a nonzero reference names the byte after the task opcode. |
| `0x297` | 24 | `[u32; 6]` | task variables | First native invocation word for each of the six slots. |
| `0x2AF` | 6 | `[u8; 6]` | task status | Blocking-status bytes. |
| `0x2B5` | 6 | `[u8; 6]` | task pause | Pause bytes. Nonzero suspends invocation and timer advancement. |
| `0x2BB` | 12 | `[u16; 6]` | task timers | Six little-endian wrapping invocation counters. |
| `0x2C7` | 24 | `[u32; 6]` | task variables 2 | Second native invocation word for each slot. |
| `0x2DF` | 24 | `[u32; 6]` | task variables 3 | Third native invocation word for each slot. |
| `0x2F7` | 24 | `[u32; 6]` | task variables 4 | Fourth native invocation word for each slot. |

Callbacks retain the saved script-reference domain and require the initialized
world's script base. The counter is not elapsed game time: a task invocation
computes elapsed input from its counter and the current frame scale as described
in [SOUP task execution](../engine/SOUP.md). Optional private-variable tails contain signed little-endian 32-bit SOUP
values separately from authored locals in `SVRG`. Playback, both local
allocations, script cursors and task slots are independent parts of a saved
continuation.


## Actor script continuation

The actor body retains script data, entry, cursor and return references at
`0x260`, `0x264`, `0x268` and `0x26C`. The current and saved script result bytes
follow at `0x270` and `0x271`. References retain the saved code domain and must
be bound against the initialized world. Result bytes and all four invocation
words of each task slot are retained without normalization, including empty
and paused slots. Restoring a cursor without its task, authored-local and
native invocation state does not restore the actor's continuation.

## Authored script locals

`SVRG` stores the current world's authored script-local storage as a flat array
of little-endian signed 32-bit words. Its length is a multiple of four. The
loader copies this array into the initialized world's variable allocation;
it does not reconstruct its ownership from adjacent saved objects.

Each `SMOB` body's word at `0x276` is a byte offset into that array. The value
`0xFFFFFFFF` means no authored-local allocation; zero is a valid offset. Script
variable indices address words relative to that actor's offset. The range
length comes from the matching initialized script definition, including its
instance allocation, rather than the optional object-private count.

The optional `SMOB` private-variable tail is a separate allocation, identified
by the body pointer marker at `0x1C` and byte count at `0x20`. An empty private
tail does not imply that the actor has no authored locals. Neither allocation
is interchangeable with the four native invocation words per task slot.

`SaveScriptLocals` encodes the complete signed-word pool without normalization.
Construction from words preserves their order; it does not derive actor offsets
or replace missing live allocations with installed initial values. Encoding a
local pool alone does not produce a complete original save slot.

`parse_save_script_locals` decodes the `SVRG` payload. `bind_actor` binds a
bounded range to the saved object and runtime identities using the installed
definition's word count. Consumers must establish the matching initialized
world and script definition before supplying that count; a name or adjacent
actor offset alone is insufficient. Binding does not relocate code references,
execute a callback, or advance a timer.

## Code and shell continuations

Code owners and shells execute through a cleared temporary actor record. The
persistent owner supplies the script references, result bytes, task callbacks,
status, timers and four native invocation words. Updated fields are copied back
after execution. Neither compact owner retains an Anchor byte or a task-pause
array. Those fields start at zero for each temporary actor execution; they are
not restored from an ordinary actor or a fresh-world baseline. Ordinary actors
retain Anchor at body offset `0x25B` and retain their own pause array.

The following offsets are relative to each packed body. Arrays have six entries
in slot order, with words stored little-endian.

| Field | Type | Code owner | Shell |
|---|---|---|---|
| Script data / entry / main / return | `[u32; 4]` | `0x23` | `0x12` |
| Current / saved result | `[u8; 2]` | `0x33` | `0x22` |
| Task callbacks | `[u32; 6]` | `0x42` | `0x30` |
| Invocation word one | `[u32; 6]` | `0x5A` | `0x48` |
| Blocking status | `[u8; 6]` | `0x72` | `0x60` |
| Invocation timers | `[u16; 6]` | `0x78` | `0x66` |
| Invocation word two | `[u32; 6]` | `0x84` | `0x72` |
| Invocation word three | `[u32; 6]` | `0x9C` | `0x8A` |
| Invocation word four | `[u32; 6]` | `0xB4` | `0xA2` |

A code owner's word at `0x3A` is its authored-local byte offset in world `SVRG`,
using the same all-ones absent sentinel as ordinary actors. The matching
initialized script definition supplies the local count. Code-owner identity
and installed source must both match before binding that range.

A shell owns its local allocation separately from `SVRG`. Its saved count at
`0x28` determines the words following the 186-byte body. Loading reconstructs
the shell by installed name, retains the new allocation pointer, and reads the
saved words into that allocation. Consumers must validate the saved count
against the matching installed definition. The pointer at `0x24` is not a
portable binding. `bind_locals` returns signed words tied to the saved name
only after count validation; `bind_code_owner` bounds a code owner's world
range and returns its identity with the words.

## External References

- [UESP: Redguard file formats](https://en.uesp.net/wiki/Mod:Redguard_File_Formats)

### Map-code allocation identity

The map-code owner counter starts at zero for a world load. Each type-two MPOB
placement consumes its next value in installed placement order, independently
of the editor object ID. The resulting identity is the SMCO owner ID. Cell
linked-list insertion and save traversal order do not change that identity.

Type-six coloured-light placements allocate compact records but do not consume
the type-two identity counter. Their compact records carry the light marker and
are excluded when collecting SMCO save owners. They must not shift the saved
code-owner binding for later type-two placements.

### Spatial task continuation words

The first two words of a RotateByAxis slot retain its signed fixed-point target
and accumulator. RotateToAxis retains its signed movement and accumulator in
those words. MoveObjectAxis retains signed target and traveled distance.
MoveByAxis retains its signed target in the first word. The remaining words of
these handlers are zero. Loading these slots preserves the retained words;
continuation must not run the initial callback to reconstruct them.

AmbientSound retains its sample slot in the first word, with all-ones meaning
no sample. Its other three words are zero. A retained active sample index refers
to the playback service owner, not to a new playback request.

### Dialogue and animation task service ownership

An RTX slot retains its signed dialogue-record index in the first native word
and its signed sample slot in the second. All-ones is the absent sample; the
remaining two words are zero. These words do not serialize the dialogue record
or audio payload. World-state loading clears the shared dialogue table and
sample channels before restoring object tasks. A resumed RTX task observes the
cleared record and completes without looking up or replaying its saved label.

PushAnimation and PushControlAnimation retain their started phase in the first
native word, zero before starting and one afterward. Their other three words
are zero. Playback belongs to the receiving actor's saved animation record,
not the task slot. Load restores that record separately, including its current
and pending group, command cursor, countdown and loop count. The first resumed
handler re-evaluates its group operand. A started task observes the receiver's
current group without repeating its request or resetting its loop count. An
unstarted task follows ordinary request arbitration and sets its requested
loop count only once the requested group is current.

## Gameplay globals and full-load ownership

The following `SVGV` fields are little-endian. Offsets are relative to its
1,542-byte payload.

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| `0x010` | 4 | `i32` | activation_consumed_use | Successful activation retains action consumption until release. |
| `0x030` | 4 | `i32` | hit_inhibited | Selected-player hit inhibition. |
| `0x034` | 4 | `i32` | death_animation_override | Requested death-animation group. |
| `0x038` | 4 | `u32` | death_progress | Death-controller elapsed nominal ticks. |
| `0x040` | 4 | `i32` | death_mode | Death-effect selector. |
| `0x048` | 12 | `[i32; 3]` | death_position | Captured local position in signed 24.8 map units. |
| `0x05C` | 4 | `u32` | falling_elapsed | World-loop falling timeout in nominal ticks. |
| `0x060` | 4 | `u32` | action_ticks | Unconditional player-controller preliminary countdown in nominal ticks. |
| `0x064` | 4 | `i32` | lineup_result | Shared lineup result and selected-player combat-entry inhibition. |
| `0x090` | 4 | `u32` | world_loop_frame | World-loop iteration counter. |
| `0x10C` | 4 | `i32` | idle_counter | Ordinary idle counter advanced by clock measurement windows. |
| `0x130` | 4 | `u32` | body_response_contacts | Retained body-response count used by the sliding-state test. |
| `0x224` | 2 | `u16` | lockout | Player lockout word. |
| `0x5EA` | 4 | `i32` | health_candle_level | Retained candle level used by health animation. |
| `0x5EE` | 4 | `i32` | health_candle_frame | Retained frame within the candle level. |
| `0x5F2` | 8 | `[i32; 2]` | health_candle_position | Health-candle drawing coordinates. |

Reading these fields and entering the world loop are separate operations.
After a full load, world-loop entry clears hit inhibition, the death-animation
override, death progress and the iteration counter. It also clears the dragon's
breath cooldown. Death mode, captured position, falling elapsed time, the saved
action countdown, idle counter, lineup result and lockout retain their loaded
values. The full-load finalizer clears the pending player-action flag before
world-loop entry.

The secondary preliminary countdown and ledge-action inhibition are the same
unsigned 32-bit global. The lineup result and selected-player combat-entry
inhibition are likewise the same signed word, rather than independent state.

A full save does not serialize every gameplay global. The loader retains the
running session's maximum health, conditional preliminary countdown, secondary
preliminary countdown, sound-maintenance inhibition, script-menu mode, potion
counters, queued movie and selected-player ledge result. These values require
an established prior session; a missing save field does not request a new
player initializer. The standalone player initializer clears the preliminary
countdowns and sound-maintenance inhibition and assigns maximum health 100.

The script menu's existing entries are rebuilt during full load. Shared
dialogue records and sample channels are cleared before actor task restoration;
no active dialogue line is reconstructed from a saved RTX task. World-loop
entry resets the sound-maintenance counter, while retaining its inhibition.


Full loading also retains the running session's dialogue-mode word, item-viewer
request, item-use latch and cached ground-prevalidation results. The item-viewer
request includes its pending switch, no-text switch, item and two request words.
The ordinary actor wrapper clears the prevalidation-used switch before its next
query; that query replaces the cached rollback and sliding results. The item-use
latch clears when the action input is released. The remembered topic-menu text
clears when `DialogBegin` or `LockoutPlayer` runs.

The actor byte at body offset `0x237` is the runtime object-update exclusion.
Zero admits ordinary updates; a nonzero byte suppresses the ordinary animation
and main-script stages. The public controller view retains this raw byte in
`active`; it is an exclusion flag rather than an inverted enable flag.
