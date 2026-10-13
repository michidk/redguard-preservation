# Cheat System

Hidden cheat system built into the Redguard game engine. The 13 cheat codes are XOR-obfuscated in the binary (each byte XOR'd with `0xAA`).

The [developer console reference](console.md) lists all other console commands and their aliases.

## Activation

Cheats can be triggered in three ways:

### During Gameplay (no console)

Type the cheat name on the keyboard during normal gameplay and press **Enter**. The engine reads keypresses every frame into a hidden 127-character input buffer. On Enter, the buffer is compared against all cheat names. No visual feedback is given — the cheat silently toggles.

### Developer Console

Press **F12** to open the console, type the cheat name, and press **Enter**. The console prints `Cheat <name> turned on` or `Cheat <name> turned off`.

### Console with Explicit Value

In the console, use `<cheatname> = <value>` to set a specific integer value instead of toggling. For example, `magiccarpet = 1` forces the cheat on, `magiccarpet = 0` forces it off.

### Toggle rules

Cheats normally store `0` for off or `1` for on. Typing a cheat name again switches between those two values, whether entered during gameplay or in the console.

The console also accepts other numbers, such as `oddjob = 2`. These are raw values, not strength levels. Their gameplay meaning depends on the cheat: some checks accept any nonzero value, while others require exactly one.

Entering the name again handles these unusual values differently:

- During gameplay, any nonzero value becomes zero.
- In the console, an even value becomes one and an odd value becomes zero.

For example, after `oddjob = 2`, entering `oddjob` in the console stores one; entering it during gameplay stores zero. The console rule is `(value + 1) & 1`.

`oracle` and `savecheats` run dedicated actions. `oddjob` follows the ordinary state-handling rules above.

## Cheat List

| # | Name | Effect |
|---|------|--------|
| 0 | `oracle` | Lists all 13 cheats and their raw integer state values in the console |
| 1 | `nodemarker` | Debug display of pathfinding node markers — renders the navigation waypoint network used for actor movement. Corresponds to the `node_marker` key in `SYSTEM.INI [debug]` (related settings: `display_node_map`, `display_nodes`, `display_markers`). The SOUP scripting layer uses this network via functions like `movenodemarker`, `atnodemarker`, `disablenodemaps`, and `enablenodemaps`. |
| 2 | `moonraker` | For the player actor, skips several combat/movement processing calls and flips a player state bit used in combat flow |
| 3 | `task` | Toggles the task scheduling system — a high-level bytecode-driven behavior scheduler that runs on top of SOUP scripts. SOUP is the low-level scripting VM ("do X right now"); the task system is the higher-level sequencer ("do X, then Y, then Z over time"). Each actor has 6 independent task slots with their own bytecode pointers and frame counters. When enabled, a state machine interpreter processes ~30 task opcodes (0x00–0x1e) per actor per frame, handling action queuing, behavior transitions, AI/combat processing (pathfinding, combat stances, NPC-vs-player logic), and actor destruction. When disabled, only SOUP script execution runs — actors lose multi-step behaviors, combat AI, and pathfinding. Corresponds to `task_system` in `SYSTEM.INI [system]` (default: `yes`). |
| 4 | `animation` | Toggles the animation state machine — the system that manages animation playback, transitions, blending, and frame timing for all actors. When enabled, actors play animations (walk, fight, idle, etc.) with managed transitions and priority blending, and combat actions are gated by animation timing (a new attack cannot start until the current animation allows it). When disabled, all animation management early-returns: actors freeze on their current frame, combat action timing checks are bypassed (attacks fire without animation constraints), and animation progress always reports zero. Corresponds to `animation_system` in `SYSTEM.INI [system]` (default: `yes`). |
| 5 | `magiccarpet` | Fly mode — disables gravity and falling, allowing the player to walk horizontally through the air at current altitude |
| 6 | `savecheats` | Writes all cheat states to `REDGUARD.CHT` so they persist across sessions |
| 7 | `drevil` | Prevents player death — suppresses death handling when health reaches zero and bypasses some player damage application paths |
| 8 | `drno` | Disables part of the non-player actor update path, reducing/skipping some NPC behavior processing |
| 9 | `goldfinger` | God mode — continuously refreshes the player's post-hit invulnerability timer while active, preventing it from expiring. Once the player takes a hit (which starts the timer), subsequent damage is blocked indefinitely. Differs from `drevil`, which prevents death when health reaches zero rather than blocking incoming damage. |
| 10 | `neversaydie` | Makes SOUP `PlayerDistance()` return `9999999` instead of calculating distance. |
| 11 | `oddjob` | Recognized state slot: accepts toggles and explicit console values, appears in `oracle`, and persists in `REDGUARD.CHT`. |
| 12 | `yeahbaby` | Vertical surface bypass — while active, **Page Up** moves the player upward through surfaces and **Page Down** clips below the current surface. The collision bypass only engages while one of these keys is held; normal surface collision applies otherwise. Not a permanent noclip — horizontal collision is unaffected. |

The cheat names are themed after James Bond films (Moonraker, Dr. No, Goldfinger, Oddjob, Never Say Never Again) and Austin Powers ("Yeah Baby").

## Values other than zero and one

The Glide engine does not apply one shared on/off rule to every cheat. The ordinary gameplay checks use these conditions:

| Cheats | Condition used | Meaning of `2` |
|---|---|---|
| `nodemarker`, `task`, `magiccarpet` | Value equals `1` | Does not enable the corresponding check |
| `moonraker`, `drevil`, `drno`, `goldfinger`, `neversaydie`, `yeahbaby` | Value is nonzero | Enables the check, just like `1` |
| `animation` | Some checks require `1`; others accept any nonzero value | Enables only the nonzero checks, so it is not equivalent to either ordinary on or off |

`oracle` and `savecheats` execute commands instead of using these gameplay conditions. `oddjob` accepts and stores arbitrary values through the shared cheat-state handling described above.

For example, `neversaydie = 2` still makes `PlayerDistance()` return `9999999`, but `magiccarpet = 2` does not enable its fly-mode check. Values above one are not additional strength levels. Use `0` and `1` for consistent on/off control.

The CHT parser preserves each raw integer. Its `is_on()` helper reports whether that integer is nonzero; it does not reproduce the per-cheat conditions in this table. Callers implementing gameplay must use the raw value and the corresponding condition.

## Persistence — REDGUARD.CHT

The `savecheats` command (cheat #6) writes the 64-slot state array, including the 13 named cheat slots, to a file called `REDGUARD.CHT` in the game directory. It clears its own slot before writing, then sets that slot to one after closing the file. On startup, the engine checks for this file and restores any previously saved cheat states. Delete the file to reset all cheats. See [CHT format specification](../formats/CHT.md) for the binary layout.

## Obfuscation

The cheat names are stored in the game binary as XOR-encoded strings (each byte XOR'd with the constant `0xAA`). This is a simple obfuscation to prevent discovery via hex editors or string dumps. The names are decoded into memory at startup.

## External References

- [UESP Redguard Console](https://en.uesp.net/wiki/Redguard:Console)
- [UESP Redguard Cheats](https://en.uesp.net/wiki/Redguard:Cheats)
