# Player movement

Player movement combines held input, actor attributes, animation state, and a shared frame scale.

## Coordinate and heading units

Runtime positions have eight fractional bits. A marker coordinate of `40816`
becomes a runtime coordinate of `10448896`. The developer console's `object coord`
command displays both representations. Its start position retains the spawn
position while the current position changes with movement and ground contact.

Headings wrap after 2048 units per revolution. For ordinary ground movement with
the sword sheathed, forward at heading zero increases Z. Right turning decreases
heading; left turning increases it. Other player states must be checked separately.

## Marker-based world entry

A marker-based world load selects the requested map marker and uses its position
for the initial player position. The marker coordinates are multiplied by 256
for runtime storage. The requested heading is wrapped modulo 2048; the marker's
auxiliary byte does not supply a heading. The player actor uses the map's `CYRUS`
actor definition, including its attributes and animation data.

Initial placement and ground settling are separate. In an observed ISLAND entry,
marker zero supplied `(40816, -699, 41024)` with requested heading zero. The
settled player position was `(40816, -744, 41024)`. Using the marker's Y coordinate
as a permanent ground height would therefore be incorrect.

## Ground-turn arithmetic

Left and right input have independent turn accumulators. Releasing one direction
resets its accumulator to zero. For ordinary movement, a held direction adds
`SYSTEM.INI` `turn_speed` while its accumulator is below `turn_max_speed`.
The addition is not clamped, so settings whose increment does not divide the
limit can overshoot it. The installed values 12 and 48 produce held-input
accumulators 12, 24, 36, 48, 48 on consecutive input updates.

Each direction is multiplied by the current frame scale and shifted right by
eight bits separately. Ordinary ground turning adds the scaled left amount and
subtracts the scaled right amount, then wraps the heading modulo 2048.
At scale 256, starting from heading zero, five right-input updates produce
headings 2036, 2012, 1976, 1928, 1880. These are arithmetic examples, not a
claim that input updates occur at a fixed wall-clock frequency.

Animation and player state control whether this heading update is allowed.
The arithmetic alone is insufficient to implement the player state machine.
Combat can select `attr_fight_turn_speed` and `attr_fight_turn_accel` instead of
the ordinary turn settings.

## Movement data and timing

Forward running and walking use the actor's `attr_forward_speed` and
`attr_forward_walk`. They are actor data, not universal controller constants.
Startup movement also depends on `attr_step_type` and animation progress.

The frame scale uses eight fractional bits, with 256 representing one nominal
update. The first twelve updates after a timing reset use 256. Subsequent timing
can use an FPS-derived scale with the configured normal, minimum and maximum
frame rates and smoothing settings. A nominal frame rate of 12 does not mean
the game runs a fixed 12 Hz simulation.

## Forward-walking startup

Ordinary forward walking keeps a startup phase from zero through three.
The selected walking speed is converted to runtime position units by multiplying
it by 256. For `attr_step_type = 1`, phases zero, one, and two use one quarter,
one third, and one half of that speed respectively, with integer division.
Phase three uses the full speed. A walking speed of 8 therefore gives startup
amounts 512, 682, 1024, and then 2048.

An eligible walking update advances the startup phase before testing whether
the current animation permits translation. Suppressed translation does not undo
that phase advance. Consequently the first visible displacement need not use
phase zero. Two observed ISLAND starts first translated at phase one and phase
two respectively. Resetting the phase whenever translation is suppressed would
change the observed startup behavior.

When animation permits translation, the startup amount is multiplied by the
frame scale and shifted right by eight bits before heading-based displacement.
One observed heading-zero sequence began at raw Z `10502144` and produced:

| Startup phase used | Frame scale | Raw Z displacement | Resulting raw Z |
| --- | --- | --- | --- |
| 1 | 161 | 428 | 10502572 |
| 2 | 161 | 644 | 10503216 |
| 3 | 153 | 1224 | 10504440 |

Each resulting position was retained as the previous position in the following
update in this case. This sample does not establish collision behavior elsewhere,
the complete animation eligibility rules.

### Ending a walking update

The actor update tracks whether it accepted forward movement. At the end of an
update without accepted forward movement, it resets the forward startup phase
to zero. This differs from suppressing translation through animation: an accepted
walking update can advance startup while producing no displacement.

