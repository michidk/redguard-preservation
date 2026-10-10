# Positional Sound

How sound effects from [MAIN.SFX](../formats/SFX.md) and speech from
[RTX](../formats/RTX.md) get their volume, stereo pan and playback rate.

**Evidence:** every rule on this page comes from static analysis of the Glide
build (`RGFX.EXE`), with constants checked against the installed `SYSTEM.INI`,
`SURFACE.INI`, `SOUP386.DEF` and `MAIN.SFX`. None of it has yet been measured
in the running game; see [Open questions](#open-questions).

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

The emitter is the emitting object's position.

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
- **Channels:** 16 sample channels. A start takes the first idle channel at or
  above a start index. When none is idle the sound is dropped; playing sounds
  are never stolen. The start index is 0 for the player's own animation sounds
  and for speech, 4 for combat hit sounds, and 6 otherwise (0 at boot until the
  first sound that resets it).
- **Ownership:** each channel records its owner object and effect index.
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

### SOUP functions

`SOUP386.DEF` declares these sound functions:

| Function | Kind | Parameters | Behaviour |
|---|---|---|---|
| `Sound` | function | effect, volume offset, rate offset | Positional start: volume from the curve with `off` = volume offset, pan at start. Starts even at `v = 0`. No later updates. |
| `FlatSound` | function | effect, volume, rate offset | Non-positional: pan 64, `v = clamp(base_volume + volume, 0, 127)`, where `base_volume` is SFX header byte `0x0C` (64 in all 118 shipped effects). `FlatSound(e, 0, 0)` plays at 64. |
| `AmbientSound` | multitask | effect, volume offset, rate offset | Each run: compute volume and pan. If `v = 0`, or a per-object flag is set, stop this task's sound and end the multitask. Otherwise start the sound if it is not playing (never started, or a one-shot finished), then set the current volume and pan. |
| `EndMySounds` | function | — | Stop every playing channel owned by this object. |
| `StopAllSounds` | function | — | Stop all 16 channels. |
| `EndSound`, `EndMySound` | function | 1 | Not yet analysed. |

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

## Test vectors

Shipped `sound_distance = 64` (`R = 2304`, `D = 3640`) and `volume = 255`,
`off = 0`:

| `d` | `a` | `v` | Miles volume |
|---|---|---|---|
| 0 | 0 | 127 | 126 |
| 18 | 0 | 127 | 126 |
| 19 | 1 | 126 | 125 |
| 100 | 5 | 122 | 121 |
| 500 | 27 | 100 | 99 |
| 1000 | 55 | 72 | 71 |
| 1152 | 63 | 64 | 63 |
| 2000 | 111 | 16 | 15 |
| 2286 | 126 | 1 | 0 |
| 2287 | 127 | 0 | 0 |
| 2305 | — | 0 (beyond `R`) | 0 |

Offsets: `d = 0, off = −40` gives `v = 87`; `d = 1000, off = +100` clamps to
127; `d = 2400, off = +100` gives 0.

Master: `v = 127` sends 127 with master 256, 63 with master 128, 0 with
master 0.

Pan with yaw 0 (`dx`, `dz` in world units):

| Emitter offset (`dx`, `dz`) | `a` | Pan |
|---|---|---|
| (0, +100) | 0 | 64 |
| (+100, +100) | 256 | 96 |
| (+100, 0) | 512 | 64 (abeam) |
| (+100, −1) | 515 | 127 |
| (0, −100) | 1024 | 64 |
| (−100, −100) | 1280 | 32 |
| (−100, 0) | 1536 | 64 (abeam) |
| (−100, +100) | 1792 | 32 |

With yaw 512 the emitter (+100, +100) gives `u = 1792` and pan 32.

Rate: a 22050 Hz effect with second field −8 plays at 21538 Hz; field +511
clamps to 44000 Hz. An 11025 Hz effect with field −200 clamps to 512 Hz.

Distance: listener at the origin, emitter at engine units (768, 1024, 0) gives
`d = 5`; emitter at (255, 0, 0) gives `d = 0`.

## Open questions

1. Which physical speaker the high pan values reach, i.e. whether `u = 512`
   is the camera's right. Needs a stereo capture of the original.
2. Which camera position feeds the listener in each camera mode (third
   person, first person, combat, rope, cutscenes).
3. How the object's surface value used by the remap is set.
4. For some object kinds the emitter position is a related sub-position
   rather than the object's own; which kinds is not established. Animation
   PlaySound volume always uses the object's own position.
5. The meaning of the `EndSound` / `EndMySound` argument.
6. The meaning of the per-object flag that stops `AmbientSound`, and the rate
   at which multitasks rerun (assumed once per frame).
7. Two further engine starts exist: a sound driven by object speed and a
   looping effect 25 with `off = −40` on one shared channel; their in-game
   identity is not established. Whether any other path refreshes
   animation-started sounds is not certain.
8. The alternate owner object scripts can designate for `Sound`, `FlatSound`,
   `AmbientSound` and `EndMySounds`.
9. How Miles maps sample volume 0–127 to output gain.

## Related

- [SFX](../formats/SFX.md) — effect headers (sample rate, base volume, loop flag)
- [RTX](../formats/RTX.md) — speech clips
- [SURFACE.INI](../config/surface-ini.md) — sound remap sections
- [SYSTEM.INI](../config/system-ini.md) — `volume`, `sound_distance`
- [Item Attachment System](attachment.md) — animation opcode 4
- [Book menu](menu.md) — Sound slider
- [Music](music.md) — CD audio, separate from effects
