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

## Incremental orientation refresh

Assigning an orientation and refreshing an existing actor matrix are distinct
operations. In the ordinary unattached refresh branch, changed angles with
`attr_rotation = 0` rotate the existing matrix by the negated differences between
current and previous heading, pitch and roll, in that order. Each difference is
wrapped modulo 2048 for the shared rotation-table lookup. Unchanged angles leave
the matrix intact. The stored matrix therefore retains its accumulated
single-precision rotation history; it is not reconstructed from the current
heading on every refresh.
The ordinary actor update copies current pitch, heading and roll into their
previous-angle fields before selecting the default animation and running the
ordinary animation block. Movement later in that update can change the current
angles. The admitted incremental refresh does not advance this snapshot.
In the normal main-loop order, map-list presentation, which can refresh actor
matrices, precedes the ordinary actor update. Explicit orientation assignment
and attached-actor composition remain separate operations.

In that branch, `attr_rotation = 2` instead rebuilds from identity using the
negated current angles. A selected player with nonzero roll takes a separate
rebuild branch. These rules do not replace attached-actor matrix composition.

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

When the selected walking-speed attribute is zero, the forward-walk helper
instead uses `attr_forward_speed >> 1` as its runtime-unit startup amount.
This fallback is not multiplied by 256. Startup division and frame scaling
still apply to that amount.

For ordinary sheathed, non-combat forward walking, the helper selects animation
group 20. It checks that the group exists before requesting it or advancing
startup. The ordinary request precedes the movement-marker test, so a request
that advances animation immediately can change whether this walk translates.

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

For an admitted ordinary sheathed actor whose current group is 20, the update
continues forward walking if earlier movement in that update did not already
accept forward motion. This continuation repeats the forward support probe and
uses the walking speed, startup phase, frame scale, and movement marker. It does
not request the walking animation again, so the pending default-group request
survives. Marker-suppressed continuation still counts as accepted forward motion
and preserves the startup phase. Once the current group becomes idle, this
walking continuation no longer applies; an update without other accepted forward
motion resets the phase.

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

### Forward slope surface selection

The forward steep-surface check uses `post_collide_height` as the half-length
of its vertical segment, centered on integer actor Y plus the positive Y extent.
It is bypassed when collision attribute bit 2 is clear, or when the designated
player's alternate collision mode is active.

For the proposed position, face-model contacts of classification 2 or 4 compete
by `abs(16384 - segment_rank)`. Strictly smaller values replace the current
contact, preserving the first equal candidate. The initial comparison value is
999999 and the initial model height is 9999999 map units.

Terrain participates when its height is no lower than the segment's lower end.
Its height difference from the segment center is truncated toward zero before
taking the absolute value. That value must be strictly less than the absolute
difference of the current model height from that center.
A winning terrain sample replaces the normal and height. When the proposed
position has a selected surface, classification 2 rejects the attempt and other
classifications allow this slope check. This selection is separate from the
ordinary ground-support rank comparison.

### Forward ground-support admission

The forward ground check saves the actor position, queries support at the
proposed position, then restores all three position coordinates before deciding
whether the attempt succeeds. The support query retains its other state changes.

The attempt fails when collision attribute bit 2 is set and the resulting
support classification is 2. It also fails when support is absent or the shared
surface veto equals one. Bypassing the earlier slope check does not bypass this
final classification check.

When the surface-state updater reaches its ordinary assignment path, it sets
the shared veto to one for actor surface states 5 and 11, and zero for other
states. Its early-return paths do not perform that assignment. The forward
check reads the shared veto after the support query; it does not recompute it
solely from the actor's current surface state.

### Surface-query state retention

The surface query has separate return, actor-surface and shared-veto results.
An excluded query returns zero without replacing the actor's surface or the
shared veto. An admitted query can also run without assigning actor state:
it returns the resolved model or terrain surface, but retains both the actor's
surface and the shared veto. For other support kinds, this path returns zero.

The ordinary assignment path stores the resolved surface in the actor, then
sets the shared veto according to that stored value. Model contacts use the
packed face texture assignment; terrain contacts use the actor's integer map
X/Z position. Other support kinds assign surface zero. A zero return alone
therefore does not establish that the actor's surface or shared veto is zero.

The ordinary support wrapper captures the actor's previous surface before
querying support. It commits the designated-player support-history result, then
invokes the surface updater only when collision attribute mask `0x04` is set.
This invocation is not conditional on finding support: a miss supplies the
cleared source kind to the updater. With that attribute mask clear, the wrapper
does not refresh the actor's surface or shared veto.

