# Configuration

INI settings for worlds, actors, rendering, input, and presentation.

## Settings and runtime behavior

| Area | Configuration | Engine reference |
|---|---|---|
| World rendering | [WORLD.INI](world-ini.md), [FOG.INI](fog-ini.md) | [Lighting](../engine/terrain-lighting.md), [fog](../engine/fog.md), [sky](../engine/sky.md), [water](../engine/water.md) |
| Actors and input | [COMBAT.INI](combat-ini.md), [ITEM.INI](item-ini.md), [KEYS.INI](keys-ini.md) | [Movement](../engine/player.md), [animation](../engine/animation.md), [attachments](../engine/attachment.md) |
| Menus and movies | [MENU.INI](menu-ini.md) | [Book menu](../engine/menu.md), [cinematics](../engine/movies.md) |
| Sound | [SURFACE.INI](surface-ini.md), [Miles settings](miles-ini.md) | [Positional sound](../engine/sound.md), [music](../engine/music.md) |

[SYSTEM.INI](system-ini.md) contains the shared engine settings, including camera, dialogue, rendering, and debug options.

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
| `SURFACE.INI` | 4.3 KB | [surface-ini.md](surface-ini.md) | Texture surface-class assignments and animation sound remapping. |

## Asset Directory Files

| File | Path | Docs | Description |
|---|---|---|---|
| `FOG.INI` | `fxart/` | [fog-ini.md](fog-ini.md) | Fog density ramp table for the terrain renderer. |
| `DIG.INI` | `sound/` | [miles-ini.md](miles-ini.md) | [Miles digital audio driver](../formats/MILES-DRIVER.md) configuration. |
| `MDI.INI` | `sound/` | [miles-ini.md](miles-ini.md) | [Miles MIDI driver](../formats/MILES-DRIVER.md) configuration. |
