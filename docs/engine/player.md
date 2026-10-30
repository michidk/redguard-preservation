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

The two standalone player actors use the shared actor allocator. Their motion
state, rope attachment, torch-burning counter and weapon-drawn flag start at
zero. Model and animation setup are separate from this allocation state.

Initial placement and ground settling are separate. Ground settling can change
the player's Y coordinate, so the marker's Y coordinate is not a permanent
ground height.

## Startup ground settling

Startup ground settling is separate from the ordinary grounded movement update.
It prepares the spatial window around the actor X/Z, using the
[static collision list order](#static-collision-list-order). It queries nearby
model candidates with radius 1000 map units, excluding the actor's current model.
The candidate list is the one the ordinary collision queries build, so it holds more than the
static placements: after them come the models of the ordinary actors that have a model, are not the
excluded player role, are not excluded from object updates, have attribute 55 equal to zero and pass the
model-distance test, and only an actor whose model kind byte is one contributes to the face-model list the
contacts are taken from. An actor-owned deck, such as a ship, therefore competes with the terrain.
The vertical segment keeps the prepared actor X/Z and
extends 500 map units in each direction from the prepared actor Y plus its
positive Y extent.

When terrain is enabled, its sampled height is truncated toward zero to an
integer map height. The terrain query does not test water cells: a water cell
supplies the same interpolated height as a dry cell. Terrain is eligible only strictly between the segment
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
Each input update recomputes both accumulators. A direction that is not held
resets to zero; with no selected player a held direction is one. Otherwise the
accumulator is incremented by the ordinary settings, or by the selected player's
fight-turn attributes when either of these holds:

- The player holds a weapon (attribute 110 is nonzero, or the item system is
  ready and the player has an assigned, drawn, not sheathing weapon) and its
  current animation group is neither 18 nor 31.
- Its `hand_item` is 51.

The fight-turn increment is `attr_fight_turn_accel` (attribute 22) and the limit
is `attr_fight_turn_speed` (attribute 21): while the accumulator is below the
limit the increment is added, with the same overshoot as the ordinary settings.
CYRUS carries accel 12 and speed 64, against the ordinary 12 and 48; the
`attr_turn_speed` attribute (15, value 32) is not read by this update.

## Ground-turn eligibility and animation

Keyboard turning runs inside the selected player's controller, after the view,
lockout, dialogue and topic-menu returns and before forward and backward
dispatch, so movement in the same update uses the turned heading. The controller
also returns before turning while the current animation group is 129 or 91 and
the item system is ready and not paused. The rules below cover a sheathed,
non-swimming player that holds no item. While the player holds item 51 the
accumulators use the fight-turn attributes, the ordinary turn animation is
suppressed and the keyboard movement dispatch after turning is skipped.

Without the walk key held, turning applies when the living state is 0 or 27 and
the actor's motion bytes at `+0x1b2` and `+0x1b3` (two of the sliding, falling
and queued-motion bytes) are both zero. The heading becomes the previous heading
minus the scaled right amount plus the scaled left amount, modulo 2048. A
nonzero net amount then makes an ordinary animation request with modifier zero:

| Net turn | Torch not burning | Torch burning |
|---|---|---|
| Left | Group 14 | Group 16 |
| Right | Group 15 | Group 17 |

No request is made while the actor is a swimmer (attribute 110 nonzero), holds a
drawn weapon with a combat record, or stands on surface 2 or 4. Equal left and
right amounts request nothing.

With the walk key held, turning applies only while forward or backward is held,
the motion bytes at `+0x1b2` and `+0x1b3` are zero and the living state is outside 10 through
19. It requests no animation and does not bank.

These rules hold with the installed `[cyrus] mouse_turn=0`. A nonzero value adds a
mouse-driven turn block that plays the same four turn groups from the sign of
the horizontal mouse movement.