An observed ISLAND walk reached phase three. After forward input was released,
an update ended at phase zero with idle animation and equal current and previous
positions. The following update retained that position and phase. This verifies
the ordinary release case, not the timing of host keyboard event delivery or
all movement interruptions.

## Ordinary forward-walk support checks

Before an ordinary ground walk is accepted, the engine probes ahead of the
actor. The local forward probe distance is the actor's forward extent plus
`attr_forward_walk`, converted to runtime position units. Actor orientation
transforms the offset; the probe retains the actor's current Y coordinate.

The probe must pass three checks, in order:

1. The actor's collision spheres at the probe must not intersect nearby model
   faces.
2. The surface check must not reject the probe as a steep surface.
3. A ground-support query at the probe must succeed.

The ground-support query temporarily uses the probe position and then restores
the actor's position. It considers terrain and nearby model faces. This is not
a permanent Y clamp and is not equivalent to accepting every horizontal step.

Surface classification uses the signed Y component of a normal scaled by 256:

| Normal Y | Classification |
| --- | --- |
| Less than −160 | Ordinary ground (4) |
| −160 through −42 | Steep ground (2) |
| −41 through 160 | Side surface (1) |
| Greater than 160 | Underside (8) |

Ground selection accepts classifications 2 and 4; the separate slope rules
control whether movement onto that support is allowed. Surface classification
alone does not describe slope sliding or collision response.

An observed ISLAND forward-walk check started at raw position
`(10448896, -190464, 10521523)`, heading zero. The actor's forward extent was 11
and walking attribute was 8, giving a 19-map-unit probe at raw Z `10526387`.
The obstacle and slope checks allowed it. Ground support selected terrain with
normal `(−39, −250, −39)`, classification 4, and actor-height result `−190976`.
The player's position remained unchanged by these checks. This sample verifies
an accepted terrain probe; it does not establish ledge behavior.

A separate ISLAND wall attempt at runtime position
`(10432684, -191744, 10739172)`, heading 26, rejected the sphere probe.
Walking returned without translating or advancing its zero startup phase.
The later ground response in the same update retained that position and the
equal previous position. The frame scale was 170 and idle group zero had no
translation-suppression marker. This verifies rejection before translation for
that wall contact; it does not establish swept correction or sliding.

An unsuccessful sphere, steep-surface, or ground-support probe marks the
forward attempt blocked and returns failure. Subsequent checks in that probe
are skipped. The blocked-attempt marker is transient; it is not a permanent
collision state.

### Converting support height to actor position

The selected ground surface height is not the actor origin height. For ordinary
support, truncate the selected surface height toward zero in map units, subtract
the actor's positive Y extent, multiply by 256, and add the configured
`standing_height` converted to runtime units. Ground-query eligibility and the
later movement response remain separate from this conversion.

A fresh ISLAND marker-zero entry settled at runtime position
`(10448896, -190464, 10502144)`. Its ground query selected a flat terrain triangle
with map-unit vertices `(40704, -680, 41216)`, `(40704, -680, 40960)` and
`(40960, -680, 40960)`. The returned surface height was −680 and its normalized
normal was `(0, −1, 0)`. The integer contact normal was `(0, −256, 0)`, classified
as ordinary terrain ground.

Cyrus's positive Y extent was 62 and the standing offset was −512 runtime units.
The conversion gives `(−680 − 62) × 256 − 512 = −190464`, or −744 map units.
The query returned that actor height while preserving its current and previous
positions. Treating the terrain height as the actor origin would place the body
64 map units too low in this case.

### Applying ordinary ground support

In an ordinary grounded update, the actor retains its previous position before
walking changes X and Z. The subsequent obstacle pass and support query precede
the ground response. When ordinary ground support is available and no airborne
or special movement state diverts the response, the response assigns the
returned actor height to Y. It preserves the translated X/Z and the saved
previous position. This is a discrete support-height assignment, not vertical
interpolation toward the terrain surface.

A four-stop observation within one ISLAND update recorded:

| Point in update | Runtime position |
| --- | --- |
| Before walking | `(10451447, -195072, 10640696)` |
| After walking | `(10451732, -195072, 10642107)` |
| Before ground response | `(10451732, -195072, 10642107)` |
| After ground response | `(10451732, -195328, 10642107)` |

The previous position remained `(10451447, -195072, 10640696)` at all four
stops. The startup phase was three, the frame scale was 180, and walking group
20 was active with its translation-suppression marker clear. The support query
returned terrain class 4, normal `(−40, −253, 0)`, and actor height −195328.
Its integer sample position `(40827, 41570)` lay on a terrain plane of height
−699.21875; the documented actor-origin conversion reproduces that result.

