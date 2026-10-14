# Positional Sound

How sound effects from [MAIN.SFX](../formats/SFX.md) and speech from
[RTX](../formats/RTX.md) get their volume, stereo pan and playback rate.

## Units

- Engine positions are stored in world units × 256 (eight fractional bits).
  World units are the units `SYSTEM.INI` distances use.
- Angles use 2048 units per turn, wrapped modulo 2048.
- Volumes and pans are Miles Sound System sample values: volume 0–127, pan
  0–127 with 64 at the centre (Miles: 0 = left, 127 = right).

## Listener

The listener is the render camera, not the player: its position is the view
position and its heading is the view yaw. In third person, distances are
therefore measured from behind Cyrus. Only the yaw enters the calculation;
camera pitch and roll do not.

## Radius and divisor

`SYSTEM.INI [system] sound_distance` (shipped `64`) is read once at startup
into two values, which the engine's debug sound page labels "Sound radius" and
"Sound divisor":

| Value | Formula | Shipped |
|---|---|---|
| Radius `R` | `sound_distance × 36` world units | 2304 |
| Divisor `D` | `floor(65536 / floor(R / 128))` | 3640 |

A `sound_distance` of 3 or less makes `floor(R / 128)` zero, a division by
zero in the original; such values are unsupported.

## Distance

For emitter `E` and listener `L` in engine units (world units × 256):

```
d = floor(round(sqrt(dx² + dy² + dz²)) / 256)      (dx, dy, dz = E − L)
```

The distance is three-dimensional (height counts) and floored to whole world
units. The square root runs in floating point and is rounded to the nearest
integer before the shift; the 32-bit float intermediate only matters beyond
about 65536 world units.

The emitter is the emitting object's position, with two exceptions:

- An object whose `attr_slave` attribute (index 26 in `SOUP386.DEF`) is set
  emits from its attached position: its master's position plus its own offset
  rotated by the master.
- Otherwise, an object that links to another positioned record emits from
  that record's position.

Animation PlaySound always uses the object's own position.

## Volume curve

With a signed per-call volume offset `off`:

```
if d > R:  v = 0
else:      a = clamp((d × D) >> 16, 0, 127)
           v = clamp(127 − a + off, 0, 127)
```

Volume falls linearly from 127 at the listener. With the shipped constants the
attenuation reaches 127 at `d = 2287`, so a sound with `off = 0` is already
silent there, slightly before the radius. A positive offset cannot make a
sound beyond `R` audible.

The addition of `off` wraps as a signed 32-bit sum before the final signed
clamp. An overflowing positive offset can therefore produce zero volume.

## Pan

Pan uses only the horizontal plane (x, z):

1. `a` = the engine's integer heading from listener to emitter: 0 when the
   emitter lies along +Z, 512 along +X, 1024 along −Z, 1536 along −X.
   - Take `|dx|` and `|dz|`; while either exceeds 511, halve both
     (arithmetic shift).
   - If `|dx|` is now 0: `a` = 0 when `dz ≥ 0`, else 1024. Otherwise, if
     `|dz|` is 0: `a` = 512 when `dx ≥ 0`, else 1536.
   - Otherwise, with `s = min(|dx|, |dz|)` and `b = max(|dx|, |dz|)`:
     `t = T[floor(s × 512 / b)]`. The base angle is `t` when `|dx| < |dz|`,
     `512 − t` when `|dx| > |dz|`, and 256 when equal.
   - Quadrant: only `dx < 0` gives `−base`; only `dz < 0` gives
     `1024 − base`; both negative give `base + 1024`. The result is taken
     modulo 2048.
   - `T[i] = trunc(atan(i / 512) × 65536 / π) >> 6` for `i` = 0..512, built at
     startup (truncation toward zero, then the shift).
2. `u = (a − yaw) mod 2048`.
3. Triangle wave:

| `u` | Offset `p` |
|---|---|
| 0–511 | `u >> 3` |
| 512–1023 | `((1024 − u) & 511) >> 3` |
| 1024–1535 | `−((u − 1024) >> 3)` |
| 1536–2047 | `−(((2048 − u) & 511) >> 3)` |

`pan = 64 + p`, range 1–127. Sounds straight ahead or straight behind are
centred; a quarter turn puts them fully to one side. At exactly `u = 512` and
`u = 1536` the `& 511` yields zero, so a sound exactly abeam is centred for
that single angle value. Pan does not depend on distance.

## Starting a sound