After turning and before forward and backward dispatch, a sidestep block runs
when the walk key is held or the living state is 15. With neither forward nor
backward held, left alone calls `MoveLeft` and right alone calls `MoveRight`
with duration 1, using the same stepper as the
[script natives](SOUP.md#moveleft-and-moveright); the controller ignores their
result. Holding forward or backward skips the sidestep.

The third actor angle is a roll that banks while running. It changes only in
these rules, all modulo 2048:

- When the current group is 18 or 30 and turning applies without the walk key, a
  left turn adds 4 (8 while roll exceeds 1024) when roll is below 32 or above
  1024; a right turn subtracts 4 (8 while roll is below 1024) when roll is above
  2016 or below 1024. Other rolls are unchanged. With no net turn the roll
  relaxes.
- When the current group is neither 18 nor 30, the roll relaxes once per update,
  and twice when turning applies without the walk key and the net turn is zero.
- Relaxing: above 1024 the roll becomes 0 when it exceeds 2040 and otherwise
  gains 8; from 1 through 1024 it becomes 0 below 8 and otherwise loses 8; zero
  stays zero.
- Any other update in groups 18 and 30 leaves the roll unchanged.

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

## Ordinary controller dispatch

The selected player's native controller runs within the ordinary script stage.
Default selection, ordinary animation advancement, and the ordinary default
request precede that stage. Queued and falling motion also precede the script
stage. Animation-based movement continuation and ground response follow it.
Returning from the controller does not skip those later ordinary stages.

A freshly loaded executable initializes the retained `LockoutPlayer` word to
zero. Any nonzero retained word returns from the controller before
keyboard movement dispatch. After that check, either the dialogue-state word or
the topic-menu-state word being exactly one also returns before movement
dispatch. These controller tests are separate from the attribute assignment
performed by [`LockoutPlayer`](hud.md#shared-rules).

The controller combat predicate is false when the actor has no combat record.
With a combat record, it is true exactly when combat flags mask `0x04` is set.
This is the same flag set by
[`beginCombat`](animation.md#combat-entry), rather than a separate controller
assist field. Combat-record allocation and reinitialization explicitly clear
the flags byte before the remaining combat setup.

For an ordinary grounded player with the sword sheathed, combat flag `0x04`
clear or no combat record, no rope attachment, and no jump or view input, the
held walk/run key inverts the retained walk-mode choice:

| Walk mode | Walk/run key held | Forward helper |
|---|---|---|
| Zero | No | Run |
| Zero | Yes | Walk |
| Nonzero | No | Walk |
| Nonzero | Yes | Run |

Walk mode comes from `[CYRUS] WALK_MODE` in `SYSTEM.INI`; Tab toggles it.
Forward input takes priority over backward input when both are held. With no
forward input, backward input takes its separate dispatch branch.
The view key returns before this ordinary non-combat movement dispatch.

The run helper is the script native `MoveForward` and the walk helper is
`WalkForward`, each called with duration 1 and an ignored result. Both use the
shared step counter, animation request and translation-suppression rules of
[`MoveForward`](SOUP.md#moveforward) and [`WalkForward`](SOUP.md#walkforward).

With no forward input and backward input held, the same walk rule selects `WalkBackward` or
`MoveBackward`, each called with duration 1: the walk helper when the walk mode and the held
walk/run key differ and the run helper otherwise. In a lifecycle state of 0x14, 0x15, 0x21 or 0x22
forward and backward dispatch is unchanged, while left and right turning does not run.

A player that holds a weapon (the weapon-holding predicate) or has combat flag `0x04` set
ignores the retained walk mode in both forward and backward dispatch: the held walk/run key
alone selects the walk helper, and its absence the run helper. Without the walk/run key, a held
jump key starts the jump entry in place of the helper for a player that holds a weapon only while the
action key is not held, and for any other player only while the current group is not a running
group (forward) or one of the walking groups 19 and 23 (backward).

The helper selection does not bypass the helper's motion, support, animation,
or movement-marker tests. Water states, drawn weapons, rope motion, jumping,
and special living states have separate dispatch branches.

### Armed attack input

The input sampler keeps a tap latch per key (up, left, right, the walk key, jump, look and action).
Once per world-loop iteration a latch whose key is not held counts frames since release and clears
its press count when that timer exceeds the tap window; a latch whose key is held and was not held
the previous iteration counts a press: the count rises when the release timer is above zero and
below the window, and restarts at zero otherwise, and the timer restarts either way. The window is
`[cyrus] tap_time` shifted left eight bits and divided by the frame scale (signed), with a floor
of six. A latch query names a key, a count and whether to consume: it matches when the press count
equals the count, and a consuming match clears count, timer and held flag.

After movement dispatch, and only while the jump key is not held, a player holding a weapon runs
the action-key handler. It does nothing in lifecycle states 20 and 21, without a combat record,
without the action key, or unless the combat action selector is -1, 0 or 1. While the queued byte is
nonzero or the falling byte is one, it records a pending jump attack, which starts the jump attack
once falling is one; otherwise the record clears. With the first directional-state byte's low two bits
set it only consumes the action latch. Otherwise it selects one move:

| Condition (in order) | Move |
|---|---|
| Up key latch consumed at count 1 (a double tap) | Lunge, move 4 |
| Up key held | 3 |
| Action latch consumed at count 2 | 2 |
| Action latch matches count 1 | 1 |
| Action latch matches count 0 | 0 |

With none of these the action latch is consumed. A move starts only when its authored attack slot
is populated and the pending move word allows it: moves 0 and 3 need it to be -1, move 1 needs 0
and move 2 needs 1. Starting writes the move to the action selector and the pending move word and
runs the executor with request 0: the move number selects the attack slot, the executor starts that
slot's first motion (animation group, motion selector zero), sets combat intent bit one, sets the
attack state to one and lowers aggression by ten. A hit on a target then follows the ordinary
attack-processing rules for the current slot and motion. The jump attack is move 6 started once the falling
byte is one: it needs its authored slot and an empty pending word, and consumes the action latch.

The lunge (move 4) first moves the player: when its lifecycle byte is zero and no rope, sliding, falling
or queued motion is set, it takes one [`MoveForward`](SOUP.md#moveforward) step of attribute byte 16
(whole units times 256) along the local Y heading, with the same forward step check below startup
counter three, the same startup ramp and frame scaling, but no animation request. The move also needs
its authored slot, a zero cooldown and no drawing or sheathing state before this step is taken.
Before any of these moves (0 to 4 and 6) starts, a reach test runs over the ordinary actor list, with
the move number selecting the attack slot and the slot's first motion supplying `arc`, `arc_center`,
`min_range` and `max_range`. An actor is in reach when its distance from the player (the rounded
three-dimensional distance between query positions) is strictly between `min_range` and `max_range` and
the absolute wrapped difference between the player's heading and the heading toward the actor is strictly
within `arc_center + 512` plus or minus `arc`. The receiver itself is not excluded from the list. The start
proceeds when either:

- an actor with a combat record is in reach and is either attackable (attribute 97 zero) or in lifecycle
  selector 6; or
- no actor is protected: a protected actor is in reach, is not in lifecycle selector 6 or 8 and has
  attribute 97 nonzero. For the forward move (3) the arc half-width used for this protected-actor test is
  six times `arc`, evaluated in single precision.

Otherwise the move does not start and the player requests an opponent-unarmed taunt: type 11 when the
protected actor's attribute 118 is nonzero, else type 10. Those taunt types use ordered selection like
[begin-combat taunts](../config/combat-ini.md#taunt-types), and are not requested while the retained
taunt cache entry still remembers a sample.

### Armed look-key handling

The look key leaves the movement dispatch, but the armed handler described above still runs, ahead of the
attack keys and only while the jump key is not held. In order:

1. With both the look and action keys held, a timer adds the nominal ticks of the iteration. Once it is
   greater than `[cyrus] sheath_sword_delay` (6 when the configured value is zero) the handler runs
   [`SheathSword`](SOUP.md#sheathsword) for the player. The iteration ends either way. Any other state of
   the two keys clears the timer.
2. A drawing or sheathing sword ends the iteration.
3. With the action key held the attack selection above follows.
4. With the action key released, a counter that adds the nominal ticks of every iteration is compared with
   COMBAT.INI `[misc] cyrus_defend_interval`. When the look key is held and the counter is greater, the
   defence stance starts (below) with `[cyrus] auto_defend` off, or only when an automatic defence is
   found with it on. A started stance sets the low two bits of the combat flags to 2 and clears the
   counter.
5. A jump attack still pending while the falling byte is one then starts as in the attack selection, and the
   action latch is consumed.

The defence stance does nothing unless no move is selected. It sets the pending defence index to zero, or
to the counter-attack reference of the first attacker when that attacker is in the player's forward arc
(strictly under 256 heading units) and its attack state is one. An attacker is an actor in the ordinary list
other than the player with a combat record whose target is the player, whose attacker-selection state is one
and that is not dead. The stance then selects move nine and runs the defence executor, which requests the
selected defence's animation group, sets attack state two and lowers aggression by ten.

The automatic defence visits the attackers in list order. An attacker qualifies when it is in the forward
arc, has combat flag `0x01` and attack state one, and the strike window of its current motion lies inside the
active window of the player's defence record for that motion's counter-attack reference. With
`gap_near = first_collide_frame - cursor` and `gap_far = first_defend_frame - cursor` (the cursor is the
attacker's animation command cursor) and the defence's `first_collide_frame` as `start` and
`first_collide_frame + collide_duration` as `end`, the window is inside when `start < gap_far` or `gap_far <= 0`,
and `end > gap_far`, and `start < gap_near`, and `end > gap_near`. The first qualifying attacker starts the stance.

### Controller initialization and preliminary state

Creating the standalone `CYRUS` and `GREMLIN` actors runs the same player
initializer for each actor, then restores `CYRUS` as the selected player. Each
initializer clears selected hit inhibition, the death-animation override group,
the pending action flag,
three preliminary countdowns and sound-maintenance inhibition.
These resets precede actor model/script setup and the initial animation request.

The preliminary countdowns are unsigned 32-bit values in whole nominal-tick
units. One countdown advances only while both queued-motion and falling states
are zero. The other two advance on every controller call. Each nonzero
countdown subtracts the nominal-tick increment with saturation at zero.
These three countdowns are separate from the
[selected-player support-history counter](#selected-player-fallback-support),
which advances earlier in the ordinary actor update.

The conditional countdown is processed before the pending action flag. A
pending action with sliding state zero selects action 74 and invokes its action
helper: it stores group 74 as the death-animation override and applies health
-255 through the death-owning selected-player health, naming the player as the
source. The flag is not cleared by the helper, so each controller call with
sliding state zero repeats it. The two unconditional countdowns are processed afterward, before the
selected hit-inhibition test. Nonzero hit inhibition invokes the death controller
and returns before idle-counter reset, lockout and ordinary keyboard dispatch.

### Idle animation requests

The ordinary idle counter starts at zero in a freshly loaded executable.
The controller resets it to zero while attached to a rope or while
the current animation group is nonzero. This reset precedes the lockout test.
With the normal BIOS clock, the clock measurement advances the counter once per
accumulated 18 BIOS timer ticks, independently of the nominal animation tick
increment.

After the lockout check, an unattached player with `attr_health < 40` requests
group 4 when the signed idle counter is greater than 4. At health 40 or above,
a counter greater than 12 requests one of groups 1, 2, or 3. Selection draws the
shared random generator's low two bits, retries value 3, and adds one to the
accepted value. Either request first sets the idle counter to -1.

These requests precede the dialogue and topic-menu checks. They use the normal
animation request operation and do not replace the actor's default group.

### Controller under queued, falling and sliding motion

The selected player's controller reads the queued, falling and sliding bytes in
only the places below; all other branches ignore them. The rows assume living state
0, no rope attachment and a sheathed sword. Rope attachment, living states other
than 0 and a held weapon take separate branches.

| Controller part | Queued or falling nonzero | Sliding nonzero, queued and falling zero |
|---|---|---|
| Preliminary resets, countdowns, hit inhibition, lockout, dialogue and topic-menu returns | Run as without motion; the conditional countdown stays frozen | Run |
| Queued value exactly 3 | Cleared when the current group equals the actor's default group, before the lockout test | Not applicable |
| Pending action 74 | Runs when sliding is zero | Skipped |
| Idle requests, view-key return, drawing and sheathing returns | Unchanged | Unchanged |
| Falling nonzero with the jump key held | Clears the companion motion byte unless either retained rope-release flag is nonzero; queued motion alone does not | Not applicable |
| Mouse turning, keyboard turning with or without the walk key | Skipped: heading, turn animation request and banking do not run | Run |
| Roll relaxation after the turning block | Runs once, unless the current group is 18 or 30 | Same |
| Sidestep, forward and backward dispatch | Selected by the same key tests; each helper refuses, so no animation request, motion write or translation | Same refusal |
| Forward-with-jump and backward-with-jump dispatch | The jump entry refuses | Refuses |
| [Jump-key arbitration](#jump-key-arbitration) | Clears the low directional-state bits | Reaches the ordinary jump entry, which refuses |
| Forward steering while queued | See below | Not applicable |
| Action key and prompt activation | No queued, falling or sliding test in the controller | Same |

Living state 0 is not required for the preliminary rows. Banking is not touched when
the turning block is skipped, but the single post-block roll relaxation still runs.

With the forward key held, a queued byte whose low seven bits equal 1 and a current
group of 32 or 33 becomes exactly 2, clearing its high bit. This runs after
movement dispatch in the same update, also when the player holds item 51, and has no weapon
or walk-key test. Queued state 2 then selects the arbitration animations of
[queued motion arbitration](#queued-motion-arbitration). Other directional input
does not alter queued or falling motion in the controller.

## Forward-walking startup

Before startup acceleration, an ordinary forward walk requests its selected
walking animation. With `[system] animation_system = 1` in `SYSTEM.INI`, if
that request leaves a different group current, the walk returns before marking
forward movement accepted, advancing the startup phase, or translating the actor.
A pending walking group therefore does not by itself permit movement. The walk
movement classification is 6. It is assigned after successful forward-support
admission and before the walking-group lookup and request gate. The shared
forward-update indicator is set only after the animation request passes that
gate. Ordinary forward running uses movement classification 2.

These classifications are distinct from the direction and packed movement
flags assigned by [directional ordinary displacement](#directional-ordinary-displacement).
Both classifications select forward direction 2 and movement mask `0x30` in
that later stage; the forward-walk helper does not itself assign that mask.

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

The body check builds the actor's working sphere shape at the proposed position.
Each Q8 position component is shifted right by eight before conversion to single
precision; the collision orientation remains the actor's current orientation.
Prepared model candidates are visited in their retained order. Queued motion 24
excludes candidates of type 128. The first blocking model rejects the attempt
without querying later models. Terrain does not participate in this body check,
and the actor's stored position is unchanged.

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
player's [support-history counter](#selected-player-fallback-support) is nonzero.

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

### Queued and falling vertical motion

Queued and falling vertical motion skip their arithmetic when an animated
model has active model-controlled vertical motion. Static models, absent
animation state and animations without that control use the following rules.
All positions and the retained fall origin have eight fractional bits.
The retained timer is a byte; falling tests it as signed.

The vertical factor sequence at indices 0 through 29 is exactly represented by
integer division of `256 * (900 - (29 - index)^2)` by 900. Indices 30 through 69
have factor 256. The next entries belong to adjacent engine state. A queued
update reads the entry at its current timer before advancing, then reads the
entry at the advanced timer to decide whether it transfers to falling.
A falling update uses entry zero when the signed timer is nonpositive.

The additional increment is a constant zero: nothing ever writes it. The fine-tick
increment is the [finer counter](#whole-nominal-ticks) increment of the update.
While an animated-model actor's falling byte is nonzero, the ordinary support query
instead uses a reduced probe distance chosen by the measured FPS that the frame scale was
derived from (twelve during the startup countdown): at 10 or more it is
`(post_collide_height >> 2) * 3`, below 10 it is `post_collide_height`.
Every other ordinary support query, including every static-model one, uses
`post_collide_height`.

Queued motion writes `origin_Y - attribute_32 * factor[timer]`, then advances
the timer by the sum of the fine-tick increment and its additional increment,
wrapping as a byte. After that advance, a factor greater than 255 transfers the
queued byte to falling and clears queued motion. A transferred high-bit motion
forces animation group 58 when the torch-burning counter is zero.

Falling limits its vertical value by `post_collide_height * 512 - 256`, with
signed comparison. With a positive signed timer, it writes origin Y plus the
smaller of that limit and `-attribute_32 * factor[timer]`, then subtracts the
same fine-tick increment sum from the byte timer. With a nonpositive timer,
it adds the smaller of that limit and
`attribute_32 * factor[0] * (fine_tick_increment + additional_increment + 2)`
to current Y. Products and position arithmetic retain 32-bit wrapping.

The queued-motion clearance probe runs from the current effective integer Y
to that Y plus the positive Y extent, retaining X and Z. It visits the prepared
face candidates without the queued-motion type exclusion. Contacts of normal
classification 2 or 4 compete by their integer world Y; strictly smaller Y
replaces the current height, initialized to 999999. With no selected height,
clearance is 256000 Q8 units. A selected height returns
`height * 256 - current_effective_Q8_Y`.
The terrain query uses current integer map X/Z and truncates its interpolated
height toward zero. That height replaces the model result only when it is
strictly greater than current integer map Y and strictly less than the retained
model height. It does not use the ordinary support-segment rank or lower end.
The terrain query runs whenever the world has terrain. It does not test water
cells: a water cell supplies the same interpolated height as a dry cell. The
probe applies no height-grid bounds check of its own: the height array is addressed linearly (row index times
the grid width plus column), so a column past the east or west edge continues into the adjacent row of the array,
and the cell interpolation then runs between the last column of one row and the first columns of the next.
A row past the north end of the array reads memory outside the array. Swimming straight north from the NECRISLE
start (west of the island) keeps the swimming state through the last array row and the row beyond it, and the player
enters animation group 18 with surface 0 within a few map units of crossing Z 0; the height then drops about 835 map
units in four seconds while Z keeps decreasing, so the player falls out of the world. The south end of the same map
instead reads level ground at Y -64.
Swimming west in the same map stops at X 261 and stays there with the forward input held. The interpolated
terrain height there, using the same linear addressing, is level (zero) at X 260 and rises steeply to the west
(-45 at X 240, -157 at X 200, -720 at X 0). The stop coincides with the start of that rise.

### Directional ordinary displacement

Directional displacement masks the input motion byte with 127. Values 2, 6,
8, 9 and 24 select forward direction 2 at the actor heading and mark movement
bits `0x30`. Values 3 and 7 select backward direction 3 at heading plus 1024
and mark `0xc0`. Value 4 selects direction 4 at heading plus 512 and marks
`0x03`; value 5 selects direction 5 at heading minus 512 and marks `0x0c`.
Other values return without these writes. Selected headings wrap to 2048.

The selected direction and movement bits are written even when the current
animation suppresses displacement. Otherwise, stored surface state 2 or 4
halves the signed distance by arithmetic right shift. A positive resulting
distance translates local X/Z using the heading-based translation rule.
The corresponding directional update indicator is also set.

For an initial ordinary animated fall, current groups 41, 42 and 43 apply
this displacement using the retained motion direction and Q8 distance 4096
before selecting their initial fall animation.

### Pre-script queued and falling stage

Every actor update runs one stage between the ordinary animation stages and the
script stage. Its only content is directional displacement. It has no test for
actor kind, selected player, living state or collision attribute mask, so the
selected player and every nonplayer take it alike. After the
[action-transition counter](SOUP.md#selected-player-additive-health)
bookkeeping, a nonzero queued byte supplies the motion value; otherwise a nonzero
falling byte does. Neither nonzero skips the stage. The sliding byte never selects
it. Directional displacement returns without effect while the actor is attached to
a rope.

For queued or falling motion 8 or 9 the displacement is forward at the actor heading. It sets motion
direction 2, the forward update indicator and movement bits `0x30`, even when the current animation
suppresses displacement. Otherwise, while the accumulated sliding distance is below `0x18000`, each
update adds `(0x10000 / attribute 31) * frame_scale >> 8`, limited so the accumulated distance
does not exceed `0x18000`, to that distance and translates by the same amount; at or above the
cap nothing moves. Motion 2 (a jump steered forward) does the same with `0xb400` and a cap of
`0x10000`. Deep water halves neither. Other motion values add no displacement here. A zero attribute
31 divides by zero in the original.

Queued arbitration, the queued and falling vertical writers, the edge service,
landing and fall start are not part of this stage. They run only inside the
[ground response](#ground-stage-by-model-kind), which an animated actor reaches
only when its collision attribute mask has bit 4 set. An actor whose ground
response does not run therefore still receives this displacement every update
while its queued or falling byte is nonzero, and never advances vertically,
lands or starts a fall.

### Ground stage by model kind

A static-model actor runs its ground stage only when attribute 108 is nonzero
and attribute 107 is zero, regardless of the collision attribute mask: the support
query, then the ground response. The animated-model ground stage is selected by
the collision attribute mask instead: a nonzero mask runs the obstacle pass
unless the selected player is in its alternate collision mode, mask bit 8 runs
the overlap pass, the support query follows, and mask bit 4 runs the ground
response. Attribute 107 nonzero also ends a fall at the first support miss: the
falling byte is cleared and nothing else changes.

### Ordinary landing decision

On support regained while falling, clear the runtime jump selector and compare
the absolute wrapping difference between current local Q8 Y and retained fall
origin. Resolve and store the landing surface before this comparison; surfaces
1 through 4 replace the compared distance with zero.

The death threshold is `SYSTEM.INI [game] fall_death_height * 256`, and the
hurt threshold is `fall_hurt_height * 256`, retaining 32-bit arithmetic.
Distance strictly above the death threshold requests fatal landing and applies
health change -256. Otherwise distance strictly above the hurt threshold
requests group 73 and applies `-fall_hurt_zap`, except when the actor's separate
hurt-exclusion predicate suppresses both. That predicate is true for stored
surface states 1 through 5 and 11. These threshold branches precede the
ordinary landing animation selection. They clear the falling byte.

For the ordinary nonplayer fatal path, group 74 is an ordinary request.
The ordinary hurt group 73 also uses ordinary request arbitration.

The selected player takes the same threshold tests and the same hurt response:
group 73, then the generic health-with-death admission with `-fall_hurt_zap`,
which forwards the selected receiver to its death-owning health with no source.
Its fatal landing makes no animation request. It stores group 74 as the
death-animation override, applies health -256 through that same admission and
then clears the falling byte. A fatal landing on a support whose stored
classification is steep (2) instead raises the controller's pending action flag
and does nothing else: no override, no health change, and falling stays set.

### Ordinary landing animation selection

After threshold handling admits an ordinary landing, low falling state 8
requests group 67 without a burning torch or 68 with one when the selected
player is not holding a weapon; otherwise it requests 130. Low state 9
similarly requests 69 or 70 without that selected-player weapon predicate,
or 131 with it. These branches read the selected player's weapon state,
which is distinct from the landing actor's weapon state.

Other falling states request 65 without a burning torch or 66 with one when
the landing actor is unarmed. A weapon-holding actor requests 114 unless its
current group is 115 or 116. All these landing requests use ordinary animation
arbitration and clear falling after dispatch. The retained death-response byte
is set to 8 by the actor death notification service.

### Nonplayer health mutation

The shared health service returns without mutation when attribute 120 is
nonzero. With attribute 97 nonzero, it also returns when the actor is neither
holding a weapon nor in combat mode with packed bit 2 set.

Nonplayer health is attribute 3. Add the signed health change using wrapping
32-bit arithmetic, then clamp the signed result to 0 through 255. A result of
zero dispatches death animation and notification. For an unarmed actor, force
group 74 when falling is nonzero or current group is 114, 116, 130 or 131;
otherwise force 134. Death notification sets the retained response byte to 8.
Armed death has separate combat animation and state services. Selected-player
health uses a separate service rather than mutating this attribute byte.

### Stable recovery probe

The late stable-position recovery probe translates the actor's query position
forward by 6144 Q8 units at its local heading, then converts all coordinates
to integer map units. Its vertical segment starts at that Y plus the actor's
positive Y extent minus `post_collide_height` and ends at that Y plus the extent
plus `post_collide_height`.

Recovery prepares its own ordered static, live-actor and transient candidate
lists. A current model supplies a search radius of twice its header radius,
truncated toward zero. With a current model, admission uses the effective query
position. Without a current model the search radius is 200 map units and admission
uses local position. The actor's own current model instance is excluded.

It visits its prepared face-model candidates in order with sidedness disabled.
The first contact with normal classification 4 accepts recovery and writes local
position to the probe's integer X/Z multiplied by 256 and contact integer Y
minus the positive extent, also multiplied by 256. It does not add standing
height or rank competing faces.

When no model contact accepts, terrain height truncated toward zero accepts
recovery if it lies inclusively between both segment endpoints. Terrain
acceptance does not write the actor position. With no animation receiver the
probe accepts before those segment tests. Candidate preparation precedes that
receiver check and is distinct from the ordinary retained collision list.

### Stable recovery response

The ordinary script wrapper retains a byte collision-response age. After its
admitted script work, an all-zero packed response record clears the age to zero.
Otherwise, an age below 200 adds the shared nominal tick increment with byte
wrapping; an age at least 200 is unchanged. The wrapper then clears the packed
response record and overlap contact identifier. Body-response and collision
transition marks occupy separate bytes of that packed record.

A nonplayer stable recovery request returns without work when the stable-position
record is invalid. It prepares the separate recovery query and returns when that
probe accepts. On rejection, it restores the stable local position, clears
attribute 6, falling and sliding, restores the saved directional movement flags
and animation collision snapshot, and marks collision transition bit 1. The
restored animation snapshot does not replace the ordinary default group.

The nonplayer then adds `Rnd(attribute_15 * 4) - attribute_15 * 2` to local heading,
wrapped to 2048. A collision-response age strictly greater than 16 also runs the
bounds-segment obstacle response. The recovery marks the shared position-changed
flag. The selected player takes neither the heading change nor the bounds response:
after the shared restore (stable position, attribute 6, falling and sliding cleared,
movement flags and animation snapshot, collision transition bit 1) it runs the
ordinary ground response once more. When that response reports no support, the
attempted position is written back, the stable-position record becomes invalid and
the falling byte returns to its value before recovery.

After reserved weight is restored, ordinary collision invokes stable recovery
when attribute 89 is nonzero and either sliding or falling is nonzero. Otherwise,
a captured landing group of 67 through 70, 130 or 131 invokes recovery only when
sliding, falling or the shared surface veto is nonzero. These checks precede the
separate water rollback.

### Ordinary water rollback

A nonplayer actor with no falling or queued motion rolls back when its stored
surface is water, codes 1 through 4. This restores a separately retained local
position and clears attribute 6. At the end of an ordinary update, a nonwater
surface or active falling state replaces this record with the current local
position. A water surface without falling leaves it unchanged. The record is
distinct from the position captured at update entry. It also restores the directional movement
flags and the animation collision snapshot, marks collision transition bit 1,
and sets the shared changed flag. Selected-player actors do not take this branch.

### Stable ordinary position

After collision attribute bit `0x04` ground response, an actor with queued
motion, falling and sliding all zero marks its stable-position record valid
and copies all three current local Q8 coordinates into that record. An update
with any of those motion states nonzero retains the existing stable record and
validity. This record is distinct from the position saved at update entry.
Jump admission clears its validity before initializing queued motion.

### Ordinary slope response

Slope response reads `[game] slide_speed` from SYSTEM.INI, clamped to at
least 8 during settings loading. It uses the current primary support result.
With no support or a classification other than 2, it requests ordinary group
75, or 77 with a burning torch, unless the current group is already 75 through
78. This fallback does not change sliding state, distance or position.

For class 2, the horizontal support normal supplies the quantized movement
heading. The comparison heading is local for an unattached actor and effective
for a family actor. A signed wrapped heading difference with magnitude below
512 selects a forward slide, groups 75 or 77, and movement bits `0x30`.
Other differences select a backward-facing slide, groups 76 or 78, and bits
`0xc0`. Backward-facing actors turn toward movement heading plus 1024; both
branches translate along the movement heading.

Turning precedes the slide animation request. It uses attribute 15 when neither
holding a weapon nor in combat mode, and attribute 21 otherwise. The turn rate
is the signed wrapping product of that byte and frame scale, shifted right by
8, then clamped to at least 8. Differences within that rate snap to the target
without requesting a turn animation. Larger differences turn by the rate and
request an ordinary turn group when an animation receiver exists. Without a
held weapon these are positive 14 or torch 16, and negative 15 or torch 17.
With a held weapon they are positive 96 and negative 97. Combat mode affects
rate selection independently of the held-weapon animation branch. Heading
writes wrap to 2048. The turn helper returns zero when the desired heading already
matches or the turn snaps to it. Otherwise it returns the signed heading
difference before applying the turn step. For the selected player the
turn also rebuilds the actor's orientation matrix from its angles (identity, then negated yaw,
pitch and roll) and copies the angles to the angle history; it changes no camera state.

The first slide forces its slide group; subsequent slides request it ordinarily.
Distance is the signed wrapping product `slide_speed * 128 * frame_scale`,
shifted right by 8. Only a positive distance translates X/Z. Primary support
is queried again with the original response probe distance; an available result
replaces local actor-origin Y. The first slide clears accumulated sliding
distance, then every class-2 response adds the computed distance, including
nonpositive values, and sets sliding state to one.

### Slide termination

When the ground response finds support and the sliding-surface result is clear (or the native
return-state byte is nonzero), a nonzero sliding byte ends the slide. If the accumulated sliding
distance exceeds `SYSTEM.INI [game] slide_hurt_height * 256`, the actor first takes directional
displacement of Q8 distance `0x800` (direction 2 in groups 75 and 77, direction 3 otherwise), then
requests group 73 and changes health by `-slide_hurt_zap`; the selected player instead stores death
override group 74 (no animation request) and takes the health change through its death-owning health.
The sliding byte then clears. The selected player also takes the hurt branch whenever its
steep-landing flag (the controller's pending action flag) is set, whatever the slide distance.

### Initial falling bookkeeping

When the ordinary support response misses and starts a fall, it records local
Q8 Y plus `256 * attribute 32` as the fall origin and sets the byte timer to
30. The ordinary falling byte becomes one. Static-model actors then run the
vertical writer without an animation request. An animated actor's initial
unarmed request is forced group 56 without a burning torch or 57 with one;
a weapon-holding actor forces 113 unless current group is 115.

Animated entry also has separate selector, stable-position recovery and
horizontal-displacement branches before this initialization. When either
directional-state byte has a low bit set, the recovery branch clears those two
bits in both bytes, restores all three coordinates from the
[stable position](#stable-ordinary-position) record, runs the ordinary jump entry
with requested motion 8, forces group 35 with a burning torch or 34 without, runs
the queued-motion arbitration once and ends the update's fall handling. Current groups
18 and 30 have an exit-marker rollback branch and otherwise force 59 or 60.
For the exit-marker branch, restore all three coordinates from the stable
position record and force the current default group, then return without starting
a fall. Without that marker, record the ordinary fall origin and timer, set
falling to one and force 59 without a burning torch or 60 with one.
These entry branches are distinct from continuation of an existing fall.

### Jump selector at a support miss

An animated actor that misses support without falling first tests the runtime jump selector. A
selector whose distance from 24 is below four (and nonzero) starts the
[ordinary jump entry](#ordinary-jump-entry) for it, which selects queued motion 24 and forces group
85, and the support miss ends there: the walk-off recovery, the fall start and the edge service do
not run. Other selectors leave the miss to those services.

### Falling animation continuation

For an animated actor already falling without support, the [edge service](#edge-service) runs
before the continuation request. When it does not handle the actor, low falling
state 3 requests group 63 without a burning torch or 64 with one. A set high bit
with no burning torch otherwise requests group 58. These cases precede the
short-distance motion variants.

Falling byte 8 with absolute local Q8 Y distance from the fall origin strictly
below 32768 requests group 59 without a burning torch or 60 with one. Byte 9
under the same condition requests 61 or 62. Other ordinary unarmed continuation
requests group 56 or 57 according to the torch-burning counter. An actor holding
a weapon instead forces group 113 unless its current
group is 115. The vertical writer runs after this request, reading the resulting
animation's model-controlled vertical-motion marker.

### Selected-player ledge entry

Ledge entry lets the selected player catch a ledge while falling or during
queued motion, hang from it, move along it and climb onto it. It uses the
living-state byte, the sliding, falling and queued-motion bytes, the retained
ledge-support actor reference, a retained ledge point and ledge heading, and the
global action-inhibition countdown. Ledge living states are 10 through 19.
Coordinates use Y increasing downward. The positive Y extent reaches from the
actor origin down to its feet; the negative Y extent reaches up to its head.

#### Edge service

The edge service returns zero unless the actor is the selected player,
attribute 105 is not one, the torch-burning counter is zero, and neither living
state 33 or 34 nor current group 149 or 150 blocks it. Those block only while
animation is enabled and shell context is clear. Then:

- Stored surface 2 or 4 returns zero. In living state 15 it first runs the
  [hang release](#ledge-hang-release) and then the
  [ledge transition](#ledge-transition) once.
- Living states 10 through 19 return one without other work.
- Any other nonzero living state returns zero.
- Living state zero becomes 19, the ledge-support reference is cleared and the
  ledge transition runs once. A nonzero resulting living state clears the
  sliding, falling and queued-motion bytes and sets the action-inhibition
  countdown to 16. A zero resulting state with a nonzero action-transition
  counter clears bit `0x80` of the queued-motion and falling bytes. The service
  returns the transition's result.

The ground response calls the service for an animated actor without support.
An actor already falling calls it before the
[continuation request](#falling-animation-continuation); an actor starting to
fall calls it after its [initial fall request](#initial-falling-bookkeeping). A
nonzero result clears the falling byte and ends that update's fall handling.
[Queued motion arbitration](#queued-motion-arbitration) also calls it; a
nonzero result clears queued motion and returns handled.

Player initialization clears the action-inhibition countdown. Each update
subtracts the whole nominal ticks; a countdown that would not stay positive
becomes zero.

#### Per-update ledge dispatch

An actor whose ground-response bit `0x04` is set and whose living state is 10 through 19 runs the
[ledge transition](#ledge-transition) once per update in place of the ordinary motion and ground
response; weapon-draw states 20 and 21 keep their own branch. For an actor other than the selected
player the transition changes nothing. When the sliding, falling and queued-motion bytes are all zero
afterwards, the stable-position-valid flag becomes one and the current position is retained as the
stable position, including when the transition ended in state 18 or cleared the state. Nonzero
bytes leave the stable record unchanged. The update does not run the body-contact response of states
other than 0, 15 and 27 through 32.

#### No ledge candidate

When the [ledge scan](#ledge-scan) finds no record, entry from living state zero
sets living state 19, selects nothing, sets the living state back to zero and
returns zero. Sliding, falling, queued motion and the action-inhibition countdown
are unchanged, except for the bit `0x80` clear above when the action-transition
counter is nonzero. The ledge-support reference stays cleared. The scan changes
no actor state, so the caller continues its ordinary fall or queued-motion
handling exactly as if the service had not run. A weapon-holding actor or a
nonzero action-transition counter always takes this path.

#### Ledge transition

The transition returns zero without changes unless the actor is the selected
player, attribute 105 is not one and the living state is 10 through 19. It
clears the global hang-model reference. A nonzero torch-burning counter or the
state-33/34 or group-149/150 block runs the hang release and returns zero.
Otherwise states are handled in the table's order within one call; a state set
by one row is handled by later rows in the same call.

| State | Meaning | Handling |
|---|---|---|
| 15 | Hanging | Scan and select the hand-nearest class 3 record. None runs the hang release and returns 0. Otherwise set the hang-model reference to the record's top model. Only when a left or right sidestep started this update, add `256 × sine[256]` to the record's X and `256 × sine[768]` to its Z, nearest rounding, and, if the current animation's `BreakPoint` marker is clear, copy the record point to the ledge point and the record heading to both the ledge heading and the actor heading. Without a sidestep, retain the existing ledge point and headings. Measure the [support below the origin](#ledge-support-below-the-origin). A distance below `(positive Y extent + 24) × 256` returns the [climb request](#ledge-climb-request)'s result. Otherwise apply the [hang pose](#ledge-hang-pose), set the ledge-support reference to the record's actor and return 1. |
| 19 | Entering | Scan. Result 1 selects the first class 1 record and state 12. Otherwise the first class 2 record selects state 10, else the hand-nearest class 3 record selects state 14. Without a record, set state 0 and return 0. Otherwise copy the record point to the ledge point, its heading to the ledge heading and its actor to the ledge-support reference. |
| 12 | Turning to a low ledge | Run the slope [turn helper](#ordinary-slope-response) toward the ledge heading. A nonzero result returns 1 when its magnitude is at most 256; otherwise set state 0 and return 0. When facing, rescan; a result other than 1 sets state 0 and returns 0. Otherwise set X/Z to the ledge point plus `low_forward[0] × 256` at ledge heading plus 1024, leave Y, force group 90 and set state 13. |
| 14 | Catching a hang | A wrapped difference between actor heading and ledge heading above 256 in magnitude sets state 0 and returns 0. Select the hand-nearest class 3 record from the existing table without rescanning; none sets state 0 and returns 0. Set X/Z to the ledge point plus `high_forward[0] × 256` at ledge heading plus 1024, leave Y, force group 88 when record byte 0 is zero or 89 otherwise, set state 15, the actor heading to the record heading and the ledge-support reference to the record's actor, and return 1. |
| 10 | Turning to a high ledge | As state 12, but the rescan must return 2. Set X/Z to the ledge point plus `high_forward[0] × 256`, set Y to the ledge point's Y, request group 86 with speed modifier 0 and frame threshold 2, and set state 11. |
| 13 | Low climb | While the current group is 90, with `f` the animation's `real_frame`, set X/Z to the ledge point plus `low_forward[f] × 256 − lag` at ledge heading plus 1024; when `f` exceeds 6 set Y to `ledge_Y + (low_height[f] + standing_height) × 256`. Return 1. Otherwise set state 18, clear sliding, falling and queued motion, and return 1. |
| 16 | Climb from hang | Force group 86, set state 11 and `real_frame` to zero. |
| 11 | High climb | As state 13 with group 86, `high_forward` and `high_height`, writing Y on every frame. |
| 17 | Dropping | Force group 56, set the action-transition counter to 6, clear the living state and the ledge-support reference, and return 0. |
| 18 | Climb finished | Returns 0. The ordinary actor update clears the pending group while the current group is 90 or 86; otherwise it forces the default group and clears the living state. |

Other ledge states return zero. A ledge reached from a fall or queued motion can
only produce class 3 records (see [classification](#ledge-record-classification)),
so a successful catch passes states 19, 14 and 15 in one call and returns 1.

An offset `d` at heading `h` adds `round(d × sine[u])` to X and
`round(d × sine[u + 512])` to Z, with `u = (−h) & 2047` and the shared
floating-point sine table. Ledge heading plus 1024 points from the wall back
toward open space. `lag` is `max(0, command_cursor − command_count / 2) × 256`
for the current group's signed command cursor and command count, and zero while
animation is disabled. The climb tables are indexed by `real_frame` without a
bound check:

| Table | Entries |
|---|---|
| `low_height` | 35, 35, 35, 35, 38, 29, 15, 0, −7, −21, −42, −56, −58, −60, −59, −61, −61, −61, −61, −61, −61, −61 |
| `low_forward` | 28, 28, 27, 27, 28, 24, 21, 16, 8, 7, 6, 5, 4, 4, 4, 5, 4, 3, 2, 2, 2, 2 |
| `high_height` | 81, 67, 45, 24, 11, −1, −19, −28, −29, −29, −29, −29, −45, −53, −58, −60, −61, −61, −60, −60, −60, −60, −60, −60, −60, −60 |
| `high_forward` | 30, 30, 30, 26, 20, 16, 8, 3, −6, −6, −4, 0, −6, −3, 1, 0, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2 |

#### Hanging sidestep

A left or right sidestep in living state 15 is admitted when the [ledge scan](#ledge-scan), run with
the actor shifted along its own X axis, still finds a class 3 record for the hand. The shift is the
actor's X extent on that side (the negative extent to the left, the positive to the right) plus
attribute 10, in whole map units, rotated by the actor matrix; X and Z change and Y stays. The
original position is restored whatever the result. An admitted step requests animation group 28
(left) or 29 (right) and, with the animation system enabled, continues until that group is current.
It then sets the left or right update indicator, which the [hanging transition](#ledge-transition) reads, and moves the actor along its
own X axis by attribute 10 times the frame-scaled step ramp, with no translation while the current
animation's `BreakPoint` marker is set.

#### Ledge hang release

For the selected player in living state 15 with a zero action-inhibition
countdown, set living state 17, clear the ledge-support reference and run the
transition. A nonzero resulting state clears the sliding, falling and
queued-motion bytes. Other states, and a nonzero countdown, change nothing.

#### Ledge climb request

After jump-key handling, the selected-player controller calls the climb request
while forward is held and the living state is 15. It then calls the hang release
while backward is held and the living state is still 15. These directional calls
do not require the walk key.

For the selected player in living state 15, scan and select the hand-nearest
class 3 record. Without one, or with record byte 2 zero, return 1 and keep
hanging. Otherwise set the ledge-support reference to the record's actor, set
state 16 and run the transition; a nonzero resulting state clears the sliding,
falling and queued-motion bytes. Return the transition's result.

#### Ledge hang pose

Let `g` be the grip-0 vertex of the first `ShowFrame` of the actor's animation
group 7, from the [animation-vertex query](attachment.md#actor-model-point-conversion)
in Q8 integers. Set X/Z to the ledge point plus `high_forward[0] × 256` at ledge
heading plus 1024, and Y to `ledge_Y + standing_height × 256 + abs(g.y) + 512`.
Without a group 7 frame nothing is written.

#### Ledge support below the origin

A vertical model segment runs at integer current X/Z from Y plus the positive Y
extent up to Y. The smallest contact Y among classes 4 and 2 is kept; with
terrain enabled, a terrain height strictly between Y and that value replaces it.
The result is `selected × 256 − current_Y`, or 256000 without a selection.

#### Ledge scan

The scan clears the four-record ledge table and disables polygon sidedness for
its duration, re-enabling it afterward. It finds nothing when the
action-transition counter is nonzero or the actor holds a weapon. Otherwise it
runs one pass at forward offset `p = 0` and, when that returns zero, a second at
`p = 6144` (Q8). The result is 1 when a class 1 record exists, else 2 for
class 2, else 3 for class 3, else 0.

Each pass:

1. Build a segment from actor-local Q8 `(0, 40960, 9728 + p)` to
   `(0, −40960, 9728 + p)`, rotated by the actor's orientation matrix, added to
   the current position and shifted right eight.
2. Query each prepared model candidate with the integer ground-probe adapter,
   classifying contacts with the
   [surface classification](#ordinary-forward-walk-support-checks). Keep the
   class 4 contact with the strictly greatest fraction as that candidate's top
   (zero fractions never qualify). Class 8 contacts from candidates whose type
   lacks bit `0x08` append their Y to an underside list shared by the pass.
3. For a candidate with a top, while fewer than four records exist, build a
   horizontal wall segment from local `(0, 0, 9728 + p)` to `(0, 0, p)`, rotated
   and positioned as above, at Y = top Y + 16. Over all candidates keep the class
   1 contact with the strictly greatest fraction whose normal's facing heading
   (its displacement heading plus 1024) is within 256 of the actor heading,
   differences wrapped by 2047. No such wall adds no record.
4. Repeat the wall segment 24 units lower: any such facing class 1 contact sets
   record byte 0 to one, otherwise zero. Append a record with byte 0, ledge point
   `(wall_X, top_Y, wall_Z) × 256`, the top's model, the record heading (the wall
   normal's facing heading) and the ledge-support actor (the wall candidate's
   actor, zero when it has none). A fifth record is not guarded against.

#### Ledge record classification

Let `Y` be the integer current Y, `neg` and `pos` the Y extents,
`reach = max(14, fall_step >> 9)` with `fall_step` the nonpositive-timer
[falling step](#initial-falling-bookkeeping)
`min(post_collide_height × 512 − 256, attribute_32 × factor[0] × (fine_tick_increment + 2))`,
and `hand_Y = Y + (g.y >> 8)` for the group 7 grip vertex `g`, or `Y − 1000`
without that frame. For each record with top Y `t`:

- With falling or queued motion nonzero, or living state 14 through 17: class 3
  in living state 15 or when `abs(hand_Y − t) < reach`, else 0.
- Otherwise: class 2 when `Y − neg − reach <= t < Y − (neg >> 2)`; class 1 when
  `Y − (neg >> 2) <= t < Y + pos − post_collide_height`; else 0.

Then an underside Y with `abs(t − underside) < 8` sets class 0. For a nonzero
class:

- Outside living state 15, a wrapped heading difference above 256 between actor
  and record heading sets class 0 and skips the fit test.
- The fit test places the actor's collision spheres, with its current angles, at
  the wall point plus offset 4096 at the record heading and Y
  `(t − pos − standing_height) × 256`. Candidates are tested in order; a body
  contact fails the body test and ends the loop. Until then each candidate is
  also tested with one sphere of radius `post_collide_height >> 1` centered
  `pos − (post_collide_height >> 1)` below that Y; a class 8 contact fails the
  head test.
- A failed body test sets classes 1 and 2 to 0. For class 3, record byte 2 is 0
  when the head test failed, otherwise 1 when the body test passed and 0 when
  it failed.

The hand-nearest class 3 record minimizes `abs(t × 256 − (local_Y + g.y))`,
earliest on ties, and does not exist without a group 7 frame. Other selections
take the first record of the class.

### Ordinary jump entry

Jump entry requires no active native motion task, queued motion, falling or
sliding, and actor state zero or 15. Selected-player edge and water gates run
separately. State 15 dispatches its separate exit service instead of this
ordinary initialization.

For ordinary initialization, clear the stable-position-valid flag, record
current local Q8 Y as fall origin, set the byte fall timer to zero and clear
accumulated sliding distance. With the runtime jump selector zero, current
group 18 selects queued 9 and request 36 when the requested motion is 9,
otherwise queued 8 and request 34. Current group 30 similarly requests 37 or
35. Other current groups select queued 1 and request 32 without a burning torch,
or 33 with a burning torch. A nonzero runtime jump selector instead selects
queued 24 and forces group 85. Set motion direction to 2 after the request.

The controller itself reads no surface state: water and lava surfaces change its
behavior only through the movement helpers and the support response it calls.

### Script Jump

The zero-argument SOUP function `Jump` returns zero immediately, including when
admission refuses or an edge service handles the call. It does not wait for the
jump animation or landing.

Admission refuses a selected player with a nonzero conditional controller
countdown. It also refuses surface 2 or 4, any active native task, nonzero
queued, falling or sliding motion, and living states other than 0 or 15.
Groups 114, 116, 130 and 131 refuse admission. For the selected player with
attribute 105 equal to one, all water surfaces 1 through 4 refuse admission.

After admission, clear the stable-position-valid flag. Living state 15 runs
hang release and returns. Otherwise run selected-player edge entry; a handled
entry returns before initializing queued motion.

Ordinary initialization records current local Q8 Y as fall origin, clears the
fall timer and accumulated sliding distance, then chooses the queued motion and
animation request below. A nonzero runtime jump selector overrides all rows,
selecting queued motion 24 and forcing group 85.

| Current group | Queued motion | Requested group | Additional writes |
| --- | --- | --- | --- |
| 18 | 8 | 34 | None |
| 30 | 8 | 35 | None |
| 19 | 3 | 49 | Copy heading into `ov.vy`; set the selected-player conditional countdown to 6 |
| 23 | 3 | 50 | Copy heading into `ov.vy`; set the selected-player conditional countdown to 6 |
| 31 | 8 | 34 | Copy heading into `ov.vy` |
| Other, holding a weapon or with combat flag `0x04` | 1 | 111 | None |
| Other, no held weapon or combat flag, torch burning | 1 | 33 | None |
| Other, no held weapon or combat flag, no burning torch | 1 | 32 | None |

All non-forced rows use the ordinary animation request with reverse zero.
After the request, set motion direction to 2. The script native does not select
the backward queued state 9 used by the parameterized ordinary jump entry.

### Jump-key arbitration

While the jump key is held, every ordinary controller update after movement
dispatch runs one arbitration with mask 1; the rope branch has its own jump-off
service. The arbitration does nothing when either directional-state byte already has
the mask bit set. Otherwise:

- A current group from 65 through 74, a nonzero queued byte or a nonzero falling
  byte clears the low two bits of both directional-state bytes and ends the
  arbitration. The sliding byte is not tested here.
- Otherwise a step-hold condition is true when attribute 11 is zero or the motion
  byte's step field (bits 4 and 5) equals 3. With the step-hold condition true and
  either current group 18 or 30, a clear exit marker sets the low two bits of both
  directional-state bytes and clears the byte at `+0x1be`. Any other state calls
  the ordinary jump entry.
  This jump start refuses, without any state change, while the actor stands on surface 2 or 4
  (deep water), while its current group is 114, 116, 130 or 131, and while its falling byte is nonzero.

This is the only writer that sets the directional-state low bits. The ordinary jump
entry also refuses the selected player while the conditional preliminary countdown
is nonzero, so a nonzero queued or falling byte, or a nonzero sliding byte, ends the
call without effect.

### Jump window

Every actor update, after its script stage and before its ordinary movement, adds the whole
nominal ticks to a byte counter. When the counter exceeds 36 the second directional-state
byte receives the first byte's bits, the first byte clears and the counter returns to zero. A
nonzero queued byte, or a current group other than 18 and 30, clears the low two bits of both
bytes. Then a set low bit of the second byte clears the low two bits of both bytes and starts the
[ordinary jump entry](#ordinary-jump-entry) with requested motion 8; otherwise a set second bit
does the same with requested motion 9. The pending first byte therefore survives for up to 36
ticks while a run or walk jump group loops, which is the window in which a support miss takes the
walk-off recovery.

For the selected player the entry runs the following before the ordinary initialization, in
order: it refuses while the conditional preliminary countdown is nonzero, while a rope is attached,
while the queued, falling or sliding byte is nonzero, outside living states zero and 15, and in
water or lava surfaces when attribute 105 is one. It clears the stable-position-valid flag. Living
state 15 then runs the [hang release](#ledge-hang-release) instead of starting a jump, and living
state zero runs the ledge entry (the edge service without the water exit, the action-inhibition
countdown and the fall-bit clear); a ledge held by either ends the entry.

### Queued motion arbitration

A zero queued byte returns unhandled. Low queued state 3 requires group 49
when the torch-burning counter is zero, or group 50 when it is nonzero. An
unavailable required group clears queued motion and returns unhandled. A
different current group returns unhandled without a burning torch, but handled
with a burning torch. These branches do not read the living-state byte.

Pending groups 32 through 37 and 111 defer queued motion and return unhandled.
A successful selected-player [edge service](#edge-service) clears queued motion and returns
handled. Other admitted queued states make an ordinary animation request before
vertical advancement:

| Low queued state | No torch, high bit clear | No torch, high bit set | Burning torch |
|---|---|---|---|
| 1 | 39, or 112 when holding a weapon | 40 | 38 |
| 2 | 42 | 43 | 41 |
| 3 | 49 | 49 | 50 |
| 8 or 24 | 44 | 48 | 45 |
| 9 | 46 | 48 | 47 |

Other nonzero queued states advance vertically without this animation request.
After vertical advancement, a cleared queued byte returns handled. Otherwise,
clearance strictly below `(positive_Y_extent - 4) * 256` clears queued motion
and restores all previous local position coordinates; the result remains handled.

### Ordinary ground prevalidation

Ground prevalidation uses a shared guard reset at ordinary-wrapper entry. The
first call marks the guard used before checking collision attributes. Later calls
return without changing the retained position rollback veto or sliding-surface
result, even when collision candidates or actor attributes have changed.

The first call bypasses geometry queries when collision attribute bit 2 is clear,
or for the designated player while its
[support-history counter](#selected-player-fallback-support) is nonzero. This
bypass clears both retained results. Other actors are not excluded by the counter.

It queries current and previous effective positions with the post-collision
height as vertical half-length. Face contacts use the forward steep-surface
midpoint ranking above, followed by terrain height comparison. This scan does
not apply the queued-motion candidate-type exclusion. When local X and Z have
not changed, the current result also serves as the previous result.

If both queries miss, both results clear. When exactly one query misses, the
classification of the missing side is read from uninitialised stack memory and
is not defined. Classification 4 at both positions
also clears both. Classification 2 at both positions permits motion and marks
the sliding-surface result. For a current class 2 after another previous class,
subtract the motion heading from the current normal heading minus 1024 and
wrap to 2048. Values below 512 or above 1535 request rollback and clear sliding.
For other directions, motion permits sliding only when attribute 89 is zero
and the current animation group is neither the retained default group nor any
of groups 20 through 27. Otherwise it requests rollback. Other current classes
clear both results.

The wrapper applies rollback by restoring all three previous local position
coordinates, clearing the retained movement flags, and running the ordinary
obstacle transition before the main support query.

The ground response reads the sliding-surface result after it has assigned the
support height to the actor's Y. A set result with a zero native return-state
byte clears the falling byte, runs the [ordinary slope response](#ordinary-slope-response)
and ends the ground response without its remaining handling. With a nonzero
native return-state byte the result is ignored. The rollback result is read only
by the wrapper.

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

Queued motion, either directional-state byte with either low bit set, or the
selected-player surface exclusion cause the excluded-query result. With those
gates clear, falling, sliding, or living states 27 through 29 select the
return-without-assignment path. Other ordinary actors take the assignment path.

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

#### Water and lava transition effects

The effects below run only for the selected player, only when the surface updater ran (collision
attribute mask `0x04`) and its returned value differs from the saved previous surface. Let P be the
saved previous surface and S the actor's stored surface after the update; the predicates read S.
An update that takes the excluded-query result returns zero without storing, so with a nonzero P the
comparison fires and the predicates read the unchanged stored surface; the return-without-assignment
path returns the resolved surface, which equals P when the surface did not change.

- S is outside 1 through 4 and P is 0 or above 4 (not in water before or now): when S is 5 or P is
  5, one effect of identifier `0x2580` is created at the actor; otherwise nothing happens.
  Each effect is the `AddFlatEffect` native with zero spread and speed one, placed at the actor's X and Z moved
  eight map units (0x800 in Q8) toward the camera, along the bearing from the camera to the actor, at the
  support height truncated toward zero plus `standing_height` (map units, scaled to Q8) plus 0x600.
- Otherwise (in water now or before):
  1. Animation request. S not 2 or 4 and P equal to 2 or 4 (leaving deep water): the actor's current
     animation motion byte 2 or 6 requests group 18, or group 30 while the torch-burning counter is
     nonzero; any other motion byte requests group 19, or 23 with the torch. S equal to 2 or 4
     (in deep water): motion byte 2 or 6 requests group 142, any other motion byte group 143.
     Shallow-to-shallow changes request nothing.
  2. One effect of identifier `0x2500` is created, placed as above.
  3. A positional sound at the actor with volume zero and a rate offset equal to a random value
     below 64 minus 32: effect 70 when S is outside 1 through 4 (left the water); otherwise effect 71
     when P is 0 or above 4 or S is 2 or 4 (entered water, or in deep water); otherwise effect 70.

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

### Contact body-shape selection

Contact queries use the actor's initialized ordinary body shape unless its
current living animation belongs to class 2, a combat record exists, and that
record has a tertiary MPSZ shape. That combination selects the tertiary shape.
Fighting class 1 does not select the tertiary shape. An absent tertiary shape
falls back to the ordinary shape.

The animated actor loader builds the tertiary shape from the third RAHD MPSZ
index using the signed RAEX radius scale. A negative third index leaves that
shape absent. Combat-record reinitialization preserves this shape. The selector
updates the chosen shape's whole-map translation and wrapped inverse angles
from the actor's current effective pose before returning it.

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

Collision queries rebuild their orientation from these angle fields, rather
than using the actor's retained presentation matrix. Let `sp`, `sy`, and `sr`
be shared-table sine samples at the wrapped negative pitch, yaw, and roll;
`cp`, `cy`, and `cr` are the samples 512 entries later. The collision matrix
has these rows:

| Row | X | Y | Z |
| --- | --- | --- | --- |
| 0 | `sp * sr * sy + cr * cy` | `-sr * cp` | `sr * sp * cy - sy * cr` |
| 1 | `sr * cy - sp * sy * cr` | `cr * cp` | `-sr * sy - cr * sp * cy` |
| 2 | `cp * sy` | `sp` | `cp * cy` |

Each resulting component is stored at single precision. Multiplications and
additions retain extended intermediates until that store. For an upright
actor this matrix maps local forward to `(-sy, 0, cy)`, unlike the retained
actor matrix's `(sy, 0, cy)` mapping. The primary body query uses the selected
shape's current local coordinates and angles. Residual response uses the first
working record's copied angles, with its origin refreshed from the actor's
current effective map position.

An actor whose definition carries vertex-bound collision records (`RAVC`) selects a shape
rebuilt from them instead of its initial shape: one sphere per record, in record order. Each center
axis is the record's model vertex divided by 256, truncating toward zero, plus the record's signed
offset on that axis, and the radius is the record's radius. The vertex is read in model units from the
actor's current animation frame when the actor has an animated model source, otherwise from its
base model. The rebuilt spheres then follow the position and orientation preparation above.

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

The ordinary actor exclusion test rejects the unselected member of the two
player roles. It admits the selected player and actors outside those roles.
Selection zero chooses the first role; every nonzero selection chooses the
second. The remaining candidate gates still apply to the selected player.

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
centers receive the actor matrix transform and its shape origin truncated to
integer map coordinates.

Sphere pairs are visited in shape order. The actor-to-actor query truncates
the radius sum and each center difference toward zero. The distance calculation
stores integer differences as single-precision values, sums their squares,
computes the square root, stores that result at single precision, and converts
it to an integer with nearest-even rounding. A pair overlaps only when this
three-dimensional distance is strictly smaller than the truncated radius sum.
The first overlapping pair returns that sum minus the rounded horizontal
distance, with the vertical difference set to zero for that second distance.
When no pair overlaps, the query returns zero.

An actor-style static placement contributes one sphere at its stored placement
position, with the loaded model radius. The actor-to-placement query truncates
each sphere radius separately toward zero before adding them. Both queries
truncate center differences before the distance calculation and return
horizontal separation from the first overlapping pair.

### Weighted actor overlap response

For an ordinary actor, a nonzero `attr_weight` supplies mobility
`256 - attr_weight`; weight zero supplies mobility zero. Actor overlap
response excludes actors with nonzero attribute 26.

Grounded living states use the same weighted actor and static-placement overlap
response as ordinary standing, including weapon drawing and sheathing, states
20 and 21. They do not bypass overlap queries. The grounded state exception is
27 through 32: an actor-pair response skips the updating actor's current support
owner after the small-contact check. Other actor pairs still use weighted
separation, and this exception does not change static-placement response.

An actor-to-actor separation below three map units marks contact on both
actors and ends the overlap pass without displacement. For larger separation,
when both mobilities are zero the updating actor returns to its previous
position. Otherwise the divisor is the sum of mobilities when both are
nonzero, or the nonzero mobility plus 256 when one is zero. The integer factor
is `131072 / divisor`. Each movable actor's Q8 displacement distance is
`(factor * mobility * separation >> 9) + 255`, with wrapping signed 32-bit
multiplications and an arithmetic right shift. The actors move in opposite
horizontal directions. A zero-mobility actor is not displaced.

An actor-style static placement does not share this divisor. An updating
actor with zero mobility returns to its previous position. Otherwise the
factor is `131072 / mobility` and its Q8 displacement distance uses the same
multiply, shift and addition expression. Static-placement separation below
three still takes this response; the small-contact early return applies only
to actor pairs.

For a positive actor-pair overlap, collision-attribute bit 3 on the other
actor marks that actor's body-response bit `0x04` before displacement handling.
A separation below three marks body-response bit `0x04` on both actors and
stores each actor's low-word runtime identifier on the other. Weighted actor
separation marks the updating actor's body-response bit `0x04` and stores the
other actor's identifier. Both-immovable rollback does not add that mark to the
updating actor. Existing result bits are preserved.

Weighted static-placement separation marks body-response bit `0x04` on the
updating actor and stores contact identifier zero. Its immovable rollback does
not add a result mark. Separation translates only X and Z, preserving Y. The
updating actor moves away from the other origin; a movable other actor moves
in the opposite direction. Coordinate addition is stored at single precision
and then truncated toward zero, using the shared heading-translation rule.

Overlap marks share the body-response byte used by primary, residual, bounds
and forward response. They do not write the separate rollback-result byte.

An actor-pair overlap with a separation of three or more takes a landing branch
before any weighted separation when the updating actor's falling state is
nonzero, the other actor's attribute 98 (`attr_bounce_power`) is nonzero, and
the absolute difference between the actor's Q8 Y and its retained fall origin
is at least `SYSTEM.INI` `[game] fall_bounce_height` times 256 and the actor's
positive Y extent is greater than zero. The branch
attempts to land the actor on the other actor and ends the overlap pass with
body-response bit `0x80` set whether or not the landing is admitted; without
these conditions the weighted response applies. The landing is not admitted when
the actor is in living states 10 through 19, 27 or 28, or its surface is
deepwater or scapedeepwater, or the other actor's model link is empty, and then
changes nothing else. An admitted landing makes the other actor the
actor's support, clears the actor's sliding, falling and queued-motion bytes
and its two retained motion words, sets living state 27, requests animation
group 10 on the other actor, and requests the landing animation group on the
actor: 114 when the actor holds a weapon (attribute 110,
`attr_holding_weapon`, nonzero, or the drawn-weapon test), otherwise 66 when its
burning-torch byte is nonzero, else 65.

The overlap pass visits its retained actor-style candidate list in order. Later
queries read the positions left by earlier responses. If any weighted
separation occurred and the loop completes, the forward obstacle response
runs again. A small actor-pair contact returns immediately and skips this
repeat, even when an earlier candidate caused weighted separation.

### Ordinary forward segment response

The ordinary motion response probes a segment from the previous effective
origin to the current effective origin. Both endpoints are shifted from Q8
to integer map units and receive the same caller-supplied vertical offset.
It visits retained face-model candidates in order. The ordinary segment path
uses the enabled polygon-sidedness flag. The model-query argument selects
early return after subobject admission; it does not set polygon sidedness.
Queued motion 24 excludes candidate type `0x80`.

World contact points are converted to integers with nearest-even rounding.
World normals are multiplied by 256, stored at single precision, and converted
to integers with nearest-even rounding. Neither conversion uses the radius
truncation rule. The stored contact rank is unchanged.

The motion heading comes from the horizontal current-minus-previous effective
position, using wrapping integer differences and the shared quantized heading
lookup. A normal's facing heading is its horizontal displacement heading plus
1024, wrapped to 2048. Subtract the motion heading; differences below -1024 add
2047, and differences above 1024 subtract 2047. Only differences in the inclusive
range -511 through 511 accept the contact.

Accepted facing contacts contribute their integer world normals in query
order. An empty normal accumulator takes the next normal unchanged. Otherwise,
the two normals are added, normalized to Q16, and shifted right by eight to
produce the new accumulated normal. This ordered combination is not an
arithmetic average of all contacts.

The chosen contact point comes from the smallest `rank & 0x7fff` among accepted
contacts. Equal ranks retain the earlier point. Normals from later accepted
contacts still contribute even when their point does not replace it. A
response-disabled query reports contact without changing position. A
response-enabled query truncates the first selected sphere radius toward zero
and writes X and Z as `point * 256 + accumulated_normal * radius`, preserving Y.

### Forward obstacle wrapper and motion transition

The wrapper first runs a responding forward query at vertical offset zero.
A hit marks body-response bit `0x08`, then runs the motion transition unless the
enabled selected-player auto-grab path handles it. If queued motion or falling
is nonzero, it next runs a nonresponding query at
`positive_y - (post_collide_height >> 2)` and transitions on a hit. Finally it
runs a responding query at `positive_y - post_collide_height`; a hit marks
`0x08` and uses the same auto-grab exception before transitioning.

Living state 15 bypasses the motion transition. Living states 27 through 32,
or queued motion 24, first replace queued motion with 1, living state with 0,
and clear the retained integer motion vector. With queued motion zero, a
nonzero falling byte marks `0x20` and becomes `(falling & 0x80) | 1`.

Nonzero queued motion marks `0x10`, sets falling to `(queued & 0x80) | 1`, and
clears queued motion. It records fall-origin Y as the current logical Q8 Y plus
`256 * attribute 32`, and sets the byte fall timer to 30. The high falling bit
selects forced animation group 58. Otherwise, holding a weapon selects group
113 unless that group is already current; not holding selects group 56.
These are forced requests with modifier zero, not changes to the default group.
The animation-system and static-model gates still apply to the request.

The `0x08`, `0x10`, and `0x20` marks belong to the body-response flags,
together with overlap mark `0x04`, independently of rollback transition flags.

The wrapper's auto-grab switch is enabled only when `[cyrus] auto_grab` equals
1. It delegates to the selected-player grabbing path before the normal motion
transition; it is not a general obstacle rollback switch.

### State-fifteen bounds response

Living state 15 replaces each forward motion probe with two bounds segments
and returns no forward-motion hit. These segments use the retained actor matrix
and current effective origin shifted to integer map units. The local endpoints
are `(positive_x, positive_y + 2, positive_z)` to
`(negative_x, positive_y - 2, negative_z)`, and
`(negative_x, positive_y + 2, positive_z)` to
`(positive_x, positive_y - 2, negative_z)`. Both bound records contain the stored
nonnegative extent magnitudes; this query does not negate the negative extents.

The query disables polygon sidedness, visits face-model candidates in retained
order, and tests both segments for each candidate before visiting the next.
Queued motion 24 excludes candidate type `0x80`. Integer world contacts qualify
only when their normal Y component is between -41 and 160 inclusive.

Each segment tracks its own greatest negative rank and smallest positive rank,
initialized to -999999 and 999999. Rank zero does not contribute. A qualifying
contact contributes its full integer normal only when it strictly improves one
of those bounds. Earlier contributions remain in the sum when later contacts
improve a bound. The sum is not normalized. A nonzero sum adds directly to all
three logical Q8 position components and marks body-response bit `0x40`.
The polygon-sidedness flag is restored to enabled after the query.

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

Queued motion 24 excludes candidate type 128 before the primary support model
query. The selected-player fallback applies the same exclusion.

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

The model body-query adapter rejects working shapes with more than 24 spheres.
During a model query, a subobject face scan stops when the successful-contact
count reaches 32. Reaching that count is not a query failure: the returned
contacts still pass through the world-coordinate integer adapter and the primary
collector's separate sixteen-contact limit.

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

### Ordinary candidate lifetime

An admitted ordinary actor prepares its collision candidate lists before the
main script wrapper. Later support, primary body and residual face queries reuse
those lists. Movement and script changes do not rerun spatial admission between
these stages. Query origins still follow the owning stage's current actor
position; retained candidate membership is separate from that position.

### Primary body-contact collection

The primary body query retains contacts in candidate order and each model's
returned contact order. Collection returns immediately after its sixteenth model
contact. Later candidates and the optional terrain contact are then omitted.
When fewer than sixteen model contacts were collected, an available terrain
contact is appended after them. This limit belongs to the primary body query;
the residual separation query is separate.

The ordinary actor update clears the shared extra-contact append flag before
animation-default and movement processing. The primary collector appends the
retained normal only when that flag is set. Heightmap presence alone does not
admit this extra contact. Ordinary terrain height/support queries are separate
from this retained body-contact normal.

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
The ordinary update runs this response only for living states 0, 15 and 27
through 32; other living states skip primary body response. The response uses the displacement from the
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

### Body-contact motion states

Before the dot product, each contact that passes facing admission first checks
the enabled auto-grab path. When the `[cyrus] auto_grab` switch equals 1 and the
selected-player grabbing path handles the contact, the whole primary body
response ends at once: later contacts, residual separation and the rest of the
response are skipped. Otherwise the contact runs the
[motion transition](#forward-obstacle-wrapper-and-motion-transition) and then
the dot-product test. The transition runs once per admitted facing contact,
whether that contact then projects or rolls back:

- A falling actor keeps falling as `(falling & 0x80) | 1` and receives the
  `0x20` mark.
- Queued motion becomes falling, with the `0x10` mark, fall origin, timer 30
  and forced animation group of that transition.
- Living states 27 through 32, and queued motion 24, first become queued motion
  1 with living state 0, then start falling the same way.
- Living state 15 is unchanged.

The response never changes Y, the fall origin or the fall timer itself. The
heading-adjustment branch tests the motion bytes after this transition.

While the sliding byte is nonzero, contacts whose candidate type is 64 are
skipped before classification and facing admission. A sliding actor still
projects a contact above `slide_range`, but a contact at or below it leaves the
position unchanged and sets no body-result bit.

The response counts the retained contacts minus the type-64 contacts it skipped
while sliding. After the response stage, the ordinary update clears a nonzero
sliding byte when that count is nonzero. The update performs this test even
when the response did not run for this actor; the count then remains from the
most recent primary body response.

### Grounded wall-facing adjustment

After a projected wall response, an actor adjusts its heading only when its
queued, falling, living-state and sliding bytes are all zero and its current
X, Y or Z position differs from its previous position. The starting
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

The response stores body-result bits independently of collision attributes. A
projected wall contact sets bit 1 when the ordinary heading-adjustment branch
runs; otherwise it sets bit 2. Horizontal rollback also sets bit 2. Each mark
preserves the other result bits. Ordinary state 15 still projects or rolls back
horizontal movement, but does not enter the state-zero heading adjustment.
A falling actor, or one that the motion transition has just made falling,
also sets bit 2 for a projected contact and never adjusts its heading.

### Residual face-contact separation

After the primary body response and the later overlap stage, the actor queries
the prepared face-model list again, including admitted live models.
This residual query has no primary sixteen-contact cap and does not append a
terrain contact. It retains each model query's thirty-two-contact buffer limit.
Queued-motion state 24 excludes candidate type 128 from this scan.
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

The residual query uses the first collision working record, retaining its
ordinary sphere count. It rounds the first sphere's radius to an integer and
assigns a common radius to every working sphere. State 15 uses half that radius
through an arithmetic right shift. Other states retain the original radius when
neither falling nor queued motion is active. Queued motion or falling normally
uses `radius * 200 >> 8`; falling with movement direction 3 uses that reduction
even when its low seven state bits are zero. These working radii do not replace
the actor's retained initial shape. A nonempty residual result sets body-result
bit 4 while preserving the other bits.

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

A separate finer counter adds the same scale to an accumulator and shifts the
total right by five bits. It therefore advances eight units per nominal tick,
with five fractional bits carried between updates. Its update increment is the
difference of whole counter values.

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
A world change saves the departing world's temporary snapshot before the next
world loads. Entering a world whose snapshot exists restores it and then places
the player records at the destination marker; entering a world with no snapshot
starts from its authored placement. New Game deletes every snapshot.

## Player contact lists

The selected player's update clears two contact lists at its start. Its body
contact pass (run for motion states 0, 15 and 27 through 32) appends each object
whose collision model the body touches to both lists once: the first list accepts
an append while it holds at most sixteen entries and the second while it holds at
most thirty-two. The selected player's primary ground-support query also appends
to the first list each model that becomes its best support candidate, under the
same rule, and the last such model is recorded as the support owner. `PlayerCollide` is true when the selected player is supported by
the receiver or the receiver is in the first list; `PlayerCollideStatic` is true
when the receiver is in the second list. Hazard objects such as the swinging
swords and hooks of the Drinal dungeon poll `PlayerCollideStatic` and damage the
player on contact; the Mages Guild doors poll `PlayerCollide` to open.

