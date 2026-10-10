# SOUP Scripting

Bytecode scripting engine used by the Redguard runtime for actor behavior, dialogue, puzzle logic, and scene control.

## Script Data Sources

| Source | Container | Notes |
|---|---|---|
| Map script bytecode | `maps/*.RGM` (`RASC` section) | Main per-actor compiled script payload. Offsets and lengths are stored per actor in `RAHD` (`script_data_offset`, `script_length`, `script_pc`). |
| Standalone AI script files | `soup386/*.AI` (for example `CAMERA.AI`, `sword.ai`, `projtle.ai`) | Loaded separately from map RGM files. Uses the same SOUP bytecode model as map scripts. No `.AI` files are included in the shipped game — the `soup386/` directory contains only `SOUP386.DEF` in both the GOG and original CD releases. The engine's `.AI` loading path was most likely used during development. |
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

Thread 1 enables actors to remain activatable while performing another action (for example, an NPC walking a patrol route can still respond to player interaction). The two threads share the bytecode array but maintain independent program counters and independent call stacks.

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
| `0x11` | Gosub | `u32` target | Subroutine call (pushes return address) |
| `0x12` | Return | *(none)* | Return from subroutine |
| `0x13` | Endint | *(none)* | End secondary thread |
| `0x14` | ObjectDot | object encoding, `u16` ref_id, mode-dependent | Object property read/write |
| `0x15` | String | `u32` string_index | String literal from RASB/RAST table |
| `0x16` | NumericAlt | `i32` value | Same as `0x07`; used for global flag function arguments |
| `0x17` | Anchor | `u8` anchor_value | Anchor assignment |
| `0x19` | ObjDotTask | object encoding, task encoding | Task call on named object |
| `0x1A` | ObjDotFunc | object encoding, task encoding | Function call on named object |
| `0x1B` | TaskPause | `u32` label | Pause until task completes |
| `0x1E` | ScriptRV | `u8` expected, block | Branch on script return value |

Bytes with no known opcode: `0x08`, `0x09`, `0x0B`–`0x0E`, `0x18`, `0x1C`–`0x1D`, `0x1F`+. These bytes are not used in any shipped script.

### Operand Encoding

Flag (`0x06`) and LocalVar (`0x0A`) consume different trailing bytes depending on value mode:

**Flag (`0x06`)**

| Mode | Bytes after `u16` flag_id |
|---|---|
| MAIN | Formula (assignment to flag) |
| LHS, RHS | `u8` operator byte |
| PARAMETER | `u16` padding (always `0x0000`) |
| FORMULA | *(none)* |

**LocalVar (`0x0A`)**

| Mode | Bytes after `u8` var_index |
|---|---|
| MAIN | Formula (assignment to variable) |
| LHS, RHS | `u8` operator byte |
| PARAMETER | 3 padding bytes (`0x000000`) |
| FORMULA | *(none)* |

**ObjectDot (`0x14`)**

| Mode | Bytes after object encoding + `u16` ref_id |
|---|---|
| MAIN | Formula (assignment to property) |
| LHS | `u8` operator byte |
| Other | *(none)* |

### Object Name Encoding

Used by opcodes `0x0F`, `0x10`, `0x14`, `0x19`, `0x1A`. A leading byte selects the object target:

| Byte | Additional | Object |
|---|---|---|
| `0x00` | `u8` padding | `Me` — the script's own actor |
| `0x01` | `u8` padding | `Player` — Cyrus |
| `0x02` | `u8` padding | `Camera` |
| `0x04` | `u8` string index | Named object from per-script string table (RASB/RAST) |
| `0x0A` | `u8` var index | Object name from local variable |

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
| 10 | `++` | unary | (increment; terminates formula) |
| 11 | `--` | unary | (decrement; terminates formula) |

Operator bytes outside 1–9 terminate the formula loop. Bytes 10–11 (unary increment/decrement) are consumed as the final operator and also terminate. Bytes 12+ are fatal. Division by zero is not checked in software — it raises a CPU divide-by-zero exception caught by the C runtime signal handler.

> **Note:** The Dillonn241 disassembler/assembler tools swap operators 3 and 4 (`/` and `*`). This swap is internally consistent within those tools (scripts round-trip correctly) but does not match the engine binary, where byte 3 maps to `imul` (multiply) and byte 4 maps to `div` (divide).

### Comparison Operators

Used in `if` conditions. Each comparison pairs a LHS value, comparison byte, and RHS value.

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

`Gosub` pushes the current PC onto the per-thread call stack, then jumps to the target address. `Return` pops the return address and resumes. Subroutines share the same local variable scope — no parameters are passed through the call stack.

### Endint (`0x13`)

Resets the active cursor to the script base and returns from the current invocation. Script-entry processing preserves the main continuation while advancing its separate entry cursor.

## Function Dispatch

### Call Types

SOUP386 distinguishes three call types, encoded in the opcode byte:

| Type | Behavior | Script syntax |
|---|---|---|
| Task | Blocking — script waits for completion | `FunctionName(...)` |
| Multitask | Asynchronous — script continues immediately | `@FunctionName(...)` |
| Function | Immediate — returns a value | `FunctionName(...)` (context determines) |

Self-calls use opcodes `0x00`/`0x01`/`0x02`. Object-targeted calls use `0x19`/`0x1A` with an additional dispatch-type byte (`0x00` = task, `0x01` = multitask, `0x02` = function) when used as a top-level statement. In non-MAIN modes, the dispatch-type byte is absent and the call is always treated as a function.

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

### Function Table

`SOUP386.DEF` declares 367 callable functions (indices 1–367; index 0 is the synthetic `NullFunction`). Functions span the following categories:

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