Every effect start takes an effect index, a playback-rate offset in Hz, a
volume and a pan.

- **Playback rate:** `clamp(effect sample_rate + rate_offset, 512, 44000)` Hz.
  The addition wraps at 32 bits before the signed clamp.
- **Looping:** see [SFX loop behaviour](../formats/SFX.md#loop-behavior).
- **Effect range:** a negative effect index or an index at or beyond the number
  of effects in `MAIN.SFX` (118 shipped) starts nothing.
- **Channels:** 16 sample channels. A start takes the first idle channel at or
  above a start index. When none is idle the sound is dropped; playing sounds
  are never stolen. The start index is 0 for the player's own animation sounds
  and for speech, 4 for combat hit sounds, and 6 otherwise (0 at boot until the
  first sound that resets it).
- **Ownership:** each channel records its owner object and the requested
  effect index (before any SURFACE.INI remap).
- **Handles:** a remembered sample handle identifies a reusable channel slot,
  not a particular start. Stopping by handle acts on the slot's current sample
  without comparing its owner or requested effect.
- **Master volume:** every volume set, at start and on later updates, sends
  `(v × master) >> 8` to Miles, where `master` is `SYSTEM.INI [system]
  volume` (shipped 255, engine default 256 before the file is read) or the
  menu Sound slider (see [menu](menu.md)). There is no other global effect
  gain. A changed master reaches a playing sound only at its next volume set;
  one-shots keep their start volume.

### Animation PlaySound

Animation opcode 4 ([attachment](attachment.md)) starts a positional sound:

- `effect` = first signed 10-bit field, remapped through
  [SURFACE.INI](../config/surface-ini.md#sound-remap-records) by the animated
  object's surface.
- `rate_offset` = second signed 10-bit field × 64 Hz.
- Volume from the curve with `off = 0`, pan from the animated object.
- The sound starts even when `v = 0`; it then occupies a channel silently.
- The handle is stored on the animation; EndAnimation (opcode 1) stops it.
  No later volume or pan update was found for these sounds.

Volume and pan use the actor and camera positions when the PlaySound command
executes. They do not use the actor's later position at the end of the update.
During ordinary player walking, animation advancement can start a sound before
forward movement and ground response change the player's position.

Each encountered PlaySound advances to the next command without consuming a
frame skip. A second PlaySound replaces the animation's remembered handle
without stopping the previous channel. Selecting a replacement group clears
that handle without issuing a stop. EndAnimation stops only the handle still
remembered by that animation. Channel validity and completion are handled by
the audio mixer independently.

The metadata field historically named `volume_shift` contains the second
signed ten-bit operand. It is a rate offset in units of 64 Hz, not a volume
adjustment.

### SOUP functions

`SOUP386.DEF` declares these sound functions:

| Function | Kind | Parameters | Behaviour |
|---|---|---|---|
| `Sound` | function | effect, volume offset, rate offset | Positional start: volume from the curve with `off` = volume offset, pan at start. Starts even at `v = 0`. No later updates. |
| `FlatSound` | function | effect, volume, rate offset | Non-positional: pan 64, `v = clamp(base_volume + volume, 0, 127)`, where `base_volume` is SFX header byte `0x0C` (64 in all 118 shipped effects). `FlatSound(e, 0, 0)` plays at 64. |
| `AmbientSound` | multitask | effect, volume offset, rate offset | Each run: compute volume and pan. If `v = 0`, or the runtime object-update exclusion is set, stop this task's sound and end the multitask. Otherwise start the sound if it is not playing (never started, or a one-shot finished), then set the current volume and pan. |
| `AmbientRTX` | multitask | RTX label, volume offset, unused | First run: start the label's [RTX](../formats/RTX.md) clip at pan 64 and its header volume, start index 0. Each run: compute volume and pan; if `v = 0` or the object has been killed, stop the clip and end the multitask; otherwise set the volume and pan. The multitask ends once the clip has finished; it is not restarted. No dialogue line or subtitle is created. |
| `EndSound` | function | effect | Stop every playing channel whose requested effect is `effect`, whatever its owner. |
| `EndMySound` | function | effect | Stop every playing channel whose requested effect is `effect` and whose owner is this object. |
| `EndMySounds` | function | — | Stop every playing channel owned by this object. |
| `StopAllSounds` | function | — | Stop all 16 channels. |

`FlatSound` adds its volume offset to the header base volume with 32-bit
wrapping arithmetic before clamping the signed result to 0–127.

`FlatSound`, `EndSound`, `EndMySound` and `EndMySounds` return zero,
including when they start or stop no channel.

`Sound` returns zero as its SOUP scalar result, including when no sample starts.

Each `AmbientSound` task retains its own sample handle in native task state.
Whenever its elapsed input is zero, or the shared sample-reset flag is set,
it discards that remembered handle before querying playback status. Discarding
the handle does not itself stop a channel. The engine's periodic sound-reset
path stops all sample channels and sets this flag, allowing active ambient
tasks to acquire replacement channels when they next run.
The `RTX` dialogue task instead completes immediately when this reset flag is set,
before checking its distance or loading an entry.

Sound maintenance runs after clock sampling and before ordinary object updates.
It clears the shared reset flag each invocation. Its accumulator starts at zero
when the world loop starts and advances by the shared whole nominal ticks when
the runtime inhibition flag is clear. A signed accumulator value greater than
1400 triggers the reset and clears the accumulator; exactly 1400 does not.
RTX initialization creates twenty cache slots with no text or audio payload,
cleared occupancy, and no remembered sample. Dialogue initialization allocates
`max(dialog_max_dialog_lines, 16)` records from `SYSTEM.INI [dialog]` and clears
their stored state. The topic-menu capacity is `max(dialog_max_menu_items, 300)`,
and text width is `max(dialog_max_text_width, 150)`. A configured
`menu_traverse_delay` of zero becomes one BIOS tick. `WaitOnDialog` scans every record and remains active while
any record has its active word equal to one. It does not filter by the caller.

Dialogue creation selects the first inactive record starting at index one. If
no record is available, it returns no record. An admitted record stores the
speaker, formatted text, active state and the RTX entry's remembered sample
handle. Clearing a record removes its active state and text and replaces the
remembered sample handle with the absent-handle value. It leaves the remaining
record fields intact. Querying an absent record reports inactive.

`RTXp` uses record zero and assigns the selected player as its speaker,
regardless of the calling actor. It replaces that record directly rather than
searching for an inactive record. Clearing dialogue by an RTX cache entry's
remembered sample scans active records starting at one and leaves record zero
alone. Matching uses the remembered sample slot, not speaker identity.

The `RTXp` task completes when the shared sample-reset flag is set. Otherwise,
its first zero-elapsed invocation loads the label once and replaces record zero
when lookup succeeds. Later invocations query that record's activity without
reloading the label. The handler has no speaker-distance gate.

Dialogue lifetime is initialized from the loaded audio duration when a sample
handle is remembered. Duration uses integer units of 1/256 second:
`(PCM_byte_count << 8) / (channels * sample_rate * bytes_per_sample)`.
When no sample is remembered, lifetime is fifteen times the original text byte
count before display formatting.

RTX lookup first reuses a cached entry with the same label, marking it used and
resetting its age to eight nominal ticks. Otherwise it selects the first entry
whose used flag is clear, even if that entry still retains a different label and
payload. Loading a new entry replaces its retained text and audio payloads and
marks it used with age eight. If every entry is marked used, lookup reports a
cache-exhaustion error rather than evicting a used entry. File lookup lowercases ASCII uppercase letters in the four-byte label.
A voiced entry with no remembered sample starts playback at sample start index
zero, then restores the shared start index to six.

Periodic cache updates retain an entry unchanged while its remembered sample has
playing status four. Otherwise they clear the remembered sample. An entry used
since the preceding cache update clears its used flag and resets its age to eight
nominal ticks. An unused entry is released when its age is strictly less than the
current nominal tick count; equality subtracts to zero and retains the payload.
Release clears text, audio, occupancy, label and remembered sample.
The full cache reset releases text first, stops a remembered sample only when its
status is four, then releases audio and clears occupancy, sample and label.

Dialogue record updates visit records in index order. Each active record first
sets its speaker's dialogue-inhibition attribute to one, then subtracts 21 times
the whole nominal tick count from its signed lifetime counter. With speech
enabled, a remembered sample that is still playing has its volume refreshed
and its line presented; a finished sample clears the line. Either sample case
ends the update. A line without a remembered sample remains active and traversal
continues. With speech disabled, a positive remaining lifetime presents the line
and ends the update; an expired line is cleared and traversal continues.

Before record traversal, dialogue updates edge-detect their skip-input signal.
The signal is the combined `KEYS.INI` `key_a` binding pair, the walk/run modifier.
A released signal clears the latch. A new pressed signal sets the latch and,
only when player lockout equals one, visits every record, stops any remembered
sample that is playing, clears the record, and ends the update. Holding the
signal does not repeat that cleanup. The latch starts clear. Fresh dialogue
records have zero-valued sample fields; clearing replaces these with the absent
handle, so fresh inactive records and cleared records have different stored
sample values.

The reset releases cached RTX text and audio payloads and clears their occupied
state. It also clears active dialogue records and their text buffers, resets
their sample handles. These operations precede stopping all sample channels
and setting the shared reset flag. It then clears the ownership byte of every
node in every loaded walk-node map, regardless of the owning actor.

The runtime menu-state word has an initial value of zero.
`MenuBegin` sets it to one; `MenuEnd` clears it.
The SOUP `menuProc` task activates this state when it is inactive and clears
it when a selection is accepted. These calls occur inside the native task
handler.
During ordinary actor processing, an active menu-state word or a nonzero
`WaitOnDialog` result sets the maintenance inhibition flag. When both are zero,
the flag clears only if player lockout is also zero. A nonzero player lockout
therefore retains the prior inhibition value. This menu-state word is separate
from the dialogue-state word controlled by `DialogBegin` and `DialogEnd`.

[`TaskStop`](SOUP.md#animation-task-ownership-and-stopping) removes the native
task slot without invoking its handler. Stopping an ambient task therefore
does not itself issue a sample-stop command. Sound ownership commands are separate from task-slot removal.

`KillMe` sets the object's killed flag and stops every channel the object
owns; `Resurrect` clears the flag.
`KillMe` releases held walk-node reservations, clears the current model
reference, sets the object's killed flag to 1 and runtime death-state selector
to 8, and stops every channel the object owns.

`Resurrect` always releases held walk-node reservations. When the killed flag
is 1 or the runtime death-state selector is 8, it clears the killed flag. If
the actor has combat state, it reinitializes that state and consumes one waiting
SOUP message when present.

**Owner:** the owner recorded by `Sound`, `FlatSound`, `AmbientSound` and
`AmbientRTX`, and matched by `EndMySound` and `EndMySounds`, is the calling
object, except for two kinds of script that run on a temporary object:

- Scripts run by the shell system (for example an item's `use_script`, see
  [ITEM.INI](../config/item-ini.md)) record the shell entry as owner.
- Scripts of code objects (scripts without a world object) record the code
  object as owner.

So an ambient sound follows the listener while its multitask runs, restarts
non-looping effects when they end, and stops for good once the listener leaves
its range; the script must call it again to resume.

### Engine-internal starts

- **Combat hits:** effect from the attacker's `hit_sound` attribute,
  `off = 0`, `rate_offset = random(0..127) × 16 − 1024` Hz (−1024 to
  +1008), start index 4. Not started when `v = 0`.
- **Speech:** [RTX](../formats/RTX.md) clips start at pan 64 with rate offset
  0 and volume `clamp(header byte 0x0C, 0, 127)` (100 in the shipped
  `ENGLISH.RTX` audio headers), start index 0. While the dialogue runs, the
  volume is refreshed every frame from the speaker's distance (curve with
  `off = 0`); the pan is not updated. The raw RTX sample start clears its
  sample user-data slots and does not assign an object owner. Dialogue speaker
  identity is retained separately by the dialogue record.
- **Rope sounds:** each [MPRP](../formats/RGM.md#mprp-rope-chains) rope
  record names three effects. A rope plays one of them when its swing
  velocity changes sign (or stays at zero), chosen by its current swing speed
  against `q = trunc(m / 4)`, where `m` is the rope's maximum swing speed:
  the third effect from `3q`, the second from
  `2q`, the first from `q`; a zero effect falls through to the next lower one.
  Ropes whose `q` is 2 or less play nothing. Volume from the curve with
  `off = 0` at the rope's position, rate offset 0, start index 6; not started
  when `v = 0`. Of the 29 shipped rope records, 23 name effects 62, 62, 63,
  one 62, 62, 62, and five none.
- **Torch:** while the player's torch burns, effect 25 plays from the player
  with `off = −40` on one shared channel, owned by the player. It is
  restarted whenever it has finished, its volume and pan are refreshed every
  frame, and it stops when the torch goes out.

## Mixer gain and pan law

The statically linked Miles mixer turns a sample's volume and pan into two
linear scale factors whenever either changes:

- Volume and pan are clamped to 0–127. The effective volume is
  `trunc(volume × driver_master / 127)`, clamped to 0–127; the driver master
  starts at 127 and the game never changes it, so it equals the volume.
- A non-zero effective volume `v` scales samples by `(v + 1) / 128`; zero is
  silent. Loudness is therefore linear in volume.
- Pan uses a 128-entry table `T`: `T[p] = 2p` for `p` 0–62 and `T[p] = 128`
  for 63–127. One output side scales by `T[127 − pan] / 128`, the other by
  `T[pan] / 128`. At the centre (64) both sides play at full level; towards
  either extreme the far side fades linearly to silence while the near side
  stays at full level. By Miles' convention the `T[127 − pan]` side is left.

The pan table contains `0, 2, 4, …, 124`, followed by `128` from entry 63.

## SURFACE.INI remap

At startup the engine builds a table `[surface 0..11][sound 0..255] → effect`,
each row initialised to identity, from the [SURFACE.INI](../config/surface-ini.md)
sound sections in this order: `unknown` (0), `water` (1), `deepwater` (2),
`scapewater` (3), `scapedeepwater` (4), `lava` (5), `sand` (6), `wood` (7),
`tile` (8), `scape` (9), `rock` (10), `gloop` (11).

Only animation PlaySound uses it: when the animated object's surface value is
non-zero, `effect = table[surface][effect]`. `Sound`, `AmbientSound`, combat
hits and speech are not remapped. The configured `unknown` row is bypassed.
Missing sound sections retain identity; repeated entries overwrite earlier
ones. Targets are unsigned 16-bit effect IDs and still pass through the sound
bank's playback validation.

The lookup helper accepts nonzero surface codes up to and including 12, though
only rows 0 through 11 exist. Code 12 reads beyond the allocated table and has
no defined remap. Codes greater than 12 and an absent remap table return the
input effect unchanged. There is no source-ID bounds check in this helper;
animation callers must supply an ID in `0..255` for a valid row lookup.

An object's surface value comes from the face it stands on:

- A model face: the surface type that `[surfaces]` assigns to the face's
  texture, or 0 when the texture has no entry.
- Terrain: each query first resolves texture set 302 and the tile's low-six-bit
  image index through `[surfaces]`. The terrain blend mask can replace that
  value with sand or rock at dry shore pixels. A nonzero water result then
  selects `scapewater` (3), or `scapedeepwater` (4) for full water coverage.
  A dry result keeps the surface computed by this query, not a previous
  object's terrain-water result.
- Other ground-query source kinds assign 0 on the ordinary assignment path.

This uses the ground-support query, not an arbitrary side collision. Model
support supplies the packed texture of the selected face; solid-color faces
supply zero. Terrain support queries the actor's integer map X/Z. An
out-of-grid terrain tile lookup returns `-1`, which supplies image 63 after
the low-six-bit mask, before the usual assignment and coverage rules.

The support wrapper updates actor surface state only when collision attribute
mask `0x04` is set. Excluded and query-only paths retain the previous actor
surface. A support miss reaches the updater with a cleared source kind, so it
assigns zero only if the ordinary assignment path is reached. See
[surface-query state retention](player.md#surface-query-state-retention).

## Shipped script calls

The 27 shipped RGM files contain 137 `Sound`, 477 `FlatSound`, 91
`AmbientSound`, 97 `AmbientRTX`, 33 `EndMySounds` and 12 `StopAllSounds`
calls, and none to `EndSound` or `EndMySound`. Values outside the usual
ranges occur: `FlatSound` volume 128 (clamped to 127) and rate offset −10000
(clamped to 512 Hz), `AmbientSound` rate offset 2000, `AmbientSound` effect 127
(beyond the 118 shipped effects, so it never sounds), and `AmbientRTX` volume
offsets of 127 and 128 (volume 127 everywhere within the radius `R`). Every
`AmbientRTX` call passes 0 as its third argument.

## Related

- [SFX](../formats/SFX.md) — effect headers (sample rate, base volume, loop flag)
- [RTX](../formats/RTX.md) — speech clips
- [SURFACE.INI](../config/surface-ini.md) — sound remap sections
- [SYSTEM.INI](../config/system-ini.md) — `volume`, `sound_distance`
- [Item Attachment System](attachment.md) — animation opcode 4
- [Book menu](menu.md) — Sound slider
- [Music](music.md) — CD audio, separate from effects
