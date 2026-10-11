# Player movement

Player movement combines held input, actor attributes, animation state, and a shared frame scale.

## Coordinate and heading units

Runtime positions have eight fractional bits. A marker coordinate of `40816`
becomes a runtime coordinate of `10448896`. The developer console's `object coord`
command displays both representations. Its start position retains the spawn
position while the current position changes with movement and ground contact.

Headings wrap after 2048 units per revolution. For ordinary ground movement with
the sword sheathed, forward at heading zero increases Z. Right turning decreases
heading; left turning increases it. Combat and special movement states use
different rules.

## Marker-based world entry

A marker-based world load selects the requested map marker and uses its position
for the initial player position. The marker coordinates are multiplied by 256
for runtime storage. The requested heading is wrapped modulo 2048; the marker's
auxiliary byte does not supply a heading. The player actor uses the map's `CYRUS`
actor definition, including its attributes and animation data.

Initial placement and ground settling are separate. Ground settling can change
the player's Y coordinate, so the marker's Y coordinate is not a permanent
ground height.

## Startup ground settling

Startup ground settling is separate from the ordinary grounded movement update.
It prepares the spatial window around the actor X/Z, using the
[static collision list order](#static-collision-list-order). It queries nearby
model candidates with radius 1000 map units, excluding the actor's current model.
The vertical segment keeps the prepared actor X/Z and
extends 500 map units in each direction from the prepared actor Y plus its
positive Y extent.

When terrain is enabled, its sampled height is truncated toward zero to an
integer map height. Terrain is eligible only strictly between the segment
endpoints. Its rank is `abs(16384 - ((abs(height - start_y) * 8388) >> 8))`.
The initial rank without eligible terrain is 65536.

Model contacts are visited in candidate and contact order. Only surface
classifications 2 and 4 participate. Their rank is
`abs(16384 - contact_rank)`. A model contact replaces the current selection
only when its rank is strictly smaller. Equal ranks retain the earlier
selection, including terrain when it was selected first. This differs from
ordinary support selection's later-equal replacement.

If a support is selected, the actor Y becomes
`(selected_height - positive_y_extent + standing_height) * 256`, where
`standing_height` is the configured map-unit value from `SYSTEM.INI`.
Without a selection, the actor Y is unchanged. This pass does not translate X/Z.

## Ordinary idle animation default

For the ordinary grounded player at rest with the weapon sheathed and no special
action override, default animation selection reads `attr_panic_group` from the
actor's attributes. This is a group number, not a boolean or a hardcoded request
for group zero. The installed ISLAND `CYRUS` definition supplies zero.

This selection precedes the ordinary animation advancement and default request.
Combat, rope and special-action states use separate selection branches.

## Upright actor orientation

Actor orientation stores pitch, heading and roll modulo 2048. Assigning an
orientation rebuilds the actor matrix from identity, applying negative heading,
negative pitch and negative roll in that order. The rotations use the floating
point [shared rotation table](water.md#wave-lookup-table).

For zero pitch and roll, let `s` be the table sample at negative heading modulo
2048, and `c` the sample 512 entries later. The matrix rows are `(c, 0, s)`,
`(0, 1, 0)` and `(-s, 0, c)`. It maps local forward onto `(s, 0, c)` in original
world axes. Use the stored table samples even at quarter and half turns;
replacing their small residual values with exact mathematical zeros changes
integer collision conversion at some distances.

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

### Heading-based translation

Horizontal translation uses the floating-point [shared rotation table](water.md#wave-lookup-table),
not the separate integer sine table. Negate the actor heading and mask it to
2047 to select the sine sample; cosine uses the sample 512 entries later.
Multiply the integer movement distance by each sample and add it to the current
X or Z position converted to single precision. Store the sum at single
precision, then truncate it to the runtime integer coordinate. Rounding the
sum before integer conversion is observable at ordinary world coordinates.

For heading 2036, distance 680, and initial X/Z `(10446802, 10818615)`, the result
is `(10446827, 10819295)`. Keeping the sum at double precision until truncation
would give a different Z result.

### Direction recovered from displacement

Collision response derives a movement heading from the integer X/Z displacement.
Take the absolute components and repeatedly halve both, discarding remainders,
until neither exceeds 511. Axis-aligned vectors select the corresponding quarter
revolution directly. For two nonzero components, equal magnitudes select 256;
otherwise index a first-octant table with
`floor(512 × smaller_component / larger_component)`. When X is larger, subtract
the table result from 512. Reflect this first-quadrant angle according to the
original component signs, negate it for the actor-heading convention, and mask
to 2047.

The 512 used first-octant entries are
`truncate(atan(index / 512) × 65536 / 3.141592654) >> 6`.
The diagonal case uses its explicit value rather than indexing an extra
endpoint. Directly rounding a continuous `atan2` angle produces different
results.

## Forward-walking startup

Before startup acceleration, an ordinary forward walk requests its selected
walking animation. With `[system] animation_system = 1` in `SYSTEM.INI`, if
that request leaves a different group current, the walk returns before marking
forward movement accepted, advancing the startup phase, or translating the actor.
A pending walking group therefore does not by itself permit movement. The walk
movement classification is set before this gate.

This differs from the animation movement marker gate below, which suppresses
translation after startup has advanced.

Ordinary forward walking keeps a startup phase from zero through three.
The selected walking speed is converted to runtime position units by multiplying
it by 256. For `attr_step_type = 1`, phases zero, one, and two use one quarter,
one third, and one half of that speed respectively, with integer division.
Phase three uses the full speed. A walking speed of 8 therefore gives startup
amounts 512, 682, 1024, and then 2048.

An eligible walking update advances the startup phase before testing whether
the current animation permits translation. Suppressed translation does not undo
that phase advance. Consequently the first visible displacement need not use
phase zero. Resetting the phase whenever translation is suppressed changes the
startup behavior.

When animation permits translation, the startup amount is multiplied by the
frame scale and shifted right by eight bits before heading-based displacement.
Walking groups can use animation type 1; a player animation adapter restricted
to type 0 cannot reproduce ordinary walking.

### Ending a walking update

The actor update tracks whether it accepted forward movement. At the end of an
update without accepted forward movement, it resets the forward startup phase
to zero. This differs from suppressing translation through animation: an accepted
walking update can advance startup while producing no displacement.

### Continuation after input release

Accepted movement is not equivalent to a held input. The actor update can
continue movement from its current animation group after the input has cleared.
The run-group continuation still passes through the animation marker gate and
counts as accepted forward movement when that marker suppresses translation.

For ordinary type-1 walking, a pending default-group request does not immediately
replace the current group. A `GoToFuture` command takes its exit target when a
different pending group exists, or when exit processing is already active. The
taken jump marks exit processing active and clears the movement marker; a
`BreakPoint` at the target can set the marker again. A pending default remains
pending through these exit frames until `EndAnimation` starts it in the same
advancement call, using that call's tick input.

## Ordinary forward-walk support checks

Before an ordinary ground walk is accepted, the engine probes ahead of the
actor. The local forward probe distance is the actor's forward extent plus
`attr_forward_walk`, converted to runtime position units. Actor orientation
transforms the offset; the probe retains the actor's current Y coordinate.

The local offset has zero X and Y components. Its Z component is the sum of
forward extent and `attr_forward_walk`, multiplied by 256. The integer matrix
transform converts its input components to single precision, computes each
rotated component, and stores that component as single precision before
converting it back to an integer with nearest-even rounding. The transformed
integer X and Z offsets are added to the actor's runtime X and Z coordinates;
the transformed Y offset is discarded.

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

### Applying ordinary ground support

In an ordinary grounded update, the actor retains its previous position before
walking changes X and Z. The subsequent obstacle pass and support query precede
the ground response. When ordinary ground support is available and no airborne
or special movement state diverts the response, the response assigns the
returned actor height to Y. It preserves the translated X/Z and the saved
previous position. This is a discrete support-height assignment, not vertical
interpolation toward the terrain surface.

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

For interior terrain cells, runtime grid row `j` corresponds to WLD source row
`256-j`.
Near-camera queries use the current terrain vertex cache; queries more than
15 grid cells away on either axis use decoded heightmap values directly.

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
spheres at local Y −42 and −12. An additional support sphere is stored separately
and is excluded from the ordinary obstacle-query count. X- and Z-dominant shapes
use different construction rules.

### Static collision list order

Static models use the same eighteen-cell spatial window as the
[placed-actor update list](animation.md#placed-actor-initialization-and-eligibility).
The four corner cells are omitted. Cells are visited by increasing X within
increasing Z rows, and each cell's static list retains its existing order.
For nonnegative map coordinates, static placements are assigned to cells by
their X and Z coordinates divided by 512 with integer truncation. Within a
cell, placements occur in reverse
`MPSO` record order. The resulting lists are concatenated in that order before
model-distance selection. Actor-only border eligibility does not remove static models.
For each horizontal center coordinate, the starting map coordinate is zero
below 4,608; it is 56,319 when the center plus 4,608 exceeds 65,535; otherwise
it is the center minus 4,608. Divide that starting coordinate by 512 to obtain
the first cell. The upper-edge start therefore occupies cell 109.

### Model-distance candidate test

An ordinary actor's collision query converts its runtime origin to integer map
units by arithmetic right shift by eight. If the actor has a model, the search
radius is twice the model radius, truncated to an integer. Without a model, the
search radius is 200 map units. The actor's own model is excluded.

A model passes the distance test when the squared distance from the query origin
to the model placement is no greater than the square of the sum of the query
radius and the model radius. Model placement coordinates and radii are floating
point. The coordinate differences, squared-distance sum, and combined radius
are stored at single precision before the final comparison. Equality passes.

### Static candidate list separation

Static collision flag 2 excludes a placement before distance selection. Flag 1
routes an admitted placement to the actor-style list; other admitted placements
go to the face-model list. Each list holds at most 64 entries. A full actor-style
list skips further actor-style entries and continues the static scan. Encountering
another admitted face model when the face-model list is full ends the static scan,
including any later actor-style placements.

Ground support reads the face-model list, both for its primary segment query and
its selected-player fallback sphere query. It does not query the separate
actor-style list. This separation does not exclude actor-style contacts from
other collision responses.

### Collision subobject admission

For a sphere center in model coordinates, let `d` be its displacement from a
subobject center, `R` the subobject radius, `e` its three extent factors, and `r`
the sphere radius. For nonzero `length(d)`, form each component of
`d × R / length(d) × e` and store it at single precision. The subobject passes
when the squared displacement is no greater than the sum of those component
squares plus `r²`. This is not a test against the subobject radius alone.

### Segment model admission

Before visiting subobjects, transform both segment endpoints into model-local
coordinates and test the model radius around the model origin. An endpoint
inside or on the radius passes. When both endpoints are outside, use the same
bounded segment/radius test described below, without extent scaling. Here the
projection may be zero: it must be nonnegative and no greater than the squared
segment length. The minimum squared length is the single-precision
representation of 0.1, with equality accepted.

### Segment subobject admission

For segment queries, subtract the subobject center from each model-local
endpoint and multiply each component by the reciprocal of its corresponding
extent factor. The reciprocals and scaled endpoint components are stored at
single precision. Let these scaled endpoints be `a` and `b`, and let `r` be
the subobject radius.

An endpoint inside or on the radius admits the subobject. When both endpoints
are outside, set `d = a - b`, `L = d · d`, and `P = a · d`. Admission requires
all of the following:

- `L` is at least the single-precision representation of 0.1.
- `P` is greater than zero and no greater than `L`.
- `(a · a) × L - P × P` is no greater than `r² × L`.

The squared start distance, squared radius, `L`, and `P` are stored at single
precision before their later comparisons. This test differs from sphere-query
subobject admission.

Admitted subobjects visit their face references in authored order. Segment
queries retain repeated successful face references; the successful-face
suppression used by sphere queries does not apply here. Each model query stops
after its first 32 successful contacts, including repeated references.

### Collision face traversal

A sphere-to-model query uses the model's authored [subobject face lists](../formats/models/3d.md#section4-subobject-bvh).
It visits actor spheres in order. For each admitted sphere, it visits admitted
subobjects in their stored order and then their face references in stored order.
Faces omitted from those lists do not participate in this query.

The query tracks faces that have produced a contact. Once a face succeeds, later
references to that face are skipped for the remainder of the model query,
including references reached by later spheres. A failed test does not mark the
face: another subobject reference or another sphere may test it again. Clearing
the accepted-face set between spheres, or marking every attempted face, changes
this behavior. Returned contacts retain the order of first successful tests.

### Ground-support ranking

The ordinary vertical support segment is centered on the actor's integer map
Y plus its positive Y extent, extending the supplied search distance R in both
directions. Selection begins with rank 65,536 and no support. Terrain below
the lower end of this segment is not a candidate. Terrain above the upper end
receives rank 65,535. Terrain within the segment receives
`(truncate(16777216 / (2 × R)) × (truncate(terrain_Y) − segment_start_Y)) >> 8`.

Model contacts with normal classification 2 or 4 compete in candidate and
contact order. A contact replaces the current support when its stored segment
rank is less than or equal to the current rank, so a later equal rank replaces
an earlier one. Model segment ranks use the 32,768 scale described below;
terrain's rank calculation is retained separately.

### Segment contact with model faces

Ground probes intersect the original model polygons. In model coordinates,
let A and B be segment endpoints, V a face vertex, and N the stored face normal.
The signed endpoint distances are `(V − A) · N` and `(V − B) · N`.
When collision sidedness is enabled, an A distance greater than 4 rejects the
face. A crossing requires one endpoint distance to be negative and the other
nonnegative. A segment lying entirely in the plane is rejected.

The intersection fraction is the A distance divided by the difference between
A and B distances. The intersection must pass the same oriented polygon-edge
inside test as a sphere's projected contact. The model placement transform
returns the intersection and normal to world coordinates. The stored fraction
uses a scale of 32,768 and nearest-integer conversion. The integer ground-probe
adapter rounds world coordinates and normals multiplied by 256; it retains
the stored fraction unchanged.

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

### Horizontal body-contact response

Body contacts are processed after the movement and ground-support response.
For an ordinary grounded actor, the response uses the displacement from the
previous position to the attempted position, including its vertical component.
It normalizes that displacement to length 65536. The length divided by 65536 is
stored at single precision before dividing the displacement components. Each
quotient is stored at single precision before nearest-integer conversion.

For an eligible facing wall contact, the horizontal dot product is
`(movement_x × normal_x + movement_z × normal_z) >> 8`, using the normalized
movement and the contact's integer normal. The comparison threshold is
`SYSTEM.INI` `slide_range`, which is −60000 in the installed configuration.
When the dot product is strictly greater than the threshold, the response
subtracts the normal component from the normalized horizontal movement. Each
projected component is shifted arithmetically right by four and added to the
previous X or Z position. Otherwise, an actor outside the sliding state restores
its previous X and Z. This step leaves Y unchanged.

### Grounded wall-facing adjustment

After a projected wall response, an ordinary grounded actor whose current
position differs from its previous position adjusts its heading. This branch
requires no active sliding, airborne, or special movement state. The starting
heading for this calculation is the attempted displacement's quantized heading,
not the actor's current facing. Convert the contact normal to the opposing
actor-facing heading by adding half a turn to its negated quantized heading
and masking to 2047.

Subtract the movement heading from that normal-facing heading. Differences
below −1024 add 2047; differences above 1024 subtract 2047. A positive difference
subtracts up to 32 from the movement heading; a negative difference adds up to
32. Zero leaves the movement heading unchanged. This difference correction uses
2047, unlike the ordinary 2048-unit turn wrap.

Separately compare the actor's pre-response heading with the movement heading,
masking both to 2047 and wrapping their difference to the interval −1024 through
1024 using a full 2048-unit turn. When the absolute difference is greater than
512, add 1024 to the adjusted heading and mask the result to 2047.

### Residual static-contact separation

After the primary body response, the actor queries static face contacts again.
The separation distance is the current horizontal displacement length shifted
right by eight, with a minimum from `SYSTEM.INI` `poly_push_units`. The installed
minimum is 16. For remaining contacts, sum their integer normals, divide each
component by the contact count with truncation toward zero, and normalize the
result to length 65536 using the body-response normalization above. Shift each
normalized component arithmetically right by eight. Add its X and Z components
multiplied by the separation distance to the actor position. Y is unchanged.
A nonempty contact set marks the residual-separation result even when its
averaged normal has no horizontal component.

## FPS-derived frame scale

The normal FPS measurement path counts one frame per update and accumulates
elapsed BIOS ticks. A measurement window contains 18 BIOS ticks. Once at least
one complete window has elapsed, measured FPS becomes the frame count divided
by the number of complete windows, using integer division. The frame count
resets to zero; the tick remainder is retained. A negative accumulated tick
count is clamped to zero. This uses the BIOS clock, not exactly one wall-clock
second; see the [water clock](water.md) for its frequency.

Selected blocking operations, including model resource loading, save the timing
state. Restoring that state reads the current BIOS clock and uses it as the next
measurement baseline, so suspended time is excluded. The restore keeps the
saved frame count, measured FPS, next scale, and fractional accumulators. The
FPS tick remainder is not part of the saved state.

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

The 3dfx executable also contains an alternate path driven by a reprogrammed PIT interrupt, but
nothing installs that interrupt (the installer has no caller or relocation), so
the BIOS-clock path above is the one that runs.

### Whole nominal ticks

The computed frame scale is accumulated in units of 1/256 nominal tick. Each
update's whole-tick increment is the difference between the accumulated whole
ticks after and before adding the new scale. The fractional remainder carries
to the next update. A movement update can therefore receive zero animation
and script ticks while still having a nonzero movement scale. Rounding each
update's scale independently, or forcing at least one animation tick per
movement update, changes behavior.

A separate counter uses the same accumulation in units of 1/32 nominal tick;
its update increment is the corresponding difference of whole counter values.

A clock reset clears the measurement frame count, tick remainder, BIOS
baseline, and fractional nominal accumulator. Measured and smoothed FPS start
at 12, and both the used and next scales start at 256. The reset does not seed
the baseline with the current BIOS time.

The reset also starts a twelve-update startup countdown. During that countdown,
the exposed movement scale is 256, the whole nominal-tick increment is 1, and
the finer counter increment is 8. The underlying FPS calculation and
accumulation continue. Measured FPS replaces smoothed FPS for scale selection,
and divisor smoothing is bypassed. After the countdown, the exposed values use
the calculated scale and accumulated increments.

## Related documentation

- [Keyboard bindings](../config/keys-ini.md)
- [System configuration](../config/system-ini.md)
- [RGM markers and actor data](../formats/RGM.md)

## Initial player animation setup

A newly allocated player actor starts with animation default group zero.
Player entry assigns the supplied map position with eight fractional bits and
sets pitch and roll to zero and heading modulo 2048 before binding the actor
definition and building its animation groups.

For an animated player definition, group setup makes two ordinary requests for
group zero with modifier zero. It then clears the current model reference while retaining the selected animation
model and frame identifiers. For an admitted animated actor, presentation resolves
the model reference from those identifiers; a cleared reference alone does not
mean that no animation frame is selected.
Player entry follows that setup with a forced request for the actor's default
group, again with modifier zero. These requests use the existing shared nominal
ticks; they do not establish a new clock interval.

The final request follows the ordinary [forced-request rules](animation.md#forced-requests).
Requesting an already-current group before its exit marker is set only clears
the pending request; it does not necessarily restart or advance the animation.
The initial default group is distinct from the state-dependent default selected
during subsequent ordinary player updates.

A world reload can restore temporary per-world saved state after player
initialization. The restored player record replaces position and animation state;
passing through marker initialization does not establish that the final loaded
player is a fresh spawn. Temporary world snapshots use the map's `.TSG` file.
