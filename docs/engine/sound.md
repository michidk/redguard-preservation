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
| `AmbientSound` | multitask | effect, volume offset, rate offset | Each run: compute volume and pan. If `v = 0`, or the object has been killed, stop this task's sound and end the multitask. Otherwise start the sound if it is not playing (never started, or a one-shot finished), then set the current volume and pan. |
| `AmbientRTX` | multitask | RTX label, volume offset, unused | First run: start the label's [RTX](../formats/RTX.md) clip at pan 64 and its header volume, start index 0. Each run: compute volume and pan; if `v = 0` or the object has been killed, stop the clip and end the multitask; otherwise set the volume and pan. The multitask ends once the clip has finished; it is not restarted. No dialogue line or subtitle is created. |
| `EndSound` | function | effect | Stop every playing channel whose requested effect is `effect`, whatever its owner. |
| `EndMySound` | function | effect | Stop every playing channel whose requested effect is `effect` and whose owner is this object. |
| `EndMySounds` | function | — | Stop every playing channel owned by this object. |
| `StopAllSounds` | function | — | Stop all 16 channels. |

`FlatSound`, `EndSound`, `EndMySound` and `EndMySounds` return zero,
including when they start or stop no channel.

`KillMe` sets the object's killed flag and stops every channel the object
owns; `Resurrect` clears the flag.

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
  `off = 0`); the pan is not updated.
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
hits and speech are not remapped. With the shipped file, effect 4 plays 108 on
sand, 89 on wood, 112 on tile, 102 on rock and 69 on water.

An object's surface value comes from the face it stands on:

- A model face: the surface type that `[surfaces]` assigns to the face's
  texture, or 0 when the texture has no entry.
- Terrain: each query first resolves texture set 302 and the tile's low-six-bit
  image index through `[surfaces]`. The terrain blend mask can replace that
  value with sand or rock at dry shore pixels. A nonzero water result then
  selects `scapewater` (3), or `scapedeepwater` (4) for full water coverage.
  A dry result keeps the surface computed by this query, not a previous
  object's terrain-water result.
- Any other face: 0.

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
