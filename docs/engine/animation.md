# Actor animation playback

Actor animation selects polygon-mesh frames through RAAN references and RAGR
command groups. Model frames contain vertex animation, not skeletal animation.

## Model and frame selection

A file `ShowFrame` command uses an unsigned twenty-bit reference into the
actor's concatenated RAAN table. Each RAAN entry contributes the indicated
number of model frames, starting at zero, in entry order. The loader resolves
the reference to a runtime model handle and frame index and patches those two
values into ten-bit command fields. Playback stores that pair as the actor's
current pose. It does not interpret the second value as an attachment vertex.

## Animation tasks

`PushAnimation(group, count)` requests the group if it is not yet current.
When the group becomes current, the task sets its byte loop countdown to count.
It remains active while that group is current and completes when the group
changes. Thus count is a finite loop input, not a speed or wall-clock duration.
The animation request made by this task has a speed modifier of zero.

The first invocation has a separate not-started phase. If the requested group
is not current, it makes an ordinary request and remains active. Even when
that request starts the group immediately, this invocation does not set the
loop count. A subsequent invocation that finds the group current sets the byte
count and enters the started phase, still returning active. Once started, it
completes only when the current group differs; it does not re-request the group.
An initial invocation that already finds its group current sets the count
immediately and remains active. Each invocation evaluates the call's operands
again. Task creation and later task invocations are separate scheduling stages.

For forward playback, `GoToPrevious` with a nonzero loop countdown decrements
that countdown. If it becomes zero, playback continues after the command;
otherwise it jumps to the command's target. With a zero countdown it jumps
without decrementing. `GoToFuture` continues past itself while a finite loop
countdown is nonzero. With no countdown, a pending transition or the animation's
exit flag can make it take its target instead.

The group header currently called `anim_id` supplies the animation's byte
frame countdown. An animation request adds its speed modifier to that value.
This is separate from the task's loop countdown.

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
ordinary object with an animation model table, group setup builds the RAGR table
and makes two ordinary requests for group zero with modifier zero. These requests
reuse the shared nominal ticks.

The MPOB activation byte selects storage and update-list membership. Zero stores
an ordinary placed actor in its spatial map cell; a nonzero value uses the
persistent actor list. The active update list combines nearby cell actors with
that persistent list. A zero activation byte therefore does not mean that an
actor can never animate.

The cell window starts nine 512-unit cells before the supplied horizontal
center, clamped at the map edge. The main loop supplies the current view
coordinates, not the actor body position. It spans eighteen cells per axis, omitting its
four corners. The inner sixteen by sixteen cells include ordinary actors.
On the outer border, actors with a nonzero attribute byte 113 are excluded.
Debug and global object-processing switches can further restrict this list.
Membership alone does not make an actor eligible for an update;
its runtime exclusion, pause, state and script checks still apply.

Ordinary default selection can use attribute byte 44, but combat, injury,
attachment, movement and player-specific states can select other groups.
Changing the default can force an immediate replacement when the old default
was current and had not reached an exit. It is not safe to select attribute
byte 44 unconditionally for every actor.

The installed ISLAND map has three FLAG2 placements, all ordinary objects with
a zero activation byte and no static model override. Their FLAG2 attribute
record is all zero, including byte 44. These values define their initial state.

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

The type-two `GoToFuture` exception for a pending default uses separate rules.

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

For forward playback, advancing the command cursor when its frame countdown
expires clears the marker. Skipping a `ShowFrame`, taking a `GoToPrevious`
backward jump, or taking a `GoToFuture` exit jump also clears it. A zero-tick
update or a countdown decrement that keeps the same cursor does not clear it.
A later `BreakPoint` can set it again before the selected frame is returned.
Selecting a new group resets the marker before interpreting its commands.
