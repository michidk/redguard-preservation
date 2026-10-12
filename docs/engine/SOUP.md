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

Each script supports up to two concurrent threads sharing the same bytecode buffer:

| Thread | Start Address | Purpose |
|---|---|---|
| Thread 0 | `RAHD.script_pc` | Main execution (dialogue, activation, behavior) |
| Thread 1 | Offset `0x00` | Interrupt handler — only created when `script_pc != 0` |

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

### Ordinary update gates

`SYSTEM.INI`'s `[system] task_system` enables the ordinary entry, task, and main
script stages. Disabling it does not disable every script invocation: an
available interrupt cursor can still run unless its runtime suppression flag
is set. Actor exclusions and script-name filters apply before these stages.

The actor's runtime script-control byte has independent effects:

| Mask | Effect in the ordinary actor update |
|---|---|
| `0x01` | Suppress the ordinary main stage, including its entry and task work |
| `0x02` | Suppress the separate interrupt invocation |
| `0x04` | Use entry and main-body processing without the existing-task pass |
| `0x08` | Exclude the actor from the ordinary actor-update body |

The `0x04` path is distinct from the usual entry, existing-task, then main-body
order. These runtime flags are not RAHD attribute indices. See
[actor update order](animation.md#ordinary-actor-update-order).

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

CYRUS and GREMLIN player instances load independently of MPOB placements. Each uses identifier one; the initial player selector chooses CYRUS. The camera instance loads CAMERA independently of MPOB and uses identifier two. Fresh world loading creates and registers ordinary MPOB actors first, followed by CYRUS, GREMLIN and CAMERA.

### Reference Name Encoding

After object name in property/method opcodes: `u16` reference ID. Only the low byte is used as an index into the global references table (loaded from `[refs]` section of `SOUP386.DEF`). The high byte is discarded.

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
| `ActiveItem`, `AddItem`, `DropItem`, `HandItem`, `HaveItem`, `SelectItem`, `ShowItem`, `ShowItemNoRtx` | Item ID | `i32` item identifier |
| All others | Integer | `i32` signed integer |

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

`GetMyAttr` reads an unsigned byte from the receiving actor's attribute table. It returns zero when the table is absent or the index is outside 0–255. `SetMyAttr` returns zero and ignores an absent table or an out-of-range index. A valid write truncates the supplied value to its low byte. Attribute 44 also stores that truncated byte as the model's default group when the actor has an active model record.

### Attribute default-group side effects

The `SetAttribute` attribute-44 animation-default path releases the actor's two
held walk-node reservations before storing the full supplied value as the
model's default group. A node's ownership byte is cleared only when it matches
the low byte of the actor identifier; both actor-held node indices are then
cleared. This operation does not reset the animation cursor, current group,
or loop countdown.

`SetMyAttr` does not release walk-node reservations.

The model default and the animation playback default are separate state.
Ordinary actor updates select the playback default using actor state; these
attribute writes do not directly replace that selection step. See
[ordinary default selection](animation.md#placed-actor-initialization-and-eligibility).

### Player distance and facing

`PlayerDistance()` returns the sum of the absolute X, Y and Z differences
between the selected player's current position and the receiver's effective
position. Positions and the result use the runtime coordinate scale of 256
units per world unit. Without a selected player, it uses the global fallback
position. A global override selects the fixed result `9999999` instead of
calculating distance.

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
- `RAHK` stores hook entry offsets rebased against script base addresses.
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
