# Actor animation playback

Actor animation selects polygon-mesh frames through RAAN references and RAGR
command groups. Model frames contain vertex animation, not skeletal animation.

## Model and frame selection

A file `ShowFrame` command uses an unsigned twenty-bit reference into the
actor instance's frame table. Each RAAN entry contributes the indicated
number of model frames, starting at zero, in entry order. The loader resolves
the reference to a runtime model handle and frame index and patches those two
values into ten-bit command fields. Playback stores that pair as the actor's
current pose. It does not interpret the second value as an attachment vertex.

### Placed-object frame sources

RAAN is not the only source of an instance's frame table. After initializing
a type-1 [MPOB object](../formats/RGM.md#mpob-object-instances), the loader uses
its `model_name` when the instance has no frame table, its static-model flag
is clear, and the name is nonempty.

- An MPOB `is_static` value of 1 selects the static model path and leaves no
  animation frame table.
- Other values select the animated model path. The byte is the number of
  frame-table entries, each referring to that model and successive frame
  indices starting at zero. A zero value allocates an empty table.

The group loader installs the actor definition's RAGR groups only when the
instance has a frame table and its static-model flag is not set. It resolves
ShowFrame operands against that instance's table and rejects references outside
the table. RAGR commands can therefore exist in a definition without becoming
runtime animation groups for a particular placement. Instances sharing one
definition can obtain their frames from different placed models.

## Animation tasks

`PushAnimation(group, count)` requests the group if it is not yet current.
When the group becomes current, the task sets its byte loop countdown to count.
It remains active while that group is current and completes when the group
changes. Thus count is a finite loop input, not a speed or wall-clock duration.
The animation request made by this task has a speed modifier of zero.

The current group is read as a signed sixteen-bit value and compared with the
complete thirty-two-bit group argument. This comparison does not first narrow
the argument to sixteen bits. The loop count stores only the low eight bits of
its argument. Changing the count operand after the task enters its started
phase does not rewrite the countdown; that phase only compares the current
group with the reevaluated group operand.

The first invocation has a separate not-started phase. If the requested group
is not current, it makes an ordinary request and remains active. Even when
that request starts the group immediately, this invocation does not set the
loop count. A subsequent invocation that finds the group current sets the byte
count and enters the started phase, still returning active. Once started, it
completes only when the current group differs; it does not re-request the group.
An initial invocation that already finds its group current sets the count
immediately and remains active. Each invocation evaluates the call's operands
again. Task creation and later task invocations are separate scheduling stages.

The task does not test whether the requested group exists. Ordinary request
fallback still applies, but selecting the default does not change the task's
requested group. While that requested group is not current, the task remains
in its not-started phase and makes another ordinary request on each invocation.
A missing group therefore does not by itself complete the task.

For forward playback, `GoToPrevious` with a nonzero loop countdown decrements
that countdown. If it becomes zero, playback continues after the command;
otherwise it jumps to the command's target. With a zero countdown it jumps
without decrementing. `GoToFuture` continues past itself while a finite loop
countdown is nonzero. With no countdown, a pending transition or the animation's
exit flag can make it take its target instead.

The group header currently called `anim_id` supplies the animation's byte
frame countdown. An animation request adds its speed modifier to that value.
This is separate from the task's loop countdown.

### Frame waits

`WaitAnimFrame(group, frame)` inspects the receiver's animation state.
`WaitPlayerAnimFrame(group, frame)` applies the same test to the currently
selected player, independently of the task's receiver. Either task completes
when its inspected actor or animation association is absent.

If neither the current nor pending group matches `group`, the task completes.
If only the pending group matches, it keeps waiting. When the current group
matches, it completes once the unsigned byte `anim.real_frame` is greater than
or equal to `frame`; otherwise it keeps waiting. This counter is distinct from
the `anim.frame` command cursor and the selected model-file frame.

Selecting a new group resets `anim.real_frame` to zero. Ordinary frame
advancement increments it when the frame countdown expires, and each skipped
`ShowFrame` increments it again. The byte wraps from 255 to zero. A zero-tick
update or a countdown decrement that retains the same frame leaves it unchanged.

Command processing has a separate signed sixteen-bit cursor. It indexes the
packed command table and remains on the command displaying the current frame.
Selecting a new group resets it to zero. `BreakPoint` and `DelayCounter`
advance that cursor without incrementing `anim.real_frame`; a `GoToPrevious`
backward jump assigns its command target without resetting the counter.
Combat timing queries read this signed cursor, rather than `anim.real_frame`.

Current and pending group values are signed sixteen-bit values compared with
the full script integer argument. The frame threshold also remains a full
script integer; neither argument is narrowed before comparison. These tasks
read the current state on each invocation without requesting or advancing
animation, changing actor state, or calling another native handler.

## Ordinary requests and default groups

An actor keeps a current group, one pending group, and a default group. Ordinary
requests do not form a queue. When the current group is the default, requesting
the default again returns without restarting it. Requesting another group
clears the current group so that the new request can start immediately.

Otherwise, an ordinary request replaces the pending group when there is no
current group, no pending group, the pending group number is less than or equal
to the requested number, or the pending group is the current group. A lower
number cannot replace a higher pending number outside those cases.

An absent requested group falls back to the actor's default group if that group
exists. If neither exists, the pending request is cleared. The accepted group's
low header byte plus the request modifier becomes its byte countdown, with
byte wrapping. A request with no current group invokes animation advancement
immediately using the existing shared tick input; it does not invent a tick.

When neither a current nor pending group exists, advancement requests the
default group with modifier zero. `EndAnimation` clears the current group and
invokes advancement again, so a pending request or the default can start in
the same update. Treating End as a permanently stopped actor loses this behavior.

Forced requests, group-type interruption rules, actor eligibility, and the
number of advancement calls made by an actor update use additional rules.
Request arbitration does not activate placed actors automatically.

## Placed actor initialization and eligibility

New actor animation state initializes its default group to zero. For a placed
object with MPOB type 1 and an animation model table, group setup builds the RAGR table
and makes two ordinary requests for group zero with modifier zero. These requests
reuse the shared nominal ticks.

The actor loader chooses its model path from the RAHD record. A zero
`raan_count` means no model. Otherwise, the static-model shortcut applies when
`raan_count` is one, the complete signed `anim_control_prefix` field is −1,
and the signed `anim_frame_data` field is less than two. All other nonzero-model
cases build the animation model table. This classification precedes group
requests and the model-path message-group attribute assignment. Exported actor
metadata records the result as `model_load_path`: `none`, `static`, or
`animation_table`.

The MPOB activation byte selects storage and update-list membership. Zero stores
an ordinary placed actor in its spatial map cell; a nonzero value uses the
persistent actor list. The active update list combines nearby cell actors with
that persistent list. A zero activation byte therefore does not mean that an
actor can never animate.

Cell assignment uses the decoded MPOB X/Z coordinates directly, with 512
coordinate units per cell. The runtime position has eight fractional bits;
shifting that position by eight recovers the decoded file coordinate. The
three-byte MPOB coordinate itself is not divided by 256 again.

The cell window starts nine 512-unit cells before the supplied horizontal
center, clamped at the map edge. The main loop supplies the current view
coordinates, not the actor body position. It spans eighteen cells per axis, omitting its
four corners. The inner sixteen by sixteen cells include ordinary actors.
On the outer border, actors with a nonzero attribute byte 113 are excluded.
Debug and global object-processing switches can further restrict this list.
Membership alone does not make an actor eligible for an update;
its runtime exclusion, pause, state and script checks still apply.

Cells are visited in increasing X within increasing Z rows. Eligible actors
retain their current per-cell linked-list order; the persistent actor list is
appended after the cell actors. New cell actors are inserted at the front of
their cell list, and new persistent actors at the front of the persistent list.
Consequently, actor creation order is not ordinary update order.

For each view coordinate, the window origin is zero below 4608, 56319 when
the coordinate plus 4608 exceeds 65535, and otherwise the coordinate minus
4608. Dividing this origin by 512 gives the first cell. The regional update
compares that starting cell with its cached entry and rebuilds the lists when
they differ.

Moved actors enter a deduplicated deferred list in first-registration order.
The frame loop consumes this list before regional actor selection, clearing its
entries and count after the pass. Map-excluded script context suppresses
relocation; pooled persistent actors also retain their existing membership.
For a cell actor, relocation uses its linked position when present, otherwise
its local position. Each X/Z coordinate is shifted right eight bits and masked
to 16 bits before selecting its 512-unit cell.

An unchanged cell retains list order. A changed cell unlinks the actor from
its previous cell and prepends it to the destination cell. After a cell change,
the rounded horizontal distance from the camera to the actor's local position,
shifted right eight bits, invalidates the cached regional list when it exceeds
3584. Nearby relocation retains the cached traversal until another rebuild
condition applies. Removing a queued actor clears its pending entry.

Ordinary default selection can use attribute byte 44, but combat, injury,
attachment, movement and player-specific states can select other groups.
Changing the default can force an immediate replacement when the old default
was current and had not reached an exit. It is not safe to select attribute
byte 44 unconditionally for every actor.

The installed ISLAND map has three FLAG2 placements, all ordinary objects with
a zero activation byte and no static model override. Their FLAG2 attribute
record is all zero, including byte 44. These values define their initial state.

## Living actor default selection

For an actor that does not satisfy `IsDead`, ordinary default selection uses
these branches in order. The first matching branch wins.

| Condition | Default group |
|---|---|
| Selected player satisfies `IsInDeepWater` | 141 |
| Actor is attached to a rope | 79 |
| Actor state is 15 | 7 |
| `IsSheathingSword` is true | Attribute byte 44 |
| `IsDrawingSword` or `IsHoldingWeapon` is true | 8 |
| Actor's torch-burning counter is nonzero | 5 |
| Otherwise | Attribute byte 44 |

The selected-player deep-water branch also invokes weapon-state cleanup;
selecting group 141 alone does not perform that transition.

`IsSheathingSword` and `IsDrawingSword` require the item system to be loaded,
an actor with animation state, and ordinary rather than shell execution.
`IsSheathingSword` accepts actor state 21 or current group 129;
`IsDrawingSword` accepts actor state 20 or current group 91.

`IsHoldingWeapon` returns false for an absent actor. For a present actor,
nonzero attribute byte 110 returns true before the item-system, animation-state
and shell checks. Otherwise it requires those three gates, combat capability,
a set weapon-drawn flag, and a false `IsSheathingSword` result.

After selection, when animation state exists and execution is outside the
shell context, a changed default replaces the current group immediately if
the current group was the old default and its exit marker is clear. A pending
group equal to the old default is separately replaced by an ordinary request
for the new default. These checks use the old default captured before selection.

## Combat entry

`beginCombat` uses the lifecycle selector read by `IsDead`, independently of the
motion state used by weapon and living-default selection. On a nonplayer actor,
it first sets script-control bit `0x02`, including when the lifecycle selector
already equals 6. A selector already equal to 6 skips the remaining entry work.
Otherwise entry writes selector 6, handles weapon setup, sets combat flag `0x04`,
and requests the begin-combat taunt type 0, in that order.

Weapon setup checks attribute byte 23. When it is nonzero and animation group 91
exists, an actor whose weapon-drawn flag is clear runs weapon-draw admission and
then immediately requests group 91 with modifier zero. An already drawn weapon
skips both operations. When group 91 is absent, entry runs weapon cleanup, then
sets the weapon-drawn flag and clears motion state to zero. A zero attribute byte
23 directly sets that flag and clears motion state, without weapon cleanup.
These motion-state writes do not replace the lifecycle selector.

### Weapon draw admission

Weapon-draw admission requires the item system, an animation state, no shell
execution, no rope attachment, no motion state from 27 through 32, neither a
draw nor sheath transition, no held-weapon predicate, no falling or queued
motion, and motion state other than 15. For the selected player, attribute byte
105 equal to 1 also prevents admission.

After those gates, a nonzero held-item byte runs weapon cleanup. With a zero
held-item byte, absence of assigned weapon state also runs cleanup. Otherwise edge
motion states 10 through 19 and stored surfaces 2 or 4 prevent the draw. A
successful draw sets motion state 20. For the selected player it also sets
combat flag `0x04`. `DrawSword` runs this admission for its receiver; the repositioning step that
follows it in the routine is inactive in the shipped game, so a draw never moves
the actor. This admission does not itself set the weapon-drawn flag.

### Equipped-weapon transition

The equipped-weapon drawing pass requires the item system, an actor with
animation state, ordinary execution outside the shell, and an assigned weapon.
The actor's weapon attribute selects an item of weapon type. Without combat
state, this pass clears the drawn flag before processing motion state.

For motion state 20, the pass requests group 91 with modifier zero. While
that group is current, its wrapping frame-progress byte is compared with attribute
byte 35, `attr_draw_frame`. Before the threshold, the sheathed model is drawn.
At or beyond the threshold, the held model is drawn, the weapon-drawn flag is
set and motion state becomes zero. If group 91 is not current, the sheathed
model is drawn and the transition remains pending.

For motion state 21, current group 129 uses attribute byte 34,
`attr_sheath_frame`, as its frame-progress threshold. Before the threshold,
the held model is drawn. At or beyond it, the sheathed model is drawn, the
drawn flag clears and motion state becomes zero. A pending group 129 is
cancelled when that transition completes. If group 129 is not current, the
pass requests it with modifier zero and draws the held model. That invocation
does not test the sheath threshold again, even if the request immediately makes
group 129 current. Threshold processing resumes on a later drawing pass.

These transitions do not clear the companion motion byte. Outside these two
motion states, a set drawn flag selects the held model. A clear flag selects
the sheathed model, using the attached drawing path while group 129 remains
current.

### Weapon cleanup

Weapon cleanup restores the selected player's stored playback-default group,
with modifier zero, when its current group is 91 or 129. Other actors and other
current groups do not receive that request. Motion state 20 or 21 is reset to
zero together with its companion motion byte, independently of the drawn flag.
The companion byte is separate from motion direction.

When the weapon-drawn flag is set, cleanup clears it and clears combat flag
`0x04`. For the selected player it also sets the
[health-candle show timer](hud.md#visibility) to 25 ticks. If the drawn flag was
already clear, cleanup leaves the candle timer and combat flag unchanged.

## Dead actor default selection

`IsDead` reads a runtime state selector rather than the health attribute. It
succeeds when that selector is 8, or when the selected player's death override
is active. This selector is distinct from the actor state used by the living
default-selection branches.

When ordinary default selection reaches the dead-actor branch, an actor with
combat state takes its default group from that state's death-animation selection.
Without combat state, it selects group 123 when that group exists, otherwise
group 124. Group existence requires an animation state with a populated entry
for the requested group.

Animation-table setup clears the actor's combat-state association and creates
combat state when attribute byte 14 is nonzero. This state is separate from the
animation group's playback cursor and from the actor's attribute table. The combat
record's death-animation selection starts at zero; constructing the record does
not copy attribute byte 44 into that selection.
A newly allocated combat record receives the next combat index. Reinitializing
an existing record retains its index. The target starts absent, the action
selector starts at -1, and the cooldown starts at zero. Move bindings follow
[COMBAT.INI actor move selection](../config/combat-ini.md#actor-move-selection).


Ordinary cell and persistent actor allocation clears the actor record before
common initialization. The death selector and object-update exclusion therefore
start at zero, and the combat-state association starts absent. These are
allocation values, separate from later player-role setup and gameplay changes.

Both standalone player roles use the same cleared object allocator before
player-specific initialization. Their collision metadata therefore also starts
at zero.

A fresh ordinary actor's collision metadata byte starts at zero. When its initial
model uses the static load path, the loader copies the collision byte from the
matching named ROB segment when that segment exists. The MPOB static-model
fallback performs the same copy. Loading geometry and retaining this per-actor
collision metadata are separate operations; a standalone model resource does
not itself supply the ROB byte. Live static face candidates retain the actor's
stored byte as support metadata.

The nonplayer death-animation paths pair a forced request for group 122 with
stored death default 124, and a forced request for group 121 with stored death
default 123. The requested death animation and the subsequent ordinary default
are separate values; the default selector reads the stored combat-state value.
Storing the combat death default first releases the actor's held walk-node
reservations. Each node's ownership byte is cleared only when it matches the
actor identifier's low byte; both actor-side reservation indices are reset.

## Forced requests

A forced request for the current group, before its exit marker is set, clears
the pending request without restarting the cursor, countdown or finite loop
count. Otherwise it clears the current group, replaces the pending group, and
resets cursor, direction, delays, markers and finite loop state. A missing group
falls back to the existing default, using the same byte speed-modifier addition
as an ordinary request. With an accepted group it immediately advances using
the existing shared nominal ticks. Ordinary numeric pending priority does not
apply to a forced request.

These rules do not make an ineligible actor eligible. The request and advance
boundaries reject globally disabled animation, an absent animation state, and
actors using the static model shortcut.

## Ordinary actor update order

After the actor passes the update-list exclusions, its state-dependent default
group is selected. For an actor whose animation-update exclusion is clear,
the update then advances the current animation and makes an ordinary request
for the selected default. The actor's main script stage comes later in that
same update, after this advancement and the intervening movement handling.

Within the enabled main script stage, the engine processes script entry work,
then existing tasks. Task processing can leave the main script body eligible
to continue in that update. Actor flags and global script switches can suppress
these stages independently; nearby-cell membership does not bypass them.

Consequently, a script animation request can follow an animation advancement
in the same actor update. If the request clears the current default or starts
an actor with no current group, its immediate advancement reuses the shared
tick increment. A renderer that advances each actor exactly once before or
after all script execution loses this ordering.

## Ordinary collision stage

The ordinary collision branch follows main-script processing and the periodic
actor update. In its admitted nonplayer path, a nonzero collision attribute byte
1 runs the support query before the movement and ground response selected by
bit `0x04`. Body-contact collection and response follow that work. Bit `0x01`
enables those contacts only for living actor state zero, state 15, or states 27
through 32 inclusive.

Bit `0x04` dispatches states 10 through 19 to a selected-player-only response
handler. For a nonplayer actor that handler returns without changing actor
state. Outside that range, states 20 and 21 run
[weapon-draw admission](#weapon-draw-admission), including its cleanup branch.
They do not also run the ordinary motion and ground response. Other states
select that ordinary response. This dispatch does not
suppress the preceding support query or subsequent eligible body contacts.

Before support, ordinary nonplayer collision runs the forward obstacle wrapper,
except for living states 10 through 19, then the overlap stage when attribute 1
has bit `0x08`. Slope prevalidation precedes the primary support query. Ground
response selected by bit `0x04` precedes stable-position retention and eligible
body response. Conditional recovery and water rollback follow body response.
Before recovery the actor's weight attribute 6 is restored from reserved weight
attribute 115.

After collision handling, support-following runs before attribute-controlled
axis restoration. These stages can change the result of earlier script movement;
updating the animation or local transform alone does not perform them.

### Supporting actor motion

The ordinary nonplayer support-following stage uses the retained supporting actor.
A missing actor owner or a supporting actor with attribute 24 zero leaves the rider
unchanged. Static-placement ownership alone does not carry a rider.

For an unattached support the stage reads its current local position and
update-entry local position. The latter is separate from its stable grounded
position. A support whose linked-position record names an owner supplies the
record's cached position and its position before the latest refresh instead;
angles always come from the support's own local angles and angle history. It
first adds the support's position
delta to the rider's local position and adds the support's heading delta to the
rider's local heading, wrapping heading to 2048 units.

It then subtracts the support's current position from the translated rider. It
builds an identity matrix rotated in yaw, pitch, roll order using the negative
previous support angles, and applies the column transform to that offset. It
builds another matrix with the negative current support angles and applies the
row transform. Both transforms store input coordinates as single precision and
convert each result to an integer with nearest-even rounding. The final rider
position is the resulting offset plus the current support position. Pitch and
roll affect position but do not change the rider's pitch or roll.

An attached support uses current and previous effective positions and angles
(each family refresh stores the effective values it replaces as the previous ones);
this path does nothing during the first two family generations. Living states
10 through 19 follow the [ledge-support actor](player.md#selected-player-ledge-entry)
instead of the retained ground support, with the same attribute 24 and
attachment rules. The retained ledge point receives the support's translation
and the same rotation about the support, and the ledge heading receives the
support's heading delta, wrapped to 2048 units, in the same pass as the rider.
The selected player in those states is not subject to the destination
collision queries below. Otherwise, for the selected player with collision
attribute bit `0x01` set, the support's translation and heading delta are
applied first and the candidate models are gathered again at that translated
position. The rotated destination is then rejected when the player's body
spheres, placed there with the current angles, meet a candidate (candidate type
`0x80` is skipped while queued motion is 24), or when the segment from its
update-entry position to the destination meets a candidate. Polygon sidedness is
off for both tests. A rejected destination leaves the translated position and the
turned heading in place; an accepted one stores the rotated position. These
cases are distinct from the ordinary unattached nonplayer path.

## Ordinary actor eligibility

The ordinary actor update traverses the actor update list. It skips the actor
body for the unselected member of the two designated player actors, a matching
active script-name exclusion, a nonzero runtime object-update exclusion, or
script-control mask `0x08`. This suppresses the ordinary animation stage and
later main-script stage together. Update-list membership alone does not make
an actor eligible.

The runtime animation-update exclusion is separate. A nonzero value skips the
ordinary animation stage, including current-group advancement and its default
request, without itself skipping the later main-script stage. Direct animation
requests retain their own boundary checks. At each ordinary update, the engine
clears this exclusion on the selected player actor and sets it on the other
player actor.

`SYSTEM.INI`'s `[debug] disable_slaves` sets the object-update exclusion for
actors with `attr_slave` set and `attr_master` clear. `disable_master_slaves`
sets it for actors with both attributes set. These tests set the runtime
exclusion; turning off the option does not clear an already-set exclusion in
this update branch.

## Timing boundary

The normal world loop calculates the shared FPS-derived timing before sampling
gameplay input and building the view-centered actor lists. Ordinary actor
traversal follows map-list presentation and the rope and shell systems. It runs
before the scene-rendering call. Timing is calculated for the loop, not separately
for each actor in that traversal.

Animation consumes the shared whole nominal-tick increment. The normal timing
path accumulates eight-fractional-bit frame scales and takes the difference
between consecutive whole accumulated values. It can therefore supply zero,
one, or several ticks during an update. The first twelve updates after a timing
reset use one tick. See [player timing](player.md#fps-derived-frame-scale).

Selected blocking operations, including model resource loading, rebase the FPS
measurement baseline so that suspended time does not enter the next update.
The [player timing](player.md#fps-derived-frame-scale) section documents the
save, restore, reset, and startup rules.

## Shipped FLAG2 sequence

The shipped island FLAG2 program contains finite `PushAnimation` requests.
It requests groups/counts `(1, 25)`, `(10, 1)`, `(1, 3)`, `(10, 2)` and then
ends the invocation with target zero. A later eligible script stage can restart
that authored sequence. Replacing it with a single endless idle group loses
the group changes and task boundaries. See [SOUP End](SOUP.md#end-0x05).

## Forward frame countdown

For a newly selected group, the command cursor starts at zero and the byte
countdown starts at the group's low header byte plus the request's speed
modifier. With zero input ticks, neither cursor nor countdown advances, but
control commands are still interpreted until a frame is selected.

With nonzero input ticks and a zero countdown, the cursor advances one command,
the countdown reloads, and the remaining tick count minus one becomes a frame
skip count. The countdown then decrements once, with byte wrapping. With a
nonzero countdown, it only decrements once and the current frame remains
selected. Thus the countdown and multi-tick frame skipping are separate inputs.

Control commands do not consume that frame skip count. Each encountered
`ShowFrame` consumes one skip while a skip remains. The interpreter advances
past those frames and selects the next one when no skips remain. It does not
interpolate vertex positions between model frames. These rules describe a
forward current group; reverse playback, pending-group eligibility, delays,
and event dispatch have additional state.

## Reverse frame countdown

A reverse-playback byte equal to one changes the ordinary command step to minus
one. Frame-countdown expiration decrements the signed command cursor and the
wrapping frame-progress byte. Skipped `ShowFrame` commands decrement both again.
A nonzero countdown only decrements once per nonzero-tick update, as in forward
playback; zero ticks leave the countdown and cursor unchanged.

Before each reverse command is decoded, a cursor less than one ends the current
group. Command zero is therefore not displayed by reverse traversal. Playback
clears the current group, stops its remembered sound if present, and invokes
ordinary advancement again with the same tick input. With no pending group,
that advancement requests the default group.

Delay processing still precedes reverse advancement. An unexpired or indefinite
delay yields without decrementing the command cursor. An expired delay clears
and lets reverse advancement use the call's complete tick input.

## Reverse control commands

With no finite loop countdown, reverse `GoToPrevious` and `GoToFuture` decrement
the command cursor rather than taking their targets. A nonzero finite countdown
changes this behavior. `GoToPrevious` decrements it; reaching zero advances the
cursor by one, while a remaining count takes the authored target. `GoToFuture`
with a nonzero finite countdown advances the cursor by one without changing the
countdown.

Each reverse command iteration clears the exit marker, movement-suppression
marker, and retained translation and rotation deltas before testing the cursor
boundary. `BreakPoint` sets movement suppression and steps backward, so the next
iteration clears that marker again. Translation, rotation, group-change,
loop-marker and native commands step backward without applying their forward
operations. Loop-marker commands leave their retained loop counter unchanged.

Reverse `DelayCounter` stores the narrowed byte delay and decrements the cursor.
A nonzero delay yields immediately; zero continues command processing.
`EndAnimation` still ends the group and invokes ordinary advancement.
`PlaySound` dispatches the ordinary animation sound services and retains the
returned sound slot before decrementing the cursor. Opcode eleven still invokes
the actor-event service with the authored operand and decrements the cursor.

## Actor events

Opcode eleven dispatches its operand to the actor-event service at the command,
in forward and reverse playback alike, and then moves the cursor one step in the
playback direction. The service runs synchronously, in command order with sound
commands, and never selects a frame.

The operand is a bit set. The service tests its bits in this order against the
actor's stored [sound surface](sound.md#surfaceini-remap):

| Bit | Required surface | Effect descriptor |
|---|---|---|
| `0x01` | lava (5) | `0x2580` |
| `0x02` | water (1) | `0x2500` |
| `0x04` | water (1) | `0x2500` |
| `0x08` | sand (6) | `0x2700` |

A set bit whose surface does not match does nothing, and bits `0x02` and `0x04`
together request two effects. Each effect starts at the actor's current logical
Q8 position, displaced 2048 Q8 units toward the camera, with the vertical point
logical Y plus the actor's positive vertical extent times 256 plus 1536. This is
the point the landing effect uses, with the camera-relative displacement of the
[selected death effect admission](SOUP.md#selected-death-effect-admission) rules.
Bit `0x10` then adds one [screen-shake impulse](SOUP.md#frame-view-angles-and-screen-shake).
No other bit has an effect.

## Action words

Each actor has a pending action word and an accepted action word. `SetAction(mask)` accepts only
mask `1` or `2`. When neither word already contains the requested bit, the call checks that the
current group is in `0x41..0x4a` and that falling and queued-motion bytes are clear. A failed check
clears the two low action bits in both words. A successful request sets the requested low bit in
the pending word and starts the action-specific animation request; the pending timer is reset.

The per-tick action update adds the elapsed tick count to the pending timer. After 36 ticks it
moves the pending bits into the accepted word and clears the pending word. It clears the low bits
when no queued motion is active and the actor is outside groups `0x12` and `0x1e`. Accepted bit 1
requests the default action-8 queued motion; accepted bit 2 requests default action 9. Those
requests are admitted only when the actor is not already queued, falling or sliding and has living
state 0 or 15; the selected player also requires the normal non-water admission.

`ResetAction(mask)` clears the requested bits in both action words. `ResetAllAction` clears the
pending word and copies the cleared value to the accepted word. The words are actor state and are
independent of the selected player's controller pending-action flag.

## Transition command

Opcode fifteen carries a trigger mask, a start frame and a target group. Reverse
playback only moves the cursor one command back. Forward playback asks a
decision routine, keyed on the mask, whether to take the transition:

| Mask | Condition for taking the transition |
|---|---|
| 1 and 2 | Either of the two low bits of the actor's pending action word, the word `SetAction` fills, is set. Taking mask 1 clears bit 1 of the pending and accepted action words, and taking mask 2 clears bit 0 of both; the accepted word then gains the pending word's remaining bits. |
| 4 | The actor's falling byte (the motion byte at `+0x1b2`, which every actor has and the [ground turn](player.md#ground-turn-eligibility-and-animation) rules read) is nonzero. |
| 8 | The actor is the selected player and the forward movement binding (`key_up`, either of its two codes) is pressed. |
| any other | Never taken. |

A command that is not taken advances the cursor by one. A missing actor or
animation record is never taken either.

Taking mask 8 sets byte `+0x0a` of the animation record to one, then makes a group
request on the actor through the shared group-force routine: group 31 for a selected
player holding a weapon, otherwise group 30 with a burning torch or group 18 without
one. Each path sets bits `0x30` of the actor's combat-state byte `+0x1cc` and runs one
further update routine for the actor before the decision reports success. Masks 1 and 2
report a distinct success value from mask 8 and mask 4.

The decision result selects the command's outcome. Not taken advances the cursor by one.
A mask 8 transition leaves the cursor on the command and ends the update with the group
request already made. A mask 1 or 2 transition sets the cursor to the command's second
operand less two (the start frame field) and, like a taken `GoToFuture`, sets the exit marker and clears
the two six-byte delta records. A mask 4 transition does the same with the third operand,
as described next.

Taking mask 4 jumps the cursor to the command's third operand (`target_group`, a
command index below 128) and continues interpreting there, exactly as a taken
`GoToFuture` does: it sets the exit marker, clears the movement-suppression
marker and clears the two six-byte delta records. The decision for mask 4 has no
other effect.

## Forward type-one transitions

A forward type-one group uses the ordinary frame countdown and control commands.
With no finite loop countdown, a pending group different from the current group
makes `GoToFuture` take its exit target. The jump sets the exit marker and clears
the movement-suppression marker before interpreting the target commands.

A pending default does not immediately replace a type-one group, even after the
exit marker is set. Playback continues through the exit tail until
`EndAnimation`, which selects the pending default in the same advancement with
the same nominal ticks. With a pending non-default group, an already-set exit
marker instead permits replacement at the beginning of a subsequent advance,
before its ordinary countdown update. The pending group number may equal the
current number. The ordinary request priority rules still determine which
request becomes pending.

Delay processing precedes that replacement test. An unexpired finite delay or
an indefinite delay yields before replacement eligibility is checked. When a
finite delay expires, replacement receives the call's complete tick input.
Selecting the replacement resets its cursor, countdown, loop state and markers.

Zero-tick updates retain the currently selected frame. The walk group's
PlaySound command in the exit tail does not end the group or consume a frame
skip. See [animation sound requests](sound.md#animation-playsound).

## Forward type-two groups

A forward type-two group (ledge-hang loops, and the player's and pirates' idle and
movement groups) uses the same exit-marker replacement rule as type one: once the
exit marker is set, a pending group that exists and differs from the actor's default
group replaces the current group at the start of the next advancement.

`GoToFuture` differs. With no finite loop countdown and either the exit marker set or a
pending group differing from the current one, a type-two group whose pending group is
the actor's default group does not take the exit target: the cursor advances by one
command, the exit marker is not set, and playback continues past the jump. Any other
pending group takes the exit target as in type one.

## Forward animation delays

`DelayCounter`, opcode twelve, narrows its operand to a byte, advances the
command cursor, and yields without selecting a new model frame when that byte
is nonzero. Zero continues interpreting commands in the same invocation.
The previous displayed pose remains selected during the wait.

At the start of subsequent advancements, byte value 255 yields indefinitely.
For another nonzero value, the engine subtracts the shared whole nominal ticks
with wrapping 32-bit arithmetic and tests the signed result. A positive result
is narrowed back to the delay byte and yields. Otherwise the delay clears and
ordinary advancement continues in that same call, using the original tick
input in full. The delay does not consume a portion of the frame-skip input.
A zero-tick advancement therefore preserves a finite delay.

Waiting neither ends the current group nor completes a task waiting for a group
change. Selecting a replacement group clears its delay state. The cursor has
already moved past the delay command; the usual countdown and frame-skip rules
still apply when the delay expires. `ConditionalDelay` and `LoopControl` use
separate state from `DelayCounter`.

## Breakpoints and walking translation

`BreakPoint`, opcode five, sets a marker in the current animation state and
continues interpreting commands. It does not pause the game or the animation.
Ordinary forward walking checks this marker after advancing its startup phase;
a set marker suppresses that update's translation.
Queued and falling vertical writers read the same marker after their preceding
animation request. A set marker suppresses their vertical arithmetic and timer
updates. Static models and absent animation state do not apply this suppression.

For forward playback, advancing the command cursor when its frame countdown
expires clears the marker. Skipping a `ShowFrame`, taking a `GoToPrevious`
backward jump, or taking a `GoToFuture` exit jump also clears it. A zero-tick
update or a countdown decrement that keeps the same cursor does not clear it.
A later `BreakPoint` can set it again before the selected frame is returned.
Selecting a new group resets the marker before interpreting its commands.

### Ordinary collision rollback snapshot

After ordinary animation advancement, the update saves the current animation
receiver and the two selected model-frame words, together with the directional
movement flags. Late collision rollback restores those saved values. The receiver
snapshot includes the current group, pending request, command cursor, countdowns,
loop state, exit and motion markers, and remembered animation sample slot.
The separately stored default animation selection is not restored.

Restoring a remembered sample slot does not stop channels started after the
snapshot or restart a channel that has already stopped. A later animation stop
uses the restored reusable slot, including a replacement sound occupying it.

## Animation group classes

The engine keeps a table of one class value per animation group number (0 through 177), consulted
by combat and movement code to decide whether a group is a fighting group or a reaction group.
Groups not listed have class zero.

| Class | Meaning | Groups |
|---|---|---|
| 1 | Fighting group | 8, 31, 91 through 120, 129 through 133, 135 through 140, 175, 176, 177 |
| 2 | Hit-reaction or death group | 74, 121 through 128, 134 |