This sample establishes the ordinary supported branch. Falling, jumping,
sliding, rollback against obstacles, and special movement states have additional
responses; they are not equivalent to assigning a terrain height unconditionally.

### Terrain support sampling

The ground query converts actor coordinates to integer map units by arithmetic
right shift by eight before sampling terrain. Fractional runtime coordinates
therefore do not reach the terrain interpolator. This coordinate conversion is
separate from truncating the resulting surface height toward zero.

Terrain cell selection rounds `map_x / 256 - 0.5` and
`map_z / 256 + 0.5` to the nearest integer, with ties going to the even integer.
Let the selected cell coordinates be `i` and `j`. Its four horizontal corners
are `(256i, 256j)`, `(256(i+1), 256j)`, `(256i, 256(j-1))` and
`(256(i+1), 256(j-1))`. The diagonal joins the first and last corners.
If `map_x - 256i` is strictly greater than `256j - map_z`, use the triangle
containing the second corner; otherwise use the triangle containing the third.
The surface height comes from that triangle's plane. Its normalized face normal
is multiplied by 256 and rounded to nearest-even for the integer contact normal;
it is not an interpolated shading normal.

For the observed ISLAND interior cells, runtime grid row `j` corresponds to
WLD source row `256-j`. All 65,280 height bytes in runtime rows 1 through 255
matched that source mapping. This check does not establish out-of-grid behavior.
Near-camera queries use the current terrain vertex cache; queries more than
15 grid cells away on either axis use decoded heightmap values directly.
The observed samples below use the near-camera path on dry terrain. They do not
establish water-contact behavior or cache-update timing.

At marker zero, map X 40816 and Z 41024 select cell `(159, 161)` and the flat
triangle described above. A later forward probe at raw Z 10526387 samples integer
map Z 41118 in the other triangle of the same cell. Its vertices are
`(40704, -680, 41216)`, `(40960, -680, 40960)` and `(40960, -720, 41216)`.
At X 40816, the plane height is −682.1875. Truncation and the actor-origin
conversion produce −190976 runtime units, matching the observed ground result.
Its integer normal is `(−39, −250, −39)`, also matching the observed contact.

### Upright actor collision spheres

For bounds whose X and Z totals are not strictly greater than both other totals,
the ordinary collision shape is a vertical series of spheres. Its unscaled
radius is half the larger X/Z total, truncated, with a minimum of 4 map units.
The sphere spacing is three times half that radius, with the division performed
first. The available height is total Y minus `SYSTEM.INI` `post_collide_height`;
a negative available height is replaced by `post_collide_height`. The sphere
count is the available height divided by the spacing, truncated, with a minimum
of two.

The radius scale comes from the signed RAEX field at 0x12. Nonpositive scales
select 256; the scaled radius is the integer product of radius and scale shifted
right by eight. All spheres have X and Z centers zero. All except the last start
at Y = radius minus the negative Y extent, increasing by the spacing. The last
center is positive Y extent minus `post_collide_height` minus scaled radius.

For ISLAND Cyrus, totals `(48, 128, 49)`, positive Y extent 62, negative Y extent
66, post-collision height 50 and the default radius scale produce two radius-24
spheres at local Y −42 and −12. Both spheres and their count were observed in the
running actor. An additional support sphere is stored separately and is excluded
from the ordinary obstacle-query count. X- and Z-dominant shapes use different
construction rules.

## FPS-derived frame scale

The normal FPS measurement path counts one frame per update and accumulates
elapsed BIOS ticks. A measurement window contains 18 BIOS ticks. Once at least
one complete window has elapsed, measured FPS becomes the frame count divided
by the number of complete windows, using integer division. The frame count
resets to zero; the tick remainder is retained. A negative accumulated tick
count is clamped to zero. This uses the BIOS clock, not exactly one wall-clock
second; see the [water clock](water.md) for its frequency.

A live nine-update sample advanced the accumulated tick remainder from 7 to 17
while the frame count advanced from 6 to 14. The next update added one tick,
produced measured FPS 15, and reset both frame count and tick remainder to zero.
The intermediate updates included two-tick increments. Debugger pauses were
excluded by using the original clock values as inputs.

