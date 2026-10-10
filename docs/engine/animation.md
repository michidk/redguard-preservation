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

For forward playback, `GoToPrevious` with a nonzero loop countdown decrements
that countdown. If it becomes zero, playback continues after the command;
otherwise it jumps to the command's target. With a zero countdown it jumps
without decrementing. `GoToFuture` continues past itself while a finite loop
countdown is nonzero. With no countdown, a pending transition or the animation's
exit flag can make it take its target instead.

The group header currently called `anim_id` supplies the animation's byte
frame countdown. An animation request adds its speed modifier to that value.
This is separate from the task's loop countdown.

## Timing boundary

Animation consumes the shared whole nominal-tick increment. The normal timing
path accumulates eight-fractional-bit frame scales and takes the difference
between consecutive whole accumulated values. It can therefore supply zero,
one, or several ticks during an update. The first twelve updates after a timing
reset use one tick. See [player timing](player.md#fps-derived-frame-scale).

A fixed one-frame-per-render update or a hardcoded twelve-Hertz playback timer
does not reproduce this boundary. Complete host-clock selection, actor
eligibility, interruption, unsupported commands, and script scheduling must be
established before automatic world playback is considered complete.

## Evidence and limits

The model/frame mapping is established by the original RAAN table construction,
RAGR load-time patching, and playback storage. Task and loop semantics are
established by the original task handler and animation interpreter. These are
engine facts; they do not establish a matched visible-playback comparison.

The shipped island FLAG2 program contains finite `PushAnimation` requests.
Replacing it with an endless automatic idle loop changes the authored behavior.

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
