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
| `PushAnimation`, `TaskStop` | [Animation ownership and stopping](#animation-task-ownership-and-stopping) |
| `GetAttribute`, `SetAttribute`, `GetMyAttr`, `SetMyAttr` | [Read and write actor attributes](#attribute-reads-and-writes) |
| `SetAttribute`, `SetMyAttr` | [Default-group side effects](#attribute-default-group-side-effects) |
| `PlayerDistance`, `PlayerFacing` | [Query distance and relative facing](#player-distance-and-facing) |
| `AtNodeMarker` | [Query distance to a node marker](#node-marker-distance) |
| `Wander` | [Choose destinations and limit movement](#wandering-without-walk-node-maps) |
| `MoveToLocation` | [Move toward an authored location](#authored-location-movement) |
| `RotateByAxis`, `sRotateByAxis` | [Relative rotation](#relative-axis-rotation), [elapsed rotation](#elapsed-relative-axis-rotation) |
| `RotateToAxis`, `sRotateToAxis` | [Absolute rotation](#absolute-axis-rotation), [elapsed rotation](#elapsed-axis-rotation) |
| `MoveObjectAxis` | [Move along the actor's orientation](#actor-local-axis-movement) |
| `MoveByAxis`, `sMoveByAxis` | [Relative movement](#relative-axis-movement) |
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
The temporary query scopes which disable it restore it to enabled before returning. The weapon contact caller does not
assign the flag. With sidedness enabled, the face query rejects a segment whose
start lies more than four map units behind the authored face plane.

A model without an actor owner uses installed `[misc] sword_clank_1`, with volume
zero and pitch offset `rnd(128) * 16 - 1024` Hz, positioned on the attacker.
Candidate type exactly one repeats the impact sound with a separate random draw.
Other candidate types request one sound. The scan continues to later candidates
and leaves attack intent set. Models with actor owners use the actor-response
path separately from this unowned-model response.

### Hit-defence response

The candidate's hit-defence query requires the attacker to have combat state and
an action other than `-1` or nine. The defender's retained defence selector must
match the attacker's current-motion defence selector. The attacker's defence
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

Object-targeted native calls evaluate local-variable and property operands against the receiving actor. Ordinary entry, task, and main processing establishes the executing actor as the native string-parameter context. A routed initial call preserves that stage actor's string table. A later ordinary task pass on the receiving actor reevaluates those string indices against the receiving actor's table. The stored task operand location remains in the calling actor's bytecode.

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

The initial task call invokes its native handler, then allocates a slot independently of that handler's return value. A self blocking task yields after this allocation, even when its first native invocation reports completion. Completion is applied by a later existing-task pass. An object-targeted task allocates on the receiver and the caller continues, including when the selector resolves to the caller itself.

Each active slot has a 16-bit invocation counter. Allocation sets it to one. An eligible existing-task pass computes its elapsed input as the low 32 bits of counter times the current frame scale, shifted right by eight, then increments the counter with 16-bit wrapping. Paused slots do not advance. The initial native call receives elapsed input zero.

#### Wait, WaitOnTasks, and WaitNonMulti

`Wait(duration)` stays active while the elapsed input is less than the wrapping 32-bit value `duration - 1`, compared as signed integers. `WaitOnTasks` stays active when more than one unpaused slot is occupied. `WaitNonMulti` stays active when more than one unpaused blocking slot is occupied. These count tests include the current wait slot when it has already been allocated.

#### Anchor, Yield, and ScriptRV

`Anchor` stores its operand byte as the actor's anchor state. `Yield` retains the following cursor without issuing the `End` notifications.

`ScriptRV` reads an expected byte. On mismatch it skips the next five bytes. On a match, a following `Return` byte advances the cursor by four bytes; a following `Gosub` saves the current script-return byte before the control instruction runs. Other following instructions remain eligible. `Return` restores that saved byte.

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

<a id="attribute-reads-and-writes"></a>

### GetAttribute, SetAttribute, GetMyAttr, and SetMyAttr

`GetAttribute` reads an unsigned byte from the receiving actor's attribute table and returns zero when the table is absent. `SetAttribute` returns zero. When the table exists, it writes the low byte of the supplied value. Writing attribute 6 also copies that byte to attribute 115; writing attribute 1 also copies it to attribute 119. These two handlers do not check the attribute index range.

`GetMyAttr` reads an unsigned byte from the receiving actor's attribute table. It returns zero when the table is absent or the index is outside 0–255. `SetMyAttr` returns zero and ignores an absent table or an out-of-range index. A valid write truncates the supplied value to its low byte. Attribute 44 also stores that truncated byte as the combat death-default group when the actor has a combat record and its object-update exclusion is nonzero.

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

Marker lookup rejects an out-of-range or absent marker. The ordinary world
marker table has 128 slots and accepts any nonzero presence byte. The alternate
map path uses its own stored count and requires a presence byte of one. The
marker argument is an unscaled signed index, distinct from the distance units.

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

The shared player-exclusion switch starts disabled during player setup. Point
movement consults this switch before falling/queued-motion handling. It is the
same switch used by walk-node admission; held-target admission has its own
position quantization and angular checks.

A falling actor or one with queued motion assigns attribute 1 to 13, selects
phase one when a next node is held or phase zero otherwise, sets lifecycle
selector seven, and returns pending. This path precedes immediate arrival and
default-animation admission.

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
A static-model actor that passes those checks restores its retained resource
and refreshes its model position and orientation. Drawing exclusion suppresses
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

### Model admission and support ownership

The current map's PVO file supplies a camera-centered selected-object list.
An empty selected list leaves ordinary model admission unrestricted. With a
nonempty list, the ordinary presentation traversal excludes actors with
`attr_non_moving` set.

With a nonempty selected list, animation-model admission also reads the actor's
retained support owner. A
static-placement support must occur in the selected MPSO IDs. Support from an
actor with `attr_non_moving` set must occur in the selected secondary object
IDs. Other support cases pass this admission check. Rejected animation-model
admission clears the selected resource and current model record.

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

### Ordinary camera selection inhibition

On the ordinary camera-update path, a nonzero scripted-camera active word or
bit `0x20` in the selected camera record suppresses automatic slot reselection.
The camera still runs its pose-processing stage. These guards precede the
holding-weapon and other player-state checks used to choose an automatic slot;
they do not suspend the whole camera update.

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
camera actor. Orientation processing follows position publication, and its
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
their flags to exactly `0x40`. Slots four, five and eight keep the copied flags.
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

### FacePlayer

`FacePlayer(duration)` is a task that turns the receiver's local Y angle toward the
selected player. Each invocation computes the bearing from the receiver's local position to
the selected player's local position with the signed-coordinate bearing used by
the `PlayerLooking` query, then:

1. `gap = (-heading - bearing) & 2047`. A zero gap completes the task without a change.
2. `remaining = |elapsed - duration|`, treated as 1 when zero.
3. If `gap` is above 1024, `gap = (-gap - 1024) & 1023` and the step is subtracted from
   the heading; otherwise the step is added.
4. `step = ((gap << 8) / remaining) >> 8`. A zero step completes the task without a change.
5. The heading wraps modulo 2048.

After a turn the task completes when `elapsed >= duration - 1`, and remains active
otherwise.

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

The defence selection runs only while the receiver is armed and the selected-player
health-death/controller override is clear. Standalone-player initialization clears
this shared override; selected-player health death sets it. With no move selected, the target must lie inside the receiver's
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
expected animation group, a following motion of type one admits the next chain
step, up to the third motion. The type test reads the following motion, not the
current one. Unfilled motion slots retain their zero-initialized type.
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
sliding state, falling state or queued-motion byte is nonzero, or its lifecycle byte is neither 0 nor
15. Otherwise it sets the receiver's motion-direction byte to 7.

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
