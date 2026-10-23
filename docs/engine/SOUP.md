# SOUP Scripting

Bytecode scripting engine used by the Redguard runtime for actor behavior, dialogue, puzzle logic, and scene control.

<a id="function-table"></a>

## Function overview

`SOUP386.DEF` declares callable functions in installed order. Index 0 is the
synthetic `NullFunction`; authored entries start at index 1. See
[SOUP386.DEF](SOUPDEF.md) for function IDs and parameter declarations, and
[function dispatch](#function-dispatch) for argument evaluation and task lifetime.

| Functions | Behavior |
|---|---|
| `TaskStop`, `TaskPause`, `TaskResume` | [Control task slots](#actor-task-controls-and-return-state) |
| `Wait`, `WaitOnTasks`, `WaitNonMulti` | [Wait for time or other tasks](#wait-waitontasks-and-waitnonmulti) |
| `Anchor`, `Yield`, `ScriptRV` | [Control continuation and return state](#anchor-yield-and-scriptrv) |
| `PushAnimation`, `PushControlAnimation`, `TaskStop` | [Animation ownership and stopping](#animation-task-ownership-and-stopping) |
| `GetAttribute`, `SetAttribute`, `GetMyAttr`, `SetMyAttr` | [Read and write actor attributes](#attribute-reads-and-writes) |
| `SetAttribute`, `SetMyAttr` | [Default-group side effects](#attribute-default-group-side-effects) |
| `PlayerDistance`, `PlayerFacing` | [Query distance and relative facing](#player-distance-and-facing) |
| `AtNodeMarker` | [Query distance to a node marker](#node-marker-distance) |
| `Wander` | [Choose destinations and limit movement](#wandering-without-walk-node-maps) |
| `MoveToLocation` | [Move toward an authored location](#authored-location-movement) |
| `RotateByAxis`, `sRotateByAxis` | [Relative rotation](#relative-axis-rotation), [elapsed rotation](#elapsed-relative-axis-rotation) |
| `RotateToAxis`, `sRotateToAxis` | [Absolute rotation](#absolute-axis-rotation), [elapsed rotation](#elapsed-axis-rotation) |
| `MoveObjectAxis` | [Move along the actor's orientation](#actor-local-axis-movement) |
| `MoveByAxis`, `sMoveByAxis`, `MoveAlongAxis` | [Relative movement](#relative-axis-movement) |
| `Rnd`, `QuickRnd` | [Draw from the shared random stream](#random-functions) |
| `HideMe`, `ShowMe` | [Change model visibility](#object-visibility) |
| `PlayerStand`, `Fighting` | [Standing query](#playerstand-query), [combat query](#fighting-query) |
| `PlayAnimation`, `PushAnimation`, `WaitAnimFrame` | [Animation playback](animation.md) |
| `Sound`, `AmbientSound`, `FlatSound`, `RTX`, `RTXp` | [Sound and dialogue](sound.md#soup-functions) |
| Weapon and attachment functions | [Item attachment](attachment.md#soup-script-interface) |

Shared actor setup, navigation, family transforms, and model admission are
specified alongside the functions that use them below.

## Script Data Sources

| Source | Container | Notes |
|---|---|---|
| Map script bytecode | `maps/*.RGM` (`RASC` section) | Main per-actor compiled script payload. Offsets and lengths are stored per actor in `RAHD` (`script_data_offset`, `script_length`, `script_pc`). |
| Standalone AI script files | `soup386/*.AI` (for example `CAMERA.AI`, `sword.ai`, `projtle.ai`) | Loaded separately from map RGM files. Uses the same SOUP bytecode model as map scripts. No `.AI` files are included in the shipped game; the `soup386/` directory contains only `SOUP386.DEF` in both the GOG and original CD releases. |
| VM definition table | `SOUP386/SOUP386.DEF` | Text definition file loaded at runtime. Defines function/task names, flags, equates, and attributes used by script execution. See [SOUP386.DEF](SOUPDEF.md). |
| EXE-embedded interpreter | Runtime binary | The executable contains the SOUP VM interpreter and native function handlers; script payload bytes are file-sourced (`RGM` sections and `.AI` files). |

## What Is In the Runtime vs Data Files

- Runtime contains the SOUP VM/interpreter and native handlers for script-callable operations.
- Script payload data is loaded from map/script assets (`RGM` and `.AI`) and executed by the VM.
- Function names and metadata come from `SOUP386.DEF`, while actor-local script bytes come from `RASC`/`.AI` content.

## VM Architecture

The SOUP386 VM is a register-free, program-counter-driven bytecode interpreter. There is no operand stack — values flow through function return values and are consumed directly by the calling instruction. Local variables (`RAVA` section, per-actor `int[]`) and global flags (369 entries shared across all scripts) provide persistent state.

### Threading Model

An actor retains a main continuation and an optional entry continuation in the
same bytecode buffer:

| Continuation | Start Address | Purpose |
|---|---|---|
| Main | `RAHD.script_pc` | Main execution (dialogue, activation, behavior) |
| Entry | Offset `0x00` | Separate entry work, present only when `script_pc != 0` |

The main and entry invocations share bytecode and local variables. Entry processing temporarily replaces the active cursor, then restores the main continuation. The actor has one subroutine return continuation, shared by these invocations, rather than a stack of return addresses.

Execution is cooperative. The original runtime consumes non-blocking instructions
until a blocking task or an explicit yield returns control. Existing tasks are
revisited before the main script body can continue.

`End` (opcode `0x05`) stores its authored target and ends the current invocation.
It does not permanently terminate the actor's main script. An eligible later
invocation resumes at the stored target. `Endint` (opcode `0x13`) resets the active
cursor to the script base and returns from the current invocation.

### World shell-script preparation

After loading map objects, world loading prepares the available `DEBUG`,
`COMMON`, and map-specific shell scripts in that order. The debug-shell
suppression switch skips `DEBUG`; disabling shell loading prevents shell-record
creation. This preparation precedes the separate CYRUS, GREMLIN, and CAMERA
initialization.

Each shell record takes its script continuations and string-table reference from
its map definition and receives its own copy of the initial local variables.
When the initial script cursor is absent or equals the script base, no separate
entry cursor is stored. New shell records are prepended to the shell list.

The map-specific shell script is the script named after the world's map file
(`START` for `START.RGM`); a name with no installed script creates no record.
`SYSTEM.INI` `[debug] final_version` set to a nonzero value skips `DEBUG`.

### Shell script execution

Every world-loop iteration, before the ordinary actor update, each shell record
runs in list order, so the newest record runs first. A record runs as an actor
without a map object: its attributes and control state are zeroed for the run,
while its locals, continuation and task slots persist between runs. Execution is
in shell context, in which the weapon, item and activation predicates report no
ordinary animation state. `KillMe` marks the record finished and the engine
frees it after that run; a script that never calls it runs every iteration for
the rest of the world.

`OpenShell(name)` and item use create a record from the installed script of that
name, copying its initial local variables; names compare without case. A world
without the script reports an error. A record created during an iteration first
runs in the next one.

### Ordinary update gates

`SYSTEM.INI`'s `[system] task_system` enables the ordinary entry, task, and main
script stages when its numeric value is one. The per-actor RAHK native hook is a separate call record, not the
entry continuation. With the task system enabled, the hook is dispatched only
when no unpaused task slot exists. With the task system disabled, task-slot
occupancy does not suppress that hook. Its own suppression flag still applies
in both paths. Actor exclusions and script-name filters apply before these stages.

The actor's runtime script-control byte has independent effects:

| Mask | Effect in the ordinary actor update |
|---|---|
| `0x01` | Suppress the ordinary main stage, including its entry and task work |
| `0x02` | Suppress the RAHK native hook |
| `0x04` | Use entry and main-body processing without the existing-task pass |
| `0x08` | Exclude the actor from the ordinary actor-update body |

The `0x04` path is distinct from the usual entry, existing-task, then main-body
order. It retains the entry continuation's blocking-operand gate, but bypasses
both the existing-task pass and its blocking-task gate before the main body.
An existing unpaused blocking slot therefore does not by itself suppress the
main body on this path. These runtime flags are not RAHD attribute indices. See
[actor update order](animation.md#ordinary-actor-update-order).

`DisableScript` sets mask `0x01`; `EnableScript` clears only that mask. Both
return zero and preserve the other bits. They do not terminate the native call's
current interpreter invocation or clear existing tasks. The wrapper checks the
main-stage suppression flag after the native hook and combat processing, so a
hook can change eligibility for the same update. Once the wrapper enters either
main-stage path, it does not recheck those flags between entry work and the main
body.

The native hook precedes the ordinary entry/task/main sequence. When the task
system is enabled and no unpaused task exists, combat processing follows the
hook for actors with combat state outside shell execution. This includes the
selected player; the wrapper does not restrict combat processing to map actors
or exclude the selected actor. Combat maintenance applies its own player and
nonplayer branches. The entry continuation
is processed later inside the enabled main stage. Mask `0x01` does not suppress
the separate native hook.

### PlayerStand query

`PlayerStand(receiver)` compares the receiver identity with the selected player's
retained supporting-actor reference. It returns one for equality and zero
otherwise. The query performs no collision search and does not update either
actor or the retained reference.

### PlayerArc query

`PlayerArc(offset, tolerance)` tests the selected player's horizontal direction
relative to the receiver's heading. Angles use 2048 units per turn. The query uses
the receiver's attached position when attached, otherwise its linked query position
when present, otherwise its local position. It reads the selected player's local
position. Vertical separation does not affect the result.

The receiver heading comes from its attached pose when attached and its local pose
otherwise. A linked query position alone does not replace that heading. The query
forms the horizontal vector from the player to the receiver with wrapping signed
32-bit subtraction, computes its quantized bearing, and negates that bearing modulo
2048. It subtracts `offset` from the receiver heading modulo 2048, then compares the
absolute shortest wrapped angular difference with the signed `tolerance`.
Equality succeeds; negative tolerance fails.

The query returns one for success and zero otherwise. It performs no visibility
or distance test and does not update either actor.

### Fighting query

`isFighting` searches the retained ordinary actor traversal for another actor
whose combat target is the receiver. A candidate must have a combat record,
its retained attacker-selection state must equal exactly one, and `IsDead`
must return false. Combat construction and reinitialization clear that selection
state to zero. This state is distinct from the combat action, maintenance phase
and weapon-drawn flag.

The search chooses the first candidate with the smallest rounded three-dimensional
distance. Its initial distance bound is 65535 in Q8 coordinates, and replacement
requires a strictly smaller distance. Candidates at that bound are excluded;
equal distances preserve the earlier actor in traversal order. Position lookup
uses each actor's effective query position, including its normal attachment or
linked-position handling.

When a candidate is selected, `isFighting` returns one only when the rounded
horizontal distance to that candidate is strictly less than `[misc]
in_combat_threshold` from [COMBAT.INI](../config/combat-ini.md), shifted left eight
bits with 32-bit wrapping. Otherwise it returns zero. The horizontal comparison
uses the candidate chosen by three-dimensional distance; it does not perform a
second nearest-candidate search. Both distance calculations use the shared
mixed-precision integer-distance calculation.

### Combat maintenance

A newly constructed combat record has no target, an action selector of -1 and a
zero cooldown. Combat construction clears the record before assigning its
attribute-derived counters and attack metadata. First allocation assigns the
current combat-record allocation counter as its retained allocation index, then
increments the counter with 32-bit wrapping. Reinitializing an existing record
preserves its allocation index across the clear. Opening a world opens a new
object-system session, whose combat-record allocation counter starts at zero.
Closing that session releases its object pool.

Fresh map loading initializes type-one placement models in MPOB file order.
Cell-owned and separately allocated placements share this traversal; spatial
cell order does not reorder their model construction. These map models initialize
before the two standalone player models. The initially selected player model is
constructed before the alternate player model.

Combat construction copies actor attribute 2 into the record's strength value and attribute 4 into its
armor value (the values `Strength`, `Armor`, `setStrength` and `setArmor` use). It stores five
counters as 129 minus attribute 58, 62, 63, 87 and 88 respectively, and four more as copies of attributes
59, 64, 65 and 66. The nonplayer decision stage uses the counters from attributes 58, 62 and 63 as divisors
of its tick-based pacing tests. The action, pending-move and defence selectors start at -1, the aggression
budget at 50, the attack state at 0, and combat flag mask 0x08 is cleared.

Combat construction copies RAEX `v_vertex` as a signed model-vertex index and
converts signed `v_size` to runtime Q8 contact radius by shifting left eight bits.
An absent actor-extra record supplies zero for both values.

Combat construction derives three spacing ranges from the nine attack slots.
Every populated slot contributes to the divisor. Only slots whose first motion
has kind 1 contribute range values. The near range is the smallest attack minimum.
The far range starts at zero; when a qualifying first motion's maximum exceeds
the retained far range, the attack maximum replaces that range. The comparison
uses the motion maximum, while the assignment uses the attack maximum.
For each qualifying slot, the constructor adds the attack minimum plus half of
the difference between its maximum and minimum to a signed midpoint sum. The
subtraction and additions wrap at 32 bits; division truncates toward zero.
The third spacing range is that sum divided by the number of populated slots,
including slots whose first motion has another kind. If no slot qualifies,
the near, far and third ranges are 10000, 30000 and 20000 respectively.

Fight spacing runs only when the action selector is -1. With the combat-stance
predicate set, it uses the constructor's spacing ranges directly. Both stance
paths turn toward the target after preparing their ranges and period, and continue
only when the absolute returned angle is strictly below 128. Below the near range it tries backward, right and left
movement admission in that order, taking the first result equal to 1.
Above three times the far range it requests forward walking only if animation
group 31 exists. Otherwise, above the third range plus half the difference
between the far and third ranges, it requires group 92 and queries forward
movement for one update. It requests forward movement only if the target distance minus that amount is
at least the third range.
All these movement requests use duration 1.

The facing wrapper uses both actors' local X/Z positions. It first runs the
[facing-arrival test](#alignment-distance-thresholds), using the speed selected
by the receiver's current animation group. Arrival copies the target's local
X/Z to the receiver, leaves Y and heading unchanged, registers the receiver
for a deferred map update and returns zero. Otherwise, the [quantized actor heading](player.md#direction-recovered-from-displacement)
of the local target displacement supplies the desired heading for the
[ordinary turn helper](player.md#ordinary-slope-response). Positive X selects
1536, negative X selects 512, positive Z selects zero and negative Z selects 1024.
The lower-level bearing result has the opposite sign; that result is negated
once to obtain the actor heading. Its result is zero
on a snap and otherwise the signed difference from before the turn step;
spacing applies its strict angle gate to that result.

The disengaged decision branch aligns toward the retained target point rather
than reading the target's current position. If alignment returns zero, it tries
a separate three-dimensional point-arrival test. That test shifts each target
and local coordinate right by eight before subtraction. Its Y difference is the
target Y minus the sum of local Y and the actor's positive Y bounds extent, all
in integer map units. It uses the same current-group movement attribute as facing,
but admits only when rounded distance is strictly less than `(speed * 384) >> 8`;
frame scale does not multiply this threshold. Missing animation or attributes
supply speed zero. Admission copies all three target coordinates after shifting
right and then left by eight, preserving heading, and refreshes spatial/model
registration. Rejection leaves position unchanged. This helper snaps on arrival;
it does not move along a path. If neither alignment nor point arrival handles the
update, the disengaged branch requests `MoveForward(1)` when the shared target
distance is below 500001, and `WalkForward(1)` otherwise.

When the combat-stance predicate is zero, spacing replaces the near range with
30000 and triples the far range. It consumes two inclusive 0..32767 random draws.
The third range becomes three times its original value plus the allocation index
times the first draw scaled by 100 and divided by 32767. The spacing period
becomes twice its original value plus the allocation index times the second draw
scaled by 10 and divided by 32767. Each random scaling division precedes the
allocation-index multiplication. Range, period and index arithmetic wrap at 32 bits.

When execution reaches the remaining spacing branch, it first runs the
multiple-attacker spacing helper. If that helper requests no sidestep, it adds
the receiver's allocation index to the shared tick counter with signed 32-bit
wrapping. Only a signed remainder of exactly one after division by the effective
spacing period requests a random movement. The division truncates toward zero;
a zero period or a quotient outside the signed 32-bit range causes a division
fault.

The movement choice is an inclusive random draw from 0..32767 multiplied by 20
and divided by 32767. Values below 16 choose backward movement, values 16 and 17
choose right movement, and values 18 through 20 choose left movement. Only the
chosen direction's admission is tested. A result equal to one requests that
movement with duration one; any other result leaves movement unchanged, without
trying another direction. A successful multiple-attacker sidestep skips both the
pacing division and this random draw.

A spacing pass that passes the action-selector and turning gates registers the
receiver in the deferred map-update list. This registration also occurs when
movement admission rejects every direction, the required animation group is
absent, or no movement is requested. Repeated registration retains the first
occurrence of each actor. The deferred pass processes registrations in that
order, clears the consumed entries and resets the list count.

The combat coordination selector starts at zero, separately from the consecutive
close-distance counter. Assigning the coordination selector first clears that
counter, then stores the requested selector value.

### Combat coordination

The coordination selector is also the attacker-selection state used by attacker
iteration. Exactly one admits an attacker. The waiting-state lookup scans the
ordinary actor list from its head, excludes the supplied target actor, and chooses
the first actor with a combat record, a nonnull combat target, lifecycle selector
other than eight and coordination state one. It does not require that actor's
target to equal the supplied target.

The active-state lookup starts after the receiver in ordinary list order and
wraps at the list end, stopping before the receiver. It chooses the first other
combat actor with the same combat target, lifecycle selector other than eight,
and either a held weapon or combat flag mask `0x04`. The selected player is
excluded.

After successful engagement and both cleanup executors reporting inactive,
coordination state one selects the active branch. With negative aggression and
an active-state candidate, the candidate's three-dimensional distance to the
selected player must be strictly below three times its far spacing range to
send addressed message `0xd002`. The range multiplication wraps at 32 bits.
That send precedes a kind-nine timed taunt request, then assignment of coordination
state zero. Spacing, counter-attack and defense selection still follow this
originally selected active branch.

Other coordination values select the waiting branch. An absent waiting-state
candidate assigns state one. With a candidate and more than ten consecutive
close-distance passes, the receiver sends `0xd003`, then requests a kind-eight
timed taunt. Spacing follows the waiting branch without counter-attack or defense
selection on that pass.

Without engagement, state one runs the active-state lookup and sends `0xd002`
when a candidate exists, then assigns state zero. Other states remain unchanged.
The retained-target movement helper follows in either case.

### Counter-attack timing

Counter selection subtracts the target's current signed animation command
cursor from its current attack motion's `first_collide_frame` and
`first_defend_frame`, producing two signed wrapping gaps. It does not use the
frame-wait progress byte or the displayed model frame. The target's retained
attack-slot selector and current motion selector choose the motion.

The target motion's one-based `defense` reference, minus one with signed
wrapping, selects one of the receiver's four defense slots. That record's
`first_collide_frame` and `collide_duration` supply the counter's timing start
and duration. These values are distinct from spatial `min_range` and `max_range`.

Counter selection first requires a held weapon, distance strictly inside the
target's spacing window, and an absolute wrapped heading difference greater than
768. The target action must be neither minus one nor nine, the receiver action
must be minus one, at least one bound defense must be available, and the target's
attacked flag `0x10` must be clear. Timing and counter-reference queries follow
these gates.

An accepted counter preserves the low two intent bits while setting bit `0x02`,
stores the pending counter index, invokes the synchronous attack starter, then
sets the target's attacked flag. The target mark follows the starter's return.
A deterministic timing miss steps right for an even pacing sum or left for an
odd sum when the target's retained attack selector is three or four; otherwise
it steps backward. These movements use duration one and call the movement
primitive directly, without the spacing helper's additional obstacle admission.

The attack starter first rejects drawing or sheathing, then requires action minus
one. For a nonplayer it retains the pending index, assigns action nine and invokes
the attack executor. The selected player's pending-index selection is separate.

### Attack executor

The attack executor resolves its pending move through the four defense slots
bound from attributes 79 through 82, rather than the nine attack slots.
An idle cleanup request does not inspect move metadata. An idle non-cleanup
request compares the current animation group with the pending move's group.
If they differ, it makes a forced animation request with modifier zero, sets
the finite loop countdown to one and clears the animation's pending group.
Only then does it set combat attack state to two and subtract ten from the
aggression budget with signed 32-bit wrapping. An already-current group leaves
the record unchanged.

With nonzero attack state and a pending move, leaving that move's animation
group clears attack state and the pending move. The action selector also clears
when it is nine. The executor reports active exactly when attack state is
nonzero and a move remains pending.

### Multiple-attacker spacing

The multiple-attacker spacing helper traverses the current target's attacker
list in ordinary actor-list order. First and next queries exclude the target itself
and require a combat record targeting it, retained attacker-selection state exactly
one, and a false `IsDead` result. Its initial primary neighbor is the first attacker, unless that attacker
is the receiver. It starts a retained distance at 65535. For each later attacker
other than the receiver, a horizontal query distance strictly below the retained
distance replaces the primary neighbor and moves the former primary into the
secondary slot. Equal distances retain the existing pair. The initial attacker
is not distance-tested, and the secondary slot is the preceding primary rather
than a separately selected second-nearest attacker.

The neighbor-distance query uses horizontal Euclidean distance in runtime Q8
units. For each actor, a nonzero `attr_slave` selects attached position; otherwise
a valid linked positioned record supplies position, with ordinary local position
as the fallback. It subtracts the receiver's X/Z from the candidate's X/Z with
signed 32-bit wrapping and supplies zero for the Y difference. Each difference is
stored as single precision. The sum of squares passes through a double-precision
store before an extended-precision square root; the stored single-precision root
converts to an integer with nearest-even rounding. This position selection differs
from the local-position vectors used by the separation query below.

For each retained neighbor, the helper forms receiver-minus-target and
neighbor-minus-target vectors from local Q8 positions. Each subtraction wraps
at signed 32 bits, then each component shifts arithmetically right eight before
vector-to-angle conversion. The separation is the receiver vector's Y angle
minus the neighbor vector's Y angle: both angles are masked to 2047 and the
difference wraps into the inclusive range -1024 through 1024. The two exact
half-turn signs remain distinct. An absent neighbor supplies 1024 without
converting either vector.
Vector-to-angle conversion first normalizes all three vector components to
length 65536. Each signed component is stored as single precision. Their sum of
squares passes through a double-precision store before an extended-precision
square root. The root multiplied by 1/65536 is stored as a single-precision
divisor. A zero divisor produces a zero vector. Otherwise each component divided
by that divisor is stored as single precision, then converted to a signed integer
with nearest-even rounding. The angle conversion consumes this normalized vector.
Its Y angle is the negative of the [quantized actor heading](player.md#direction-recovered-from-displacement)
from normalized X/Z, minus 1024, wrapped modulo 2048. Normalized Y influences
this result through the shared three-dimensional normalization, even though the
heading query itself uses only X/Z. The zero vector produces Y angle 1024.
Normalization can round a small horizontal component to zero, so taking the
heading directly from the unnormalized displacement is not equivalent.

The threshold is `[misc] non_engaged_spacing` from `COMBAT.INI`, stored without
scaling. Absolute values use signed wrapping negation. With primary and secondary
separations `a` and `b`, and threshold `s`:

- If `abs(a) < s`, it requests a sidestep only when `abs(b) > s / 3`, using the sign of `a`.
- Otherwise, if `abs(b) < s`, it requests a sidestep only when `abs(a) > s / 2`, using the sign of `b`.

Division truncates toward zero. A negative chosen separation requests right;
zero or positive requests left. Both use duration one. The helper returns one
after making that request, without using the movement routine's numeric return,
and zero when it makes no request.

Selected-player maintenance is skipped when its retained entry-inhibition value
is nonzero. Player initialization clears this value. Player activation replaces
it with the activation operation's result, so initialization does not make it
permanently zero. This gate does not inhibit nonplayer maintenance.

The target/perception prepass returns false for the selected player before
nonplayer perception processing. For a nonplayer actor, it returns false when
combat flag mask 0x04 is clear or the target is absent. When maintenance reaches a
combat record, it restores attribute 1 from attribute 119 before processing
messages. The nonplayer decision stage runs only for death/state selector 6 and
an actor that is not dead. This stage can select targets and
consume random values.

After walk-map or free-node admission, the nonplayer perception prepass samples
visibility at twice the combat far range. The combat record retains a signed
lost-sight timer and a lost-sight routing flag, both cleared by combat construction.
The delay is `[misc] node_timer` from `COMBAT.INI`, stored without scaling.

When visibility is absent, the timer is compared with the delay before any
increment. Only a timer strictly greater than the delay enters routed pursuit.
At or below the delay, the timer adds the shared nominal tick count with 32-bit
wrapping and the prepass returns false. Crossing the delay during that addition
therefore takes effect on a later pass.

Above the delay, a zero lost-sight flag permits a coordination-peer lookup. A
successful lookup sends message 0xD002 to that peer and sets the actor's
coordination selection state to zero. The lost-sight flag is then set to one,
point navigation is requested toward the target's current query position with
zero tolerance, and the prepass returns true. This path does not increment or
clear the timer. A nonzero flag skips the peer lookup on later pursuit passes.

Visible targets reset the timer to zero. If the lost-sight flag is nonzero,
reacquisition restores lifecycle selector six, clears the flag, releases held
next/current nodes, sets coordination selection state to one, and clears all
walk-node reservation bytes, in that order. The prepass then returns false.

Selected-player combat processing uses the attack/defence executor state as its
phase value. This state is separate from both the cooldown and the retained
combat allocation index. After message processing, phase zero with a nonzero action selector
resets the action selector and both retained action indices to `-1`. An action
selector of zero leaves all three values unchanged at this step.

It then calls the first maintenance helper followed by the second, both with
action `-1`. With phase zero and argument `-1`, the first helper returns false
without invoking other services or changing actor or combat state. With the
combat record in its freshly initialized state, the second helper also returns
false without invoking other services or changing actor or combat state.

The selected player's drawing or sheathing state suspends action and cooldown
maintenance. Otherwise a nonzero cooldown decreases by the shared nominal tick
count, saturating at zero. With zero cooldown, action -1 does no attack work;
actions 0 through 6 enter attack processing. Other action values reset to -1.

### Attack-processing admission

Attack processing requires a combat record and an action other than `-1` or nine.
The animation admission query reads the animation state's unsigned reverse-playback
byte. It must return zero, and combat flag `0x01` must be set. Only then does processing query the current attack's active collision
window. A true result skips the extended-window query. A false result consults
the extended window instead. If either window admits the attack, processing
sets combat flag `0x20` before traversing either collision-candidate list.
Empty candidate lists leave that flag set.

The active collision window uses the signed animation command cursor and the
current motion's `first_collide_frame` and `collide_duration`. It includes the
start and excludes the end: `start <= cursor < start + duration`. The end uses
signed 32-bit wrapping addition. A cursor before the start rejects the window
without querying duration.

The extended window first requires a nonzero duration. It then requires
`cursor >= start + duration`, using the same wrapping end calculation, and
combat flag `0x20` clear. It reads that flag only after the cursor comparison
succeeds. Negative durations receive no separate rejection. Both windows use
the command cursor, not the displayed model frame or frame-progress byte.

### Combat candidate dispatch

Attack processing scans its ordered actor candidates, skipping empty entries.
For a candidate with a combat record, it first runs that candidate's defence
query against the attacker. A nonzero defence result skips the contact query.
Otherwise it queries contact from the attacker to the candidate. A zero contact
result continues the scan.

On the first nonzero contact result, it clears the attacker's combat flag masks
`0x01` and `0x02`, emits message `0xd001` from the attacker to the candidate,
and requests the attacker's combat taunt category 3, in that order. It then
returns without scanning later actor or static candidates.

A candidate without a combat record skips the defence query. The contact query
from the attacker to it must succeed and the candidate must not be dead (`IsDead`).
An attacker with nonzero attribute 97 that neither holds a weapon nor has combat
flag four then returns without scanning later candidates and without any response.
Otherwise the same impact response as for an actor-owned face runs: the attacker's
attribute-86 effect at volume zero and pitch offset `rnd(128) * 16 - 1024` Hz, then,
for a candidate with nonzero health, negative attack damage and a death dispatch
with selector zero below one. Attack processing then clears the attacker's combat
flag masks `0x01` and `0x02` and returns without scanning later candidates.

### Unowned static weapon contact

After actor candidates, attack processing visits retained face-query models in
candidate order. Each model receives the retained weapon tip followed by the
base as segment endpoints, with query option zero. Results zero and minus one
are misses; every other result admits the unowned-model impact response.

Query mode zero scans authored faces after model and face-group admission.
Mode one returns zero when a face-group bound admits the segment, without
scanning its faces. Mode two returns zero after model-radius admission, without
visiting face groups. These modes are separate from the shared polygon-sidedness
flag. Its initial stored value is enabled, and initialization also enables it.
The temporary query scopes which disable it restore it to enabled before
returning. The weapon contact caller does not assign the flag. With sidedness enabled, the face query rejects a segment whose
start lies more than four map units behind the authored face plane.

A model without an actor owner uses installed `[misc] sword_clank_1`, with volume
zero and pitch offset `rnd(128) * 16 - 1024` Hz, positioned on the attacker.
Candidate type exactly one repeats the impact sound with a separate random draw.
Other candidate types request one sound. The scan continues to later candidates
and leaves attack intent set. Models with actor owners use the actor-response
path separately from this unowned-model response.

### Actor-owned face contact

An actor-owned face candidate still receives the model segment query, but the
result does not gate its actor contact test. A successful actor contact must also
pass the target rejection check. For attackers with attribute 97 equal to zero,
this admits the impact response directly. Other attackers require either the
weapon-holding predicate or combat flag four.

The response requests the attacker's attribute-86 effect at volume zero and
pitch offset `rnd(128) * 16 - 1024` Hz, then flushes the sound request. A target
with nonzero health receives negative attack damage, followed by a health read.
A result below one dispatches its death response with selector zero. Zero health
skips damage and death dispatch. Processing continues to later face candidates
and retains attack intent, including after a successful response.

### Contact damage and nonplayer health

Contact damage reads the receiver's retained attack slot and motion selector.
A negative attack slot returns zero; otherwise the selected motion's signed
`damage` value is returned without additional arithmetic.

The nonplayer additive health routine first rejects a nonzero attribute 120.
If attribute 97 is nonzero, it also requires either the weapon-holding predicate or
combat flag four. These checks precede the health write. An admitted change adds
the signed change to the unsigned health byte with signed 32-bit wrapping, then
clamps the result to zero through 255. This additive routine does not dispatch
death itself; the contact caller reads health and owns that dispatch.

### Combat hit-reaction admission and animation

A nonplayer hit receiver requires either attribute 102 equal to zero or its
value equal to the sender's attribute 23. A mismatch skips displacement,
animation and damage. The selected player bypasses this outer admission test.

An admitted reaction displaces the receiver relative to the sender using the
same 3000-unit horizontal translation as the block-message response. It leaves
the sender's position unchanged. For the selected player, a nonzero retained
death-animation override then returns before weapon queries, animation or damage.

A nonplayer reaction starts animation group 117 without querying weapon state.
The selected player first tests weapon holding, then combat stance only when
not holding a weapon. Either predicate selects group 117 on an even world-loop
counter or 118 on an odd counter. Otherwise it queries availability of group
119. When available, the receiver-to-sender forward-orientation check selects
119 for a hit in front or 120 otherwise. When unavailable, world-loop parity
selects group 132 or 133 instead. All these starters receive loop argument zero.

After animation, damage still requires attribute 102 to be zero or to match
the sender's attribute 23, including for the selected player. A selected-player
reaction admitted only by identity therefore animates without applying this
damage response. After admitted damage returns, sender attribute 23 equal to 75
requests the sender's ordinary weapon-restoration service.

### Combat direct-damage response

The `0xd005` response helper first displaces the receiver relative to the sender
using the 3000-unit horizontal translation. For the selected player, a nonzero
retained death-animation override then returns before weapon queries, animation
or damage.

A nonplayer receiver starts animation group 117. The selected player tests
weapon holding, then combat stance only when not holding a weapon. Either
predicate selects group 117 on even world-loop parity or 118 on odd parity;
otherwise parity selects group 132 or 133. All starters receive loop argument
zero. This response does not query group 119 or forward orientation.

After animation, an enabled selected-player damage-suppression flag skips the
health response for the selected player. Other receivers continue regardless
of that flag. The response negates the sender's unsigned attribute 96 and passes
that signed change to generic additive health with mode zero, then reads the
receiver's health. Health below one requests receiver taunt category six only
for a nonplayer receiver, then invokes the [combat death service](#combat-death-service)
with selector zero for either identity. Nonfatal health skips taunt and death dispatch.

### Combat hit-response damage scaling

The admitted combat hit response reads the attacker's retained strength and
current-motion damage, then the receiver's retained armor. Compute the product
`(strength + 100) * damage`, then divide it by 100 using signed truncation
toward zero. Call that quotient `scaled`. The health change is
`-(scaled - (scaled * armor) / 100)`, with the armor division also truncating
toward zero. Integer additions, multiplications, subtractions and negation wrap
at 32 bits. No separate strength, damage or armor clamp occurs here.

The response passes this signed change to generic additive health with mode
zero, then reads the receiver's health. Health below one requests receiver taunt
category six, followed by attacker taunt category seven. It then reads the
attacker's current motion kind and invokes the receiver's
[combat death service](#combat-death-service) with selector zero for kind zero, or selector one
for any nonzero kind.
Nonfatal health skips these taunts, the motion-kind read and death dispatch.

### Health native routing

`SetHealth(receiver, value)` first raises the selected player's shared HUD
feedback timer to 25 when it is below 26. It computes `value - current_health`
with signed 32-bit wrapping and passes that delta through the generic
health-with-death admission service, including for the selected player.

`AdjustHealth(receiver, change, mode)` performs the same early selected-player
feedback update. For the selected player, mode zero directly invokes additive
health; every nonzero mode directly invokes death-owning health. These direct
selected-player calls bypass generic attribute admission. Their source actor is
the retained native-command source described below, except that a source equal
to the health receiver is passed as absent.

For other receivers, only mode one selects generic health-with-death admission;
all other mode values select generic additive-health admission. These generic
services apply their attribute gates before forwarding selected receivers or
changing nonplayer health. The early native feedback update remains visible
when a later admission rejects the health change.

Both generic services reject nonzero attribute 120 before any weapon or combat
query. With nonzero attribute 97, they first test weapon holding. Only when that
is false do they read combat flag four. A missing combat record supplies a clear
flag. The receiver is admitted when either test succeeds; otherwise it is left
unchanged. An admitted selected receiver forwards to the corresponding selected
health service with no source actor.

### Retained health source context

The ordinary entry/task/main interpreter entry establishes its executing actor
as the shared native-command source before entry continuation or the existing-task
gate. It establishes this source even when an unpaused task prevents the main body.
The separate task-actor update uses the same interpreter entry.

The alternate entry/main path selected by script-control mask `0x04` does not
replace that source actor. It retains the source from the previous assigning
entry while bypassing the existing-task pass. Both entry paths capture the
current shell-execution context before interpreting entry work.

`AdjustHealth` reads this retained source independently of the receiving actor.
For a selected-player receiver, identity equality converts the source to null;
a different retained actor remains the source. The source is therefore not
recomputed from an object-targeted health receiver or from the actor entering
the alternate main path.

### Selected-player additive health

Selected-player initialization clears hit inhibition, sets session maximum health
to 100 and copies the actor's current health byte into retained displayed health.

The selected-player additive routine returns immediately when hit inhibition is
active. Otherwise it sets a feedback timer below 50 to 25, adds the signed change
to current health with signed 32-bit wrapping, and clamps the result to zero
through the session maximum health. It writes the health byte first. A negative change requests ordered taunt category
two followed by the damage-reaction service, even when the resulting health is
zero. Retained displayed health is written after these services return; both
services observe the previous displayed-health value.
Category two uses the in-order taunt scan and bypasses target-distance and
clock-period gates.

The reaction service suppresses its work when either current or pending animation
is group 73, 74, 117, 118, 132 or 133. Otherwise it draws random parity and tests
the weapon-holding predicate. With that predicate false, even parity selects group
133 and odd parity selects 132. With it true, even selects 118 and odd selects
117. The selected animation starts before the reaction's action and motion
follow-up services.

The action follow-up runs only for the selected player in living state 15
with its inhibition flag clear. Before its transition service, it sets living
state 17 and clears its retained ledge-support actor reference, including when
the transition is subsequently blocked. If the transition leaves living state
nonzero, it clears the three
directional state bytes afterward. Other receivers and living states leave this
state unchanged. The motion follow-up calls its destruction service only when
the receiver has a rope attachment. Without that attachment it does nothing.

Player initialization clears the separate action-inhibition byte. Ledge entry
from living state zero requests state 19 and runs the transition. If a nonzero
living state remains, it clears the three directional bytes and sets action
inhibition to 16. This byte is separate from hit rejection and combat-decision
inhibition.

The state-17 transition requests group 56 and clears living state when the
receiver is the selected player, attribute 105 is not one, the torch-burning
counter is zero, and neither current group 149 nor 150 blocks it. Those groups block
only while animation is enabled and shell context is clear. The animation
request precedes the living-state clear. Other combinations retain state 17.

The admitted state-17 transition sets the retained action-transition counter to
six. Ordinary actor updates leave a zero counter unchanged. For a nonzero
counter, if either of the first two directional bytes is zero and the signed
counter is less than three, the counter clears immediately. This test does not
depend on living state. Otherwise the update clears the high bit of both
bytes, subtracts the low byte of the shared tick count with byte wrapping, and
clamps a negative signed-byte result to zero.

### Replenishing health

`ReplenishHealth` takes no arguments. If the receiver's health byte is at least
100, it returns zero without changing health or feedback. Otherwise it sets the
health byte to 100 and returns one. For the selected player, the change first
raises the shared HUD health-feedback timer to 25 when it is at most 25; larger
timers remain unchanged. This command does not write retained displayed health
or invoke additive-health inhibition, hurt reactions or death services.

### Rope construction and swing

Loading a map builds one rope per `MPRP` record, only when SYSTEM.INI `ropes_load` is
set. At most 255 ropes exist; another is a fatal error. A rope retains a link array and
an attachment list and joins the head of the rope list.

Constructor inputs, from the record: the link model is `rope_model` and the optional end model
`static_model` (an empty name is none); the base position; `length` links (an `i16`); the
`swing` maximum amplitude; the `speed` rate, where a value below 1 becomes 199; and the type
byte. An end model sets bit 0x10 of the type byte. Bit 0x20 starts the live amplitude at the maximum
(otherwise 0). Heading `angle_y` and the three swing sound effects at 0x34–0x3C are stored on the rope. A nonzero
word at 0x44 lowers the base Y by the link length times `length`, plus 256 times an engine constant. A link's length is a fixed-point value
taken from its model record. Link `i` records its position along the rope as
`(link length / 2 + i × link length) >> 8` and its extent as `link length >> 8`.

Each update, for ropes whose active word (set to 1 at construction) is 1, the amplitude is clamped to 0..maximum. With
type bits 0x4 and 0x8 clear and a nonzero amplitude, the phase advances by
`speed × 0x40 × frame scale >> 8` and the step angle is
`sine[(phase >> 8) & 0x7FF] × amplitude >> 9`. A zero amplitude gives step angle 0. Attached
actors playing animation 0x52 move one node toward the base, and 0x53 one node down while the
limit routine allows it; each such step also sets the amplitude to 2 and the phase to 0 when the amplitude and the rope's word at offset 0x62 are zero.
The integer sine table holds, for `i` in 0..1023, `trunc(sin(i × π / 1024) × 65536)`, with the
entry at `2048 − i` negated and the entry at 1024 zero.

Placement runs from the map render list. Link 0 sits at the rope position, yawed by
`−angle_y & 0x7FF` and pitched by `−(amplitude >> 8) & 0x7FF`. Link `i ≥ 1` sits at the previous
link position plus the previous link's matrix applied to `(0, link length, 0)`, and is yawed the same
way and pitched by `−(sum of the step angles of links 1..i−1 >> 8) & 0x7FF`, so link 1 has pitch 0 and an
unswinging rope hangs straight. Positions are stored as single-precision values divided
by 256. An actor attached to node `i` is placed on that link position.
Each placement then offsets the actor by its hand: the vector `(0, −hand Y, −hand Z)` of the same grip
vertex, rotated by the actor's own orientation (heading, pitch, roll 0), is added to the actor's position,
so the hand rests on the link. The Y component is skipped while the actor plays group 0x54 with the animation
record byte `+0x11` below 3.

### Rope attachment

Each ordinary actor update of an actor whose movement profile enables the rope check runs an
attachment test before ordinary movement. When the test reports true the ordinary movement and
combat stages are skipped for that update. The test is false for every actor except the selected
player, and false while the player is holding a weapon (see `IsHoldingWeapon`).

For the selected player the test returns true without further work when it already has an
attachment. It returns false when its directional state bytes 0x1B2 and 0x1B3 are both zero, and when
byte 0xC1 is zero while the actor has no animation record or that record's word at offset 0x161 is
zero. A nonzero rope countdown (byte 0x1B0) then decrements by the low byte of the shared tick count;
when it would reach zero, the stored rope identifier (byte 0x1AF) and the countdown both clear. The
rope identifier is the byte at rope offset 0xC, so a rope the player just left is skipped until the
countdown expires.

Ropes are tried in list order, head first. A rope is a candidate when it is not skipped by the
identifier rule, it has a link array and more than one link, and it has a contact sphere list.
Contact is decided by one of two tests:

- With type bit 0x1 clear, any overlap between the player's body spheres and the rope's contact
  spheres.
- With type bit 0x1 set, the absolute heading difference (the smaller of the differences from the
  rope heading and from the rope heading plus 0x400, over the 0..0x7FF circle) must not exceed SYSTEM.INI `rope_attach_angle` masked with 0x3FF (512 as shipped),
  and the same sphere overlap must hold.

The player's spheres are its RAVC records (flag 2 always, flag 4 only while the actor's current
animation group pair matches the record) followed, unless attribute 56 is nonzero, by its ordinary
collision shape spheres; each center is rotated by the actor orientation and added to its position.
At most 128 take part. The rope's spheres are the records of its own sphere list (11 bytes each:
flags, a 16-bit position triple and radius, each `>> 16 << 8`), translated by the rope position, at
most 128. Two spheres overlap when the rounded three-axis distance between their centers is less than
the sum of their radii. Contact also records the lowest rope sphere index that touched.

On contact the actor requests animation group 0x54, then attaches (below). For the selected player the
rope's type bit 0x4 clears afterwards and a retained flag is set to one.

Attach removes any attachment the actor already has (the removal above), allocates a 20-byte node,
links it at the head of the rope's attachment list, stores the actor in the node and the node in the
actor, copies the rope identifier into the actor and clears the countdown. The requested link index
is the lowest touching sphere index, clamped to `[0, link count − 1]`, then replaced by the link
nearest to the actor's hand height: starting at index − 8 (floor 0), the link whose stored Y (a
single-precision value) is at or below the target height and closest to it wins. The hand point is
grip vertex 0 (the RAEX `grip0` vertex) of the first `ShowFrame` of animation group 79, as the
integer Q8 model vertex. The target is `(hand Y + actor Y + 2 × link length − 1) >> 8`, the link length
in Q8 as stored on the rope. When its distance from the floor probe is less than the actor height limit
plus the vertex's absolute Y, it becomes the floor probe minus that limit. With no group 79 frame the
requested index stands. The node stores that chosen link index.

node stores that chosen link index.

Then, with type bit 0x1 clear the rope heading becomes the actor heading; with it set the actor
heading adds the signed heading difference selected above. When either of the actor's directional
bytes 0x1B2 and 0x1B3 exceeds one, the rope word at offset 0x6A becomes zero while actor byte 0x1CB is
3 or 7 and 0x40000 otherwise, and the rope amplitude becomes `(d / 1000) × (link count / link index)` rounded, with `d` the rounded planar distance of the actor's last step in Q8 and the integer division of `d` by 1000 truncated, and the rope word at offset 0x62 clears. Finally bytes 0x1B1 through
0x1B4 of the actor clear, and animation group 0x54 is requested when it is not already current.

### Rope control while attached

An actor attached to a rope skips the ordinary movement and combat stages (see
[Rope attachment](#rope-attachment)). The selected player's movement dispatch has a rope branch that
runs while it is attached. Keys are the KEYS.INI bindings: Up, Down, Left, Right, A (`key_a`), B (`key_b`,
the jump key) and C (`key_c`, the view key).

- **Idle.** When the actor holds a weapon, or neither Up nor Down is held, the branch requests group 6
  (the idle-rope entry of the named group table).
- **Jump release.** B is latched on attach: the latch stays set while B is held and clears on an update
  with B up. With the latch clear and B held, the branch removes the attachment (the
  [removal](#rope-attachment-removal) with its detach motion), runs the ordinary jump entry (which, with the rope
  countdown now nonzero, requests group 85 and sets the second directional-state byte to 24), clears the
  heading-turn rate below and sets the latch. Damage and a world change also remove the attachment.
- **Climb.** Without a held weapon and with C up, Up runs the climb-up helper and otherwise Down runs
  the climb-down helper. Climb up does nothing unless the attachment's node index is at least 2; it requests
  group 82 when the current group is not 82 and returns true, otherwise it requests group 82 again while
  the animation record's byte `+0x0a` is zero and returns whether byte `+0x0b` is nonzero. Climb down asks
  the [hand-height link choice](#rope-attachment) for node index + 1; it does nothing unless the result is
  greater than the current index, requests group 83 and returns true when the group was not already 83, and
  otherwise returns whether byte `+0x0b` is nonzero. Each update the rope owner reads the current group:
  group 82 moves the attachment one node toward the base when the shared tick count is nonzero, and group 83 one
  node down while the limit routine allows it, and either step sets the amplitude to 2 and the phase to zero when
  both the amplitude and the pending push (the word at rope offset 0x62) are zero. The selected player's ordinary
  update also requests group 83 for Down and else group 82 for Up while attached and C is up.
- **Turn.** A nonzero heading-turn rate adds `(rate >> 4) × tick scale >> 8` to the actor heading (mod 2048) each update and
  accumulates the step; the rate clears once the accumulated total exceeds the rate in magnitude. With a zero
  rate and A held, a rope whose amplitude is below 5 either (type bit 0x1 set) starts a turn of rate 1024 for Left or
  −1024 for Right with the accumulated total cleared, or (bit clear) adds `Right × tick scale >> 8 − Left × tick scale >> 8`
  to the heading.
- **Pump.** With C and A up, the pump service runs on the attached rope unless its type bit 0x8 is set. With neither
  Left nor Right held it clears the pending push. Otherwise, while the amplitude is zero it adds 3 to the pending push,
  sets the amplitude to 2 and, with `d` the absolute heading difference between the actor and the rope heading, sets the
  phase (rope offset 0x6A) to 0x40000 for Left or zero for Right when `d` is below 16, and the opposite when it is not.
  While the amplitude is nonzero it takes the rope's direction byte (offset 0x38), flipped when type bit 0x1 is set and `d` exceeds
  16, and adds 3 to the pending push for Left when the byte is 1 and for Right when it is 0; the other key subtracts 3
  while the amplitude exceeds 2.

While an attachment is placed on its node during the chain update, a selected player whose pending group is
unset (or is group 79) and who holds neither Up nor Down requests group 80 when the rope's direction byte
is nonzero. With that byte zero it requests group 81 when its pitch is nonzero.

#### Rope amplitude updates

Each chain update ends by passing the sum `t` of the link step angles to the amplitude service, which keeps the previous sum
`t'`. A zero crossing (`t ≥ 0` with `t' < 0`, `t < 0` with `t' ≥ 0`, or both zero) runs two things. First, when the rope has
attachments and `L = maximum / 4` exceeds 2, it plays a swing sound by amplitude `a`: sound 3 when `a ≥ 3L`, else sound 2 when
`a ≥ 2L`, else sound 1 when `a ≥ L`, each only when its effect number is nonzero. Second, with `u = maximum / 5` and `p` the pending
push (the word at offset 0x62), it adjusts the amplitude and the push by one step:

- `p > 0`: the step is 5 when `p ≥ 5` and `p > 4u`, else 4 when `p ≥ 4` and `p > 3u`, else 2 when `p ≥ 2` and `p > 2u`, else 1.
  The amplitude gains the step and the push loses it.
- `p < 0`, with `n = −p`: the step is 1 when `n ≤ 3u` and (`n > 2` or `n ≤ 2u`), 2 when `n ≤ 3u` otherwise, 4 when `n > 3u` and `n ≤ 4u`, and
  5 when `n > 4u`. The amplitude loses the step and the push gains it.
- `p = 0` and `t` or `t'` nonzero: the amplitude falls by 5 when it exceeds `4u`, else 4 above `3u`, else 2 above `2u`, else 1 while nonzero.

After either branch the direction byte (offset 0x38, copied to 0x39) is 0 when `t ≠ 0` and `t' < t`, and 1 otherwise, and `t'` becomes `t`. The
amplitude is then clamped to `0..maximum` by the next update.

### Rope update order

Each world-loop iteration builds the map render lists first, which places every rope's links and attached actors and runs
each rope's amplitude service, then runs the rope update (swing step and attachment climbs), then the shell scripts, then
the ordinary actors, whose update runs the [attachment test](#rope-attachment) and the [attached controls](#rope-control-while-attached).

### Rope attachment removal

Ropes own doubly linked lists of attachment nodes. Each node retains its actor,
rope position and motion value. The actor retains the node identity. Owner
lookup scans the rope list and each attachment list by node identity; an absent
owner prevents removal.

Removal reconnects both neighboring nodes and replaces the rope's head when the
removed node is the head. Removing the only attachment leaves the owner with an
empty attachment list. With a retained actor whose attachment is either this
node or null, removal applies detach motion, clears the actor's attachment and
pitch, and sets its rope countdown to 24 before releasing the node. If the actor
instead names another attachment, those actor writes are skipped. The removed
node is released after the list and eligible actor updates.

The detach motion is the actor's forward speed word, which queued motion 24 turns into a horizontal move of `|speed| × frame scale >> 8` Q8 units per update, forward
at the actor heading for a nonnegative word and backward (heading plus 1024) for a negative one. With `j` the SYSTEM.INI `rope_jump_add` shifted left by eight (4608 as
shipped) and `m` the node's motion value (the planar speed the chain update measured between the node's previous and new link position, in Q8, negated when the rope's
direction byte is zero): when the rope amplitude is below 6 and `|m|` is below `frame scale × 0x1800 >> 8`, the word is zero, except that for the selected player Up sets it to `j` and Down
to `−j`. Otherwise, with the direction byte flipped when type bit 0x1 is set and the actor's heading differs from the rope heading by more than 16, a byte of 1 sets the word to
`j + (|m| << 8) / frame scale` and a byte of 0 sets it to `−j − (|m| << 8) / frame scale`.

Destroying an entire rope uses a separate path. It unlinks the rope from the
rope list, walks and releases its attachment nodes, and clears each retained
actor's attachment link, even when that link names a different attachment. It
does not invoke attachment detach motion or change actor pitch and rope countdown.
Attachment nodes are released in list order, followed by the optional auxiliary
record, optional end-model record and the rope itself. A null rope does nothing.

For the detach direction branch, the rope's angle flag enables a heading test.
The signed heading difference wraps over 2048 units. An absolute difference
strictly greater than 16 flips the rope's direction bit for that detach.

### Selected-player death-owning health mode

The death-owning health service is separate from selected additive health. Hit
inhibition rejects it before any service. Otherwise, a health-feedback timer below
50 is set to 25 before action cleanup or the health write. For a negative change,
action reset and rope detach run before the actor health write. The service then clamps the
wrapping sum to the session maximum and zero, and publishes actor and retained
displayed health before later notification.

When health becomes zero and the session death-suppression flag is clear, it
requests taunt category six, enables hit inhibition, clears the retained death-progress word,
captures the selected actor's local position and sends death notification, which
sets its death selector to eight without changing other actor state. An
existing controller animation override is preserved. If no override is set, a
nonnull source actor's nonzero attribute 109 supplies it before ordinary death
animation selection.

When neither an existing override nor the source supplies a group, underwater
state two or four sets the retained death mode to two. Attribute 105 equal to
one skips the remaining animation choice. Otherwise, a nonzero `fall_flag` or
current group 114, 116, 130 or 131 sets the override to group 74. With none of
those conditions, an unarmed receiver force-starts group 134. A receiver holding
a weapon starts group 122 when present, otherwise group 121, then stores death
default 124 or 123 respectively and releases its held walk nodes, in that order.

The nonfatal or death-suppressed path instead clears the override, hit inhibition
and the retained death-progress word. This word is separate from the controller's
pending action flag. A negative change then requests taunt category two and
the damage reaction after health publication. These service orders differ from
selected additive health and are not interchangeable.

### Selected-player death admission state

`IsDead` returns true for a selected player with nonzero hit inhibition, or for
any receiver whose death selector is eight. The retained death-animation group
does not participate in this predicate. The same hit-inhibition flag gates
selected-controller death execution, ordinary defence sampling and dead-receiver
taunt rejection. It is separate from the retained death-animation group.

### Selected-player death animation override

The selected player's retained controller override stores an animation group
identity, not only an active flag. When that override and selected hit inhibition
are both nonzero, default-animation selection copies the override group into the
actor's default animation. If the current group already matches, it cancels the
pending group, clears the current playback exit marker, and sets its finite loop
countdown to 32. This replaces any previous loop countdown without restarting
the current frame or resetting its command cursor. Otherwise it force-starts
the override group. This branch bypasses
ordinary default-animation selection.

### Selected-player death controller

Hit inhibition admits the selected death controller after the ordinary
preliminary controller countdowns. If the retained animation override is zero,
the controller assigns group 122 while the actor holds a weapon, or group 134
otherwise. A nonzero retained group bypasses the weapon query.

Every admitted call force-requests the retained group with modifier zero before
adding the nominal-tick increment to the retained death-progress word. The
addition wraps at 32 bits; the starter observes the previous progress value.
Only death modes one and two perform environmental effect updates. Zero and
other mode values return after progress accumulation.
The player initializer clears both death mode and progress to zero. These words
are separate from the controller pending-action flag and preliminary countdowns.

For a selected player with hit inhibition, attribute 105 equal to one and death
mode two, the world loop exits with outcome 5000 only when signed death progress
is strictly greater than 12. Progress 12 does not exit. This branch bypasses the
ordinary retained-group animation-completion comparison.

Otherwise, inhibited selected-player death completes only while the current
animation group equals the retained override. For override groups 13 and 74,
the signed command cursor must be less than or equal to the unsigned wrapping
frame-progress byte, `anim.real_frame`. For other override groups, completion
requires that cursor and progress byte differ. This compares command progress
with frame progress, not the displayed model frame or finite loop countdown.
Completion returns world-loop outcome 5000.

### World-loop death exit ordering

The world loop checks death exits after map and separate-object updates,
effects, scene and HUD presentation, and fade handling. A pending world
transition returns its destination before the falling timeout or selected
inhibition checks. The win request is checked after those death exits.

A selected player whose falling byte is zero clears the retained falling
elapsed counter. A nonzero falling byte adds the shared nominal-tick increment
to that counter, wrapping at 32 bits. When the resulting signed value is
strictly greater than the configured falling-death limit, the loop returns
outcome 5000. Equality does not exit. This check precedes, and does not require,
selected-player hit inhibition. The player initializer clears this counter
independently of selected death mode and progress.

The limit is `max(player_fall_dead_time, 3) * 12`, using signed configuration
comparison and 32-bit multiplication. It is measured in nominal ticks, not
rendered frames. A pending world transition bypasses the counter update as well
as the death checks.

### Death completion dispatch

After world-loop outcome 5000, the main dispatcher checks the retained last-save
slot. An absent slot opens the book menu with entry context three. An available
slot presents a Yes/No request to load the last game, with the retained save
description appended to the prompt. No opens the same book-menu context. Yes
copies that slot into the pending load selection and selects saved-game loading
for the next dispatcher pass.

The dispatcher clears selected-player hit inhibition after the confirmation or
book-menu call returns. The saved-game loading pass receives the retained slot,
then clears the pending selection and returns the dispatcher to world execution.

### Selected death effect admission

Death-controller environmental effects read progress after its nominal-tick
increment. Mode one always emits its first effect. It then makes two independent
bounded random requests, each with bound `progress / 6 + 1` using signed
progress and division truncating toward zero; a zero result emits
the corresponding additional effect. These random requests remain ordered and
independent. A final effect is emitted only when signed progress exceeds 12.

Mode two consumes one raw engine random value. It emits only when the bitwise
AND of that value and wrapping `progress + 1` is zero. An admitted emission uses
one effect variant before signed progress 12 and another from 12 onward. It
then requests actor sound 71 before 12 or sound 70 from 12 onward, after a
bounded random request of 64 whose result is reduced by 32 for the sound's
signed variation argument. No effect or sound request occurs when the initial
bitwise test rejects the call.

The packed descriptors passed to effect construction are:

| Emission | Descriptor |
|---|---|
| Mode one, unconditional first effect | `0xb081` |
| Mode one, first bounded-random effect | `0x22480` |
| Mode one, second bounded-random effect | `0x2580` |
| Mode one, final effect after progress 12 | `0x32480` |
| Mode two, before progress 12 | `0x2500` |
| Mode two, from progress 12 onward | `0x12500` |

The low sixteen bits identify the texture; the upper sixteen bits are effect
flags. Every selected-death construction supplies zero spread and speed one.

Each emission first computes the camera-relative point described below, then
consumes two bounded random requests in X-then-Z order. The unconditional mode-one
image uses bound 32 and adds `draw * 256 - 4096` to each horizontal coordinate.
Every other selected-death image uses bound eight and adds `draw * 256 - 1024`.
Y is the transformed point's Y plus the receiver's retained positive Y bounds
extent multiplied by 256, plus 1536. These coordinate operations wrap at 32 bits.
The extent is the stored bounds record's positive Y extent, not the actor's
current origin, animation image height or effect texture height. These point and
random operations still run when effect construction later finds a disabled or
full pool.

The mode-two sound is positioned from the receiver's ordinary sound query and
uses zero volume offset. Its signed variation is passed directly as the
playback-rate offset in Hz.

The effect-position helper copies the supplied integer point and preserves its
Y coordinate. It subtracts the published current camera X/Z coordinates from
the point using signed wrapping 32-bit subtraction, then computes the shared
quantized horizontal bearing in 2048 units per turn. Camera height does not
participate. With `index = (bearing - 1024) & 2047`, it adds
`offset * sine[index]` to X and `offset * sine[index + 512]` to Z. The death
controller supplies offset 2048.

Each original X/Z coordinate is converted to single precision before the
displacement is added. Each sum is stored in single precision, then converted
to an integer by truncating toward zero. The helper returns a shared scratch
point and leaves the supplied point unchanged.

### Environmental effect animation

World initialization tears down the previous environmental effect pool before
creating a pool with `[system] max_effects` from `SYSTEM.INI`. Negative configured
capacity is clamped to zero. A zero capacity or a nonzero `disable_effects`
setting leaves the pool disabled. An enabled pool starts with every slot free.

Allocation scans the retained pool list from its head and takes the first free
slot. It marks that slot active and decrements the free count without moving the
slot in the list. Release marks an active slot free and restores the free count,
without exceeding capacity. Releasing an already free slot has no effect. Update
and drawing traverse the same retained list order.

The type-one effect constructor returns no effect when the pool is disabled or
allocation fails. It separates the supplied packed descriptor into upper-16-bit
flags and a lower-16-bit texture identity. The resolved texture supplies the
target frame as its runtime frame count minus one. Current and reference frames
start at zero, and the constructor retains the supplied animation speed.

For zero spread, the constructor subtracts `texture_height * 128` from the
supplied fixed-point Y coordinate with wrapping 32-bit arithmetic. Position
publication then arithmetic-shifts each signed coordinate right by eight bits.
This places the effect half its texture height above the supplied point in
the engine's downward-positive Y convention.

The world loop submits environmental effect billboards while building map draw
lists, after actor model-position processing and before the object and
code-object systems run. It updates effect lifetimes
after those systems, before the rain-drop update and rendering the queued scene.
Effects created by those later object updates miss the current loop's billboard
submission but participate in its lifetime update. Surviving effects can submit
on the next loop. An effect already submitted can expire during the subsequent
lifetime update; releasing its slot does not remove the submitted billboard.

The type-one environmental effect updater rejects a disabled effect pool, a
null effect, a different effect type or a zero texture descriptor. Otherwise
it leaves the current frame unchanged when that frame is `-1` or the retained
reference frame equals the target frame.

When the reference frame is below the target, the updater adds nominal ticks
multiplied by animation speed to the current frame. It keeps the effect alive
through equality with the target and expires it only above the target. When
the reference frame is above the target, it subtracts the same product and
expires only below the target. Multiplication and frame arithmetic wrap at
32 bits; reference, target and expiration comparisons are signed. The retained
reference, rather than the current frame, chooses the direction.

### Environmental effect frame submission

The enabled pool's draw traversal runs only when at least one slot is active and
walks the retained list in order. Before checking a record's type or texture,
packed flag `0x20000` subtracts `nominal_ticks + 2` from its retained whole-map Y
coordinate with wrapping 32-bit arithmetic. This changes retained state even
when that record does not submit a draw. Disabled or entirely free pools skip
the traversal and this position change.

For type-one records, a positive signed texture descriptor admits the draw
wrapper. Construction zero-extends the lower sixteen-bit texture identity, so
identities with bit fifteen set remain positive. Identity zero skips the wrapper;
identity 65535 reaches the wrapper but submits no billboard. Packed flag `0x10000` selects caller scale 90; otherwise the caller
scale is 256. The alternate drawing path retains its own fixed geometry scale.

Ordinary type-one effect drawing submits the resolved texture with automatic
frame selection, independently of the effect's retained animation frame. The
retained frame still controls effect lifetime. Packed effect flag `0x20000`
selects the alternate drawing path, which submits that retained frame explicitly
instead. Both paths submit the retained integer position converted to single
precision. The alternate path bypasses the ordinary effect metadata lookup.

The ordinary path submits face brightness 255 and geometry scale equal to the
draw caller's integer scale divided by 256. Its default constant alpha is 255.
When texture metadata exists, metadata byte two plus one supplies alpha, and
metadata byte one is ORed into drawing flags initially set to one. The alternate
path submits geometry scale one half, constant alpha 140 and drawing flags
`0x201`.

The alternate path takes face brightness from the current world's
`world_ambientfx` value in `WORLD.INI`. World preparation stores that integer in
sixteen bits and replaces a stored zero with 128. World entry publishes it as a
signed sixteen-bit value before effect drawing. This scalar is separate from
`world_ambientrgb` and the ordinary effect path's fixed brightness 255.

The ordinary metadata lookup uses the current map's
[`FLAT` texture attributes](../formats/RGM.md#flat), shared with placed flats.
It splits the texture identity into the archive and seven-bit image IDs and
uses the first matching attribute record. Missing records leave the defaults
unchanged.

Both paths use a centered camera-facing quad. Its half extents are the source
image width and height multiplied by half the texture scale and geometry scale.
Texture-coordinate scale is one. Alpha, face brightness and frame selection
remain separate inputs to the shared billboard submission. The source bound uses
the same [quantized source-image diagonal](model-lighting.md#flat-image-dimensions)
as centered ordinary flats, multiplied by half the texture scale and geometry
scale. Reduced texture allocation dimensions do not determine the bound.

Shared billboard submission transforms the retained center into camera space and
tests its source bound before requesting a texture frame. A rejected bound
neither requests the image nor appends a draw packet. An accepted bound requests
the automatic or explicit frame, binds that frame's face and vertex sources,
and appends the packet in submission order. This image request precedes later
polygon processing; it does not require raster coverage or the placed-flat
spatial-cell selection.

Scene drawing submits ordinary spatial-list billboards before traversing the
environmental effect pool. Accepted effect packets append to the same queue in
retained pool order and participate in its shared
[alpha partition](model-lighting.md#flat-submission-order).

For centered billboard geometry, drawing flag `0x20` negates the local X half
extent and flag `0x40` negates the local Y half extent before corner generation.
These flags reverse the corresponding geometry axis while retaining the same
source-image bound. Drawing flag `0x100` becomes queued packet flag `0x10`.

Billboard drawing flag `0x200` becomes queued packet flag `0x20`, which disables
depth writes. Packets without that flag enable depth writes. The alternate
effect path therefore tests depth without updating the depth buffer. Its depth
comparison remains less-than, as on the ordinary path. Packet flag `0x10`
selects an always-pass depth comparison instead.

Both effect paths select point texture filtering and constant alpha. Alpha
values at least 254 select source-one/destination-zero blending; lower values
select source-alpha/one-minus-source-alpha blending. The constant-color alpha
channel retains the low byte of the converted alpha value.

### Contact death animation dispatch

A null receiver does nothing. Selector zero first checks for animation group 122
and starts it when available. Otherwise the dispatcher checks group 121 and
starts it when available. A nonzero selector goes directly to the group-121
check. Missing groups do not prevent the final death-selector assignment.

After a group starts, a receiver with combat state requests walk-node release
and stores its death default as 124 for group 122 or 123 for group 121.
Receivers without combat state skip those writes. Every nonnull receiver then
receives death selector eight.

### Combat death service

The direct-damage and scaled hit responses end a fatal hit with this service; the weapon-contact
caller uses the [contact death animation dispatch](#contact-death-animation-dispatch) instead.
A null receiver does nothing. A selected-player receiver does nothing while the session
death-suppression flag is set or hit inhibition is active. The service takes a selector: zero
prefers death group 122 when the receiver has it, any other selector or a missing group 121.

With attribute 69 equal to zero, the selected player stores the chosen group as the death-animation
override when none is set; it does not start the animation here, the
[selected death controller](#selected-player-death-controller) does. A nonplayer receiver instead
force-starts the group, releases its held walk nodes while storing death default 124 (group 122) or
123 (group 121), sends coordination message `0xd002` from itself to the
[active-state coordinator](#combat-coordination) when one exists, and clears its close-distance
pass counter and coordination selection state. A nonplayer without a combat record faults.

Every receiver then resets its combat action selector to -1. A receiver with death selector eight
releases its held walk nodes; any other receiver takes the combat reset: lifecycle selector one
(with its response byte cleared when the selector was not already one) and, when the killed flag
is set, the script-control hook enabled. The service sets death selector eight, clears combat
flag `0x04` and zeroes the close-distance pass counter and coordination selection state. The
selected player finally applies health -256 through the
[death-owning health](#selected-player-death-owning-health-mode) with no source actor, which
enables hit inhibition and ends the world loop once the death animation completes. The
death-owning armed branch and the nonplayer death admission also reset the combat action
selector to -1 after storing the death default.

### Hit-defence response

The candidate's hit-defence query requires the attacker to have combat state and
an action other than `-1` or nine. The defender's pending defence index, the word the
[attack executor](#attack-executor) resolves through the four defense slots (written by an ordinary
counter and by the selected player's [defence stance](player.md#armed-look-key-handling)), must
equal the attacker's current-motion counter-attack reference. The defender's attack-slot selector
does not take part. The attacker's defence
window must admit processing, and the defender must have combat flag `0x02` set.
Only then does the query test contact from attacker to defender.

An admitted contact miss clears defender flag `0x08` and returns zero. Earlier
admission failures leave this flag unchanged. A contact hit sets the attacker's
reverse-playback byte to one before playing the defender's impact sound. It then
emits message `0xd000` from defender to attacker, requests defender taunt category
4, clears intent masks `0x01` and `0x02` on the defender followed by the attacker,
and toggles defender flag `0x08`. It returns one. Other combat flag bits survive.

The impact cue comes from `COMBAT.INI` section `[misc]`: `sword_clank_1`
on an even world-loop counter and `sword_clank_2` on an odd counter. It uses
one random draw with bound 128 and pitch offset `draw * 16 - 1024` Hz.
The defender supplies the sound position. Sound dispatch and flushing precede
the response message.

### Defence contact window

The defence window compares the signed animation command cursor against the
current motion's `first_defend_frame` and `defend_duration`. It includes the
start and excludes the end: `start <= cursor < start + duration`. The end uses signed 32-bit
wrapping addition. A cursor before the start returns zero without reading the
duration. Otherwise the query reads start again, then duration, then the cursor
again before comparing the end. Zero and negative durations have no separate
admission rule. This query does not read the attack-contact mark or playback
direction.

### Motion contact selection

A motion whose `col_sphere_size` is zero uses the range/arc contact path. A
nonzero value selects sphere contact. Sphere contact resolves the current
motion's `col_vertex` through the actor's model-point query, then subtracts the
candidate actor's local position component by component with signed 32-bit
wrapping. The three differences feed the distance query. Contact succeeds only
when its signed result is strictly less than the current motion's
`col_sphere_size`. Equality rejects contact.

### Contact orientation

Contact orientation subtracts receiver local position from the supplied point
with signed 32-bit wrapping, then shifts each component arithmetically right
eight bits. The normalized vector's Y angle is converted to heading by
`(1024 - angle) & 2047`. The absolute wrapped difference from receiver yaw must
be strictly less than the current attack motion's `arc + arc_center`, with
signed 32-bit wrapping in the sum. The current retained attack slot and motion
selector supply both fields. Equality rejects orientation. This threshold differs
from the first-motion defence-selection arc interval.

### Range/arc motion contact

For a candidate with no combat record or a zero combat radius, the range/arc
contact path checks orientation, then range, then elevation. A failed check skips
the later checks. Orientation and range accept a nonzero result; elevation must
return exactly one. Range uses three-dimensional query-position distance.
Range and elevation use the receiver's retained attack-slot selector.

For a candidate with nonzero combat radius and attribute 126 equal to zero, the
path first resolves the candidate's retained model vertex. It tests orientation
against that point before querying horizontal query-position distance. The
distance must be strictly
between the minimum and maximum ranges of the retained attack slot's first
motion, and strictly less than the candidate's combat radius. Equality at any
of these boundaries rejects contact.

For a candidate with nonzero combat radius and attribute 126 nonzero, the
range/arc contact path resolves its retained model vertex, then calls the
sphere/segment query. The point coordinates and candidate radius shift
arithmetically right eight bits for this query. Both segment endpoints use the
same retained combat point of the receiver. The contact path returns the query's
result without converting it to a boolean.

With identical segment endpoints, the sphere/segment query tests the endpoint
against the sphere. Each endpoint-minus-center coordinate wraps at signed 32 bits
and stores as single precision. The squared components add in extended precision,
then their sum stores as single precision and truncates to a signed 32-bit integer.
An invalid integer conversion produces the signed minimum value. The radius
square and its subtraction of that integer both wrap at 32 bits. A nonnegative
signed residual returns one, including equality. A negative residual returns
zero because the segment has zero length. Radius sign is not separately rejected.

### Equipped-weapon combat endpoints

Drawn-weapon attachment requests use the signed RAEX `grip0` and `grip1` source
vertex indices. Stowed-weapon attachment requests use `scabbard0` and `scabbard1`.
Both requests use the actor's current animation handle and frame. The loader
copies these four signed words into the actor before attachment queries.

After selecting an equipped-weapon attachment, an actor with combat state retains
two endpoints and the attachment record. The base endpoint is the attachment's
floating position truncated component by component to signed integers. For the
tip, the local vector `(0, -(weapon_length << 8), 0)` uses signed 32-bit wrapping
and transforms through the attachment matrix. Each transformed integer component
shifts arithmetically right eight bits. That integer adds to the corresponding
floating attachment position in extended precision, then truncates to a signed
integer without an intermediate single-precision sum store.

An absent attachment clears the retained attachment record and leaves both
previous endpoints unchanged. An actor without combat state receives none of
these writes. The attribute-126 contact path uses the retained tip for both of
its segment endpoints.

Combat messages use 256 queues selected by attribute 125. Queue initialization
clears each queue; startup and combat reset both allocate eight entries per
queue. New Game and save restoration reset these queues. Actors retain separate
byte read cursors. A message contains
sender, receiver and code. Reading skips messages addressed to another actor,
advancing and wrapping the read cursor. A receiver-zero message is accepted by
any actor in that class. Sending a nonzero code writes at the class write cursor,
advances and wraps it, then clears the next message code. A zero code does not
send a message. The combat message stage consumes at most one addressed message
per invocation.

Coordination message `0xd002` uses the receiver's current combat target; `0xd003`
uses the sender's current combat target. Starting at the ordinary list head,
they clear coordination state for each living actor whose target equals that
target and whose coordination state is exactly one, excluding the target itself.
Traversal continues from each matched actor's successor without wrapping. A null
target skips this scan. Clearing or assigning coordination state also resets the
actor's consecutive close-distance counter. After the scan, `0xd002` assigns the
receiver coordination state one and aggression 50; `0xd003` assigns those values
to the sender. These assignments still occur with a null target. Neither message
changes the selected actor's combat target. Unrecognized nonzero message codes
are consumed without combat effects.


Message `0xd000` clears intent bits `0x01` and `0x02` on the receiver,
then on the sender. It next displaces the sender horizontally relative to the
receiver with a movement amount of 3000. This operation writes position, not
actor heading. A null actor skips the displacement.
The bearing uses the moving actor's local X/Z minus the other actor's local
X/Z, with signed 32-bit wrapping. The translation heading is zero on positive Z, 1536 on positive X,
1024 on negative Z and 512 on negative X. The translation uses the original sine and
cosine tables, stores the old coordinate and the final sum as single precision,
and truncates the final sum toward zero. Y, actor angles and transform matrices
are unchanged; this helper does not register a deferred map update.

After displacement, handoff runs only when the receiver's coordination state
is zero, the receiver is not the selected player, and the receiver's death
query returns zero. It searches from the ordinary list head for the first
actor other than the sender that has combat state, a nonnull target, death
selector other than eight and coordination state exactly one. A match receives
message `0xd003` from the receiver. Without a match, the receiver's coordination
state becomes one, resetting its consecutive close-distance counter. This
fallback does not assign aggression 50.

Message `0xd001` clears receiver intent bits before sender intent bits. If
receiver attribute 97 is nonzero, it tests weapon holding, then combat stance
only when not holding a weapon. When both tests are false, response processing
returns after the intent clears.

An admitted response draws a random value with bound 128 and requests the
receiver's attribute-86 positional effect with volume zero and pitch
`draw * 16 - 1024`. It completes the positional sound request, requests receiver taunt category
two, and reads the sender's combat action. Sender action eight requests receiver
weapon cleanup before the hit-reaction service runs with receiver then sender.

After the reaction returns, receiver action other than minus one or nine resets
the sender's action to minus one. The receiver's coordination state is then
read. State zero, nonselected identity and a zero death query admit the same
ordered coordinator search and fallback as the block response. The search
excludes the sender. A match receives `0xd003` from the receiver; without a match,
the receiver receives coordination state one without assigning aggression 50.

The ordinary `0xd005` response draws with bound 128, requests the attribute-86 positional effect
with volume zero and pitch `draw * 16 - 1024`, completes the positional sound request, requests
receiver taunt category two, then invokes the direct-damage response with
receiver followed by sender. After that returns, coordination state zero and
nonselected identity admit the ordinary coordinator search excluding the sender.
Unlike block and hit messages, this tail does not query death. A match receives
`0xd003` from the receiver; no match assigns receiver coordination one without
assigning aggression 50. This message does not clear either actor's intent bits.

Messages `0xd000`, `0xd001` and `0xd005` consume their addressed message before
performing response work. Message `0xd001` also clears the receiver's intent
bits before the sender's. Message `0xd005` does not perform those intent clears.

Ordinary entry processing is suppressed by an unpaused blocking slot whose call operand address is at or before the saved entry cursor. This test is separate from the later main-body blocking test and from direct interrupt invocation.

### Value Modes

The same opcode byte is interpreted differently depending on the calling context. There is no separate addressing-mode byte — the calling instruction determines how trailing bytes are consumed.

| Mode | Context | Effect |
|---|---|---|
| MAIN | Top-level statement | Performs assignment (writes to flag/variable/property) |
| LHS | Left side of `if` comparison | Reads value; consumes 1 extra operator byte |
| RHS | Right side of `if` comparison | Reads value; consumes 1 extra operator byte |
| PARAMETER | Argument to a task/function call | Reads value; consumes mode-specific padding bytes |
| FORMULA | Right side of an assignment expression | Reads value only, no extra bytes |

The number of trailing bytes consumed after an opcode depends on both the opcode and the current value mode. See [Operand Encoding](#operand-encoding) for per-opcode details.

## Bytecode Encoding

### Opcode Table

All integers are little-endian. The VM reads a leading opcode byte and dispatches. Unrecognized bytes are fatal.

| Opcode | Name | Encoding (after opcode byte) | Description |
|---|---|---|---|
| `0x00` | Task | `u16` func_id, `u8` param_count, params... | Call task on self |
| `0x01` | Multitask | same as `0x00` | Async task on self (non-blocking) |
| `0x02` | Function | same as `0x00` | Call function on self (returns value) |
| `0x03` | If | condition chain, `u32` end_offset, block | Conditional branch |
| `0x04` | Goto | `u32` target | Unconditional jump |
| `0x05` | End | `u32` target | Store target and return from the current invocation |
| `0x06` | Flag | `u16` flag_id, mode-dependent | Global flag read/write |
| `0x07` | Numeric | `i32` value | Immediate 32-bit integer |
| `0x0A` | LocalVar | `u8` var_index, mode-dependent | Local variable read/write |
| `0x0F` | ObjDot++ | object encoding, `u16` ref_id | Increment object property |
| `0x10` | ObjDot-- | object encoding, `u16` ref_id | Decrement object property |
| `0x11` | Gosub | `u32` target | Subroutine call (overwrites actor return continuation) |
| `0x12` | Return | *(none)* | Return from subroutine |
| `0x13` | Endint | *(none)* | End secondary thread |
| `0x14` | ObjectDot | object encoding, `u16` ref_id, mode-dependent | Object property read/write |
| `0x15` | String | `u32` string_index | String literal from RASB/RAST table |
| `0x16` | NumericAlt | `i32` value | Same as `0x07`; used for global flag function arguments |
| `0x17` | Anchor | `u8` anchor_value | Anchor assignment |
| `0x19` | ObjDotTask | object encoding, task encoding | Task call on named object |
| `0x1A` | ObjDotFunc | object encoding, task encoding | Function call on named object |
| `0x18` | Yield | none | Save the following cursor and end the invocation |
| `0x1B` | TaskStop | `u32` label | Remove matching actor task slots |
| `0x1C` | TaskPause | `u32` label | Pause matching actor task slots |
| `0x1D` | TaskResume | `u32` label | Resume matching actor task slots |
| `0x1E` | ScriptRV | `u8` expected, block | Branch on script return value |

The typed decoder leaves unimplemented opcode encodings explicit rather than interpreting them as successful operations.

### Operand Encoding

Flag (`0x06`) and LocalVar (`0x0A`) consume different trailing bytes depending on value mode:

**Flag (`0x06`)**

| Mode | Bytes after `u16` flag_id |
|---|---|
| MAIN | Formula (assignment to flag) |
| LHS, RHS | `u8` update byte: zero reads, 10 increments, 11 decrements |
| PARAMETER | `u16` padding (always `0x0000`) |
| FORMULA | *(none)* |

**LocalVar (`0x0A`)**

| Mode | Bytes after `u8` var_index |
|---|---|
| MAIN | Formula (assignment to variable) |
| LHS, RHS | `u8` update byte: zero reads, 10 increments, 11 decrements |
| PARAMETER | 3 padding bytes (`0x000000`) |
| FORMULA | *(none)* |

**ObjectDot (`0x14`)**

| Mode | Bytes after object encoding + `u16` ref_id |
|---|---|
| MAIN | Formula (assignment to property) |
| LHS, RHS | `u8` update byte: zero reads, 10 increments, 11 decrements |
| PARAMETER, FORMULA | *(none)* |

Conditional updates write the adjusted value before the comparison and compare that adjusted value. Both sides of every comparison are evaluated, including their updates and function calls.

In PARAMETER mode, NumericAlt consumes four bytes but returns the unsigned low 16 bits. A String parameter consumes four bytes but uses the low byte as its actor-local string index.

Native parameters use numeric, global flag, local variable, string, or object-property operands. Native calls are value expressions outside PARAMETER mode; PARAMETER dispatch does not evaluate nested native calls.

Object-targeted native calls evaluate local-variable and property operands against
the receiving actor. Native string parameters instead use the retained
native-command source described under [Retained health source context](#retained-health-source-context).
Normal interpreter entry establishes that source before entry continuation and
existing-task admission. The script-control mask `0x04` alternate path retains the
previous source, including its string table. A string parameter therefore need
not use either the native receiver's table or the alternate path's executing
actor's table.

A routed initial call preserves the retained source's string table. A later
normal interpreter entry on the receiving actor establishes that receiver as the
source before reevaluating its active task's string indices. The stored task
operand location remains in the calling actor's bytecode.

### Object Name Encoding

Used by opcodes `0x0F`, `0x10`, `0x14`, `0x19`, `0x1A`. A leading byte selects the object target:

| Byte | Additional | Object |
|---|---|---|
| `0x00` | `u8` padding | `Me` — the script's own actor |
| `0x01` | `u8` padding | `Player` — Cyrus |
| `0x02` | `u8` padding | `Camera` |
| `0x04` | `u8` string index | Named object from per-script string table (RASB/RAST) |
| `0x0A` | `u8` var index | Object name from local variable |

Named object lookup compares bytes with ASCII letters A–Z folded to lowercase. It searches registered object namespace names, rather than actor script names. The registry orders entries by their signed 16-bit runtime object identifier; lookup returns the first matching name. Registering an identifier already in that registry does not add another entry.

### Runtime object identifiers

Runtime identifiers are separate from the MPOB record's object id. The reusable actor pool has capacity `max(10, SYSTEM.INI [system] max_active_objects)`. An allocation uses the first free pool slot and initially assigns its zero-based index plus three as the actor's 16-bit identifier.

World object loading resets its identifier counter to the pool capacity plus three. It visits MPOB records in file order. Types two and six use separate object paths. Each other record creates an ordinary actor. A nonzero activation flag allocates from the reusable pool and increments the counter once; a zero flag allocates outside that pool. The ordinary actor path then increments the counter and assigns its low 16 bits as the actor identifier before loading its script and registering its namespace.

Objects created after world loading by `shoot`, `shootPlayer` or `DragonBreath` are allocated as standalone records rather than from the reusable actor pool. Their runtime identifier is zero. If the installed script supplies an object namespace, creation registers that namespace under identifier zero; repeated registrations of the same identifier retain the first entry.

CYRUS and GREMLIN player instances load independently of MPOB placements. Each uses identifier one; the initial player selector chooses CYRUS. The camera instance loads CAMERA independently of MPOB and uses identifier two. Fresh world loading creates and registers ordinary MPOB actors first, followed by CYRUS, GREMLIN and CAMERA. Each player role is temporarily selected during its own initialization. After GREMLIN initialization, world setup marks it excluded from ordinary object updates and restores CYRUS as the selected player.

### Separate map object script state

MPOB types two and six use separate object records rather than ordinary actor
records. Script binding uses the same authored instance allocator as ordinary
objects: each matching load advances the instance counter and, after the first
instance, advances the local-variable pointer by the variable count times four.
These objects therefore consume their authored local-variable allocation even
though they do not consume the ordinary actor identifier counter.

The separate-object update converts each record into a cleared temporary actor
and clears its attribute table. It restores the object's position, script
continuations, local-variable and string pointers, and native task-slot state.
After processing, it copies position, continuations and task state back to the
separate record. Attribute writes in that temporary actor are not copied back.
The separate-object traversal runs after ordinary actor traversal and retains
its own list membership. Outside editor mode, it maintains the optional light
record after script/task dispatch and before copying the temporary actor back
to its separate record. This light maintenance also runs when task-system
advancement is disabled.
Separate records always belong to their position's 512-unit map cell, regardless
of the MPOB activation byte. Loading prepends each record to its cell's separate
list. The cached view-window rebuild appends those lists in the same cell order
as ordinary traversal, skipping the four corners. It does not apply the ordinary
outer-border attribute filter to separate records.

### Initial ordinary motion state

Fresh ordinary allocation clears the complete actor record before applying
loader defaults. Its sliding state, falling state, queued-motion state, body
response flags and collision-transition flags therefore start at zero. These
runtime bytes are separate from the installed attributes and current animation
group. The ordinary update invokes queued motion when its state is nonzero,
using that state before the falling state when both are present.

### Reference Name Encoding

Object-property operands carry a little-endian `u16` reference ID after the object encoding. Property reads use the full reference ID to index the global references table loaded from the `[refs]` section of `SOUP386.DEF`. The high byte is not discarded. With native validation enabled, a reference outside the table enters the invalid-reference error path.

### Local pose properties

`pos.vx`, `pos.vy` and `pos.vz` access the resolved actor's local position.
`angle.vx`, `angle.vy` and `angle.vz` access its local angles. Each property
reads or writes the complete 32-bit field selected by its installed descriptor.
`Player` resolves the currently selected player before accessing the field.

These accesses do not substitute an attached pose, normalize angles, recompute
matrices or update other actor fields. Each write changes only its selected
field; subsequent actor processing performs the usual transform work. Ordinary
heading values use 2048 units per turn.

### Current animation property

`anim.current` reads the resolved actor's current animation group as a signed
16-bit value, extended to the script integer width. It preserves negative
values, including the no-current-group value of `-1`. If the actor has no
animation association, the read returns zero. Reading this property neither
requests nor advances an animation and does not change actor or animation state.

### Torch counter property

`torch_timer` accesses the resolved actor's torch-burning counter. Reads return
the unsigned byte as a script integer. Writes retain only the low eight bits of
the assigned value and change no other actor field. The property access itself
does not run torch effects or decrement the counter.

## Operators

### Arithmetic Operators

Assignments use a terminated list of (value, operator) pairs. The formula loop reads a value (any value-producing opcode in FORMULA mode), then an operator byte, repeating until the terminator.

| Byte | Operator | Arity | Operation |
|---|---|---|---|
| 0 | `;` (end) | — | Return result |
| 1 | `+` | binary | Addition |
| 2 | `-` | binary | Subtraction |
| 3 | `*` | binary | Signed multiplication |
| 4 | `/` | binary | Unsigned division |
| 5 | `<<` | binary | Left shift |
| 6 | `>>` | binary | Arithmetic right shift |
| 7 | `&` | binary | Bitwise AND |
| 8 | `\|` | binary | Bitwise OR |
| 9 | `^` | binary | Bitwise XOR |

The formula evaluator accepts only zero and binary operator bytes 1–9. Other operator bytes enter the fatal error path. Division by zero is not checked in software — it raises a CPU divide-by-zero exception caught by the C runtime signal handler.

The [ScriptReader and ScriptParser tools](https://github.com/Dillonn241/redguard-mod-manager/tree/main/src/redguard) interchange multiplication and division bytes. Engine bytecode uses 3 for multiplication and 4 for division.

### Comparison Operators

Used in `if` conditions. Each comparison pairs a LHS value, comparison byte, and RHS value. Ordered comparisons use signed 32-bit integers.

| Byte | Operator |
|---|---|
| 0 | `=` (equal) |
| 1 | `!=` |
| 2 | `<` |
| 3 | `>` |
| 4 | `<=` |
| 5 | `>=` |

### Conjunctions

Multiple comparisons in a single `if` are chained with conjunction bytes:

| Byte | Meaning |
|---|---|
| 0 | End of condition list |
| 1 | `and` |
| 2 | `or` |

Conditions are evaluated left-to-right with no operator precedence — each conjunction folds the running boolean with the next comparison result.

## Control Flow

### Conditional Branch (If — `0x03`)

Evaluates one or more comparisons chained with `and`/`or`. If the condition is false, the PC jumps to `end_offset` (absolute). If true, execution falls through into the inline block.

```
[0x03]
  repeat:
    [value: LHS mode]
    [u8 comparison]
    [value: RHS mode]
    [u8 conjunction]       // 0 = end, 1 = and, 2 = or
  until conjunction == 0
[u32 end_offset LE]        // false-branch target (absolute)
[block of instructions]    // executed when true; ends at end_offset
```

### Goto (`0x04`)

Unconditional jump. Sets PC to the `u32` target address (absolute).

### End (`0x05`)

Reads a `u32` offset relative to the actor's script base, stores that address as
the main continuation, and returns from the current invocation. It also clears
pending synchronization state for the actor's configured group and user channels
and releases the current activation reference when it refers to that actor.
These notifications do not clear the stored continuation or disable the actor.

When the actor's script remains eligible and has no blocking task, a later main
script stage starts at that continuation. Target zero therefore restarts at the
script base on that later invocation, rather than executing again immediately
or stopping permanently.

The installed ISLAND `FLAG2` script is 61 bytes: four blocking `PushAnimation`
calls request groups/counts `(1, 25)`, `(10, 1)`, `(1, 3)`, `(10, 2)`, followed
by `End` with target zero. Its finite task counts do not make the whole script
finite. The authored sequence can repeat while the actor remains eligible.

### Gosub / Return (`0x11`, `0x12`)

`Gosub` stores the address following its target operand in the actor's single return continuation, then jumps to the target. A later `Gosub` overwrites that continuation. `Return` resumes at the stored continuation and restores the actor's saved script-return byte. It does not pop a stack or clear the saved continuation. Subroutines share actor-local variables.

A fresh actor's return continuation is null. A `Return` executed while it is null (installed scripts reach this by entering a subroutine body with `End`, for example the `VILEDOG` actor) behaves like `Endint`: the invocation ends and the actor's main continuation becomes the script base. The same holds when the `Return` runs from the entry continuation; the main continuation is reset to the script base there too, unlike `Endint`, which preserves it. The game continues running; nothing faults.

### Endint (`0x13`)

Resets the active cursor to the script base and returns from the current invocation. Script-entry processing preserves the main continuation while advancing its separate entry cursor.

## Function Dispatch

### Call Types

Active tasks reevaluate their encoded operands on every task invocation. A task therefore observes changes to locals, global flags, and queried values while it is active. Literal arguments are a special case of this rule.

Native calls evaluate their parameters from the last authored argument to the first. The handler receives the resulting values in authored argument order. This evaluation order also applies when starting or revisiting a task; nested function side effects are observable by earlier arguments.

SOUP386 distinguishes three call types, encoded in the opcode byte:

| Type | Behavior | Script syntax |
|---|---|---|
| `Task` | Blocking — script waits for completion | `FunctionName(...)` |
| `Multitask` | Asynchronous — script continues immediately | `@FunctionName(...)` |
| `Function` | Immediate — returns a value | `FunctionName(...)` (context determines) |

Self-calls use opcodes `0x00`/`0x01`/`0x02`. Object-targeted calls use `0x19`/`0x1A` with an additional dispatch-type byte (`0x00` = task, `0x01` = multitask, `0x02` = function) when used as a top-level statement. In non-MAIN modes, the dispatch-type byte is absent and the call is always treated as a function.

The runtime function table keeps only each function's handler; the `task`, `function` or `multitask` keyword in `SOUP386.DEF` is parsed and discarded. A call therefore runs the handler of whatever function it names, whichever call type the script uses, and a handler that returns zero completes a task or multitask call at once. Shipped scripts use this for `FlatAnimate` and `FlatSound` (both return zero) in multitask form.

Object-targeted task dispatch differs from a self task statement. The receiver owns the task slot, and the calling body continues immediately. The self task statement checks the active result and yields when blocking. Both encodings must remain distinguishable even when the explicit object selector resolves to the current actor.

### Call Encoding

```
[u16 func_id LE]     // index into SOUP386.DEF function table
[u8 param_count]     // number of parameters (0 if func_id == 0)
[param_count × value in PARAMETER mode]
```

Function index 0 is always `NullFunction` (synthetic; prepended by the parser, not present in `SOUP386.DEF`). The `func_id` is multiplied by the function-table entry stride (49 bytes) to index the runtime table.

### Parameter Type Overrides

When a Numeric (`0x07`) or NumericAlt (`0x16`) opcode appears in PARAMETER mode, the 4-byte value may be reinterpreted based on the calling function:

| Functions | Type | Encoding |
|---|---|---|
| `ACTIVATE`, `AddLog`, `AmbientRtx`, `menuAddItem`, `RTX`, `rtxAnim`, `RTXp`, `RTXpAnim`, `TorchActivate` | `Dialogue key` | 4-byte ASCII string (RTX lookup key) |
| `LoadWorld` | `Map ID` | `i32` map identifier |
| `AddItem`, `DropItem`, `HandItem`, `HaveItem`, `SelectItem`, `ShowItem`, `ShowItemNoRtx` | `Item ID` | `i32` item identifier |
| All others | `Integer` | `i32` signed integer |

`ACTIVATE` and `TorchActivate` receive the dialogue key as a packed 32-bit
numeric value, not a string-table index. Its least significant byte is the
first RTX key character. RTX lookup folds ASCII uppercase bytes to lowercase.

### Ordinary entry gate

Ordinary entry processing uses the actor's saved entry continuation, distinct
from its main continuation. When the initial main entry offset is zero,
the separate entry continuation is absent. A nonzero initial main offset starts
main there and initializes the separate entry continuation at the script base.
An absent entry continuation makes ordinary entry processing return immediately.
Before running it, the engine checks occupied, unpaused blocking task slots.
A slot suppresses entry when its stored operand address is less than or equal
to the saved entry address. This is an address
comparison, not a comparison of offsets relative to each task's own script.
For programs within the same RASC allocation, adding each script's RASC offset
to its local cursor preserves that ordering.

When entry runs, its saved cursor temporarily becomes the interpreter's active
cursor. The resulting cursor is saved back as the entry continuation, and the
main continuation is restored. A yielding entry therefore resumes from its
saved position. `Endint` resets that entry continuation to the script base.

<a id="actor-task-controls-and-return-state"></a>

### TaskStop, TaskPause, and TaskResume

An actor has six task slots shared by its entry and main invocations. Existing-task processing traverses those slots in allocation order, skips paused slots, reevaluates encoded operands, and removes a slot when its handler reports completion. Unpaused blocking slots suppress the subsequent main-body invocation; multitask slots do not. Reissuing a call whose operand address is already in a slot does not execute its handler again. Task labels resolve against the receiving actor's script base and match the byte immediately after the labelled dispatch opcode. Self task calls store the operand address immediately after their Task or Multitask opcode. Object task calls store the operand address immediately after the embedded dispatch byte following the two-byte object selector. Routing retains that encoded operand location; it does not rebase foreign bytecode into the receiver's script. `TaskStop` removes every matching slot and clears its blocking and pause state. `TaskPause` and `TaskResume` set and clear the matching slots' pause state.

The `KillMyTasks` function takes no arguments and returns zero. It clears all six of the receiver's task slots at
once, whatever their operand addresses, including their blocking and pause state and retained handler scratch,
without invoking any handler. It acts immediately, so a task started by a later statement of the same invocation
occupies a fresh slot.

The initial task call invokes its native handler, then allocates a slot independently of that handler's return value. A self blocking task yields after this allocation, even when its first native invocation reports completion. Completion is applied by a later existing-task pass. An object-targeted task allocates on the receiver and the caller continues, including when the selector resolves to the caller itself.

Each active slot has a 16-bit invocation counter. Allocation sets it to one. An eligible existing-task pass computes its elapsed input as the low 32 bits of counter times the current frame scale, shifted right by eight, then increments the counter with 16-bit wrapping. Paused slots do not advance. The initial native call receives elapsed input zero.

#### Wait, WaitOnTasks, and WaitNonMulti

`Wait(duration)` stays active while the elapsed input is less than the wrapping 32-bit value `duration - 1`, compared as signed integers. `WaitOnTasks` stays active when more than one unpaused slot is occupied. `WaitNonMulti` stays active when more than one unpaused blocking slot is occupied. These count tests include the current wait slot when it has already been allocated.

#### Anchor, Yield, and ScriptRV

`Anchor` stores its operand byte as the actor's anchor state. `Yield` retains the following cursor without issuing the `End` notifications.

`ScriptRV` reads an expected byte. On mismatch it skips the next five bytes. On a match, a following `Return` byte advances the cursor by four bytes; a following `Gosub` saves the current script-return byte before the control instruction runs. Other following instructions remain eligible. `Return` restores that saved byte.

### Group synchronization channels

Actor data byte 29 names the actor's group channel (1 through 255; zero means none), byte 30 its user
channel, and a nonzero byte 43 excludes the actor from its group channel's rendezvous. Actors join their
channel's member list when they are created. World initialization enables the system and clears every
channel's released value; with the system disabled, or for an actor without a channel or member entry,
`SyncWithGroup` completes immediately.

Each member holds a posted value, initially zero. `SyncWithGroup(group)` is a task. Each invocation stores
`group | 0x80` (the group's low eight bits with the posted mark) as the actor's posted value, then compares the
channel's released value with that stored byte. When they are equal it clears the mark, keeping the group, and
completes; otherwise it keeps waiting. `ResetGroupSync` and the `End` operation set the actor's posted value to
zero.

Once per world-loop iteration, after the family system update, every channel is updated. The channel carries the
mark when every non-excluded member's posted value has its mark. The channel then releases a group: it scans
cyclically over 0 through 127 for a value that some non-excluded member holds (its posted value without the
mark; a member that has not posted holds zero, so zero counts), starting at the previous released byte itself.
Because a marked released byte is at least 0x80, a marked release starts the scan just after its group, while an
unmarked release starts at its own group. The first value found, combined with the mark, becomes the released
byte; when none is found the released byte is just the mark. A member that completed keeps its group but loses
its mark, so the channel advances to the next posted group unmarked until that member posts again or ends.

<a id="animation-task-ownership-and-stopping"></a>

### PushAnimation and TaskStop: animation ownership

Each task slot saves and restores its native handler state independently.
Multiple `PushAnimation` slots therefore retain separate not-started/started
phases while requesting and observing the same receiving actor's animation.
Each invocation sees animation changes made by earlier calls; the animation is
not copied into the task slot.

`TaskStop` clears matching operand slots and their blocking and pause flags.
It does not invoke the native handler or restore earlier animation state.
Stopping a `PushAnimation` task therefore leaves its animation request and
current playback state in place.

`PushControlAnimation(group, count)` is a task with the same two phases as
`PushAnimation`. Not started, it requests the group through the ordinary
request unless the receiver's current group already equals it; in that case it
sets the loop count and becomes started. Both outcomes keep the task pending.
Started, it completes when the current group no longer equals the group.
Otherwise it turns the receiver and stays pending: the heading becomes the
heading minus the scaled right turn amount plus the scaled left turn amount,
modulo 2048, using the [ground-turn accumulators](player.md#ground-turn-arithmetic)
and frame scale. The accumulators update every input update, whether or not
the player controller runs.

### Player alignment ownership and targets

`PlayerLineUp(angle, x_offset, z_offset)` moves the selected player relative to
its receiving actor. Both distance modes move the player; neither moves the
receiver. Its return value is also stored in the shared player-lineup result
used by [activation admission](hud.md#activation-prompt).

The first invocation in a native handler state selects the distance mode.
It compares the shared integer three-axis distance between the effective player
and receiver positions against `SYSTEM.INI` `lineup_distance * 256`. A strictly
greater distance selects approach mode; equality selects close alignment.
Subsequent invocations retain that choice in the native state. The initialized
marker, mode, fallback flag and retained close-alignment reference belong to
that handler state. Active task slots save and restore them independently.
They are separate from the shared activation-admission result.

For the normal target calculation, add `angle` to the reference actor's heading
and wrap to 2048. Rotate the supplied horizontal offsets by that heading and
add the reference actor's local X/Z. The final player heading is the rotated
heading plus 1024, wrapped to 2048. Approach mode uses the current receiver as
reference. Close alignment retains its first reference in native state and
uses that actor on later invocations. The target position is recalculated on
each invocation; it is not a captured destination.

In the normal path, approach mode faces the destination before forward movement. Close alignment
faces the final heading first, then chooses forward, sideways or backward
movement from the relative destination bearing. Reaching the destination does
not by itself finish alignment. The player must finish turning to the final
heading and request its default animation. A completed alignment clears the
player's alignment flag and returns zero; ongoing alignment returns one.
In approach mode, a retained fallback flag or a nonzero packed collision-response
word masked by `0x01e2` switches to facing the receiver's current local X/Z.
Completion in that branch also sets the native return-state byte to 6. Close
alignment's fallback instead turns to the final heading derived from its
retained reference, requests the default animation, and sets that byte to 6.
Both completion branches clear the player's alignment flag.

Approach mode's movement step runs after the player has finished facing the
destination and the movement-distance check permits movement. It is one call of
the player's `WalkForward` helper with duration 1, preceded by a saved copy of the
player's position, step counter and animation state. If the bearing from the
new position to the destination, relative to the player's heading, then lies from
256 through 1792 (the destination is no longer ahead), the step is undone, the
player's X/Z are set to the destination and the retained fallback flag is set.
After the movement, a nonzero collision-response word masked by `0x01e2` together
with a collision counter above 8 also sets the fallback flag. Approach mode
returns one on every call that does not complete.

Close alignment's movement step runs when the player has finished turning to the
final heading and the movement-distance check permits movement. Let `b` be the
player's heading minus the heading of the displacement to the target (the heading
the facing step turns to), wrapped to 2048 (the same quantity approach mode tests). The helper is
chosen by `b`: right for 257 through 768, backward for 769 through 1280, left for
1281 through 1792, forward otherwise. Each helper first saves the player's
position, packed step counter, animation state and the two animation words that
follow it, then calls its stepper with duration 1: `MoveRight`, `MoveLeft`,
`WalkForward` or the walking backward step. It keeps the step when its test
passes, otherwise it restores everything it saved, clears that direction's shared
called flag, sets the player's X/Z to the target and reports failure, which sets
the retained fallback flag. With `b'` the bearing measured again after the step,
the keep tests are: right, `b'` from 257 through 767; left, `b'` from 1281
through 1791; forward, `b'` above 1792 or below 256; backward, the horizontal
distance to the target no greater than before the step. After the step, a nonzero
collision-response word masked by `0x01e2` also sets the fallback flag. Close
alignment returns one on every call that does not complete.

### PlayerPickupLineUp and PlayerParallelLineUp

`PlayerPickupLineUp(angle, x_offset, z_offset)` and `PlayerParallelLineUp(angle, x_offset, z_offset)` are
tasks that move the selected player relative to the receiving actor; `ObjectPickupLineUp` and
`ObjectParallelLineUp` run the same two movers on another actor. They do not store the shared
player-lineup result. Each invocation first pushes a collision query frame for the player (position, a
radius from the player's model record scaled by a constant, or 200 when it has none,
height word) and pops it afterwards. Unlike `PlayerLineUp` the offsets are added to the receiver's local X/Z
unrotated: the target is the receiver's position plus the offsets (Y from the receiver), and the target
heading is the receiver's heading plus `angle`, wrapped to 2048.

The pickup mover returns zero when done and one while continuing. On its first invocation (elapsed time zero)
a blocked straight path to the target (a segment of at most `0x20000` units, tested against objects but not
the receiver or the player) selects the parallel mover for the rest of the task; the choice is retained in
the task's native state. Otherwise each invocation, in order:

1. A nonzero packed collision-response word with an age above 8 clears nothing but abandons the task: return
   state 6, return zero.
2. The player's roll is leveled and the player faces the target; while still turning it returns one.
3. The facing arrival test snaps onto the target: all four called flags are cleared, the default animation is
   requested and the task returns zero.
4. When the movement-distance check permits movement, one forward close step toward the target runs (as in
   close alignment); a failed step also clears the called flags and completes as in 3.
5. Otherwise a stale collision response abandons the task as in 1, clearing the player's alignment flag, and
   the task returns one.

The parallel mover works from the final heading `angle + 1024` (wrapped): when the player's heading is more than
768 from it the player turns to face `angle + 2048`, backs toward the target with a backward close step while the
movement-distance check permits it, and completes once the backward step fails and the turn to the final heading
has finished (called flags set); otherwise it turns to the final heading (returning one while turning), snaps on
arrival (called flags set, default animation requested, return zero) and otherwise takes the close step that the
relative bearing selects, as in close alignment, returning one unless the collision response is stale.

### Alignment distance thresholds

Alignment position tests use local Q8 X/Z and the shared integer distance
function with Y set to zero. They select a speed byte from the current
animation group and actor attributes. Missing animation or attributes selects
zero. Other groups also select zero.

| Current animation group | Speed attribute index |
|---|---|
| 18, 30, 142 | 8 |
| 19, 143 | 9 |
| 20, 21 | 12 |
| 22, 23 | 13 |
| 24 through 27 | 10 |
| 31 | 19 |
| 92, 93 | 17 |

Let `speed` be that byte and `scale` the Q8 frame scale. The alignment arrival
test uses `(((2 * speed - 1) << 8) * scale) >> 8`. The facing helper's arrival
test instead uses `(((speed + 1) << 8) * scale) >> 8`. Products wrap to signed
32 bits and the final shifts are arithmetic. A horizontal distance less than
or equal to the threshold snaps local X/Z to the target and registers a map
update. Y remains unchanged. With speed zero and a positive ordinary scale,
the first threshold is negative and cannot admit a nonnegative distance.
The facing helper's test can still snap in that case.

The movement-distance check is a third test. It uses
`((speed * 512 - 1) * scale) >> 8`, with the same wrapping and signed shift.
It permits a movement attempt for actor state 7, speed zero, a nonpositive
threshold, or a distance strictly greater than the threshold. It does not
snap or register the actor.

Alignment turning uses the [support-response turn rules](player.md#ordinary-slope-response),
including selected-player camera updates. Returning to the default animation
reports success on every normal return; it does not wait for playback to finish.

### Player turn, speed, arctangent and location-distance queries

`PlayerArctan()` returns the arctangent angle (0 through 2047) of the horizontal displacement from the
selected player to the receiver (or to the stored fallback position when no player is selected), the
negation modulo 2048 of the bearing `PlayerLooking` uses.

`PlayerTurned()` returns 1 when any of the selected player's three angles differs from the angles saved
at the ordinary update's entry, otherwise 0. `PlayerTurnedDirection()` returns the sign of the change of
the player's negated yaw (`(2048 - yaw) & 2047`) since the saved one: 1 when it grew, -1 when it shrank
and 0 when unchanged, with the sign inverted when the change exceeds 1024. Both return 0 for an actor
without a model handle.

`PlayerMovementSpeed()` returns an attribute byte of the selected player selected by its current animation
group: groups 18 and 30 attribute 8, 19 attribute 9, 20 and 21 attribute 12, 22 and 23 attribute 13, 24
through 27 attribute 10. Other groups, and a player without animation state or attributes, return zero.

`DistanceToLocation(location)` returns the rounded Euclidean length, in Q8 units, from the receiver's
position (its linked position when it has one) to the location, selected as in `MoveToLocation`.

### PlayerOnScape

`PlayerOnScape()` returns 1 when the selected player's last ground-response record is a terrain contact and
0 otherwise. The ground update clears the record's kind byte, sets it to `0x40` for a terrain contact and to
`0x20` for a model or actor face, and retains the record for the selected player after its update.

### PlayerStatus

`PlayerStatus()` classifies the selected player and returns the class of the first matching rule, or
zero. The rules test, in order: the queued-motion byte (1 gives 1, 2 gives 2, 8 gives 3, 3 gives 4),
the falling byte (2 gives 5, 8 gives 6, 3 gives 7), a nonzero sliding byte (8), the living-state
byte (10 gives 9, 11 gives 10, 20 gives 11, 21 gives 12), the current animation group (18 or 30 gives
15, 19 or 23 gives 16, 24 or 25 gives 17, 26 or 27 gives 18, 20 or 21 gives 19, 22 gives 20), and last
whether the player's local position differs from its position saved at the update's entry (21).
Classes 13 and 14 repeat the living-state tests of 9 and 10 and are never returned.

### BounceObject

`BounceObject(power, speed, steer)` is a task that launches its object (the selected player in
installed content: `BounceObject(160, 40, 1)`) up and forward, then leaves the descent to the ordinary
falling controller. It reports one while it runs and zero when done. The lifecycle byte moves through
30 (entry requested), 31 (rising) and 32 (falling). The first invocation refuses, ending the task with no
effect, when the item system is not ready, the lifecycle byte is 30 or 31 or in 10 to 19, the surface
is deep water (2 or 4), or `power` is zero. Otherwise it clears the falling, queued-motion and sliding
bytes and the first two motion-vector words, sets lifecycle 30 and requests animation group 114 with a
weapon held, 66 with a burning torch, otherwise 65.

Every invocation then runs, in order:

1. While the lifecycle byte is 30: if the current animation group is not 114, 66 or 65, clear the same
   motion bytes and words and report one. Otherwise set lifecycle 31 and set the lift to attribute 32
   (jump height in map units) times 256.
2. If the object is the selected player and `steer` is nonzero, subtract the ground-turn delta (scaled
   right amount minus scaled left amount) from the heading, modulo 2048.
3. Move forward along the heading by `((128 - speed) << 8) / 3` (truncating division) times the frame
   scale, shifted right by 8, in Q8 units.
4. While the lifecycle byte is 31: subtract `lift * scale >> 8` from Y and `((256 - power) << 4) * scale >> 8`
   from the lift. A lift above zero requests group 112 with a weapon held or the stance bit `0x04`
   set, 38 with a burning torch, otherwise 39, and clears the motion bytes and words as above. A lift of
   zero or less starts an ordinary fall (fall origin Y plus attribute 32 times 256, timer 30, falling
   byte 1), requests group 113 (forced) with a weapon held or the stance bit set unless the current
   group is already 115, otherwise group 57 with a burning torch or 56 (forced), and sets lifecycle 32.
   Either way the call reports one.
5. Otherwise, when the falling byte is zero set lifecycle 0 and report zero; else report one.

The lift shares the fall-origin word. "Weapon held" is the ordinary holding-weapon predicate.

### PlayerFaceObj

`PlayerFaceObj` is a task that turns the selected player to face its receiving actor. It
reports one while the player is still turning and zero when done, and each invocation sets or
clears the player's alignment flag accordingly. The alignment flag is the hook-disabled bit that
`DisableHook` sets, so the player's native hook (its controller) does not run while the player is
being aligned.

An invocation first checks the player's packed collision-response word (the body-response
byte, the transition byte and the two auxiliary bytes). When the word is nonzero and the
collision-response age is above 8, it sets the native return-state byte to 6, clears the
alignment flag and reports zero without turning.

Otherwise it applies the alignment roll leveling, then the facing step. Using the player's
[alignment speed](#alignment-distance-thresholds), a horizontal distance (Y zero, local Q8 X/Z)
to the receiver at or below the facing arrival threshold snaps the player's X/Z to the
receiver's, registers a map update and reports zero. Farther away, the target heading is the
negated arctangent angle of the horizontal displacement from the player to the receiver
(the heading the `PlayerLooking` bearing reports), wrapped to 2048, and the player turns to
it by the [support-response turn rules](player.md#ordinary-slope-response): zero when the
heading already matches or the turn snaps to it, otherwise nonzero. A zero result clears the
alignment flag and a nonzero one sets it, and the task reports that result.

<a id="attribute-reads-and-writes"></a>

### GetAttribute, SetAttribute, GetMyAttr, and SetMyAttr

`GetAttribute` reads an unsigned byte from the receiving actor's attribute table and returns zero when the table is absent. `SetAttribute` returns zero. When the table exists, it writes the low byte of the supplied value. Writing attribute 6 also copies that byte to attribute 115; writing attribute 1 also copies it to attribute 119. These two handlers do not check the attribute index range.

`GetMyAttr` reads an unsigned byte from the receiving actor's attribute table. It returns zero when the table is absent or the index is outside 0–255. `SetMyAttr` returns zero and ignores an absent table or an out-of-range index. A valid write truncates the supplied value to its low byte. Attribute 44 also stores that truncated byte as the combat death-default group when the actor has a combat record and its object-update exclusion is nonzero. No other index has a write-time effect: attachment, rotation, family and model attributes are read where they are used, and a write neither rebinds the family nor refreshes the model.

### Initial attribute copy and defaults

Loading an actor creates an independent 256-byte copy of its definition's RAAT
attributes, then applies these adjustments before registering its groups:

| Attribute | Initial adjustment |
|---|---|
| 31, `attr_jump_time` | A signed byte value at or below zero is replaced by the low byte of the effective system jump time. Thus zero and byte values 128–255 use the fallback. |
| 32, `attr_jump_height` | Zero is replaced by the low byte of the effective system jump height. |
| 35, `attr_draw_frame` | Zero becomes 2. |
| 34, `attr_sheath_frame` | Zero becomes 10. |
| 25 and 26, `attr_master` and `attr_slave` | Both become zero when `[debug] disable_family` is exactly 1. |
| 115, `attr_reserved_weight` | Copies attribute 6, `attr_weight`. |
| 119, `attr_reserved_collide` | Copies attribute 1, `attr_collide`. |

The effective jump settings come from `[system] jump_time` and `jump_height` in
[SYSTEM.INI](../config/system-ini.md). Values below 1 use 6 and 80 respectively.
These load-time defaults do not run again on ordinary attribute writes.

Actors loaded through the animation model-table path also replace attribute 125,
`attr_mess_group`, with the next value from a shared counter. The counter returns
its current value, then increments modulo 256. Opening an actor-definition map
resets this shared counter to zero before reading the map records. The counter
is shared across actor definitions and instances, rather than restarting for
each definition. The static-model shortcut and actors without models do not
take this assignment path.

A type-1 MPOB placement has a subsequent model fallback. It runs only when
actor-definition loading left both the animation model table and the static
model absent, and the placement's model name is nonempty. If the placement's
`is_static` byte is exactly one, it loads a static model without assigning a
message group. Any other byte value builds an animation model table, using
that byte as its frame count. When the actor has an attribute table, this
fallback replaces attribute 125 with the next shared counter value. A zero
frame count still takes the animation-table branch.

<a id="attribute-default-group-side-effects"></a>

### SetAttribute and SetMyAttr: default-group side effects

`SetAttribute` first writes the low byte to attribute 44. Only when `IsDead`
returns true does it release the actor's two held walk-node reservations and
store the full supplied value as the combat death-default group. A node's ownership byte is cleared only when it matches
the low byte of the actor identifier; both actor-held node indices are then
cleared. This operation does not reset the animation cursor, current group,
or loop countdown.

`SetMyAttr` writes the combat death-default group only when a combat record
exists and the object-update exclusion is nonzero. It stores the truncated
attribute byte and does not release walk-node reservations. This exclusion
check is distinct from the `IsDead` predicate used by `SetAttribute`.

The combat death-default group and the animation playback default are separate state.
Ordinary actor updates select the playback default using actor state; these
attribute writes do not directly replace that selection step. See
[ordinary default selection](animation.md#placed-actor-initialization-and-eligibility).

<a id="player-distance-and-facing"></a>

### PlayerDistance and PlayerFacing

`PlayerDistance()` returns the sum of the absolute X, Y and Z differences
between the selected player's current position and the receiver's effective
position. Positions and the result use the runtime coordinate scale of 256
units per world unit. Without a selected player, it uses the global fallback
position. A global override selects the fixed result `9999999` instead of
calculating distance. The override is the nonzero `neversaydie` cheat state.
Cheat initialization clears it before loading the optional `REDGUARD.CHT` file;
without that file it starts disabled.

Fresh ordinary actor allocation starts without a linked-position record.
This record is separate from the attribute-based master/slave attachment.

Effective receiver position follows the same selection used by
[positional sound](sound.md#distance): a nonzero `attr_slave`
selects attached position; otherwise a valid linked positioned record supplies
position, with the receiver's ordinary position as the final fallback.

`AcuratePlayerDistance()` is `PlayerDistance` with the rounded Euclidean length of
the same three-axis difference instead of the sum of absolute differences. It
uses the same positions, the same fallback position without a selected player,
and the same fixed `9999999` result under the `neversaydie` override. The length
follows the [node-marker distance arithmetic](#node-marker-distance).

`PlayerFacing(offset, tolerance)` compares orientations. It adds `offset` to
the receiver's effective yaw, wraps the sum modulo 2048, and compares it with
the selected player's current yaw. The signed difference is wrapped into
`[-1024, 1024]`; the result is one when its absolute value is less than or equal
to the signed `tolerance`, otherwise zero. The offset and tolerance are full
signed 32-bit arguments. The receiver's attached yaw is used when `attr_slave`
is nonzero; otherwise its ordinary yaw is used. This function requires a
selected player. See [coordinate and heading units](player.md#coordinate-and-heading-units).

`PlayerLooking(tolerance)` tests the selected player's horizontal orientation
relative to the receiver. Subtract the receiver's effective X/Z position from
the selected player's ordinary X/Z position, retaining signed 32-bit differences.
Convert that displacement to the [quantized actor heading](player.md#direction-recovered-from-displacement).
This query retains signed 32-bit wrapping absolute values and signed comparisons
during bearing reduction. The minimum signed coordinate difference therefore
does not behave like a widened positive magnitude.
Compare it with the selected player's current yaw plus 1024, wrapped modulo 2048.
The result is one when the absolute wrapped angular difference is less than or
equal to the signed 32-bit tolerance, otherwise zero. A negative tolerance always
rejects. The query requires a selected player and uses the same effective receiver
position selection described above. It does not perform a visibility or line-of-sight
query.

`PrintSingleParm(value)` formats the receiving script's name (or `Unknown script`
when the actor has none) and the signed integer as `<name> <value>` and writes the
text to the first free of the console's 66 text lines. It drops the text when no
line is free and returns zero.

`PlayerInSight()` is true when the selected player is within 262144 units of the
receiver and nothing blocks the line between them. The distance is the rounded
Euclidean length of the difference between the two actors' local positions, with
no effective or attached position substitution; a longer distance is false and
exactly 262144 passes. The segment between the two local positions, in whole map
units, is then tested against each entry of the retained candidate model list, in
order, exactly as the combat "In sight" test described under the combat decision rules does. An entry that is not
hit, or whose owner is the selected player, does not block; any other hit makes
the result false.

The candidate model list is one engine-wide list that every candidate gather replaces. A receiver that
gathers no candidates of its own (an actor without collision) tests against the list the most recent gather
by any actor left, which is empty before the first gather.

<a id="node-marker-distance"></a>

### AtNodeMarker: node-marker distance

`AtNodeMarker(marker, distance)` returns one when the receiver's ordinary local
position is within the supplied distance of the indexed marker, otherwise zero.
It uses all three axes and does not select attached or linked positions.
Marker coordinates are converted from map units to runtime units by a left
shift of eight bits; coordinate subtraction retains the low 32 bits.

The distance is Euclidean. Each signed difference is first stored as a
single-precision float. The sum of squares passes through a double-precision
store before an extended-precision square root; the root is stored at single
precision and converted to a signed integer with nearest-even rounding. Rounding
the root to double precision before the single-precision store can change the
result at a single-precision midpoint. The result is compared with the
signed `distance` argument using an inclusive bound. The distance argument
therefore uses 256 units per map unit.

Marker lookup is fatal for an out-of-range or absent marker. While a world is
loaded, the lookup reads that map's own marker table: the index must be below
the map's marker count and the presence byte must equal one. A fixed 128-slot
table that accepts any nonzero presence byte is read only when no loaded map is
active. The marker argument is an unscaled signed index, distinct from the
distance units.

### Navigation map validation

Walk-map construction clears its runtime disabled byte, so newly loaded maps
are enabled. This byte is separate from installed node reservation bytes.

Common actor initialization clears the current walk map and sets the held
current-node, next-node and final-destination indices to -1. Later actor loading
or navigation can assign a map and node; raw allocation zeroes do not describe
these initialized navigation indices.
The previous-next record starts at allocation zero; common initialization does
not overwrite it.

Map placement loading supplies the authored local position before loading the
actor. Actor loading then assigns its current node and map through the general
nearest-node query, using each local coordinate shifted right by eight. This
initial association does not add the actor's Y extent and does not filter node
reservations. It is separate from the combat nearest-free-node search.

An actor without a current walk map clears its current-node, next-node and
final-destination indices to -1. An enabled map remains valid when each of those
indices is below its node count; the -1 unset sentinel is allowed.

An index at or above the node count invalidates the map. Validation releases
held current/next reservations, clears all three indices, and removes the map.
A disabled map also releases held current/next reservations and removes the map,
but retains the final-destination index until the next validation observes the
absent map.

Reservation release clears a node's ownership byte only when it matches the low
byte of the actor's runtime identifier. It clears the actor's current and next
indices even when another actor owns the node. An index at or above the node
count does not address a node during release.

Current-node assignment releases the held next node before the held current
node. When a next node was held, it first copies that index to the actor's
previous-next record. Next-node assignment instead releases current before
next and leaves the previous-next record unchanged. Both operations clear the
released indices even when another actor owns their nodes.

After releasing, assignment claims the requested node only when its ownership
byte is zero or matches the low byte of the actor identifier. A successful claim
writes that low byte and the corresponding current or next index, and returns
one. A rejected claim returns zero with both held indices cleared. With no map,
both held indices are cleared and assignment returns zero without changing the
previous-next record. Neither assignment changes the final destination.

Validation and held-node release preserve the previous-next record, including
when they invalidate or remove the map.

### Nearest free walk node

Combat perception can search all loaded walk maps for a free node before
navigation begins. This differs from the general node-selection query because
only nodes with a zero reservation byte qualify. The search reads the actor's
query position, shifts each coordinate right by eight, and adds the actor's
positive Y extent to the shifted Y coordinate. Node X/Z are unsigned 16-bit
coordinates and node Y is signed 16-bit.

Maps and nodes retain their loaded order. The search minimizes Manhattan
distance with an initial exclusive bound of 4096. Equal distances retain the
first candidate. An improving candidate below distance 12 returns immediately,
even if a later node would be closer. The shared winning map is cleared before
searching. A miss leaves it absent and returns node index -1.

### Nearest node within one walk map

Point navigation can search its current walk map for the destination node. The
search takes map-unit coordinates, ignores reservation bytes, and rejects an
absent or disabled map. It minimizes Manhattan distance with an initial exclusive
bound of 4096. Equal distances retain the first node, and an improving candidate
below distance 12 returns immediately. The shared selected map is cleared first.
A hit sets it to the supplied map; a miss leaves it absent and returns -1.
This search does not select a different map.

### Walk-route search and next-hop selection

Walk-map loading sizes the shared route queue and visited array to the largest
loaded map's node count plus eight. Search starts with the current node at depth
one. An unset current node returns -1; an equal current and destination node
returns that node before testing map availability. Other searches reject an
absent or disabled map.

The forward search processes its queue in order and visits each node's links
in authored order. A link qualifies when its unsigned cost is at most 2048, its
target passes node admission and has not been visited, and the expanded node's
reservation byte is either zero or equal to the requesting identifier. The
reservation test belongs to the expanded node, not the linked destination.
Qualifying targets are marked visited and queued with the parent's depth plus
one. Finding the destination starts backtracking immediately. Exhausting the
queue or its capacity returns -1. Costs filter forward traversal; they are not
accumulated as a shortest-path distance.

Backtracking does not retain the parent that discovered a node. For each node,
it scans earlier queue entries in reverse order for depth one lower. It looks
up the first authored link from each candidate to the current node. Zero cost
is not a predecessor. Among nonzero links, the smallest low fourteen cost bits
wins; equal costs retain the first reverse-scan candidate. Backtracking repeats
until reaching the initial queue entry and returns the next node after it.
Failure to find a predecessor returns -1. The forward admission filters are not
reapplied during this predecessor scan. Duplicate links can therefore affect
backtracking through their first authored cost even when another duplicate
caused forward admission.

Each predecessor scan clears two shared link-result flags. An improving link
sets the first flag when cost bit 14 is set and the second when bit 15 is set.
These flags are not cleared by a later improving link within that same scan.
The final scan's flags accompany the selected next hop.
A shared backtracking-inhibition word equal to one returns -1 and clears both
flags before scanning. Other values allow backtracking.

When the shared player-proximity exclusion switch is zero, node admission
returns true without reading the player. When enabled, a valid node in a
present map is rejected only when its rounded three-dimensional distance is
strictly below the shared signed threshold. Node X/Z are unsigned and node Y
is signed. The player coordinates are the local actor position shifted right
by eight. An absent map or an index outside its node range returns true without
a distance calculation.

### Point-navigation command state

The point-navigation wrapper copies four retained actor values into shared
command scratch before the navigation core runs and copies them back afterwards:
a 32-bit command value, a 16-bit command word, a phase byte and a counter byte.
The copy-back occurs for either navigation-core return value. Alternating actors
therefore retain each actor's own four values rather than inheriting the last
actor's scratch contents.

After copying command state back, the wrapper measures rounded horizontal
Q8 distance between the local position and the retained previous local position.
It compares that distance with the frame scale shifted left by 13 with 32-bit
wrapping, interpreted as signed and arithmetically shifted right by eight.
A strictly greater distance restores all three local coordinates from the
previous position. Equality retains the new position. This rollback does not
undo command-state copy-back or change the navigation core's return value.

Common actor initialization sets the command word to 0xFFFF. It leaves the
32-bit command value, phase and counter unchanged; fresh allocation clears those
fields before common initialization. A separate shared 32-bit pause delay is
outside these copies. Loading or saving an actor's command values does not reset
that delay.

Native task resumption separately loads command scratch from the actor's six
native task slots. Each slot retains four 32-bit values, including the pause
delay and packed command word/phase/counter. Saving a task copies those values
back to the same slot. The point-navigation actor fields and the native task
continuation are therefore distinct retained owners of values used during the
same command invocation.

Before evaluating native operands, a statement call clears all four retained
scratch words. A direct function expression also clears all four. An
object-targeted function expression clears the first two words and the packed
word/phase/counter, while retaining the point command's 32-bit value. This
encoding distinction applies even when the explicit object target is Me.
The direct native-hook dispatcher performs neither setup reset. A newly allocated
task slot saves the scratch resulting from its first native invocation.

The six-slot resume pass visits slots in order. An empty or paused slot saves the
current shared scratch into that slot without invoking its native call. An active
slot loads its retained scratch before invoking the native call. A still-pending
call saves the resulting scratch. A completed call clears its call pointer and
blocking flags without saving the resulting scratch back to its slot. Its resulting
shared scratch remains available to later slots and actors. Each invoked slot
increments its unsigned 16-bit call count once. The pass sums the blocking flags
of still-pending calls with byte wrapping.

The ordinary actor main stage performs native-hook and combat maintenance before
its body/task processing. Task resumption can therefore replace command scratch
for a later actor even when that later actor has no active task of its own.
The direct native-call dispatcher does not clear the shared pause delay.

Point navigation validates the actor's map and held-node indices before checking
the pause delay. A nonzero unsigned 32-bit delay consumes the shared nominal tick
count with saturation at zero and returns pending for that invocation, including
when the subtraction reaches zero. Routing resumes on a later invocation. The
delay is shared across actor invocations, so another actor can consume the
remaining delay. This delay is distinct from the signed combat lost-sight timer.

Point-navigation completion stores the low byte of its result code in the
shared navigation result byte and releases the actor's held current and next
walk-node reservations. The map and final destination remain retained. It
changes lifecycle selector seven to zero, leaves other lifecycle selectors
unchanged, assigns attribute 1 to 13, and returns zero. This completion helper
leaves the actor's four retained point-command fields and shared command words
unchanged; the enclosing wrapper still performs its command copy-back.

Before ordinary point movement, the blocked-route counter increments with unsigned
byte wrapping when the collision-response auxiliary byte has bit 7 set, a map is
present, and a next node is held. A resulting counter greater than eight selects
phase 24. Otherwise the counter resets to zero.

Before the blocked-route counter, a body-response mark of bit 2 (a contact with another
actor) whose overlap contact identifier is not one, outside phase 27, draws one raw engine
random value. Its low two bits select: 1, the held next node becomes the previous next node and
phase 27 is selected; 2, the shared pause delay is set to 36 and the call returns pending; otherwise
nothing changes.

Collision response marks are then read. When the collision-transition byte's
bit 0 is clear, any of auxiliary-byte bits 1 or 7, or body-response bits 1 or 3,
releases the actor's held walk nodes and selects phase zero, whatever phase the
blocked-route counter selected. When the transition bit is set, the held nodes are
released too; a collision-response age below 16 then restores the three local
coordinates from the actor's retained stable position, and an older response
selects phase 24 unless the phase is already 24 or 25.

Phase 24 is the back-off after a blocked route. It releases the actor's held walk nodes, then
steps backward for one update with `WalkBackward` when attribute 93 is zero and with `MoveBackward`
otherwise (both with a frame count of one, and `WalkBackward` is `MoveBackward` in deep water),
selects phase 25 and stays pending. Phase 25 changes nothing until the actor's current animation
group equals its default group, then selects phase zero. Both phases stay pending (lifecycle
selector seven, attribute 1 unchanged) and register the actor once for a deferred map update.

The shared player-exclusion switch starts disabled during player setup and nothing in the
executable ever enables it, so walk-node admission and held-target admission never reject a
node on its account. Point
movement consults this switch before falling/queued-motion handling. It is the
same switch used by walk-node admission; held-target admission has its own
position quantization and angular checks.

A falling actor or one with queued motion assigns attribute 1 to 13, selects
phase one when a next node is held or phase zero otherwise, sets lifecycle
selector seven, and returns pending. This path precedes immediate arrival and
default-animation admission.

A motion direction of 15 takes the same path after first calling a ledge update that acts only on
the selected player (and only while its attribute 105 is not one); for every other actor that call
changes nothing.

Phases 11 and 22 skip the immediate-arrival test. For other phases, a successful
immediate-arrival test copies the destination's local X and Z to the actor while
preserving local Y, admits the default animation, and completes with result zero.
Default-animation admission always succeeds. It requests the signed default group
with modifier zero through the forced animation-request path only when
SYSTEM.INI animation processing is enabled, animation state exists, the actor is
not excluded from object updates, and its current group differs from its default
group. It does not change the actor's default group.

After immediate destination arrival is rejected and collision response is
resolved, point phases 1 and 27 process the held next node. With no next node,
they release held reservations, select phase zero and remain pending. On arrival
at the held next node, default-animation admission runs before current-node
assignment. The phase then becomes eight regardless of whether the claim
succeeds. These pending paths set lifecycle selector seven and register the
actor once for a deferred map update. Attribute 1 remains 12.

When the held next node has not been reached, phases 1 and 27 face that node.
A nonzero facing result retains the phase. A zero result selects phase two.
Only after that zero result, nonzero attribute 90 calls `Rnd(attribute90)`.
A zero random result selects phase nine and stores `(draw & 127) + 16` in the
point-command word using another shared random draw. Other outcomes preserve
the command word. These paths retain held nodes and history, set lifecycle
selector seven and register one deferred map update.

Phase nine updates the point-command word using nominal ticks. It sign-extends
the 16-bit word to 32 bits, compares that value as unsigned with the tick count,
and clears the word when it is smaller. Otherwise subtraction wraps into the
16-bit word. A zero result selects phase two. A held weapon also selects phase
two, preserving the updated word.

An unarmed actor with a nonzero remaining word stays in phase nine. If its
current animation group equals its retained default group, it consumes one
shared draw. When the low five bits are all set, it draws again and masks to
two bits, rejecting three until a value from zero through two is obtained.
It requests animation group one plus that value with modifier zero. Other
animation groups consume no idle draws. These paths retain navigation indices,
set lifecycle selector seven and register the actor for a deferred map update.

Animation-stop calls in point navigation return without changing playback,
actor motion or navigation state.

Held-node arrival validates the actor's navigation and tests local horizontal Q8
distance against `((speed + 1) * 256 * frame_scale) >> 8`. Equality is accepted;
products wrap at 32 bits and the shift is signed. Speed is the current animation
group's movement speed without the immediate-destination fallback. On success,
local X/Z snap to the node's unsigned X/Z coordinates scaled by 256, local Y
is preserved, and the actor is registered for a deferred map update. Registration
deduplicates the actor, including when the enclosing pending path also registers
it. A missing map reports held-node arrival before reading a node.

Phase two releases held nodes and selects phase zero when no next node is held.
Otherwise it faces the next node when the animation exit marker is clear,
ignoring that facing result, then invokes coordinate walking toward the node
with tolerance zero. A pending walk retains phase two. A completed walk performs
default-animation admission again and the held-node arrival test with tolerance zero.
A failed arrival selects phase zero without releasing held indices. Successful
arrival assigns the next node as current and selects phase eight even when the
claim fails. These pending paths set lifecycle selector seven, leave attribute
1 at 12 and register one deferred map update.

### Point direct visibility

Point direct visibility reads the receiver's query position and compares the
rounded three-dimensional Q8 distance to the destination with an inclusive
limit of 131072. A larger distance rejects before model tests. The segment
endpoints are each shifted right by eight to integer map coordinates.

The query visits the prepared mixed-model candidates in order. A type-16
candidate is skipped before geometry when its actor is the receiver or selected
player, or its model is the receiver's own model instance. Other candidate types
do not receive that early exclusion. Model segment tests disable early return
after subobject admission; that argument does not set polygon sidedness.
Point direct visibility leaves the shared polygon-sidedness flag unchanged.
Scene initialization enables that flag, and temporary bounds queries restore it
to enabled before returning. A zero or failure result does not block. A successful model result blocks
unless the candidate actor is the receiver; that receiver exception occurs
after the geometry call. The first blocking result rejects immediately.

### Point route initialization

Phase zero releases held nodes and admits the default animation before trying
to initialize a new route. Initialization releases held nodes, performs the
ordinary nearest-free-node search and copies the shared selected map into the
receiver, including an absent map on a miss. A missing map or node returns zero.
Otherwise it assigns the found node as next and returns one after the assignment;
the assignment's claim result does not change that return value.

A nonzero initialization result selects phase one. A zero result retains phase
zero. The pending tail sets lifecycle seven and attribute 1 to 12 and registers
a deferred map update. Ordered release and assignment preserve previous-next
history.

### Phase-eight route monitoring

An unset current node releases held nodes and selects phase zero. Otherwise
phase eight searches the current map for the destination's XYZ coordinates,
each shifted right by eight. A missing destination completes point navigation
with result byte two. When the destination equals the current node, held nodes
are released. Nonzero attribute 95 selects phase 21. With attribute 95 zero,
a failed direct-visibility check completes with result byte four; an admitted
check selects phase 22.

For a different destination, route search receives the current node, destination
node and low identifier byte, followed by attributes 92 and 91. A missing next
hop completes with result byte three. A next-node claim uses the ordinary
ordered release and reservation owner. A rejected claim completes with result
byte seven. An accepted claim selects phase one when the shared first-link
`0x4000` flag is zero, or phase 12 when it is nonzero.

The pending tail sets lifecycle selector seven, leaves attribute 1 at 12 and
registers a deferred map update. Completion stores its result byte, releases
held nodes, changes lifecycle seven to zero, sets attribute 1 to 13 and returns
zero before pending registration. Completion retains the phase-eight command
phase and previous-next history.

### Direct-route phases 11, 21 and 22

Phases 11 and 22 bypass the point entry's immediate destination-arrival test.
Phase 21 admits the default animation and completes with result byte zero on
its ordinary return.

Phase 22 releases held nodes first. Nonzero attribute 95 selects phase 21.
Otherwise it faces the original destination's X/Z coordinates. A completed turn
selects phase 11; a pending turn retains phase 22. The pending tail sets lifecycle
seven and registers movement, with attribute 1 remaining at 12.

Phase 11 also releases held nodes first. Nonzero attribute 95 selects phase 21.
Otherwise a failed direct-visibility check selects phase zero. An admitted check
writes attribute 1 to 13, then invokes coordinate walking toward the original
destination with the caller's tolerance. Nonzero sliding state selects phase
zero before that call but does not skip walking. Completed walking completes
the point call with result byte zero.

Pending direct walking faces the original destination when the animation exit
marker is clear, ignoring the facing return, then registers a map update and
rechecks direct visibility. A failed recheck selects phase zero. The common
pending tail sets lifecycle seven and repeats deduplicating registration;
attribute 1 remains 13 on this path. The initial visibility rejection and the
attribute-95 transition retain attribute 1 at 12 instead.

### Phase twelve (leap link)

Phase twelve is selected when the claimed first link carries the `0x4000` flag.
An unset next node releases held nodes and selects phase zero. Otherwise the
stage runs the coordinate arrival test at the next node's X/Z with the caller's
tolerance. An arrival assigns that node as the current node through the
ordinary claim and selects phase eight. Without arrival, facing the node (the
face-node turn used by phase one) that returns zero starts the leap: the
receiver is given queued motion 2 and its animation requests group 32, or 33
with a burning torch. The pending tail is the common one, with attribute 1 at
12. Its start is refused, leaving the phase unchanged, while the receiver is
the selected player in its post-jump cooldown, its group is 114, 116, 130 or 131,
its falling, queued or sliding byte is nonzero, its stored surface is 2 or 4, it
is attached to a rope, or its living state is outside zero and 15. State 15 runs
the hang release instead of starting the leap. A started leap clears the
stable-position-valid flag, records the current Y as fall origin and zeroes the
fall timer and sliding distance.

### Coordinate walking and rollback

Coordinate walking saves local position, the packed movement byte and retained
animation group/frame words before testing destination arrival. When arrival
fails, it also saves the animation receiver. Attribute 93 equal to zero selects
`WalkForward`; a nonzero value selects `MoveForward`, with duration one in both
cases. The position-query owner is selected before movement and its coordinates
are read after movement for destination bearing. That owner can supply alternate
or linked coordinates rather than local position.

If the masked difference between bearing and local heading lies from 256 through
1792 inclusive, walking clears the shared forward-movement-called flag and
restores the saved local position, movement byte,
animation receiver and retained group/frame words, then snaps local X/Z to the
destination. Local Y remains the saved Y. Otherwise it retains the movement
result and returns one.

An initial arrival also snaps local X/Z and retains local Y. After either snap,
default-animation admission runs and the coordinate walker returns zero.
Default-animation admission returns one on every ordinary return, whether it
issues a forced request or skips it. Its result remains one after a forced
request. The empty animation-stop call changes no playback state.

### Walk-node arrival distance

Node arrival compares horizontal distance from the actor's local position to
the destination, using the same rounded Euclidean distance as node markers.
It accepts equality in:

`distance - tolerance <= (((speed + 2) * 256 * frame_scale) >> 8)`

Coordinates and tolerance use runtime Q8 units. Multiplication and subtraction
wrap at 32 bits, and the final right shift is signed. Vertical distance does
not participate. A missing navigation map reports arrival before reading a node.

Speed comes from the actor's current animation group when it provides a nonzero
movement speed. Otherwise it is the minimum nonzero attribute among indices
8, 19, 17, 12, 9, 13 and 10. When all seven are zero, the fallback speed is 8.

<a id="wandering-without-walk-node-maps"></a>

### Wander: destination selection and movement limits

`Wander(radius, tolerance)` validates the actor's current navigation-map state
and sets attribute 94, `attr_node_in_use`, to one. With no current map, validation
resets the held current-node, next-node and final-destination indices to -1.

Ordinary map loading copies the authored runtime position into the stored home
position before actor setup and attachment transforms. Actor setup selects its
initial navigation map and current node through the global nearest-node query.

When the final destination is unset, Wander clears the actor's saved navigation
scratch state and chooses a candidate around its stored home position. The
positive radius is converted from runtime units to map units by shifting right
eight bits. Home and effective current-position coordinates are converted by
integer division by 256, truncating toward zero. Candidate X and Z each equal
`home + radius - Rnd(2 * radius)`; candidate Y is the home Y. It repeats both
random draws while the rounded horizontal Euclidean distance from the effective
current position is at most half the integer radius. Distance rounding follows
the [node-marker distance arithmetic](#node-marker-distance).

The candidate then goes through the global [walk-node selection query](../formats/RGM.md#runtime-node-selection).
An empty loaded map list leaves the final destination unset. It does not bypass
candidate selection or its random draws, and the next invocation tries again.

Wander finishes by measuring horizontal displacement from the actor's saved
previous local position. Its limit in runtime coordinate units is
`(frame_scale << 13) >> 8`, with a wrapping 32-bit left shift followed by a
signed right shift. If the rounded displacement exceeds the limit, it restores
all three previous local coordinates. Equality retains the current position.
The function returns the measured displacement before any restoration.

<a id="authored-location-movement"></a>

### MoveToLocation: authored locations

Ordinary actor setup creates location 0 from the saved original home position,
then appends the actor header's RALC entries relative to that home. These stored
locations remain unchanged when the actor moves or becomes family-local.
Negative selectors or selectors greater than the stored location count use the
home position. A selector equal to the stored count falls immediately beyond the
allocated entries; ordinary valid selectors run from zero through count minus one.

`MoveToLocation(location, duration)` compares the shared signed task elapsed value
with the signed duration. When elapsed is strictly greater than duration, it
assigns the stored target to all three local position coordinates, queues the
actor's map update, and returns zero. Equality still performs a movement step.

Otherwise its source is the attached effective position when `attr_slave` is
nonzero, or the local position when that attribute is zero. It subtracts source
from target with wrapping 32-bit arithmetic and normalizes the resulting vector
to signed Q16 components. Normalization converts each component to single
precision, sums the squared components with extended intermediates, stores the
sum to double precision, and takes its square root. The divisor is that length
multiplied by 1/65536 and stored to single precision. A zero divisor produces
three zero components. Each other component is divided by the divisor, stored
to single precision, and converted to an integer with nearest-even rounding.

It separately measures the rounded three-dimensional distance from source to
target using the [node-marker distance arithmetic](#node-marker-distance).
Movement distance is signed integer division of that distance by
`duration + 1 - elapsed`, truncating toward zero. It then multiplies by
`frame_scale` with wrapping 32-bit arithmetic and shifts arithmetically right by
eight bits. Each displacement component is
`((normalized_component >> 8) * movement_distance) >> 8`, again with wrapping
multiplication and signed shifts. Displacements are added to the actor's local
position with wrapping arithmetic. The operation queues a map update and returns
one, including when all three displacements are zero.

`WanderToLocation(location, duration)` selects its target and source as
`MoveToLocation` does and normalizes the direction the same way. Its first
invocation in a task slot sets a retained speed to the signed integer division of
the rounded source-to-target distance by `duration`, truncating toward zero; the
slot keeps that speed and ignores the elapsed input afterwards. Each invocation
computes the step as the speed times `frame_scale` shifted arithmetically right by
eight. It queues the actor's map update. When the distance, measured again before
moving, is greater than the step, each displacement component is
`((normalized_component >> 8) * step) >> 8`, added to the local position, and the
task returns one. Otherwise the local position is assigned the target and the task
returns zero.

### Actor families and attached transforms

`attr_master` and `attr_slave` are byte-sized group identities. A nonzero slave
identity attaches an actor to the matching master group. An actor can be both a
slave of one group and the master of another, forming a nested family.

Family initialization scans identities 1 through 255. For each identity it
selects the first matching master without a slave identity from the actor
traversal list. It collects attached child masters separately from attached
actors without a master identity. Each collection prepends matching actors,
so its traversal reverses their encounter order.

The initialization traversal visits allocated map-cell registry identities in
increasing order, preserving each cell's actor list, then appends the persistent
actor list. Registry identities are assigned to cells as they are allocated;
sorting cell coordinates does not define this traversal. Fresh actor allocations
prepend to their cell or persistent list. Duplicate root masters are permitted:
the first matching root is selected, and the other roots remain independent.

Initialization saves each master's original position and angles before
processing its descendants. Descendants are initialized before the child
master's own coordinates become local. For each attached actor:

1. Subtract the parent's saved original position from the actor's position.
2. Multiply that displacement by the transpose of the parent's orientation
   matrix to obtain the local offset.
3. If the parent's `attr_no_slave_rot` is zero, subtract the parent's saved
   original angles from the actor's angles, wrapping each component modulo 2048.
4. Initialize the attached-position storage with the resulting local offset.

The angle subtraction tests the parent's attribute. The later composition
branch tests the attached actor's own attribute.

#### Family refresh

Each family refresh caches the root master's current position and angles.
It composes each child master before visiting that child's descendants, then
processes the remaining attached actors. Attached positions use the runtime
coordinate scale of 256 units per world unit:

```
effective_position = parent_effective_position + parent_matrix * local_offset
```

Refresh preserves the previous attached position before storing the new one.
Render position components use the effective integer coordinate shifted
arithmetically right by eight bits, then converted to single precision.

When the attached actor's `attr_no_slave_rot` is zero, refresh copies the parent
matrix and applies negative local yaw, pitch and roll in that order. Its
effective angle fields separately add the parent's effective angles and local
angles, wrapping each sum modulo 2048. A child master publishes its effective
position and angles for descendants. Matrix composition and effective Euler
angle addition are separate operations.

When the attached actor's `attr_no_slave_rot` is nonzero, position still uses
the parent matrix and translation. Its existing orientation matrix instead
receives negative local yaw, pitch and roll changes relative to the saved
previous local angles. Its effective angle fields use the local angles modulo
2048 without adding parent angles.

#### Ordinary axis locks

Near the end of an ordinary actor update, after script and movement handling,
nonzero position-lock attributes 40, 41 and 42 restore X, Y and Z respectively
from that update's saved position. Rotation-lock attributes 38, 37 and 39
restore pitch, yaw and roll respectively from the saved angles. Each axis is
independent, and the current attribute values are read when this step runs.
Actors with nonzero `attr_slave` bypass all six restorations. The step restores
coordinate and angle fields; it does not rebuild the orientation matrix.

#### Integer matrix arithmetic

Both local-offset initialization and position composition convert each signed
integer input to single precision first. Matrix elements are single precision.
Products and additions use x87 intermediates; each completed component is
stored to single precision before conversion to an integer. Single-precision
stores and integer conversion use the ordinary nearest-even rounding mode,
as in the
[forward-probe transform](player.md#ordinary-forward-walk-support-checks).
Parent translation is integer addition after conversion.

For row-major matrix entries `m00` through `m22`, ordinary composition evaluates:

```
X = (y * m01 + x * m00) + z * m02
Y = (x * m10 + y * m11) + z * m12
Z = (x * m20 + y * m21) + z * m22
```

The transpose used during initialization evaluates:

```
X = (y * m10 + x * m00) + z * m20
Y = (x * m01 + y * m11) + z * m21
Z = (x * m02 + y * m12) + z * m22
```

#### Matrix axis updates

Axis rotations use the [shared rotation table](water.md#wave-lookup-table).
For the supplied wrapped angle `a`, `s` is table entry `a` and `c` is entry
`a + 512`. For each existing matrix row `(x, y, z)`, the helpers write:

| Axis operation | Temporary single-precision result | Other single-precision result | Unchanged component |
|---|---|---|---|
| Yaw | `x_new = x * c - z * s` | `z_new = z * c + x * s` | `y` |
| Pitch | `y_new = y * c + z * s` | `z_new = z * c - y * s` | `x` |
| Roll | `x_new = x * c + y * s` | `y_new = y * c - x * s` | `z` |

Both expressions use the row values from before that axis operation. The helper
rounds the temporary result first, stores the other result in the matrix, then
copies the temporary into the matrix. Each result is single precision before
the next axis operation. Products
and sums use x87 intermediates. Family composition applies these operations to
the copied parent matrix with negative local yaw, negative local pitch and
negative local roll, each wrapped modulo 2048.

#### Angle snapshots and script ordering

Explicit [orientation assignment](player.md#upright-actor-orientation) initializes
the previous-angle fields from the assigned angles. The ordinary actor update
copies current local angles into those fields before animation, movement and
main script processing. Actors excluded from that update do not take this
snapshot.

The ordinary family-composition branch reuses these fields for propagated
angle deltas. Each component becomes
`(local_angle - saved_local_angle + parent_delta) & 2047` and supplies the delta
for descendant processing. The `attr_no_slave_rot` branch leaves these fields
unchanged and passes their stored values onward.

In the normal main loop, family refresh precedes map-list rendering and the
ordinary actor update. A separate ground-settling path also invokes family
refresh. A local-angle write does not itself refresh attached position, matrix
or effective angles. Consequently, an ordinary script's `RotateByAxis` followed
by `PlayerFacing` reads the previously composed attached yaw until another
family refresh occurs.

<a id="relative-axis-rotation"></a>

### RotateByAxis: relative rotation

`RotateByAxis(axis, delta, duration)` modifies the receiver's local angles.
Axis zero selects X, axis one selects Y, and every other axis value selects Z.
Angles wrap modulo 2048. A zero delta or zero duration completes without
changing an angle.

The task keeps a target and accumulator with eight fractional bits. When the
saved target and the task elapsed input are both zero, it initializes the target
to `((current_angle + delta) & 2047) << 8` and the accumulator to
`current_angle << 8`.

Each invocation computes a fixed-point step by shifting `delta` left eight
bits, dividing by `duration` with signed truncation toward zero, multiplying by
the current shared frame scale, then arithmetic-shifting right eight bits.
The intermediate shifts and multiplication retain their low 32 bits. If this
step is zero and the shared whole nominal-tick increment is nonzero, that tick
increment becomes the step, negated only when the saved target is negative.

The task compares the absolute shortest wrapped angle difference between the
current angle and `target >> 8` with `abs(step >> 8)`. If the difference is
larger, it adds the step to the accumulator, writes
`(accumulator >> 8) & 2047`, and remains active. Otherwise it writes
`target >> 8` and completes. The initial invocation can change the angle.
Arguments are reevaluated on later invocations, while the saved target and
accumulator persist unless the initialization condition is met again.

Each task slot owns its target and accumulator; the receiver's angle is shared.
Stopping the task removes its slot without restoring an earlier angle.
Transform and attachment updates consume the changed local angles separately;
the handler does not update their matrices itself. Frame scale and nominal ticks
are distinct inputs; see [animation timing](animation.md#timing-boundary).

<a id="elapsed-relative-axis-rotation"></a>

### sRotateByAxis: elapsed relative rotation

`sRotateByAxis(axis, delta, duration)` stores the selected angle as its origin
whenever elapsed input is zero. Only axes zero through two select and write an
angle. The origin remains task-local on later invocations.

For a nonnegative delta, the offset is
`((delta << 8) / duration * elapsed) >> 8`. For a negative delta, it computes
that expression using the absolute delta, then negates the shifted result.
Division truncates toward zero and the right shift is arithmetic. Intermediate
shifts, multiplication and negation retain their low 32 bits. The handler writes
`(origin + offset) & 2047`.

When elapsed is at least `duration - 1`, it overwrites that result with
`(origin + delta) & 2047` and completes. Otherwise it remains active. Thus the
last invocation snaps to the full delta rather than the interpolated offset.
Duration one completes on the initial invocation. Duration zero produces a
division-by-zero fault before the completion check. This task uses elapsed input,
not shared frame scale or nominal ticks, and does not refresh composed transforms.

<a id="absolute-axis-rotation"></a>

### RotateToAxis: absolute rotation

`RotateToAxis(axis, target, duration)` modifies the receiver's local angle.
Axis zero selects X, axis one selects Y, and every other value selects Z.
The target is masked to 2047. A zero duration completes without changing an
angle.

Let `difference` be the masked current angle minus the masked target. The
signed movement toward the target is `-(difference & 2047)` when the difference
is below -1024, `-difference` when it lies between -1024 and 1024 inclusive,
and `2048 - difference` when it exceeds 1024. Thus the two half-turn ties
retain the direction determined by the subtraction.

Each task slot stores this movement with eight fractional bits and a separate
angle accumulator. When the saved movement and elapsed input are both zero,
it initializes them to `movement << 8` and `current_angle << 8`. Later argument
reevaluation changes the destination used for completion, but does not
reinitialize a nonzero saved movement.

Each invocation divides the saved movement by the current duration with signed
truncation toward zero, multiplies by shared frame scale, then arithmetic-shifts
right eight bits. Shifts and multiplication retain their low 32 bits. If the
result is zero and the whole nominal-tick increment is nonzero, the step is
that increment shifted left eight bits, negated when the saved movement is
negative.

The handler compares the absolute movement from the current angle to the
current target with `abs(step >> 8)`. If it is larger, the handler adds the step
to its accumulator, writes `(accumulator >> 8) & 2047`, and remains active.
Otherwise it writes the masked target and completes. The initial invocation
can change the angle. The handler does not refresh the receiver's composed
transform or attachment matrix.

<a id="elapsed-axis-rotation"></a>

### sRotateToAxis: elapsed absolute rotation

`sRotateToAxis(axis, target, duration)` stores the selected angle as its origin
whenever the elapsed input is zero. Only axes zero through two select and
write an angle. The origin remains task-local on later invocations.

It subtracts the stored origin from the target before masking the target.
When this signed difference is below -1024, it replaces it with
`difference & 2047`. When it exceeds 1024, it replaces it with
`-(difference & 1023)`. Other differences remain unchanged.

The interpolated angle is the origin plus
`((difference << 8) / (duration - 1) * elapsed) >> 8`, masked to 2047.
Division truncates toward zero, the right shift is arithmetic, and intermediate
arithmetic retains its low 32 bits. The interpolation executes before the
completion check. A duration of one therefore produces a division-by-zero
fault even at elapsed zero.

When elapsed is at least `duration - 1`, the handler overwrites the interpolated
angle with `target & 2047` and completes. Otherwise it remains active. The
positive-difference mask can make the interpolated path approach a different
angle before this final snap. This task uses elapsed input rather than shared
frame scale or nominal ticks.

<a id="actor-local-axis-movement"></a>

### MoveObjectAxis: actor-local movement

`MoveObjectAxis(axis, delta, duration)` moves along the receiver's orientation,
using its current stored matrix or the upright forward shortcut. Delta and
position use eight fractional bits. Zero delta or duration completes immediately.
At elapsed zero, the task stores delta as its target and resets its traveled
accumulator. Each invocation computes `(delta / duration >> 8) * frame_scale`
with signed division, arithmetic shift and wrapping 32-bit multiplication, then
adds that step to the wrapping accumulator. A negative step completes when the
accumulator is at or below the target; a positive step completes when it is at
or above the target. These completion paths perform no movement and do not
snap the receiver to the target.

Otherwise, axis 0, 1 or 2 selects a local X, Y or Z displacement; other axes
select a zero vector. For axis 2, a receiver with zero slave attribute, pitch
and roll uses the [upright forward translation](player.md#heading-based-translation)
rule, including single-precision coordinate addition and truncation. Other
cases transform the displacement through the stored actor matrix, round the
transformed components to nearest-even integers, and add them to the receiver's
coordinates with wrapping arithmetic.

After movement, the receiver is registered once in the deferred map-update
list. The task remains active only while signed elapsed is less than the
wrapping value `duration - 1`. Later invocations reevaluate arguments while
retaining target and accumulator unless elapsed is zero again. The task does
not perform collision or restore position when stopped.

<a id="relative-axis-movement"></a>

### MoveByAxis and sMoveByAxis: relative movement

`MoveByAxis(axis, delta, duration)` changes a receiver's local position in the
same eight-fractional-bit units as the position fields. Axis zero selects X,
one selects Y, and every other value selects Z. A zero delta or zero duration
completes without changing position.

Each task slot retains a target, initially zero. Whenever that saved target is
zero, the handler replaces it with `current_position + delta`; this test does
not depend on elapsed time. Each invocation divides delta by duration using
signed truncation toward zero, arithmetic-shifts that quotient right eight,
then multiplies by the shared frame scale, keeping the low 32 bits. This order
quantizes the step before multiplication.

For a negative step, a proposed position at or below the target snaps to the
target and completes. For a positive step, a proposed position at or above the
target does the same. Otherwise the step is added to the current position.
A zero step leaves position unchanged at this point. The handler then compares
the signed task elapsed input against `duration - 1`: reaching that value snaps
to the saved target and completes; otherwise the task remains active. Additions
and subtraction wrap to 32 bits. The initial invocation can move the actor.

`sMoveByAxis(axis, delta, duration)` is a separate elapsed-position task. Only
axes zero, one and two read or write a coordinate. Its saved origin starts at
zero; when both origin and elapsed input are zero, it captures the selected
current coordinate. Each invocation writes origin plus
`(((delta << 8) / duration) * elapsed) >> 8`, using signed division, an arithmetic
right shift and low-32-bit intermediate results. When signed elapsed is at
least `duration - 1`, it instead writes `origin + delta` and completes. Other
axis values still perform arithmetic and completion testing but do not write
position. This handler has no zero-duration shortcut.

`MoveAlongAxis(axis, delta, duration)` is a task that moves along an axis of the
receiver's own orientation. A zero delta or zero duration completes without
change. The slot retains the delta and a running sum, both reset when the signed
elapsed input is zero. Each invocation computes the step as for `MoveByAxis`
(signed `delta / duration`, arithmetic shift right eight, times the frame scale,
low 32 bits) and adds it to the running sum. A negative step completes when the sum
is at or below the delta, and a positive step when it is at or above it; that
invocation does not move the actor. Otherwise the step becomes a vector on the
selected axis (zero X, one Y, any other value Z), is rotated and added to the local
position, and the actor registers a map update. With attribute 36 nonzero, or while
the receiver's per-update matrix-use byte is zero (it is cleared at the start of
every ordinary update), the rotation is built from the receiver's current pitch,
yaw and roll; that sense is the inverse of the stored matrix's transform, so a
quarter turn of heading sends a positive X step onto negative Z. Only with
attribute 36 zero and the matrix-use byte set does the stored matrix rotate the
vector in the stored matrix's own sense.
The task then completes when `duration - 1` is not greater than the elapsed input.
It never snaps to a target position.

Both tasks reevaluate their arguments on subsequent invocations and retain
slot state under their respective initialization conditions. Neither handler
performs collision, matrix, or attachment refresh. Stopping a task does not
restore position. Signed division overflow, including division by zero in
`sMoveByAxis`, has no successful movement result.

<a id="random-functions"></a>

### Rnd and QuickRnd

`Rnd(bound)` and `QuickRnd(mask)` each consume one draw from the shared
engine random stream, also used by [sky scroll initialization](sky.md).
The generator updates an unsigned 32-bit state with wrapping arithmetic:

```text
state = state * 1103515245 + 12345
draw = (state >> 16) & 32767
```

The executable's stored initial state is 1. The state belongs to the shared
generator, not to an individual script or actor.

`QuickRnd(mask)` returns `draw & mask`, using the complete 32-bit argument.
The argument is a bit mask rather than an exclusive upper bound. A zero mask
still consumes a draw; a negative mask is accepted.

For a positive `bound`, `Rnd(bound)` first masks the draw with the smallest
mask in `1, 3, 7, ..., 16383` whose range can contain `bound` values. Bounds
greater than 16384 leave the draw unchanged. It then repeatedly subtracts the
bound until the result is less than the bound. Equivalently, apply that mask
and take the remainder modulo the bound. The result is not uniformly
distributed for every bound. Zero and negative bounds do not define a valid
bounded draw.

<a id="object-visibility"></a>

### HideMe and ShowMe

`HideMe()` sets the receiver's hidden flag and clears its current presentation
record. `ShowMe()` clears that flag. For a static-model receiver, showing it also
restores its model presentation record and refreshes its transformed geometry.
Both calls return zero.

The hidden flag does not exclude the receiver from ordinary script or animation
updates. It suppresses the ordinary actor's optional sound and light updates.
Player selection also uses this flag to hide the unselected player model.

### Current model presentation

A retained model resource and an actor's current model presentation record are
distinct. For an actor admitted to the ordinary presentation traversal, the
current record is cleared before hidden and update-excluded state are checked.
Either exclusion therefore leaves that actor without a current model record.
The PVO actor predicate runs earlier: an actor rejected there retains its prior
current record as well as its stored coordinates.
Stored floating-point model coordinates have a separate lifetime. Actor default
initialization clears them to zero. Clearing the current record, hidden or
update exclusion, and PVO rejection preserve those coordinates until model-position
processing runs again. An absent animation frame or unavailable selected resource
also preserves the previous coordinates when position processing is skipped.
A static-model actor that passes those checks restores its retained resource
and refreshes its model position and orientation, even when the retained resource
is null. Drawing exclusion suppresses
submission without clearing the restored record.

For an animation-table actor, an unavailable selected model or an absent frame
selection clears both the selected resource and the current presentation record.
Collision candidate construction consumes the current record and its stored
model transform. A logical position change does not itself refresh that transform.
In normal main-loop order, model presentation runs before the ordinary actor
update. `ShowMe()` provides the separate immediate static-model refresh described
above.

The ordinary model-position path uses the actor's logical Y shifted right by
eight when the collision attribute mask `0x04` is clear. With that mask set,
water processing can adjust model Y for stored surface classes 1 through 5
and 11. Other stored surface classes retain the shifted logical Y. A nonzero
collision attribute by itself therefore does not establish water displacement.

The normal world loop enables water-position processing before its frame loop
and clears that enable when it leaves the loop. A nonzero attribute 26 bypasses
this model-position path before water processing; [family refresh](#family-refresh)
owns the attached actor's model coordinates.

For other actors, a linked-position record with a nonzero owner reference selects
a separate model-position calculation before water processing. Copy the record's
local offset, convert its components to single precision, and transform it by
the owner's stored orientation matrix. Each resulting component is converted to
an integer using the active floating-point rounding mode. The default mode rounds
to nearest, with ties to even.

Select the owner's [effective position](#player-distance-and-facing), add the
transformed offset and the actor's own local logical position, then shift each
signed component arithmetically right by eight and store it as a float. Both
integer additions wrap at 32 bits before the shift. The actor's logical position
is unchanged. This linked calculation returns before water displacement, health
prepasses or environmental effects. A missing linked record or zero owner
reference instead falls through to ordinary model positioning. The linked
record's cached position used by effective-position queries is a separate value
from this calculated model position.

For the selected player with attribute 105 equal to one, enabled water-position
processing and collision attribute mask `0x04` run an earlier surface prepass.
Stored surface classes 2 and 4 clear both directional-state words before a
health change of -255 through the selected death-owning health service, with no
source actor. Classes 1 and 3 clear those words without that health call. Other
surface classes do neither in this prepass. The later surface-position processing
still follows; this prepass does not replace the surface-5 and surface-11 route.

With water-position processing enabled and collision attribute mask `0x04`
set, stored surface classes 5 and 11 place the model Y at the arithmetic-shifted
logical Y plus 64. For the selected player, the retained death-progress word is
also added. These integer additions wrap at 32 bits before conversion to the
floating-point model position; the logical position is unchanged.

Stored surface class 1 places model Y at the arithmetic-shifted logical Y
plus 64. Surface classes 2, 3 and 4 first check the selected player's retained
hit inhibition. An inhibited selected player uses shifted logical Y plus 64
plus retained death progress, then returns without coverage sampling or the
height-change effect check.

Otherwise, surface 2 uses coverage 256. Surfaces 3 and 4 sample
[terrain material blend coverage](player.md#terrain-material-blend-coverage)
at the stored model X and Z, each truncated toward zero to integer map units.
Coverage zero and 256 remain unchanged. Other coverage values are doubled and
capped at 256. Model Y becomes shifted logical Y plus one quarter of that
coverage, using an arithmetic right shift by two.

Animation groups 141 through 143 then add `[game] swim_depth` to the stored
model Y. This setting replaces zero with 40 during initialization; other values
are retained. The shifted-position result is stored as a float before the
swim-depth addition, whose result is also stored as a float. Group 140 does
not receive that offset.

For the non-inhibited surface-2/3/4 path, the prior stored model Y is truncated
toward zero. The new stored Y is subtracted, and the difference is truncated
toward zero before taking its integer absolute value. A magnitude strictly
greater than 16 requests environmental effect descriptor `0x2500`; equality
does not. Surface 1 and the inhibited selected-player path skip this check.

This environmental effect starts from the actor's current logical Q8 position.
Its horizontal point is displaced 2048 Q8 units toward the camera using the
[camera-relative effect-point rules](#selected-death-effect-admission). Its vertical point
is logical Y plus the actor's positive vertical extent times 256 plus 1536,
with signed 32-bit wrapping arithmetic. The effect constructor applies texture-height
centering and publishes map-unit coordinates, descriptor flags, the texture's
last frame as the target frame, and speed one. This path consumes no random
numbers. Disabled effects or refused allocation produce no effect.

The selected player's surface-5 path sets retained death mode one after
publishing model Y. Surface 11 leaves that mode unchanged. Both paths then test
queued motion and the low two bits of each directional-state byte. A nonzero
queued-motion state or either nonzero bit pair suppresses the health call;
it does not undo the model position or death-mode write. Otherwise the receiver
gets a health change of -255 through the death-owning health route. The selected
player's mode write therefore precedes any death initialization performed by
that health service.

### Model admission and support ownership

Both standalone player roles are allocated into the persistent actor list. The
initially selected player is allocated first and the alternate second; insertion
at the front leaves the alternate before the selected player in that list.
Model-list assembly appends this persistent list after the collected cell actors,
so standalone players participate without an MPOB placement. Player setup hides
the alternate. It remains linked but hidden-state exclusion prevents its model
position processing.

The current map's PVO file supplies a camera-centered selected-object list.
An empty selected list leaves ordinary model admission unrestricted. With a
nonempty list, the ordinary presentation traversal excludes actors with
`attr_non_moving` set. This predicate also applies to standalone players.

With a nonempty selected list, animation-model admission also reads the actor's
retained support owner. A
static-placement support must occur in the selected MPSO IDs. Support from an
actor with `attr_non_moving` set must occur in the selected secondary object
IDs. Other support cases pass this admission check. Rejected animation-model
admission still runs model-position processing when a frame is selected, then
clears the selected resource and current model record. An absent frame skips
position processing. Thus rejected support can still produce model-position
health changes and environmental effects without publishing geometry.

When loading a world with a PVO file, the secondary object table is built
from the complete map actor traversal. Map cells are scanned in increasing Z,
then increasing X, preserving each cell's actor list. The persistent actor
list follows the cells. Only actors with `attr_non_moving` set are appended
to the secondary table, in that traversal order. This cell-coordinate scan
is distinct from the registry-identity traversal used for family initialization.

Ordinary actor updates clear the retained actor and static-placement support
owners before ground processing. A selected model face assigns its candidate's
actor and static-placement owners. Model presentation precedes that ordinary
update, so its admission reads support retained from earlier ground processing.
Actor-following also consumes the retained actor support owner.

## Global Flags

369 global flags (indices 0–368), shared across all scripts. Each flag has a declared type:

| Type | Semantics |
|---|---|
| `BOOL` | Binary state (0 or 1) |
| `NUMBER` | Integer counter or timer |
| `FLIPFLOP` | Toggled state (doors, switches, puzzle elements) |

The loader maps `BOOL`, `FLIPFLOP`, `COUNTER`, and `NUMBER` to runtime kinds zero through three. Initializing a flag stores its authored numeric value and clears the latch. A BOOL set toggles its numeric value only when unlatched, then latches it. FLIPFLOP toggles on every set. COUNTER decrements a nonzero value and latches it; a zero value remains unchanged. NUMBER sets the latch without changing the numeric value. The SOUP `SetGlobalFlag` and `ResetGlobalFlag` wrappers return zero. `TestGlobalFlag` returns one or zero according to the latch.

Native flag functions maintain a separate latch in addition to the numeric flag value. `TestGlobalFlag` reads that latch; `ResetGlobalFlag` clears it. `SetGlobalFlag` dispatches according to the runtime type bits and can toggle, decrement, or latch a flag. It is not a generic assignment of one to the numeric flag.

Flags are declared in the `[flags]` section of `SOUP386.DEF` and accessed by bytecode via `SetGlobalFlag`, `TestGlobalFlag`, and `ResetGlobalFlag` (which use the NumericAlt opcode `0x16` for their flag-ID parameter).

Six flags have non-zero defaults: `TimeOfDay` (1), `OB_TelV` (1), `OB_TelH` (12), `At_Shoals` (1), `Rock_1_Down` (1), `Rock_2_Down` (1).

Flag categories span narrative progression (acts 1–8, for example `After_Catacombs`, `After_League`, `Won_Game`), inventory state (`HaveAmulet`, `HaveGem`, `Equipped_Torch`), NPC dialogue tracking (`DreekiusTalk`, `TobiasTalk`, `SionaFriend`), puzzle mechanics for catacombs (`CTDoor*`, `CTWeight`), caverns (`CV_Lock*`, `CV_Pillar*`), observatory (`OB_TelV`, `OB_Platform`), palace (`PI_Door*`, `PI_Throne*`), dwarven ruins (`DR_Steam`, `DR_Boiler`, `DR_Pipe*`), and the scarab (`SCB_Position`, `SCB_ArmL`, `SCB_ArmR`), as well as runtime control (`Talking`, `MenuRet`, `StrengthTimer`, `MapTimer`).

## Relationship to RGM Sections

- `RAHD` stores per-actor script pointers, variable counts, and function-table indices.
- `RASC` stores compiled script bytecode as a contiguous blob.
- `RAST`/`RASB` store script string literal data and offset tables.
- `RAVA` stores initial local variable values (`i32` array).
- `RAHK` stores per-actor native call records addressed by RAHD section offsets.
- `RAAT` stores per-actor attribute tables (256 bytes each), addressed by names from `SOUP386.DEF`.

See [RGM.md](../formats/RGM.md) for record-level layouts and offsets.

## External References

- [UESP `Mod:RGM File Format`](https://en.uesp.net/wiki/Mod:RGM_File_Format)
- [RGUnity/redguard-unity `RGRGMScriptStore.cs`](https://github.com/RGUnity/redguard-unity/blob/master/Assets/Scripts/RGFileImport/RGMData/RGRGMScriptStore.cs) — SOUP VM interpreter with dispatch loop, threading model, and formula evaluator
- [RGUnity/redguard-unity `soupdeffcn_nimpl.cs`](https://github.com/RGUnity/redguard-unity/blob/master/Assets/Scripts/RGFileImport/RGMData/soupdeffcn_nimpl.cs) — SOUP function ID-to-name table (367 functions)
- [Dillonn241/redguard-mod-manager `ScriptReader.java`](https://github.com/Dillonn241/redguard-mod-manager/blob/main/src/redguard/ScriptReader.java) — RASC bytecode disassembler with full opcode and value-mode decoding
- [Dillonn241/redguard-mod-manager `ScriptParser.java`](https://github.com/Dillonn241/redguard-mod-manager/blob/main/src/redguard/ScriptParser.java) — RASC bytecode assembler (script text to bytecode)
- [Dillonn241/redguard-mod-manager `MapHeader.java`](https://github.com/Dillonn241/redguard-mod-manager/blob/main/src/redguard/MapHeader.java) — RAHD record parser with field offsets
- [Dillonn241/redguard-mod-manager `MapDatabase.java`](https://github.com/Dillonn241/redguard-mod-manager/blob/main/src/redguard/MapDatabase.java) — SOUP386.DEF parser (function, flag, reference, attribute definitions)

## Selected-player location camera task

`showCyrusLoc` is a three-argument task. It tests `IsHoldingWeapon` on the
selected player, independently of the receiving actor. While that predicate is
true, the task returns zero without invoking its camera-location handler.

Otherwise it passes the selected player and all three full-width arguments to
the camera-location handler and returns that handler's result. The wrapper does
not narrow or transform the arguments. Task-slot scheduling and operand
reevaluation follow the ordinary native-task rules.

### Player camera task state

`showPlayer` is a task with no arguments. Each invocation asks the player camera
selector for a slot, falling back to slot zero when the selector reports no
selection. It updates the shared camera selection to that slot.

The task increments its first private word with 32-bit wrapping. When the result
is one and elapsed input is zero, it sets bit `0x01` in the selected camera
record's flags and returns one. Otherwise it clears bits `0x01` and `0x20` in
that record and returns zero. Other bits in the record's flag byte are retained.
The private word belongs to the task continuation and is distinct from the
shared camera selection.

A nonzero task result keeps the task slot active and zero completes it. The
initial invocation therefore leaves `showPlayer` active with bit `0x01` set, and
the next task pass completes it after clearing bits `0x01` and `0x20`.

### Player camera slot selection

The slot requested for the selected player is the first that applies:

| Condition | Slot |
|---|---|
| The selected player holds a weapon (`IsHoldingWeapon`) | 3 |
| The shared debug word is nonzero (zero at program start) | 8 |
| The selected player's living state is 15 | 4 |
| The selected player has a rope association | 5 |

When none applies, no slot is requested.

A request for a different camera slot changes the selection only when the
shared transition word is zero or the current record has flag `0x01`. Otherwise
the current slot remains selected. Requesting the current slot leaves selection
unchanged. The pose stage clears this word according to its correction and
movement-limiting results.

### Actor camera offsets

Location-camera setup reads the actor's `attr_camera_target_offset_x`,
`attr_camera_target_offset_y`, `attr_camera_target_offset_z`,
`attr_camera_offset_x`, `attr_camera_offset_y`, and `attr_camera_offset_z`
attributes as unsigned bytes. Each byte independently becomes zero when it is
zero, or `(value - 127) * 500` otherwise. The signed results replace the selected
camera record's target-offset and camera-offset vectors. This conversion does
not change the actor attributes or the other fields in the camera record.

### Selected record and selected index

The engine keeps the selected camera record and a separate selected-index word.
Camera record selection sets both. The movement limiter's slot-one test, the
orientation stage and the distance-height stage read the index word; every other
stage reads the selected record. A pan leaves the index on the slot being panned
while the selected record is record two.

### showPlayerPan

`showPlayerPan(duration)` is a task with one argument. It increments the same
private word as `showPlayer`. A negative duration writes a diagnostic line and
the task returns zero before touching the private word or any camera state.

On the first invocation (private word one, elapsed zero) it:

1. takes the slot the player camera selector requests, or zero, and makes that
   record the selected record directly, without the selection guard;
2. begins a scripted camera;
3. copies the requested slot's record into record two and sets the selected
   index to the slot (the selected record stays whichever record the begin step
   left selected);
4. binds the selected record to the selected player;
5. records the camera's current offset from the player,
   `published position - record position offset - bound position`, as the start
   vector. The start distance is that vector's rounded length shifted right eight.
   The start angles are the camera orientation conversion of the vector. The
   target is the record's angle offset minus the bound angles (not wrapped), and
   the delta is the target minus the start angles with each component wrapped
   once into half a turn either side of zero (below -1024 add 2048; above 1024
   subtract 2048).

Every invocation then runs the scripted-camera release step with `duration`.
While it reports running and `duration` is greater than one, the selected record's
flags `0x04` and `0x08` are cleared and, with `span = duration - 1`:

- each angle offset component is the bound angle plus the start angle plus
  `delta * elapsed / span`;
- the distance is the start distance plus `(record distance - start distance) * elapsed / span`;

products wrap as signed 32-bit values and division truncates toward zero. The task
returns one.

Otherwise (the release step finished, or `duration` is one or less) the selected
record receives the target as its angle offset and its own distance, flags `0x01`
and `0x20` are cleared, the `showPlayer` step runs (it increments the private word
again, selects the requested slot through the selection guard and clears flags
`0x21` on it) and the task returns zero. Only record two is edited, so the slot's
own record is unchanged afterwards.

### Location camera request

The first invocation of the location-camera handler (private word one, elapsed
zero) sets flag `0x01` on the record that is selected at that moment, begins a
scripted camera (see below), binds the record that step selected (record two) to
the target actor, applies the [actor camera offsets](#actor-camera-offsets) to
it, and then writes its request from the three task arguments. The flag set on
the first record stays set: after the handler, record zero (when it was the
selected record) keeps flag `0x01`.

The binding selects one of two position and angle vector pairs on the actor,
chosen by attribute byte 26 (zero or nonzero). The camera therefore orbits the
target actor, not the player.

The arguments are shifted left eight bits. That vector and the record's
position offset (as just replaced from the actor attributes) are each rotated
by the bound actor's angles. Flag `0x01` is set on the bound record again, and
the actor's `attr_camera_move` byte then selects:

| `attr_camera_move` | Request | Flags |
|---|---|---|
| 0 | rotated argument vector + rotated offset + bound position | set `0x08`, clear `0x04` |
| 1 | rotated argument vector + rotated offset | set `0x04`, clear `0x08` |
| 2 | record distance = rounded length of the rotated argument vector >> 8; angle offset = angles of that vector (the camera orientation conversion) plus the bound actor's angles, each wrapped to 2047 | clear `0x08` and `0x04`, set `0x01` |

Other values write no request and leave the flags as the begin step set them.
The handler then runs one camera pose update with the bound record selected,
clears flag `0x01` on the selected record, and returns one. Later invocations run
the scripted-camera release step with argument zero and return zero.

`showObj` is the same handler with the argument vector used as given instead of
rotated by the bound actor's angles (only the record's position offset is
rotated); its requests are therefore vector + rotated offset + bound position
(`attr_camera_move` 0), vector + rotated offset (1) and the orbit of the unrotated
vector (2). The `showCyrus` wrappers of both handlers return zero without touching
the camera while the selected player is holding a weapon, and otherwise run the
handler with the selected player as the target actor.

### Object pans

`showObjPan` and `showObjPanLoc` take a target actor, three vector arguments and a
duration, and move the camera from where it is towards a request over `duration`
ticks. The vector arguments are shifted left eight bits. A negative duration writes
a diagnostic line and the task returns zero before touching the private word or any
camera state. Both share the first invocation's setup with the
[location camera](#location-camera-request): begin a scripted camera, bind the
selected record (record two) to the target actor, apply the
[actor camera offsets](#actor-camera-offsets), and rotate the record's position
offset by the bound actor's angles. They differ in two ways:

| | `showObjPan` | `showObjPanLoc` |
|---|---|---|
| Vector | used as given | rotated by the bound actor's angles |
| Absolute request (`attr_camera_move` 0) | vector + bound position | vector + rotated offset + bound position |
| Flag `0x40` | set on the record selected at entry (before the negative-duration check) and on the selected record at completion | not touched |

The first invocation (private word one, elapsed zero) records the start state: the
published camera position minus the bound position, as the start position; that
position minus the rotated offset, whose rounded length shifted right eight is the
start distance and whose camera orientation conversion is the start angles; the
vector's rounded length shifted right eight as the final distance; and the delta
from the start angles to the vector's angles, each component wrapped once into half a
turn either side of zero. No flag `0x01` is set and no pose update runs; the normal
camera update publishes the result.

Every invocation runs the scripted-camera release step with `duration`. While it
reports running and `duration` is greater than one, with `span = duration - 1`, the
actor's `attr_camera_pan_type` selects the interpolation:

- one: the selected record's request is the start position plus
  `(vector + rotated offset - start position) * elapsed / span` per component, with
  flag `0x04` set and `0x08` cleared;
- any other value: flags `0x04` and `0x08` are cleared, each angle offset component is
  the bound angle plus the start angle plus `delta * elapsed / span`, and the distance
  is the start distance plus `(final distance - start distance) * elapsed / span`.

Products wrap as signed 32-bit values and division truncates toward zero. The task
returns one.

Otherwise the request `attr_camera_move` selects is written exactly as the location
camera writes it (with the absolute request of the table above, the relative request
being vector + rotated offset, and the orbit using the vector alone), flag `0x01` is
cleared on the selected record, and the task returns zero.

The `showCyrusPan` and `showCyrusPanLoc` wrappers apply the same holding-weapon
guard as the [location wrappers](#location-camera-request) and then run the handler
with the selected player as the target actor.

### World-position camera requests

`showObjWorld(x, y, z)` is a location camera whose arguments are a world position. Its
vector is `(argument - P) << 8` per component, where `P` is the position offset of the
record that is selected when the task starts (the arguments are not shifted before the
subtraction). The first invocation sets flags `0x20` and `0x01` on that record, begins a
scripted camera, binds record two to the target actor, applies the
[actor camera offsets](#actor-camera-offsets), sets flag `0x01` again and writes the request
`attr_camera_move` selects. Nothing is rotated and the offsets are not added:

| `attr_camera_move` | Request | Flags |
|---|---|---|
| 0 | the vector | set `0x08`, clear `0x04` |
| 1 | vector - bound position | set `0x04`, clear `0x08` |
| 2 | orbit of vector - the actor's own position (not the bound pose, which differs for actors with attribute byte 26 set), as for the [location camera](#location-camera-request) | clear `0x08` and `0x04`, set `0x01` |

It then runs one camera pose update, clears flag `0x01` on the selected record and returns
one; later invocations run the release step with argument zero and return zero.

`showObjWorldPan(x, y, z, duration)` returns zero for a negative duration without touching
the camera. Otherwise it runs `showObjPan` with the arguments `(argument << 8) - B` per
component, where `B` is the bound position of the record selected at the call, and
`showObjPan` shifts them left eight bits again.

`showObjRot` does nothing and returns zero.

### lookObj

`lookObj(duration)` turns the camera in place to face the target actor. A negative
duration writes a diagnostic line and the task returns zero before touching the
private word or any camera state.

The first invocation (private word one, elapsed zero) begins a scripted camera, binds
the selected record (record two) to the target actor and applies the
[actor camera offsets](#actor-camera-offsets). It keeps the published camera angles as
the start angles and runs the camera's orientation stage at the published position,
with the record's target offset, to get the angles that face the target; the delta is
the difference from the start angles with each component wrapped once into half a turn
either side of zero. It then sets flags `0x08` and `0x02` (fixed orientation) on the
record, writes the published position as the record's request, and returns one. The
release step does not run on this invocation.

Every later invocation runs the scripted-camera release step with `duration`. While it
reports running and `duration` is greater than one, the published camera angles are
written directly: start angle plus `delta * elapsed / (duration - 1)` per component
(products wrap as signed 32-bit values, division truncates toward zero), and the task
returns one. Otherwise flag `0x02` is cleared and the actor's `attr_camera_move`
selects the request, using the published position `P`, the bound position `B` and the
record's position offset `O`:

| `attr_camera_move` | Request | Flags |
|---|---|---|
| 0 | `P` | set `0x08`, clear `0x04` |
| 1 | `P - B - O` | set `0x04`, clear `0x08` |
| 2 | orbit of `P - B - O`, as for the [location camera](#location-camera-request) | clear `0x08` and `0x04`, set `0x01` |

Other values write no request. The task then returns zero.

### Scripted camera begin and release

Beginning a scripted camera sets the scripted-camera active word to one,
requests slot two (the selection guard applies), clears the first twelve bytes
of the selected record twice, increments the nesting count, resets the record's
flags to `0x40` and sets flag `0x20`.

The release step takes a duration argument. It selects slot two's record,
raising a shared reselection mark when the selected record changed. While
`duration - 1` exceeds the task's elapsed input it returns one. Otherwise it
decrements the nesting count, clears the active word when the count reaches
zero, and returns zero.

### Camera marks and the render camera step

Besides the published (logical) pose the camera keeps a render pose: three floats holding
the position in whole map units and three angles. Four marks decide each frame whether the
render pose glides towards the published pose or cuts to it.

| Mark | Set by | Meaning |
|---|---|---|
| snap | camera setup; the first `showPlayer` call; the first `showObj`, `showObjLoc` and `showObjWorld` call; loading and pause paths | a cut is pending |
| hold | later `showPlayer` calls (so the `showPlayer` that ends a `showPlayerPan` too); the first `showObjLoc` call | forces a cut |
| forced glide | `showObjPan` and `showObjPanLoc`, on entry before the negative-duration test | forces a glide, and skips the pose update's obstacle segment check for the frame |
| in frame | the main loop, at the start of each frame | the frame is a normal one |

Camera setup clears hold and forced glide and cuts the render pose to the first published
pose (position shifted right eight bits, truncating toward zero; the angles unchanged).
`lookObj` and `showObjRot` set no mark.

The render step runs once per frame after the pose update. First it clamps the render
position's x and z floats into `[0, 65535]`. Then it glides when the following chain
reports a glide, the snap mark is clear and the frame mark is set:

1. hold set: cut;
2. forced glide set: glide;
3. the selected player holds a weapon, or is associated with a rope: glide;
4. the scripted-camera active word is nonzero: cut;
5. snap clear: glide;
6. the selected record has flag `0x20`: cut;
7. selected index zero, three or four: glide; otherwise cut.

A glide uses the installed `[camera]` factors `glide_x`, `glide_y`, `glide_z`
(0.9, 0.2, 0.9) and `glide_angle_x`, `glide_angle_y`, `glide_angle_z` (0.7, 0.1, 0.7).
With `p` the published position shifted right eight bits as a single float and `r` the
render position float, each axis becomes `r + (p - r) * factor` evaluated in extended
precision and stored as a single. The result times 256, truncated toward zero, goes through
the camera terrain correction, is divided by 256 truncating toward zero, and becomes the new
render float. Each published angle is replaced by
`(previous + trunc((published - previous) * factor)) & 2047`, where `previous` is the
render angle, the difference is wrapped once into half a turn either side of zero, the
single-precision product is truncated toward zero, and the factor for the first (pitch) angle
is `glide_angle_y` and for the second (yaw) angle `glide_angle_x`; the third uses
`glide_angle_z`. The published angles are therefore smoothed in place.

A cut sets the render position to the published position shifted right eight bits (truncating
toward zero) and clears snap and hold. Every pass clears forced glide, and the final published
angles become the render angles for the next frame.

### Follow camera observation

The debug overlay's `World (x, y, z)-(pitch, yaw, roll)` line is the published camera pose
and its `Player (x, y, z)-(pitch, yaw, roll)` line the selected player's, in whole map units
and 2048-unit angles. With the installed defaults (record zero position offset `(0, -14000, 0)`,
distance 250, target offset `(-6000, -14000, 0)`), a player at `(32464, -275, 33892)` with
angles `(0, 22, 0)` gave the camera `(32481, -330, 33642)` with angles `(0, 2027, 0)`: the
camera sits 250 units behind the player along the player's heading, 55 units above, and looks
at the player's height. The angle triple's orientation matrix carries local +Z along the look
direction with the X axis mirrored.

### Frame view angles and screen shake

After the render step the frame builds its view orientation from the published angles. A roll
tilt would be selected by a mode word that nothing writes (zero), so no tilt is applied. A
screen-shake level (0 to 32) wobbles the first angle: on even frames of the main loop's
frame counter the level is subtracted from the angle and then halved, on odd frames it is added
and kept; each angle is then wrapped to 2047. One shake impulse raises the level by 32, capped
at 32. Impulses come from the [actor-event service](animation.md#actor-events) (operand
bit `0x10`) and a debug key.

### Ordinary camera selection inhibition

On the ordinary camera-update path, a nonzero scripted-camera active word or
bit `0x20` in the selected camera record suppresses automatic slot reselection.
The camera still runs its pose-processing stage. These guards precede the
holding-weapon and other player-state checks used to choose an automatic slot;
they do not suspend the whole camera update.

When neither guard applies, the ordinary camera-update path requests the slot from the
[player camera slot selection](#player-camera-slot-selection) table and falls back to slot zero when no
condition applies, so a player that sheathes its weapon returns to slot zero on the next update.

### Camera request candidate position

The camera's initial position-composition stage gives record flag `0x04`
priority over `0x08`. With `0x04`, it adds the stored request vector to the
record's bound actor-position vector, with wrapping signed 32-bit components.
Otherwise, with `0x08`, it copies the stored request vector directly. These are
candidate positions supplied to subsequent camera processing, not final rendered
camera poses.

Before testing either request flag, this stage compares the stored distance
shifted left by eight against the stored minimum, using signed comparison. If
it is lower, the minimum replaces the distance without a compensating shift.
Equality leaves the distance unchanged.

Without either explicit request flag, the stage rotates the record's position
offset by the bound angles. It separately constructs a vector with zero X and Y
and negative distance shifted left by eight as Z. The angles for this second
rotation are the bound angles minus the record's angle offsets, masked to one
revolution. Flag `0x80` replaces the first of these angles with the negated first
angle offset, ignoring that bound-angle component. The candidate is the bound
position plus both rotated vectors, with wrapping signed components.

Camera vector rotation builds an identity matrix, applies negative yaw, then
positive pitch and roll, and transforms the supplied integer vector. These
angle signs differ from the ordinary actor-matrix update convention.

### Camera collision candidate radius

`SYSTEM.INI` `[camera] obstacle_size` supplies the candidate-size cutoff. Setup
stores it with eight fractional bits, and collection shifts it back before
comparing with each candidate model's radius. The model radius must be strictly
greater than this cutoff. This is separate from the added bounds-test radius
computed from the queried segment.

Candidate collection subtracts the fixed-point endpoints before arithmetic-shifting
the displacement components right by eight. It separately shifts the starting
point by eight. When the squared displacement sum fits signed 32-bit arithmetic,
the added model-bound radius is twice the floor of the Euclidean displacement
length. The center-distance test accepts equality with the squared sum of the
model radius and this added radius.

The bounds test stores each integer-origin minus model-center component at
single precision, then stores their summed squared distance at single precision.
It also stores the model radius plus added radius at single precision before
squaring that combined radius for comparison. Equality admits the model.

### Camera collision candidate eligibility

Candidate collection scans two linked object lists in order and appends model
references to one candidate array. A model equal to the caller's excluded model
is skipped in both lists.

Entries of the first list carry a model and a bounds record. An entry is a
candidate when its model and bounds are present, the bounds radius is strictly
greater than the obstacle cutoff, and the segment bounds test accepts the model.

Entries of the second list are actors. An actor is a candidate when it has a
model bounds record, is not excluded from ordinary object updates, and then
passes the same model, radius and bounds tests. The bounds-record flag is set
when the actor's script and model load establishes its bounds; the exclusion is
the flag the ordinary object-update loop tests.

The candidate array holds 64 entries. When it already holds 64, the engine writes a
diagnostic line instead of appending and stops scanning the current list; the
remaining entries of that list are dropped.

### Camera collision result selection

The query arithmetic-shifts each fixed-point endpoint component right by eight
before the geometry test. The integer wrapper converts returned hit positions
with the active x87 integer rounding mode and scales normal components by 256
before converting them. Selected output positions are shifted left by eight;
selected output normals retain the wrapper's scale.

Polygon hits store their position along the tested segment as a rank: the
segment parameter multiplied by 32768, converted with the active x87 integer
rounding mode. The segment start has rank zero and the end has rank 32768.

The camera collision query counts all returned hit records independently of
whether a record replaces either selected output. For segment ranks from zero
through 32768, minimum selection starts at 32767 and requires a strictly lower
rank. Maximum selection starts at zero and requires a strictly higher rank.
Equal ranks retain the first selected record.

A result containing only ranks 32767 or 32768 therefore reports hits without
replacing the shared minimum-position output. A result containing only rank
zero leaves the shared maximum-position output unchanged. The query does not
clear these output positions before scanning its candidates.

### Camera collision response

The pose collision stage runs when the selected camera record has flag `0x40`
and the shared collision-inhibition word is zero. It rotates the record's target
offset using its bound angles, adds the bound position, and queries the segment
from that target to the candidate camera position. Candidate collection excludes
the bound actor's model instance.

A nonzero query hit count replaces the candidate camera position with the shared
minimum-position output and reports success. A zero hit count leaves the
candidate unchanged. The response does not separately check whether the query
replaced its minimum output. An endpoint-only hit can therefore copy the retained
minimum position from before the query.

### Camera movement limiting

Camera record flag `0x01` bypasses the movement-limiting stage. Otherwise the
stage uses both the last published camera position and the preceding position.
Its scalar timing helper floors the signed frame scale at 180, multiplies with
32-bit wrapping, and arithmetic-shifts the product right by eight.

The movement budget starts with the distance between those two retained
positions plus the shared budget increment, capped by the record's unsigned
maximum. The timing helper scales that budget. When the candidate displacement
exceeds it, the stage forms an integer fraction with eight fractional bits and
scales displacement components with multiplication followed by an arithmetic
right shift. It reports that limiting occurred even when integer quantization
leaves a component unchanged.

A separate vertical limit precedes this distance test except for camera slot
one and the selected-player special-state branch. The lower bound is the timed
vertical parameter. A displacement above its negation is replaced with the
negation of the unscaled parameter. The two sides therefore do not use the same
timing conversion. The selected-player special-state branch also retains the
candidate's horizontal components during distance limiting while still applying
the vertical component and reporting limiting.

The shared budget increment is `SYSTEM.INI` `[camera] max_acc`. The vertical
parameter is `[camera] max_y_vel` negated before storage, so it is the negative
lower bound described above. Both are stored without scaling. Setup stores them
with the other shared camera values, and the same values are reloaded when the
selected player's camera defaults are reloaded.

### Camera orientation retention

Camera record flag `0x02` makes the orientation stage copy the previously
published camera angles into the new pose. This branch skips target-direction
calculation, leaves the record's bound-position history unchanged, and does not
refresh the two transformed camera vectors.

Without this flag, slots zero and one use only the target offset's vertical
component; other slots use the full vector. The stage rotates that offset by
the bound angles, adds the bound actor position, and derives the camera angles
from the direction between the published camera position and that target. It
also refreshes the bound-position history and transforms both stored camera
vectors using the derived angles.

Before angle conversion, the direction components are stored as single-precision
values. Their Euclidean length divided by 65536 is stored at single precision.
Each component is divided by that stored divisor, stored at single precision
again, and converted to an integer with the active x87 rounding mode. A zero
divisor produces a zero vector. Angle lookup consumes this quantized direction.

The direction is camera position minus the rotated target position. Using the
signed bearing lookup whose positive X axis is a quarter turn and positive Z
axis is zero, yaw is the X/Z bearing minus half a turn. Pitch is the bearing
of the rounded horizontal length against Y, minus a quarter turn. Both wrap
to 2048 units per revolution; roll is zero.

The two retained basis vectors use a direct angle matrix with positive pitch,
negative yaw and positive roll. Each output component is a dot product with a
matrix column. Vector inputs and dot products are stored at single precision;
final integer components use the active x87 rounding mode.

### Camera contact-normal response

The camera body-response stage converts the candidate fixed-point position to
floating coordinates and gathers contact normals. It sums normal components
after dividing their integer representation by 256. A nonzero sum is normalized.
If the sum cancels to zero, it instead uses the first contact normal divided by
256, without normalizing that fallback.

The response truncates the camera body's floating radius to an integer,
multiplies it by the selected response direction, scales the result by 256,
truncates each component toward zero, and adds it to the candidate position.
No contacts leave the candidate unchanged. Opposing contacts can therefore
produce a response in the first contact's direction rather than no response.

Each accumulated normal component is stored at single precision after each
addition. Normalization stores the reciprocal length at single precision, then
stores each scaled direction component at single precision. Each product of the
integer radius and selected direction is also stored at single precision before
scaling by 256 and truncating toward zero.

### Camera distance-height correction

Only slots zero and three run the distance-height correction. It subtracts the
bound actor position from the candidate, divides each signed fixed-point delta
by 256 with truncation toward zero, and computes the resulting distance. A zero
distance is replaced with 0.1 for the following comparison and division.

When that distance is strictly below the record's distance threshold, the stage
scales the vertical delta by threshold divided by distance. It replaces candidate
Y with bound actor Y minus the absolute scaled vertical delta converted back to
fixed point with truncation toward zero. Candidate X and Z remain unchanged.
Equality with the threshold leaves the candidate unchanged.

The divided deltas, summed squared distance, square root and threshold are
stored as single-precision values. The vertical division and subsequent
threshold multiplication each store a single-precision result before conversion
back to fixed point.

### Camera terrain-height clamp

When the terrain-height stage is enabled, both terrain queries receive candidate
X and Z arithmetic-shifted right by eight. The floating height result is truncated
toward zero before conversion back to fixed point. The camera body's stored
clearance parameter is multiplied by 1024 and truncated separately.

Only a candidate Y plus clearance strictly greater than the converted terrain
height triggers correction. The replacement Y is terrain height minus clearance;
X and Z remain unchanged, and the shared correction flag is set. Equality does
not set the flag or replace the pose.

The terrain query converts horizontal coordinates to cells with the active x87
integer rounding mode after applying `x / 256 - 0.5` and `z / 256 + 0.5`.
When both cell offsets from the retained terrain-window center are strictly
between -16 and 16, height comes from that retained grid. At either boundary or
outside the window, it comes from map samples through the height lookup table.
The camera query therefore does not always use the static map heights.

Each selected cell uses a triangle plane rather than bilinear interpolation.
Its diagonal joins `(i, j)` to `(i + 1, j - 1)`. When the horizontal offset
`x - i * 256` is greater than `j * 256 - z`, the third corner is `(i + 1, j)`;
otherwise it is `(i, j - 1)`. Equality uses the latter triangle. The query
publishes a normalized plane normal; the integer wrapper scales that normal
by 256 and converts it using the active x87 integer rounding mode.

For plane construction, the first vertex is `(i, j)`. In the greater-offset
branch the next vertices are `(i + 1, j - 1)` and `(i + 1, j)`; otherwise they
are `(i, j - 1)` and `(i + 1, j - 1)`. The two edges are second minus first
and third minus second. Each edge component is stored at single precision
before their cross product is formed. Replacing these adjacent edges with
mathematically equivalent edges can change the sampled height through rounding.

### Camera pose stage order

The pose stage composes a candidate and performs a segment collision check.
A hit replaces the candidate with the retained minimum hit position, then runs
distance-height correction, contact-normal response and terrain correction.

A second segment check follows on both paths. A hit again replaces the candidate
with the retained minimum position. The stage then runs distance-height
correction, contact-normal response, movement limiting and terrain correction,
in that order, before publishing the pose.

### Camera pose publication

The transition word clears when the pose stage reports a collision or terrain
correction. Without such a correction, it clears only when its value is one
and movement limiting did not occur. A limited update can retain value one;
other nonzero values also remain unchanged without correction.

After position correction, the pose stage moves the previously published camera
position and angles into their respective history vectors. It then publishes the
new position to shared camera state and to both the camera actor and alternate
camera actor. Camera setup stores the CAMERA instance as both, so the two writes
address the same actor. Orientation processing follows position publication, and its
result replaces the shared published angles.

Position history therefore contains the preceding published pose, not the new
request or a temporary collision candidate. Retaining orientation still advances
angle history to the preceding published angles.

### Camera record initialization

Camera setup clears all nine records, initializes record zero, and copies that
record into slots one through eight before applying slot-specific configuration.
The copied fields include the selected-player binding, position and angle
bindings, position offsets, angle offsets, distance limits and maximum movement
rate. Record zero has collision flag `0x40` before these copies.

The ordinary target-offset configuration is assigned to record zero after the
copies. It therefore does not initialize the other records' target offsets.
Combat target offsets are assigned separately to slot three. The remaining
slots retain their cleared target-offset vectors at this initialization stage.

### Camera initial state

Camera setup selects record zero. The shared transition word, the
scripted-camera active word and its nesting count are zero. Setup then computes
the first published pose with one pass of the pose stage that omits movement
limiting, and stores it as both the current and the previous pose.

### Camera update

The CAMERA actor's script body ends immediately. Its native hook,
`cameraController`, takes no arguments and is the camera update itself: the pose
update for the selected record's bound actor that the location camera natives
also run. The hook runs once per ordinary update, at the camera actor's position
in the ordinary actor traversal.

### ResetRotation

`ResetRotation(x, y, z)` acts on the receiver and returns zero. The actor keeps a
triple of the pitch, yaw and roll it was created with (the angles it has when it
is allocated; the player swap copies it between player records and
[`XResetCamera`](#xresetcamera) sets it to the current angles). For each argument
that is nonzero, the receiver's pitch (`x`), yaw (`y`) or roll (`z`) is replaced
by that retained component.

The previous-angle history then becomes the receiver's current angles, and its
orientation matrix is rebuilt. When the receiver's rotation mode (attribute 28)
is one, an alternate construction builds the matrix. Otherwise the matrix starts
as the identity and takes rotations of the negated yaw, then pitch, then roll.

### ReloadRotation

`ReloadRotation()` takes no arguments, acts on the receiver and returns zero. It
performs the second half of [`ResetRotation`](#resetrotation) without restoring
any angle: the orientation matrix is rebuilt from the receiver's current angles
(by the rotation mode's construction, as above) and the previous-angle history
becomes the current angles.

### ResetPosition

`ResetPosition(x, y, z)` acts on the receiver and returns zero. The actor keeps
the world position it was created at. For each argument that is nonzero, the
receiver's local position on that axis (`x`, `y`, `z`) is replaced by the
retained component, even for an attached actor. The previous-position history
then becomes the receiver's current local position.

When the receiver is the selected player, the shared view origin moves by the
change the reset made on each axis. The receiver's transform is then rebuilt.

### MovePlayerAboveMe

`MovePlayerAboveMe(x, y, z, angle)` returns zero and asks for the selected player to be placed relative to the receiver. The offset
`(x, y, z)` is rotated by the receiver's orientation matrix and added to the receiver's position (the world position of
an attached actor, otherwise its local position, taken through the parent when it has one). The requested player Y is that
sum minus the selected player's positive Y extent shifted left eight bits, minus the `[system]` `standing_height` value from `SYSTEM.INI`
shifted left eight bits; X and Z are the sum itself. The call stores this position and `angle` as one pending placement; a later call in the same frame replaces it.

The main loop applies the pending placement once per frame and then clears it. The selected player's
local position and its previous-position history both take the requested position, its yaw becomes `angle & 0x7ff`,
and the camera is marked to cut instead of glide. The call has no other effect on the receiver.

### XResetCamera

`XResetCamera` takes no arguments, acts on no receiver state and returns zero. In
order, it:

1. Runs the `LockoutPlayer` routine for the selected player with zero.
2. Runs the [`KillMyTasks`](#taskstop-taskpause-and-taskresume) operation on the selected player and
   clears five flag bits (`0x01`, `0x02`, `0x04`, `0x08`, `0x10`) of the actor byte
   that immediately precedes its task arrays.
3. Rebuilds the selected player's orientation matrix from its current angles and
   stores those angles in both of its retained angle-history triples.
4. Sets the scripted-camera active word and its nesting count to zero.
5. Applies step 2 to the camera actor.
6. Reloads record zero's [player-type defaults](#camera-initial-state-and-player-type-settings),
   selects record zero, resets its flags to `0x40` and recomputes the first
   published pose as setup does.

### Camera record installed settings

Camera setup reads `SYSTEM.INI` `[camera]` integers and stores them without
scaling except where stated. Record zero receives, before the copy to slots one
through eight:

| Record field | `[camera]` keys |
|---|---|
| Position offset | `offset_pos_x`, `offset_pos_y`, `offset_pos_z` |
| Angle offset | `offset_angle_x`, `offset_angle_y`, `offset_angle_z` |
| Distance | `camera_distance` |
| Minimum distance | `camera_min_distance` |
| Maximum movement rate | `max_vel` |
| Left position limit | `camera_left_pos` |
| Right position limit | `camera_right_pos`, negated |

Record zero also gains flag `0x40`. `target_offset_x`, `target_offset_y` and
`target_offset_z` are assigned to record zero after the copy, and to slot three
from the same keys.

Slot-specific settings follow the copy. Each replaces the slot's angle offset and
distance; a nonzero `*_max_vel` replaces its maximum movement rate, and a zero
value keeps the copied record-zero rate.

| Slot | Mode | Angle offset keys | Distance key | Maximum movement key |
|---|---|---|---|---|
| 3 | combat | `camera_combat_angle_offset_x/y/z` | `camera_combat_distance` | `camera_combat_max_vel` |
| 4 | hanging | `camera_hang_angle_offset_x/y/z` | `camera_hang_distance` | `camera_hang_max_vel` |
| 5 | rope | `camera_rope_angle_offset_x/y/z` | `camera_rope_distance` | `camera_rope_max_vel` |
| 6 | rope above | `camera_rope_above_angle_offset_x/y/z` | `camera_rope_above_distance` | `camera_rope_above_max_vel` |
| 7 | rope below | `camera_rope_below_angle_offset_x/y/z` | `camera_rope_below_distance` | `camera_rope_below_max_vel` |
| 8 | debug | `camera_debug_angle_offset_x/y/z` (see below) | `camera_debug_distance` | `camera_debug_max_vel` |

The debug slot's three angle-offset reads all store to the first angle-offset field; its second and third angle offsets keep the copied record-zero values.

Slot three gains flag `0x40` (already present from the copy). Slots six and seven
additionally load aim vectors from `camera_rope_above_aim_x/y/z` and
`camera_rope_below_aim_x/y/z`, bind the record's aim reference to them, and set
their flags to exactly `0x40`. Slots four and eight keep the copied flags. Slot five gains flag `0x80` in addition to `0x40`, giving `0xc0`.
Slots one, two and four through eight keep their cleared target-offset vectors.

`obstacle_size` and `fudge_factor` are shifted left eight bits and `max_y_vel` is
negated before storage in shared camera state, outside the records.

### SheathSword

`SheathSword` takes no arguments and returns zero. It acts only when all of these
hold for the receiver: the task system is enabled, the receiver has animation state and
a combat record, the combat cooldown is zero, the lifecycle byte is not in 27 through 32,
the receiver is neither drawing nor sheathing a sword (the `IsDrawingSword` and
`IsSheathingSword` predicates), and `IsHoldingWeapon` is true. Otherwise it changes
nothing.

When it acts, the lifecycle byte becomes 21 (sheathing), the companion motion byte is
cleared, bit `0x04` of the combat flags is cleared, and the combat cooldown becomes 24.
For the selected player it additionally sets the HUD health-display timer to 25 and selects
camera slot zero.

### endCombat

`endCombat` takes no arguments and returns zero. It first does what [SheathSword](#sheathsword) does for
the receiver, including its eligibility tests. It then unconditionally sets the lifecycle selector
(`MyAIType`) to 1 and the AI mode (`MyAIMode`) to 0, re-enables the receiver's native hook as
`EnableHook` does, clears combat flag `0x04`, and zeroes the combat record's close-distance pass counter and
coordination selection state. A receiver without a combat record faults in the original.

### FacePlayer

`FacePlayer(duration)` is a task that turns the receiver's local Y angle toward the
selected player. Each invocation computes the bearing from the receiver's local position to
the selected player's local position as the arctangent angle of the displacement, which
is the negation (modulo 2048) of the bearing the `PlayerLooking` query reports, then:

1. `gap = (-heading - angle) & 2047`, which is the `PlayerLooking` bearing minus the heading.
   A zero gap completes the task without a change.
2. `remaining = |elapsed - duration|`, treated as 1 when zero.
3. If `gap` is above 1024, `gap = (-gap - 1024) & 1023` and the step is subtracted from
   the heading; otherwise the step is added.
4. `step = ((gap << 8) / remaining) >> 8`. A zero step completes the task without a change.
5. The heading wraps modulo 2048.

After a turn the task completes when `elapsed >= duration - 1`, and remains active
otherwise.

### FacePlayerInertia

`FacePlayerInertia(duration)` follows [FacePlayer](#faceplayer) except for the step. After the gap is
reduced as FacePlayer does (a gap above 1024 becomes `(-gap - 1024) & 1023` and subtracts), the step is
`(((gap << 8) / duration) - elapsed) >> 8` with signed division and an arithmetic shift; a duration of
zero is a division fault. A zero step, including a negative one that shifts to zero, completes the task
without a change. The turn, the wrap and the completion test are the same.

### Potion counters and combat strength and armor

`SetPotion(index, value)` stores the full 32-bit `value` in a global counter table and
returns one. `GetPotion(index)` returns the stored value. The table is zero-filled when
the engine starts and is not cleared by world changes. The shipped scripts use indices
0 through 3 (blood, milk, ectoplasm and sap items).

`setStrength(value)` and `setArmor(value)` store the full 32-bit value in the strength and
armor fields of the receiver's combat record and return nothing. They apply to a receiver
that has a combat record.

### Nonplayer combat decision: target, distance and engagement

The nonplayer decision stage (death and state selector 6, actor not dead) begins as follows.

The decision entry caches the current 32-bit clock sample for combat pacing and timed
taunts. In normal clock mode this is the raw BIOS-clock sample, interpreted as signed.
It is distinct from nominal-tick increments and from the world-loop frame counter.
The world-loop counter resets to zero on loop entry and increments once at the beginning
of each iteration. In the waiting branch, after spacing, one random draw admits a
chance taunt when `draw * 100 / 32767 < 2`. The taunt type is 5 when the wrapping sum
of the receiver's allocation index and world-loop counter is even, and 8 when odd.
Timed selection of that requested type still uses the cached clock sample.

**Valid target.** An actor is a valid combat target when it exists, it is not both in
lifecycle state 8 and killed, it has a combat record, and its own combat target is not itself.

**Target selection.** When the receiver's combat target is not valid, the target becomes
the selected player, whether or not the player is itself a valid target, and the target's
local position is copied into the receiver's retained target position. A valid
target leaves that position unchanged until engagement succeeds.

**Distance and counter.** The distance from the receiver to its target uses the query
positions of both actors (attached, linked or local, as for the fighting query) and the
three-dimensional rounded distance. While that distance is below 25000 a counter in the combat
record increments once per pass with signed 32-bit wrapping; otherwise the
counter is zero. Combat construction and reinitialization clear this counter
and the retained target position. After engagement succeeds, the target's local
position replaces the retained target position before the defence and attack
executors run, in that order, each with request -1. An active result from either
executor ends this decision pass before spacing and further selections.

**Engagement.** The receiver is engaged when `attr_combat_ignore_in_sight` is nonzero.
Otherwise, with `attr_combat_long_sight` nonzero, it is engaged when the target is in sight
within a distance of `0x80000`; with it zero, within `0x40000`. Both distances use the
position's eight fractional bits.

**In sight.** The test is false for an absent actor or target and when their distance exceeds
the limit. Otherwise the segment between the two query positions, in whole map units, is tested
against each entry of the engine's retained candidate model list, in order. The test does not collect candidates itself: it reads the list collected for the receiver by ordinary actor traversal immediately before the receiver's update (see [Candidate collection for an ordinary update](#candidate-collection-for-an-ordinary-update)), so only models near the receiver can block the line. An entry that is
not hit, or whose owner is the target, does not block; any other hit makes the result false.
The result is true when no entry blocks.

### Nonplayer defence selection

The defence selection runs only while the receiver is armed and the
selected-player hit-inhibition flag is clear. Standalone-player initialization clears
this flag; selected-player health death sets it. With no move selected, the target must lie inside the receiver's
forward arc and its distance must be strictly below the receiver's far window.
The forward-arc query subtracts the receiver's local position from the target's
local position with signed 32-bit wrapping arithmetic, then arithmetic-shifts each
coordinate right by eight. It obtains the normalized vector's orientation and
compares the receiver's local yaw with `(1024 - vector_yaw) & 2047`. The absolute
wrapped difference must be strictly below 256 heading units. Composed query poses
are not substituted for these local positions.
The defence counter becomes one when the headings differ by less than 256 heading
units or the target is outside combat stance. Otherwise it becomes `129 - attribute 58`.
A target is in combat stance when it is armed or its combat flag mask `0x04` is set.
The flag query returns false without a combat record; the action query returns -1.
Heading differences mask both headings to 0 through 2047 and wrap the difference
into -1024 through 1024.

The pacing sum is the receiver's per-actor combat value plus the shared tick counter,
with signed 32-bit wrapping addition. When its signed remainder by the defence pacing
divisor is zero:

- For a target move other than -1 or 9, the target's signed animation command
  cursor must exceed the wrapping sum of its current motion's `first_collide_frame`
  and `collide_duration`. The current attack slot and motion selector choose that
  motion. This uses neither the displayed model frame nor the wrapping frame-progress
  byte. The receiver
  gains intent mask `0x01`, then asks the best-usable-defence picker for a kind.
  A successful pick starts only if its reach test succeeds.
- For target move 9, the open-defence picker supplies a kind, which starts only
  if the pick and reach test succeed. This branch does not set the intent mask.
- For target move -1, neither of these picks runs.

After this branch, independently of its pacing remainder, a zero signed remainder
of the pacing sum by the defence counter sets intent mask `0x01`, obtains the paced
pick and starts it if the reach test succeeds. Starts happen synchronously, before
any later picker reads the combat state.

With a move already selected, the selection uses only the defence counter remainder.
On zero remainder, defence selector zero requests kind one and selector one requests
kind two; either request first sets intent mask `0x01`. Other selectors do nothing.
Intent updates preserve every other combat flag bit.

### Best and open defence picks

Both pickers scan the receiver's nine authored attack slots in ascending slot order.
These are the attack slots bound from attributes 70 through 78, not the separate
four defence records. The first motion supplies each slot's distance window,
near timing and counter-attack reference.
Absent slots are skipped. A slot is usable only when the signed actor-to-target
distance is strictly greater than its minimum range and strictly less than its
maximum range. The best picker chooses the usable slot with the lowest signed
near timing value, strictly below 65536. Equal values retain the earlier slot.

The open picker applies the same rules, but excludes a slot whose one-based
counter-attack reference minus one equals the receiver's current move's
counter-attack index. The subtraction wraps as signed 32-bit arithmetic.
Best selection does not read counter-attack references. Open selection reads the
candidate counter-attack reference, then queries the current move's counter-attack
index, only after that slot passes its distance window. An excluded slot does not
read near timing. Each admitted slot repeats the current-move query.
Both pickers map a winning slot of six, seven or eight to defence kind four.
If no slot qualifies, they report failure without changing the caller's output.

The current-move counter-attack query returns zero when the receiver has no
combat record or its action selector is -1 or 9. These cases do not read attack
or motion metadata. Otherwise, it resolves the receiver's retained attack-slot
selector and current motion selector, reads that motion's authored one-based
counter-attack reference and subtracts one with signed 32-bit wrapping.
The action selector gates the query but is not the attack-table index.

### Paced defence pick

The initial choice is zero. One random draw in the inclusive range zero through
32767 produces `draw * 6 / 32767`, with unsigned integer division. That slot
replaces the initial choice only if it exists and passes the strict distance
window. The maximum draw therefore probes slot six. This fallback does not
require arc or elevation admission and does not remap the slot.

The picker then scans slots zero through five in ascending order. A populated
slot must pass distance, arc and elevation admission, in that order. A failed
test prevents the later tests for that slot. The fallback reads only existence
and the distance window. Neither path reads the counter-attack reference.
If the receiver and target
face alike, or the target is outside combat stance, a slot replaces the choice
only when its signed accumulated damage exceeds the retained damage, initially zero,
and the pacing sum has zero signed remainder by the defence pacing divisor.
A successful replacement retains that damage. Equal damage totals retain the
previous choice.

Otherwise, each admitted slot resets the defence counter to `129 - attribute 58`
before reading its near timing. This branch does not read accumulated damage.
The picker tracks the lowest near timing strictly below 65536, with the same
zero-remainder gate, but this branch does not replace the chosen slot. The final
return is the fallback or the last priority-branch replacement. No qualifying
slot leaves the initial choice of zero intact.

### Defence elevation and arc admission

Both tests read the first motion of the receiver's selected attack slot.
Elevation admission uses the receiver and target's bounds-query positions.
For each coordinate, that query computes the signed 32-bit wrapping expression
`position - (negative_extent << 8) + (positive_extent << 8)`, then divides by two
with truncation toward zero. It does not substitute the actor origin.

Let `d` be target bounds-query y minus receiver bounds-query y, with signed
32-bit wrapping subtraction. Let `w` be the wrapping sum of the receiver's
positive and negative x extents, each shifted left eight bits, and let `h` be
`w / 2`, truncating toward zero. The required elevation is:

- One when `-h > d` and `-(w + h) < d`.
- Otherwise two when `d > h` and `w + h > d`.
- Otherwise zero.

All additions and negations in those comparisons wrap as signed 32-bit values.
The motion passes only when its elevation equals the required value. Band edges
belong to elevation zero; they do not pass either outer band.

Arc admission reads both actors' ordinary local origins and the receiver's local
heading, without attached or linked query-position substitution. It subtracts
receiver position from target position in each coordinate
with signed 32-bit wrapping subtraction and arithmetic-shifts each difference
right eight bits. The bearing input is ordered `(delta_z, delta_y, delta_x)`
and normalized to signed Q16 using the [authored-location normalization](#movetolocation-authored-locations).
The normalized vector uses the signed polar lookup described by
[camera orientation](#camera-orientation-retention): its positive first axis is
a quarter turn, its positive third axis is zero, and the lookup result is reduced
by half a turn and wrapped modulo 2048. A zero horizontal vector yields 1024.
Let `a` be the
absolute wrapped heading difference between the receiver heading and that bearing,
and let `c` be the motion's arc centre plus 512. The test passes only when
`c + arc > a` and `c - arc < a`, using signed 32-bit wrapping arithmetic for the
bounds. It does not halve the configured arc or include either boundary.

### Defence start admission

The starter dispatch admits kinds zero through six and kind eight only when the
corresponding receiver attack slot is populated. Kind seven and values outside
zero through eight do nothing. Admission differs by kind:

- Kind zero, three, five and six require no selected move.
- For a nonplayer receiver, kinds one and two do not require an idle selection.
  For the selected player, kind one requires selected move zero and kind two
  requires selected move one.
- Kinds four and eight require zero combat cooldown, neither drawing nor
  sheathing, and no selected move.

An admitted start first writes its kind into the defence selector, then selects
that move, then invokes the defence executor with request zero synchronously.
The executor can therefore change state before a later picker or starter reads it.
It resolves the defence selector through the receiver's nine attack slots and
uses the move selector to choose that attack's motion. When the actor leaves the
expected animation group, a populated following motion admits the next chain
step, up to the third motion. The test reads the following motion's first word, not the
current one's. Motion loading stores one in that first word for every motion it fills,
whatever the authored `type` is (the authored `type` lives in a separate field), and unfilled
motion slots keep their zero-initialized word. A combo therefore chains through all of its
referenced moves.
For selected-player kind four, the movement update precedes those writes and uses
attribute 16 shifted left eight bits with movement mode one. Its return value does
not gate the subsequent defence start. The nonplayer kind-four path omits that
movement update.

When the defence executor starts or advances to a new motion, it requests the
animation group, then sets the animation record's finite loop countdown to one.
This is the same countdown used by `PushAnimation` and the animation interpreter.
The move selector is written before the animation request. After the request and
loop-count write, a start clears target mark `0x10`, sets executor state one,
raises intent bit one, clears mask `0x20` and subtracts ten from aggression, then
synchronously requests a type-one timed taunt. A chain step clears the target
mark, raises intent and subtracts ten before clearing mask `0x20`; it does not
request another type-one taunt. Animation services therefore run before these
combat-state changes, and the taunt service sees the committed start state.

### DisableHook and EnableHook

`DisableHook` sets and `EnableHook` clears mask `0x02` of the receiver's script-control byte
(the native-hook suppression flag above) and changes nothing else. Both take no arguments
and return zero.

### GetFramePassed

`GetFramePassed` takes no arguments and returns the shared whole nominal-tick increment of
the current update (the same value the elapsed-time tasks consume).

### ShowItem and ShowItemNoRtx

`ShowItem(item, text, first, second)` and `ShowItemNoRtx(item, first, second)`
return zero and only leave a request for the main loop's item viewer. Each stores
the item identifier and the two trailing words and marks a request pending. `ShowItem`
ignores its second argument and leaves the viewer's no-text flag as it was;
`ShowItemNoRtx` sets that flag. The flag stays set until the viewer consumes the
request. The handlers perform no display and read no inventory state.

### ViewGXA

`ViewGXA(name, frame, count)` is a task that shows one installed GXA image over the world
frame. `name` is a file name relative to the game directory (the shipped scripts name
`system\stmap01.gxa` through `stmap17.gxa` and `system\mirror1.gxa` through `mirror3.gxa`),
`frame` selects the image within the file and `count` is the number of task counter ticks.
The first invocation (counter zero, nothing loaded) loads the file. Every invocation whose
counter is below `count - 1` places the selected frame centered on the 640 by 480 screen
(x is half of the 640 minus the frame width, y half of the 480 minus the frame height),
clears the second image slot and stays active. The invocation at counter `count - 1` or later
discards the image, clears the drawn-image pointer and completes, so the image is drawn
for `count - 1` frames (the same completion test as `Wait`). The main loop draws the pending image after the world frame.

### DropAllItems

`DropAllItems` takes no arguments and returns zero. When the task system is enabled it
applies the `DropItem` routine to every item id from zero up to the item count, each with the
count currently owned, in ascending order, and then clears the selected item. Ids with a
zero count are skipped. Each id must have an `ITEM.INI` definition; the shipped file
defines ids 0 through 86 without gaps.

### InRectangle

`InRectangle(x1, z1, x2, z2)` returns one when the receiver's query position lies inside the
rectangle on both horizontal axes, otherwise zero. The query position is the attached pose
for an attached receiver, the linked position when one exists, and the local position
otherwise (the same selection as the fighting query). Each pair of corners is ordered
independently, so the corners may be given in either order, and the comparison is inclusive.
The arguments use the position's runtime units.

### Camera initial state and player-type settings

At world load the camera actor and the alternate camera actor are both the `CAMERA` instance.
The shared selection starts at slot zero and record zero is the selected record. Each record's
bound actor is the selected player. The stored bound-position history of record zero starts as
the bound actor's current position. The shared state block (transition word, scripted-camera
active word and both published poses) starts zero until setup runs the full camera pose stage;
its result then becomes both the current and the preceding published pose.

Record zero's distance, minimum distance, target offset and position offset take the
installed `[camera]` values listed in the record settings above when the selected player's
`attr_player_type` is not one. When it is one, they take a reduced set derived from the same
keys instead:

| Record field | Value for player type one |
|---|---|
| Distance | `camera_distance` arithmetic-shifted right one bit |
| Minimum distance | `camera_min_distance` arithmetic-shifted right one bit |
| Target offset | `target_offset_x >> 1`, `6400`, `target_offset_z >> 1` |
| Position offset | `offset_pos_x >> 1`, `6400`, `offset_pos_z >> 1` |

### Script light controls

`Light`, `LightRadius`, `LightIntensity`, `LightOffset`, `LightFlicker`, `FlickerLight` and
`LightFlickerOff` act on the receiver's light record and return zero. When the receiver has no
record, the first call creates one: disabled and unlocked, offsets zero, intensity 256 and
radius 100 (each current value equal to its starting value), flicker fields zero. `LightOff` does
not create a record.

Map loading gives a type-4 object a record of its own: locked and enabled, with the placement's intensity
and radius as both current and starting values and colour zero (the placement's colour is not copied, unlike
for a type-6 object), so those objects' first light call updates an existing record.

| Function | Effect |
|---|---|
| `Light(radius, intensity)` | Enable; set current and starting radius and intensity |
| `LightRadius(radius)` | Set current and starting radius; enable; clear the inactive counter |
| `LightIntensity(intensity)` | Set current and starting intensity; enable; clear the inactive counter |
| `LightOffset(x, y, z)` | Set the three offsets |
| `LightFlicker(a, b)` | Set flicker values `a` and `b` and clear the flicker phase word |
| `FlickerLight(a)` | The same as `LightFlicker(a, 1)` |
| `LightFlickerOff` | Set flicker value `a` to zero |

Every stored value keeps only its low sixteen bits.

### Nonplayer combat taunt requests

The decision stage requests a taunt of a numbered [type](../config/combat-ini.md#taunt-types) from
the receiver's taunt list (each record carries a type, a four-byte RTX label, and an animation
value). A request is ignored when the receiver is dead or its list is empty, and when type 9 is
requested while combat flag `0x40` is set.

**Distance gate.** For types other than 0, 2 and 6, a request with a combat target is ignored when
the distance to the target is `0x40000` or more.

**Busy gate.** For types 10 and 11, the request is ignored while the receiver has a current taunt
whose pending field is set.

**Types 0, 2, 6, 10 and 11** use the in-order scan with the one-in-four random stop described in the
taunt selection rules, including the death fallback to the hurt type here expressed as: when
nothing matched for type 6, the request repeats as type 2.

**All other types, including 1, 3, 4, 5, 7, 8 and 9,** consume fifteen draws, each choosing a list index `draw * count / 32767`,
even after a record has been selected. A
probed record of the requested type with a nonzero label becomes the choice when no choice exists
yet, or otherwise replaces it when `(combat value + clock sample) mod 2` is zero. The combat
value is the combat record's retained allocation index. The decision stage caches the current
32-bit clock sample. In normal clock mode this is the raw BIOS-clock sample, interpreted as a
signed value, rather than a nominal-tick increment or accumulated nominal ticks. Their sum wraps
as a signed 32-bit value. The taunt is then presented only when `(combat value + clock sample)` is a
multiple of the combat record's signed taunt period, initialized to `129 - attribute 87`;
presenting a type 9 taunt also sets combat flag `0x40`.

Presenting a taunt replaces the current taunt reference in the combat record.

### PlayAnimation

`PlayAnimation(group, modifier)` requests an animation group for the receiver and returns zero
without waiting (unlike `PushAnimation`, which is a task). The request goes through the ordinary
[request arbitration](animation.md) with the modifier narrowed to one byte. It does nothing when
the animation system is disabled, when the receiver has no animation state, or when the receiver
is excluded from ordinary object updates.

### Candidate collection for an ordinary update

When ordinary traversal reaches an actor whose collection flag is set, it fills the engine's
retained candidate model list before running that actor's update (native hook, combat decision and
script stages included). The list holds the models inside a sphere centred on the actor's
position, with the actor's own model excluded. The sphere radius is twice the stored radius of the
actor's current model, rounded to the nearest integer, or 200 when the actor has no model. The
collector keeps at most 64 entries. Queries made later in the same update (the in-sight test and the
nonplayer movement probes) read this list without collecting again.

### Nonplayer blocked-ahead probe

The movement helpers used by the combat decision ask whether the space ahead of the receiver is
blocked by `distance`. The probe applies only when the receiver has a combat record and its current
animation group has [class](animation.md#animation-group-classes) one. It forms a local offset with
X zero, Y equal to `(positive Y extent of the actor's body - SYSTEM.INI post_collide_height - distance)`
in eight-fractional-bit units, and Z equal to `-14336`; rotates it by the actor's matrix; and adds the
actor's query position. The resulting single point (radius zero) is tested against each model in the
actor's [candidate list](#candidate-collection-for-an-ordinary-update): any contact blocks. Candidates are
typed by source: static placements and the secondary list carry type `0x20`, actor-owned models carry type
`0x80`. While the actor's queued-motion byte is `0x18` (24), actor-owned candidates are skipped, so
only static models can block.

### Session requests: LoadWorld, WonGame, PushPlayer and PopPlayer

`LoadWorld(world, marker, heading)`, `WonGame`, `PushPlayer` and `PopPlayer` return zero and only record a
request; the main loop acts on it after the current script step. `LoadWorld` retains its three integers.
The main loop then loads the world: the world index must be 0 through 99 and name a WORLD.INI entry
(otherwise loading ends with an error), the current-world index becomes that value, the selected start marker
(the second argument) supplies the player's starting position triple, and the third argument, masked to 0
through 2047, supplies the starting heading. The three words are cleared when the load begins.
`WonGame`, `PushPlayer` and `PopPlayer` each set one request flag for the main loop, which consumes them.
`WonGame` ends the world loop and starts the [ending](menu.md#selection-and-actions).

`PushPlayer` and `PopPlayer` are consumed at the end of a world-loop iteration that does not leave the loop:
push first, then pop. Each clears its own flag when consumed, whether or not it acts. The engine keeps the
CYRUS and GREMLIN player records and one swap state, clear in a fresh world. While the swap state is clear
CYRUS is the selected player and GREMLIN is excluded from ordinary object updates; while it is set GREMLIN is
selected and CYRUS is excluded. The world loop re-asserts that selection and exclusion every iteration.

- `PushPlayer` acts only while the swap state is clear. GREMLIN becomes the selected player, the swap state is
  set, and GREMLIN takes CYRUS's player transfer (below). GREMLIN stops being excluded
  from ordinary updates and CYRUS becomes excluded.
- `PopPlayer` acts only while the swap state is set. CYRUS becomes the selected player again, the swap state is
  cleared, and CYRUS takes GREMLIN's position, heading and retained movement state with the same exclusion
  exchange reversed. CYRUS then receives an [ordinary animation group request](animation.md) for group 0, issued twice.

A second `PushPlayer` while swapped, or a `PopPlayer` while not swapped, only clears its flag.

The player transfer copies from the role that was selected to the role that becomes selected: the local position,
two further triples of 32-bit words at offsets `0xFB` and `0x108`, the three angles, which the orientation setter
masks to 0 through 2047 and also stores as the creation angles (`start_angle`, offset `0x144`) and the previous-angle history (`last_angle`, offset `0x150`) before it
rebuilds the orientation matrix, the four motion bytes at `+0x1B1` through `+0x1B4` (the last is the living
state), the 32-bit word at `+0x1CD`, the two words at `+0x08` and `+0x0C`, and byte 3 of the attribute block (the
health attribute). It zeroes the words at `+0xC7` and `+0x25`, re-registers the new role in its map cell, and clears the
new role's object-update exclusion byte while setting the old role's.

### KillMe and Resurrect

`KillMe` and `Resurrect` take no arguments, act on the receiver and return zero. Both first release the
receiver's two held walk-node reservations: a reserved node is cleared only when its recorded owner is the
receiver, and both reservation slots then hold no node.

`KillMe` then clears the receiver's current model presentation record, releases its two retained per-actor
buffers, sets its killed flag and its death selector to 8, and stops every active sound owned by the receiver.
The two buffers are the receiver's [light record](#script-light-controls) (41 bytes) and its 20-byte flat
sprite record, the record the `Flat` functions control: an enable byte, a pending byte, three 16-bit offsets, the
texture identifier, a frame cache and the playback counters. Each pointer is cleared after its buffer is freed,
so a later light or flat call creates a new record with the defaults.

Map loading creates a flat record for every type-3 placement, enabled, with zero offsets and the placement's
texture value as its texture. While a record is enabled and its texture is nonzero, the ordinary actor drawing
traversal draws the actor's flat as a camera-facing sprite at the actor's position plus the offset resolved
through its orientation, using the texture's attributes for its flags and alpha. The installed type-3
placements are the `LAVABUB1` lava bubbles (`DRINT`, `ISLAND`, `CATACOMB`) and the `LH_FIRE` and `CT_FIRE` fires.

The drawing routine runs once per actor in the ordinary traversal, for actors whose draw-suppress byte is clear. It
draws nothing unless the record's enable byte and texture identifier are nonzero. The three signed 16-bit offsets
are transformed by the actor's current pose, and the integer result (the transformed point shifted right by eight)
is the sprite's world position. The record caches the loaded image slot for its texture; while that cache is zero it
is filled from the texture identifier, and a zero slot or one above 30000 is a fatal error. The texture's attribute
record, when present, gives the draw flags (the flag word is `1` plus the attribute's flag byte) and the alpha (the
attribute's alpha byte plus one); without it the alpha is 255. The sprite is then submitted through the same routine
as [placed flats](texture-animation.md#placed-flat-admission) with the cached slot, the record's frame word, the
position, scale 1 and those flags and alpha, with the world's `world_ambientfx` value (see [WORLD.INI](../config/world-ini.md#lighting)) as the brightness. When flag
`0x80` is set the flag word also gains `0x200` and the brightness, alpha and size inputs are replaced by 200, 200 and
200, which that submission reads as a solid-colour tint (red, green and blue all 200).

Each update of an actor that is not hidden also steps its flat record, beside the light maintenance. A zero or
`0xffff` texture disables the record. A disabled record adds the elapsed ticks (low byte, wrapping) to its pending
byte and is freed once that byte exceeds 32. An enabled record first consumes a wrap marker (the high bit of its
loop byte): the marker clears and the frame returns to the record's counter. A frame of `0xffff`, or a counter equal
to the last frame, shows a static image. Otherwise the frame moves by the elapsed ticks times the speed byte, up when
the counter is below the last frame and down when it is above. Passing the last frame disables the record when the
loop byte is zero; otherwise it sets the wrap marker and restarts the frame at the counter plus the overshoot (clamped
to the counter if that still passes the last frame).

`Resurrect` acts only while the killed flag is set or the death selector is 8; otherwise nothing further
happens. It clears the killed flag. When the receiver has combat state and its death selector is 6, it first sets the
selector to 1 and the death response to 0, clears combat flag `0x04` and zeroes the close-distance pass counter and
the attacker selection state (the unconditional part of the end-of-combat reset); a receiver that `KillMe` just
killed has selector 8, so Resurrect leaves its selector unchanged. With combat state it then re-initializes that
state from the receiver's attributes.

<a id="killed-object-removal"></a>

### Removal of killed created objects

Each world-loop iteration, before the family update, removes every object that was created after the world loaded
(it has no map object) and whose killed flag is set: it leaves its map-cell list and the actor table, its `KillMe`
cleanup runs again and its record is freed. A killed map actor stays allocated; the ordinary actor update skips
killed actors.

<a id="projectile-ai"></a>

### projectileAI

`projectileAI` is a task without arguments; its update runs on the receiver, a created object whose parent is its
creator. The update returns one to keep flying and zero to end the task:

1. The speed is the receiver's attribute 8 shifted left by eight, multiplied by the frame scale and shifted right by
   eight (signed). The previous local position is saved, then the offset `(0, 0, speed)` rotated by the receiver's
   pitch, yaw `& 0x7ff` and roll is added to its local position.
2. A distance from the parent of `0xbb800` (Q8) or more ends the task, keeping the moved position. The distance is the
   single-precision magnitude of the X, Y and Z differences rounded to nearest-even; it is zero without a parent.
3. When the world has terrain and the receiver's Y is strictly greater than the terrain height at its integer X/Z
   (the [terrain support sample](player.md#terrain-support-sampling), times 256 and rounded), the previous position is
   restored and the task ends.
4. Candidates are gathered around the receiver's position with the radius attribute 8 (map units), excluding the
   receiver's own model, by the [live and transient candidate rules](player.md#live-and-transient-collision-candidates)
   (static placements, ordinary actors, transient models). The segment runs from the previous to the new position,
   both shifted right by eight.
5. Every face-model candidate not owned by the parent is queried with that segment, without polygon sidedness. A
   segment that ends exactly on a face plane does not cross it. Over
   all contacts, the one with the smallest rank (strictly less than `0x7fff` and every earlier rank) is kept. When any
   contact exists, the receiver moves to that contact's integer point shifted left by eight and the task ends.
6. Otherwise every actor-style candidate except the parent is tested in candidate order with the actor's collision
   spheres. A sphere is hit when either segment endpoint lies within its radius (squared integer distance computed in
   single precision and rounded, compared with the squared radius). The spheres are the actor's RAVC records (flag 2
   always, flag 4 only while the actor's current animation group pair matches the record), followed, unless
   attribute 56 is nonzero, by the actor's [ordinary collision shape](player.md#ordinary-actor-collision-shape-initialization)
   spheres; each center is rotated by the actor's orientation and added to its position. At most 128 spheres take part.
7. On the first hit, an actor with combat state is sent combat message `0xd005` from the parent; with or without
   combat state, the previous position is restored and the task ends. Without a hit the task continues.

### InKey

`InKey(code)` returns one when a keyboard key is down and zero otherwise. The low byte of the argument
is a character; its key is found by matching the character against the keyboard table's unshifted
and shifted characters, and a character with no table entry returns zero. The next three bytes each
name a required modifier by a character, either case: `s` Shift (either shift key), `c` Control and
`a` Alt; the result is zero when a named modifier is not held. Otherwise the result is one when the
key's state word is nonzero or the key is the latest key event the keyboard handler recorded. The
debug shell reads it for the letters `p`, `c` and `d` and for Alt+`q`, and a few bridge and door
scripts read `r`.

### GolemSteam and DragonBreath

`GolemSteam` takes no arguments and returns zero. A receiver excluded from object updates does nothing. Otherwise
one `Rnd(4)` draw that is not zero ends the call. On zero a second `Rnd(4)` draw selects one of the receiver's four RAEX
attachment vertices in the order `grip0`, `scabbard1`, `grip1`, `scabbard0`. The vertex, rotated by the receiver's
orientation and added to its position, becomes the point of a steam effect: descriptor `0x32480` when the world-loop
frame counter is odd and `0x22480` when it is even, with speed one.

`DragonBreath` takes no arguments and is also the update of the multitask `DragonBreathTask`, which completes when the
update returns zero. It acts on the receiver, a dragon with combat state:

1. A receiver whose death selector is 8 returns zero. First, when the player holds item `0x4b` with a nonzero count, one
   unit of item 1 is added, the player's held-item attribute (23) becomes 1, item `0x4b` is emptied and item 1 becomes
   the selected item.
2. One `Rnd(4096 / frame scale)` draw of zero, while a dialogue line owned by the receiver is active, spawns the smoke
   effect `0x32480` at an attachment vertex: `scabbard0` when a following draw masked with one is nonzero, else
   `scabbard1` (rotated and offset as for `GolemSteam`).
3. A nonzero fireball cooldown, a process-wide counter that starts at zero, is reduced by the nominal ticks of the
   frame (to zero at the least) and the update returns one.
4. Without combat flag `0x04` or without a combat target the update returns one. A target whose rounded 3-D distance is
   less than `0x19000` (Q8) also returns one.
5. Otherwise the fireball is created: an object of the installed script `DRAGBALL` whose parent is the receiver, at the
   rotated `grip0` vertex plus the receiver's position. Its orientation is the aim from `grip0` to `grip1`: yaw is the
   bearing of the horizontal difference plus 1024; the pitch is the elevation of the difference minus 512, then
   `120 - pitch`, all masked to 2047; roll is zero. The cooldown becomes `Rnd(32) + 12` and the update returns one.

### Nonplayer deep-water and edge probe

The strafing steppers (`MoveLeft`, `MoveRight`) and `MoveForward` call one probe before they move. It
applies only to receivers whose surface byte is 2 or 4 (the deep-water classes); every other
receiver passes. The probe point is the receiver's position plus a step of 16384 (units of 1/256
map unit) along the direction chosen by the motion-direction byte (2 and 6 forward, 3 and 7
backward, 4 left, 5 right, any other value forward), rotated by the receiver's heading. The point is
looked up in the terrain material rules of [terrain material blend coverage](player.md#terrain-material-blend-coverage).

The probe fails (blocks the move) only when the resolved surface is 10 with blend coverage 0 and the
terrain slope class at the point is 2. A point with blend coverage 256, a point with partial coverage
(surface 3), a point without terrain, and every other surface pass. The slope class is the
classification of the terrain face normal as for ground support. A failing probe returns zero to its caller.

### Nonplayer step hazard

The step hazard test answers whether moving a nonplayer receiver to a destination lands on a steep surface. It
applies only to receivers whose attribute 1 has bit 1 set; others never report a hazard. The surface at a point
is resolved with a vertical segment of the post-collide height about the point's centre (the point's Y plus the
receiver's positive Y extent) against the candidate models and the terrain. Among the contacts, only floor
(class 4) and steep (class 2) surfaces compete, and the one whose rank is nearest 0x4000 wins; the terrain
replaces that surface when the segment's top is at or below the terrain height and the model surface is farther from the
destination's centre than the terrain is. A point with no surface counts as class 1, and when neither the
destination nor the current position has a surface the result is no hazard. The class comes from the contact
normal's Y (scale 256): at most -161 is 4, at least 161 is 8, at most -42 is 2, otherwise 1. The test reports a
hazard when the destination's class is 2.

The selected player runs the same test, except that it reports no hazard while the player's
[support-history counter](player.md#selected-player-fallback-support) is nonzero.

### MoveForward

`MoveForward(duration)` is a task that steps the receiver forward along its heading. It shares
[MoveBackward](#movebackward)'s result and movement rules except as listed here.

**Admission.** The call completes without moving when the receiver's rope attachment, sliding state,
falling state or queued-motion byte is nonzero. There is no lifecycle-byte test. While the two-bit step
counter in the receiver's motion byte (bits 4 and 5) is below three, the forward step check must also
pass: it passes when the lifecycle byte is 7 and attribute 1 has bit 0 clear; otherwise it fails when the
receiver's attribute 1 has bit 1 set and the [step hazard](#nonplayer-step-hazard)
at the destination reports a hazard, where the destination is the receiver's position moved forward by
half of attribute byte 8 (whole units, times the frame scale, arithmetic shift right eight). A failed check
completes the call. Admission then stores motion direction 2.

**Combat stance** (the receiver holds a weapon or its combat flag `0x04` is set). The call completes
without moving when the receiver already has a selected combat move. Otherwise the group is 92 and the
step length is attribute byte 17.

**Outside combat stance.** In deep water (surface 2 or 4) the edge probe must pass, the group is 142 and the
distance is halved; otherwise the group is 30 while a torch burns and 18 when not, with step length attribute byte 8.

**Animation and movement.** The call completes without moving when the receiver has no entry for the group.
Otherwise the group is requested as `PlayAnimation(group, 0)` does, and with the animation system enabled and
the current group not the requested one the call waits like MoveBackward. The movement heading is the receiver's
local Y angle (no offset) and the distance is the step length in whole units times 256, scaled and translated as
for MoveBackward.

### Forward movement amount query

Fight spacing's forward amount query shares the forward startup counter and
step-called state with actual movement. It refuses nonzero rope attachment,
sliding, falling, queued-motion or living-state values. Below startup counter
three, it requires the forward step check to pass. It then stores motion
direction 2.

A held weapon or combat flag `0x04` selects group 106 and attribute 16 for action
selector 4, or group 92 and attribute 17 otherwise. Outside combat stance it
selects attribute 8 and group 142 in deep water, group 30 with a burning torch,
or group 18 otherwise. It requests the group with frame zero. When animation is
enabled and the requested group is not current, it returns the ordinary duration
continuation result without marking forward called or advancing the counter.

Once animation permits the step, it marks forward called, multiplies the selected
step byte by 256 and halves that distance in deep water. Below counter three it
applies the ordinary startup adjustment and increments the shared counter.
With translation suppression clear, its return value is the signed wrapping
product of that adjusted amount and frame scale, shifted right eight. With
suppression set, it instead returns the adjusted amount without frame scaling.
It does not translate the receiver. A subsequent movement request therefore
reads the counter and animation state changed by this query.

### WalkForward

`WalkForward(duration)` delegates to `MoveForward(duration)` in deep water before
checking its own admission state. For a nonplayer receiver outside deep water,
a rope attachment, sliding state, falling state or queued motion refuses the
call. The walk-forward step check must pass before motion direction 6 is stored.

The walk-forward step check refuses a rope attachment. It passes immediately for
a receiver whose lifecycle selector is 7 and whose attribute 1 has bit zero clear.
Otherwise it builds a destination from the local offset
`[0, 0, (attribute12 + positive_z_extent) * 256]`, with signed wrapping arithmetic,
rotated by the receiver's stored matrix. It adds the rotated X/Z to the receiver's
position and retains the receiver's Y coordinate. It requires the destination
body-contact test to pass, the step-hazard result to differ from one, then the
additional destination-admission test to pass, in that order. A failed test
sets bit `0x80` in the receiver's step-status byte and returns false. Early
rope refusal and lifecycle acceptance leave that byte unchanged.
The step's destination support test passes whenever attribute 1 bit 1 is clear.
The additional destination-admission test runs the ordinary collision pass with
the receiver temporarily at the destination. It restores the original position
afterwards, while retaining other collision writes.

A held weapon or combat flag mask `0x04` selects animation group 31 and attribute
byte 19 as the step length. Outside combat stance, a burning torch selects group
21 and otherwise group 20; both use attribute byte 12. A missing group skips
animation requests and translation, while duration completion is still evaluated. An available group is requested
with frame zero. With animation enabled, a different current group waits without
advancing the step counter or translating.

The initial movement distance is the selected step length multiplied by 256.
When that length is zero, the initial distance is instead half of attribute byte
8, without the multiplication by 256. While the two-bit step counter is below
three, the existing step-distance adjustment runs and the counter increments.
Animation translation suppression prevents movement after that adjustment.
Otherwise, signed wrapping multiplication by frame scale followed by arithmetic
shift right eight gives the distance. Positive distances translate along the
receiver's local Y heading, masked to 2047. Duration completion follows
`MoveBackward`.

### MoveLeft and MoveRight

`MoveLeft(duration)` and `MoveRight(duration)` are tasks that strafe the receiver. They share
[MoveBackward](#movebackward)'s admission, result and movement rules except as listed here. Admission is
identical. A call stores motion direction 4 (left) or 5 (right) once admitted.

**Combat stance** (the receiver holds a weapon or its combat flag `0x04` is set). The call completes
without moving when the receiver's current group is already the strafe group (94 for left, 95 for right,
or either of the two) or when the receiver has no entry for the strafe group. Otherwise it requests the
group as `PlayAnimation(group, 0)` does, then turns the receiver's heading: the left call subtracts, and the
right call adds, attribute byte 84 times the frame scale (arithmetic shift right eight), wrapped to 2047.
The movement heading is that new heading plus 512 (left) or minus 512 (right), wrapped, and the step length is
attribute byte 83. No safety check applies.

**Outside combat stance.** The call completes without moving when the lateral step check fails (the
destination of a step of attribute byte 10 plus the receiver's retained step value, to the left or right of
the heading, fails the model contact test). In deep water (surface 2 or 4) the edge probe must also pass and the
group is 143; otherwise the group is 25 left or 27 right while a torch burns, else 24 left or 26 right.
When the animation system is enabled and the receiver's current group is not the requested one the call
waits like MoveBackward: one while `duration - 1 > elapsed`, zero otherwise, without moving. With the
lifecycle byte 15 (hanging at an edge) the call instead runs the lateral edge test and requests group 28 left
or 29 right. The movement heading is the receiver's heading plus 512 (left) or minus 512 (right) and the step
length is attribute byte 10.

**Movement and result.** The shared step counter, translation-suppression marker, frame scaling and result
are those of MoveBackward.

### Fight-spacing directional admission

The left, right and backward destination checks used by fight spacing refuse a
rope attachment before reading geometry. They accept immediately when lifecycle
selector 7 has attribute 1 bit zero clear. These early results leave the
receiver's step-status bytes unchanged.

Otherwise, each check builds its directional local offset with signed wrapping
arithmetic:

| Direction | Local offset |
|---|---|
| Left | `[-(attribute10 + negative_x_extent) * 256, 0, 0]` |
| Right | `[(attribute10 + positive_x_extent) * 256, 0, 0]` |
| Backward | `[0, 0, -(attribute13 + negative_z_extent) * 256]` |

It rotates the offset by the receiver's stored matrix, adds the rotated X/Z to
the local position and preserves the receiver's Y. It requires a
nonzero destination body-contact result, a step-hazard result other than exactly
one, then a nonzero additional destination-admission result, in that order.
A failed stage skips the later stages and returns zero. Failure ORs `0x20` into
the step-status byte for left, `0x40` for right, or `0x01` into its companion
byte for backward. Success returns one without clearing retained status bits.
The checks themselves do not replace the receiver's position.

### MoveBackward

`MoveBackward(duration)` is a task that steps the receiver backward relative to its heading. Each
invocation returns zero (complete) or one (continue); it reads the task's elapsed input.

**Admission.** The call does nothing, completing immediately, when the receiver's rope attachment,
sliding state, falling state or queued-motion byte is nonzero, or its lifecycle byte is not 0 (byte 15
is refused too, unlike the strafes). Otherwise it sets the receiver's motion-direction byte to 7.

**Animation.** The receiver is in the combat stance when it is holding a weapon or its combat flag
`0x04` is set. In combat stance the [blocked-ahead probe](#nonplayer-blocked-ahead-probe) runs with
distance 30; a blocked probe completes the call. The group is then 93 and the step length is the
receiver's attribute byte 18. Outside combat stance the group is 143 in deep water (surface value 2
or 4), 23 while a torch burns, and 19 otherwise, with step length attribute byte 9. When the receiver
has no entry for the group, the call skips the rest and uses the final duration test below. Otherwise
the group is requested as with `PlayAnimation(group, 0)`. When the animation system is enabled and
the receiver's current group is still not the requested one, the call returns one while
`duration - 1 > elapsed` and zero otherwise, without moving.

**Movement.** The heading is the receiver's local Y angle plus 1024, wrapped to 2047. The distance is
the step length in whole units times 256. A two-bit step counter in the receiver's motion byte
(bits 6 and 7) selects an initial-distance adjustment while below three, then increments.
The adjustment reads attribute byte 11 independently of animation translation suppression.
For input distance `d`, it follows this table. Products wrap as signed 32-bit integers,
right shifts are arithmetic, and division truncates toward zero.

| Attribute 11 | Counter 0 | Counter 1 | Counter 2 |
|---|---|---|---|
| 1 | `d >> 2` | `d / 3` | `d >> 1` |
| 2 | 0 | `(d * 85) >> 8` | `(d * 170) >> 8` |
| 3 | 0 | 0 | `d >> 1` |
| Other | `d` | `d` | `d` |

At counter three the distance stays unchanged. The packed byte holds left, right,
forward/walk and backward counters in consecutive two-bit fields, from least to
most significant. Each direction has a shared called flag, reset at ordinary wrapper entry.
At ordinary update cleanup, a direction's counter is cleared only if its called flag is zero;
called directions retain their counters. Directional displacement and sliding mark
this same packed byte, saturating the selected direction's counter rather than
writing a separate movement record. Collision rollback can separately clear
the packed byte when restoring the previous position. Entering collision
processing does not unconditionally clear it.

Unless the animation's translation-suppression marker is set, the distance
is multiplied by the frame scale (arithmetic shift right eight) and, when positive, the receiver's X
and Z move that far along the heading by the heading-based translation rule.

**Result.** One while `duration - 1 > elapsed`, otherwise zero.

### WalkBackward

`WalkBackward(duration)` is a task. In deep water (surface value 2 or 4) it runs
[MoveBackward](#movebackward) unchanged. Otherwise it follows MoveBackward's animation, movement and
result rules with these differences.

**Admission.** The call completes immediately when the receiver's rope attachment, sliding, falling or
queued-motion byte is nonzero or its lifecycle byte is not 0. For the selected player it also completes
immediately while a retained player-mode word equals one and the queued-motion and falling bytes are
both zero. It then runs the backward form of the ordinary directional step check, which uses attribute
byte 13, and completes when that check refuses. Only then does it set the motion-direction byte to 7.

**Animation.** In combat stance the group is 93 and the step length is attribute byte 20. Otherwise the
group is 23 while a torch burns and 22 otherwise, with step length attribute byte 13. There is no
blocked-ahead probe.

**Movement.** When the step length is zero, the initial distance is attribute byte 9 shifted right by
one, not multiplied by 256; the step-counter adjustment and frame scaling then apply as for
MoveBackward.