Surface-transition handling compares the updater's return value with the saved
actor surface, rather than checking only whether assignment changed the actor.
Water surfaces are values 1 through 4; lava is value 5. When both the previous
and current actor surfaces are outside water and lava, this wrapper does not
produce water/lava transition effects.

SOUP `IsInDeepWater()` reads the receiving actor's stored surface state.
It returns one for states 2 and 4, and zero for every other state. It does not
perform a new terrain or support query.

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

### Ordinary actor collision-shape initialization

The ordinary actor loader copies the primary MPSZ record when an animation
model table and a nonnegative primary bounds index are present. A static-model
actor with an MPOB `model_id` other than -1 also takes the primary-record path.
The copied bounds supply the sphere builder with the actor's signed RAEX radius
scale. The builder replaces the record's orientation byte according to its
dominant-axis comparisons.

A static-model actor with `model_id` -1 derives its bounds from the loaded
model's vertices. Positive and negative extents start at zero. Vertices are
visited in stored order; each component that extends the current signed extent
replaces it after nearest-even integer conversion. Negative extents then become
nonnegative magnitudes. The positive and negative X/Z extents are multiplied by
`SYSTEM.INI` `sphere_object_scale` and shifted right by eight; Y extents are not
scaled. A nonpositive system scale uses 256. Each total is the sum of its two
extents, and the center offsets are zero. The usual sphere builder then applies
the separate RAEX radius scale.

An actor with neither that static model nor an applicable primary-record path receives
positive and negative extents of 12, totals of 24, zero center offsets, and one
sphere centered at zero with radius 12. This bypasses the usual sphere builder.
This initialization precedes the type-one placed-model fallback; loading a
placement model afterward does not rerun collision-shape initialization.

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

### Horizontal actor collision spheres

An X total strictly greater than both Y and Z selects the X-dominant shape.
Otherwise, a Z total strictly greater than both X and Y selects the Z-dominant
shape. Every tie selects the upright shape.

The horizontal shape starts with radius `max(4, max(other totals) / 2)`, using
signed integer division. If `post_collide_height < 2 * radius`, subtract half
`post_collide_height` from the radius, clamp it to at least 4, and set the Y
center to that radius minus the negative Y extent. Otherwise retain the radius
and set the Y center to negative `post_collide_height`.

The spacing is `(3 * radius) >> 1`. The ordinary sphere count is the dominant
total divided by spacing, with a minimum of one. The stored radius is
`(radius * scale) >> 8`, using the same scale default as the upright shape.
For X-dominant bounds, every ordinary sphere has the same center
`(radius - negative X extent, Y center, 0)`. The loop advances a separate
accumulator without applying it to the stored X centers. For Z-dominant bounds,
centers start at `(0, Y center, radius - negative Z extent)` and advance in Z by
the spacing. Both horizontal shapes store one additional sphere copied from the
first ordinary sphere; their ordinary query count excludes this copy.

The upright shape's additional sphere has center
`(0, positive Y extent - scaled radius - (standing_height << 8) - 2, 0)` and the
same scaled radius. This expression uses the runtime-scaled standing-height
value directly, while the stored sphere center is a map-coordinate value.

### Per-update collision shape preparation

An actor's selected collision shape receives its current position before the
ordinary update copies the shape into working buffers. Each signed runtime
coordinate is arithmetic-shifted right eight bits, then stored as a
single-precision map coordinate. Each orientation component stores the integer
`(-angle) & 2047`; these angle fields are not floating-point values.

The ordinary update copies the selected shape into two working records. For
upright bounds, the second record's sphere count is increased by one, including
the additional stored support sphere. The first record retains the ordinary
obstacle count. X- and Z-dominant bounds retain the same count in both records.
If the actor has no selected shape, this preparation uses the saved
selected-player shape and the selected player's bounds orientation instead.

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

### Terrain material lookup

Terrain material queries use integer map X/Z converted to single precision.
Multiply each coordinate by `1/256`, subtract `0.5` from X and add `0.5` to Z,
store both results at single precision, and round each to the nearest integer
with ties to even. Valid grid columns are 0 through 254; valid rows are 1
through 255. Outside that range the lookup returns `-1`.

