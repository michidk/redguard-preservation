# Configuration Files

Text-based INI configuration files shipped with the game.

## Game Root Directory

| File | Size | Docs | Description |
|---|---|---|---|
| `SYSTEM.INI` | 5.2 KB | [system-ini.md](system-ini.md) | Primary engine configuration: rendering, gameplay, camera, dialog, debug, and 3D subsystem parameters. |
| `COMBAT.INI` | 54 KB | [combat-ini.md](combat-ini.md) | Combat system: attack/defense moves, combos, and dialogue taunts for all combatants. |
| `ITEM.INI` | 30 KB | [item-ini.md](item-ini.md) | Item database: all collectible objects, weapons, potions, keys, and quest items. |
| `WORLD.INI` | 19 KB | [world-ini.md](world-ini.md) | World/level database: map files, palettes, lighting, sky, weather, and PVO node maps. |
| `MENU.INI` | 42 KB | [menu-ini.md](menu-ini.md) | Menu system layout: page structure, text placement, textures, and movie definitions. |
| `KEYS.INI` | 3 KB | [keys-ini.md](keys-ini.md) | Input bindings: keyboard scancodes, mouse buttons, and joystick axes to game actions. |
| `REGISTRY.INI` | 302 B | [registry-ini.md](registry-ini.md) | File-system abstraction: archive lookup and 32-bit file access (non-functional in shipped game). |
| `SURFACE.INI` | 4.3 KB | [surface-ini.md](surface-ini.md) | Terrain surface-type assignments, blend behavior, and sound remapping. |

## Asset Directory Files

| File | Path | Docs | Description |
|---|---|---|---|
| `FOG.INI` | `fxart/` | [fog-ini.md](fog-ini.md) | Fog density ramp table for the terrain renderer. |
| `DIG.INI` | `sound/` | — | Miles Sound System digital audio driver configuration. |
| `MDI.INI` | `sound/` | — | Miles Sound System MIDI driver configuration. |
