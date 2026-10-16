# SOUP Scripting

Bytecode scripting engine used by the Redguard runtime for actor behavior, dialogue, puzzle logic, and scene control.

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
revisited before the main script body can continue. It does not execute exactly
one instruction per nominal tick; that describes a reimplementation rather than
the original runtime.

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
attribute-derived counters and attack metadata.

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

Selected-player combat processing uses a phase value separate from the
cooldown. After message processing, phase zero with a nonzero action selector
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
its own list membership.
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

| Byte | Operator | Arity | Engine instruction |
|---|---|---|---|
| 0 | `;` (end) | — | (return result) |
| 1 | `+` | binary | `add eax, ebx` |
| 2 | `-` | binary | `sub eax, ebx` |
| 3 | `*` | binary | `imul ebx` (signed) |
| 4 | `/` | binary | `div ebx` (unsigned, zero-check via CPU trap) |
| 5 | `<<` | binary | `shl eax, cl` |
| 6 | `>>` | binary | `sar eax, cl` (arithmetic/signed) |
| 7 | `&` | binary | `and eax, ebx` |
| 8 | `\|` | binary | `or eax, ebx` |
| 9 | `^` | binary | `xor eax, ebx` |

The formula evaluator accepts only zero and binary operator bytes 1–9. Other operator bytes enter the fatal error path. Division by zero is not checked in software — it raises a CPU divide-by-zero exception caught by the C runtime signal handler.

> **Note:** The Dillonn241 disassembler/assembler tools swap operators 3 and 4 (`/` and `*`). This swap is internally consistent within those tools (scripts round-trip correctly) but does not match the engine binary, where byte 3 maps to `imul` (multiply) and byte 4 maps to `div` (divide).

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
| Task | Blocking — script waits for completion | `FunctionName(...)` |
| Multitask | Asynchronous — script continues immediately | `@FunctionName(...)` |
| Function | Immediate — returns a value | `FunctionName(...)` (context determines) |

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
| `ACTIVATE`, `AddLog`, `AmbientRtx`, `menuAddItem`, `RTX`, `rtxAnim`, `RTXp`, `RTXpAnim`, `TorchActivate` | Dialogue key | 4-byte ASCII string (RTX lookup key) |
| `LoadWorld` | Map ID | `i32` map identifier |
| `AddItem`, `DropItem`, `HandItem`, `HaveItem`, `SelectItem`, `ShowItem`, `ShowItemNoRtx` | Item ID | `i32` item identifier |
| All others | Integer | `i32` signed integer |

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

### Actor task controls and return state

An actor has six task slots shared by its entry and main invocations. Existing-task processing traverses those slots in allocation order, skips paused slots, reevaluates encoded operands, and removes a slot when its handler reports completion. Unpaused blocking slots suppress the subsequent main-body invocation; multitask slots do not. Reissuing a call whose operand address is already in a slot does not execute its handler again. Task labels resolve against the receiving actor's script base and match the byte immediately after the labelled dispatch opcode. Self task calls store the operand address immediately after their Task or Multitask opcode. Object task calls store the operand address immediately after the embedded dispatch byte following the two-byte object selector. Routing retains that encoded operand location; it does not rebase foreign bytecode into the receiver's script. `TaskStop` removes every matching slot and clears its blocking and pause state. `TaskPause` and `TaskResume` set and clear the matching slots' pause state.

The initial task call invokes its native handler, then allocates a slot independently of that handler's return value. A self blocking task yields after this allocation, even when its first native invocation reports completion. Completion is applied by a later existing-task pass. An object-targeted task allocates on the receiver and the caller continues, including when the selector resolves to the caller itself.

Each active slot has a 16-bit invocation counter. Allocation sets it to one. An eligible existing-task pass computes its elapsed input as the low 32 bits of counter times the current frame scale, shifted right by eight, then increments the counter with 16-bit wrapping. Paused slots do not advance. The initial native call receives elapsed input zero.

`Wait(duration)` stays active while the elapsed input is less than the wrapping 32-bit value `duration - 1`, compared as signed integers. `WaitOnTasks` stays active when more than one unpaused slot is occupied. `WaitNonMulti` stays active when more than one unpaused blocking slot is occupied. These count tests include the current wait slot when it has already been allocated.