For valid cells, runtime row `j` reads WLD Map 3 source row `256-j`.
The returned byte retains the low-six-bit texture index and high-two-bit
rotation. Surface lookup uses texture set 302 and that texture index before
applying terrain blend-mask and water overrides, as described in
[sound surface selection](sound.md#surfaceini-remap).

### Terrain material blend coverage

Material lookup derives local mask coordinates from integer map coordinates:
`u = (X >> 1) & 127`, `v = 127 - ((Z >> 1) & 127)`. The tile's rotation
selects `(u,v)`, `(127-v,u)`, `(127-u,127-v)`, or `(v,127-u)` for rotations
zero through three respectively. Call the rotated coordinates `a,b`.

The generated 256-by-256 mask has value
`clamp(2 × (x + y - 254), 0, 255)` at `(x,y)`. Terrain image indices select
these coverage rules:

| Image | Coverage |
| --- | --- |
| 0 | 256 |
| 5 | Mask at `(a+109,b+109)` |
| 6, 52 | Zero for `a <= 28`; `((a-28) × 455) >> 7` for `29 <= a < 100`; 256 otherwise |
| 7 | Mask at `(a+32,b+32)` |
| 30 | Mask at `(a+128,b+128)` |
| 31 | Zero for `a < 5`, otherwise 256 |
| 32 | Zero when mask at `(a+64,b+64)` is zero, otherwise 256 |
| Other indices | Zero |

Start with the surface assignment for texture set 302 and the image index.
Dry pixels on images 6 and 52 select sand, and dry pixels on images 31 and 32
select rock. On the other mask-backed images, zero coverage replaces a water
assignment with sand; a deepwater assignment also becomes sand before the
water override. Other image indices retain their base assignment.

After these rules, nonzero coverage selects surface 3, except coverage 256
selects surface 4. Coverage 255 is therefore distinct from full coverage.

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
The ordinary forward obstacle probe also reads only the face-model list and
reports blocked when any queried model produces a sphere contact.

Each static face candidate retains its collision flag byte as support metadata.
When primary or fallback support selects that candidate, the selected support
receives that byte unchanged. Terrain selection instead sets support metadata
to zero.

### Live and transient collision candidates

Candidate construction visits prepared static placements first, ordinary live
actors second, and transient models last. The face-model and actor-style lists
each retain their separate limit of 64 entries across these phases.

A live actor is admitted only when its ordinary actor exclusion test passes,
its runtime animation-update exclusion is zero, its loaded model exists and
differs from the querying actor's model, attribute 55 is zero, and the model
distance test passes. A live actor using the static model load path enters the
face-model list with its actor owner and collision flags. Other admitted live
actors enter the actor-style list with their actor owner. A full destination
list skips that entry. When both lists are full, traversal returns before
processing later live actors or transient models.

An active transient record with a loaded model and flag `0x10` enters the
face-model list until that list is full. These entries have candidate type
`0x80`, zero collision metadata, and neither a static-placement owner nor an
actor owner. They do not run the static-placement distance filter.

### Actor sphere overlap

An actor does not overlap itself. Actor overlap uses each actor's selected
runtime shape, or one model-radius sphere for an actor using the static model
path. At most the first 32 spheres of each shape participate. Local sphere
centers receive the actor matrix transform and its shape origin rounded to
integer map coordinates.

Sphere pairs are visited in shape order. Their summed radius and each center
difference are rounded to integers. A pair overlaps only when the rounded
three-dimensional distance is strictly smaller than the summed radius. The
first overlapping pair returns the summed radius minus the rounded horizontal
distance, with the vertical difference set to zero for that second distance.
When no pair overlaps, the query returns zero.

### Sphere model admission

A sphere query first tests each body sphere against the model radius. Express
the displacement from the actor origin to the model placement in actor-local
axes, then subtract it from the local sphere center. The sphere passes when
this squared displacement is no greater than the sum of the squared model
radius and squared sphere radius. This is not the square of the sum of the
radii. Only admitted spheres proceed through the actor/world/model transforms
to the subgroup and polygon tests.

### Collision subobject admission

The model runtime loader retains authored subobjects only for version 5 models.
For other model versions, including version 4, it clears both the subobject
list and its count. Version 4 subobjects therefore do not participate in these
queries; the loader does not synthesize extent factors for them.

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

### Selected-player fallback support

When the primary terrain/model segment query finds no support, only the
designated player receives a secondary sphere query. Its world-space center is
the actor's position shifted right by eight bits plus the collision center
offset. Add the positive Y extent and subtract three from the center's Y.
The radius is 10 map units when the shared support-history counter is zero,
and 14 when that counter is nonzero. The query uses an identity actor rotation.

During the designated player's admitted update, before ordinary animation and
movement handling, the support-history counter decreases by the supplied nominal
tick increment, saturating at zero. Other actors do not perform this decrement.
After a designated-player support query, support metadata with mask `0x04` set
resets the counter to 12. This also applies to support queries made while probing
a temporary forward position; restoring the position does not undo the counter.

After the per-candidate actor-state exclusion, this fallback visits face-model
candidates in order, then each model's returned sphere contacts in order.
It accepts the first contact whose
normal classification is not side surface (1), including underside (8).
It retains that contact's model, normal, texture identity and support metadata,
and returns the actor's existing runtime Y unchanged. It does not move the
actor to the contact's projected height or rank the fallback contacts by distance.
If no such contact exists, support remains absent.

For both primary and fallback model support, a steep classification (2) becomes
ordinary ground (4) in the stored support state when collision-attribute bit 2
is clear. The geometric normal itself is unchanged.

### Support-query misses

A support-query miss clears the selected model, source kind and classification,
and sets the returned actor height to the no-support sentinel. It retains the
previous contact normal, texture identity and support metadata byte. These
retained fields do not constitute a valid contact when the source kind is zero.

The designated-player wrapper copies the result and tests its support metadata
for the support-history reset even when the query returned no support. A retained
metadata byte with mask `0x04` therefore resets the counter to 12 after a miss.

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

### Body-contact integer conversion

After transforming sphere contacts to world coordinates, the adapter converts
contact positions to nearest integers in map units. Each world normal component
is multiplied by 256, stored at single precision, then converted to a nearest
integer under the default nearest-even rounding mode. It does not normalize the
transformed normal again.

The ordinary body query copies these integer normals into its ordered contact
list and classifies their integer Y components using the ground-normal thresholds
above. Classification therefore follows world transformation and integer
conversion; it is not the source polygon's local classification.

### Primary body-contact collection

The primary body query retains contacts in candidate order and each model's
returned contact order. Collection returns immediately after its sixteenth model
contact. Later candidates and the optional terrain contact are then omitted.
When fewer than sixteen model contacts were collected, an available terrain
contact is appended after them. This limit belongs to the primary body query;
the residual separation query is separate.

### Body-contact facing admission

The primary body-response loop visits contacts in stored order. Contacts whose
stored classification is ordinary ground (4) do not enter wall response.
For other contacts, it computes the opposing normal heading by adding 1024
to the normal's quantized actor heading and masking to 2047. Subtract the
movement heading. Differences below −1024 add 2047; differences above 1024
subtract 2047. A contact faces movement only when the corrected difference is
strictly greater than −512 and strictly less than 512. The 2047 correction
makes geometric quarter-turn boundaries asymmetric: an uncorrected difference
of 1536 becomes −511 and passes, whereas 512 remains 512 and fails.

The movement heading comes from the current-minus-previous X/Z displacement.
When both horizontal coordinates are unchanged, the actor's current heading
is used instead. The response retains this heading across the contact loop;
position or facing changes from earlier contacts do not recompute it.

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
right by eight, with a minimum from `SYSTEM.INI` `poly_push_units`. To obtain
that length, first store each integer X/Z displacement component at single
precision. Sum their squares at extended precision, store the sum at double
precision, take its square root, and store the result at single precision.
Convert that result to an integer using the default nearest-even rounding mode
before shifting. The displacement is measured after primary body response. The installed
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
FPS tick remainder, smoothed FPS, and startup countdown are not part of the
saved state.

The book menu uses this save/restore pair. While it is open, each menu iteration
waits for a changed BIOS tick, updates the book, advances the shared clock, and
then reads input. Leaving the menu restores the saved measurement frame count,
measured FPS, next scale, and nominal/fine accumulators. The measurement
remainder, smoothed FPS, and remaining startup countdown retain their values
from menu processing. The currently exposed movement scale and nominal/fine
increments are not restored; the next clock update replaces them.

The saved baseline is relative: the last sampled BIOS tick minus the measurement
baseline. Restoration subtracts that saved difference from the current BIOS
tick. On the normal BIOS measurement path, measurement finishes by making its
baseline equal to its sample, so a save after that measurement has a zero
difference.

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

Entering the world loop starts the twelve-update countdown again without
resetting the measurement or accumulation state.

## Related documentation

- [Keyboard bindings](../config/keys-ini.md)
- [System configuration](../config/system-ini.md)
- [RGM markers and actor data](../formats/RGM.md)

## Initial player animation setup

A newly allocated player actor starts with animation default group zero and
stored surface state zero. Player definition and animation-group setup retain
that surface value; subsequent support updates or restored world state can
replace it.
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