The FPS-derived timing path keeps a measured FPS and a smoothed FPS. On each
update, the smoothed value moves toward the measured value by one. If their
initial difference is at least five, it first moves by half that difference,
rounded down. Thus a measured change from 19 to 17 produces smoothed values
18, then 17 on successive updates. At the acceleration boundary:

| Smoothed FPS before update | Measured FPS | Smoothed FPS after update |
| --- | --- | --- |
| 10 | 14 | 11 |
| 10 | 15 | 13 |
| 14 | 10 | 13 |
| 15 | 10 | 12 |

The half-difference adjustment applies to a difference of five in both directions.

`use_smooth_fps` selects the smoothed value instead of the measured value.
The selected FPS is clamped to `min_frame_rate` and `max_frame_rate`. The target
frame scale is the integer quotient of `normal_frame_rate * 256` divided by
that FPS. A zero configured normal rate is replaced with 12 during loading.

With `use_smooth_divisor` disabled, the scale becomes the target immediately.
With it enabled, the scale first moves one unit toward the target. If it is
still above the target, it becomes the target immediately. If it is below the
target, the remaining difference determines a second increase:

| Remaining difference | Additional increase |
| --- | --- |
| 1–5 | 1 |
| 6–10 | 5 |
| 11–20 | 10 |
| 21 or more | 20 |

An exact match receives no additional increase. This smoothing is asymmetric:
a falling target can take effect immediately, while a rising target can take
several updates.

An observed ordinary-walking sequence with normal rate 12 had measured FPS 19
and scale 161. When measured FPS became 17, the following three scales were
167, 178, and 180. The intermediate smoothed FPS values 18 and 17 give targets
170 and 180, reproducing that scale sequence. Earlier in the same sequence,
measured FPS rose from 18 to 19 and the scale changed directly from 170 to 161.

This describes the FPS-derived path after timing startup. It does not establish
selection of that path in every game state, alternate-timer behavior, or the
mapping from host elapsed time to original timer ticks.

## Remaining coverage

Complete animation eligibility, collision, other startup paths and stop movement, timer-mode
selection, jumping, gravity, combat, swimming, climbing, and follow-camera behavior
are not specified here. Reproducing only the arithmetic above does not establish
controller parity.

## Related documentation

- [Keyboard bindings](../config/keys-ini.md)
- [System configuration](../config/system-ini.md)
- [RGM markers and actor data](../formats/RGM.md)

### Sphere contact with model faces

The forward obstacle query converts the proposed actor origin to integer map
units by arithmetic right shift by eight. Actor orientation transforms each
local collision-sphere center; the model's inverse placement transform then
expresses the center in model coordinates. The face test uses the model's
stored face normal and polygon vertices, rather than a rendered triangle mesh.

For a sphere center C, radius r, a face vertex V, and face normal N, the signed
plane distance is `(V − C) · N`. When collision sidedness is enabled, a distance
greater than 4 map units rejects the face. Independently, a squared distance
greater than `r²` rejects it. The candidate contact point is the perpendicular
projection `C + N × distance` onto the face plane.

A face passes the remaining test when any polygon vertex is within the sphere,
any polygon edge segment intersects the sphere, or the projected contact lies
inside every oriented polygon edge. Boundary equality counts as contact.
For the inside test, each consecutive vertex pair A/B must satisfy
`((A − contact) × (B − contact)) · N >= 0`. A face that passed for one sphere
is not added again for another sphere in the same model query.

The reported contact remains the plane projection even when a vertex or edge
intersection accepted the face. The model placement transform returns the
contact point and normal to world coordinates. The obstacle probe needs only
whether any model reported a contact; it does not move the actor to that point.

In the blocked ISLAND sample above, the first contacting sphere had radius 24
and the face-plane distance was −19.6171875 map units. The projection was inside
the polygon. The reported world contact was
`(40751, −791, 41987.6171875)` with normal `(0, 0, −1)` before integer contact
conversion. Recomputing the plane projection from the observed model geometry
reproduced that contact. A radius of 19 rejects it at the plane-distance test.
The following forward-probe result was failure and the player remained at the
same position. This sample verifies a face-interior contact; edge-only and
vertex-only contacts still need independent live samples.

These are the narrow face-contact rules. Model and collision-group broad-phase
selection, transformed/scaled models, swept correction, moving geometry, and
non-upright actor shapes require their own coverage before claiming a complete
collision implementation.