`Anchor` stores its operand byte as the actor's anchor state. `Yield` retains the following cursor without issuing the `End` notifications.

`ScriptRV` reads an expected byte. On mismatch it skips the next five bytes. On a match, a following `Return` byte advances the cursor by four bytes; a following `Gosub` saves the current script-return byte before the control instruction runs. Other following instructions remain eligible. `Return` restores that saved byte.

### Animation task ownership and stopping

Each task slot saves and restores its native handler state independently.
Multiple `PushAnimation` slots therefore retain separate not-started/started
phases while requesting and observing the same receiving actor's animation.
Each invocation sees animation changes made by earlier calls; the animation is
not copied into the task slot.

`TaskStop` clears matching operand slots and their blocking and pause flags.
It does not invoke the native handler or restore earlier animation state.
Stopping a `PushAnimation` task therefore leaves its animation request and
current playback state in place.

### Attribute reads and writes

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

### Attribute default-group side effects

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

### Player distance and facing

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

### Node-marker distance

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

### Wandering without walk-node maps

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

### Authored location movement

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

### Relative axis rotation

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

### Elapsed relative axis rotation

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

### Absolute axis rotation

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

### Elapsed axis rotation

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

### Actor-local axis movement

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

### Relative axis movement

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

### Random functions

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

### Object visibility

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

### Function Table

`SOUP386.DEF` declares callable functions in installed order. Index 0 is the synthetic `NullFunction`; authored entries start at index 1. Functions span the following categories:

| Category | Examples | Count |
|---|---|---|
| Movement | `Move`, `WalkForward`, `MoveToLocation`, `WanderToLocation` | ~16 |
| Rotation / Facing | `Rotate`, `RotateByAxis`, `FacePlayer`, `FaceAngle`, `FaceObject` | ~10 |
| Camera | `showObj`, `showPlayer`, `lookCyrus`, `showCyrusPan` | ~13 |
| Dialogue / RTX | `RTX`, `rtxAnim`, `menuNew`, `menuProc`, `menuAddItem`, `menuSelection` | ~8 |
| Animation | `PlayAnimation`, `PushAnimation`, `WaitAnimFrame`, `SetAction` | ~10 |
| Combat | `beginCombat`, `endCombat`, `isDead`, `adjustHealth`, `shoot`, `shootPlayer` | ~16 |
| Lighting / FX | `Light`, `LightRadius`, `LightFlicker`, `FxPhase`, `FxFlickerOnOff` | ~14 |
| Object control | `EnableObject`, `DisableObject`, `HideMe`, `ShowMe`, `KillMe` | ~12 |
| Inventory | `AddItem`, `DropItem`, `HaveItem`, `HandItem`, `ShowItem`, `SelectItem` | ~9 |
| Sound | `Sound`, `AmbientSound`, `EndSound`, `StopAllSounds` | ~5 |
| Weapon / Attachment | `handitem`, `displayhandmodel`, `drawsword`, `sheathsword` | ~11 |
| Spatial queries | `InRectangle`, `InCircle`, `DistanceFromStart`, `AtPos` | ~5 |
| Flat (billboards) | `Flat`, `FlatSetTexture`, `FlatAnimate`, `FlatOff` | ~7 |
| AI | `SetAiType`, `SetAiMode`, `Guard`, `Animal` | ~5 |
| Static objects | `LoadStatic`, `UnLoadStatic`, `PointAt` | ~3 |
| Global flags | `SetGlobalFlag`, `TestGlobalFlag`, `ResetGlobalFlag` | 3 |
| Attributes | `SetAttribute`, `GetAttribute`, `SetMyAttr`, `GetMyAttr` | 4 |
| Debug | `PrintParms`, `LogParms`, `PrintStringParm` | ~3 |

For the weapon/attachment function interface, see [Item Attachment — SOUP Script Interface](attachment.md#soup-script-interface). For the definition-file format and section layout, see [SOUP386.DEF](SOUPDEF.md).

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
unchanged. The pose-publication stage clears the transition word.

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

### Camera pose publication

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
