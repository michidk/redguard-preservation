# CHT — Cheat Persistence

Raw dump of the engine's cheat state array, written by the `savecheats` cheat command to `REDGUARD.CHT` in the game directory.

## Overall Structure

```
[Cheat state array — 64 × u32 LE values, 256 bytes total]
```

## Format

The file is exactly **256 bytes**: 64 little-endian `u32` slots. Each slot holds the state value for one cheat. The first 13 slots correspond to the built-in cheat codes; slots 13–63 are unused (always zero in practice).

| Offset | Size | Type | Name | Description |
|--------|------|------|------|-------------|
| 0x00 | 4 | `u32` | `oracle` | Cheat #0 state |
| 0x04 | 4 | `u32` | `nodemarker` | Cheat #1 state |
| 0x08 | 4 | `u32` | `moonraker` | Cheat #2 state |
| 0x0C | 4 | `u32` | `task` | Cheat #3 state |
| 0x10 | 4 | `u32` | `animation` | Cheat #4 state |
| 0x14 | 4 | `u32` | `magiccarpet` | Cheat #5 state |
| 0x18 | 4 | `u32` | `savecheats` | Cheat #6 state |
| 0x1C | 4 | `u32` | `drevil` | Cheat #7 state |
| 0x20 | 4 | `u32` | `drno` | Cheat #8 state |
| 0x24 | 4 | `u32` | `goldfinger` | Cheat #9 state |
| 0x28 | 4 | `u32` | `neversaydie` | Cheat #10 state |
| 0x2C | 4 | `u32` | `oddjob` | Cheat #11 state |
| 0x30 | 4 | `u32` | `yeahbaby` | Cheat #12 state |
| 0x34 | 204 | — | unused | Slots 13–63, always zero |

State values: `0` = off, `1` = on. The console `cheat = N` syntax can set arbitrary integer values, but normal toggle usage produces only 0 or 1.

## Behavior

- **Write**: The `savecheats` cheat (index 6) writes the current 256-byte state array to `REDGUARD.CHT`.
- **Read**: On startup, the engine checks for `REDGUARD.CHT` and restores all cheat states from it.
- **Reset**: Delete the file to clear all persisted cheats.

See [Cheat System](../engine/cheats.md) for cheat descriptions and activation methods.

## External References

- [UESP Redguard Console](https://en.uesp.net/wiki/Redguard:Console)
