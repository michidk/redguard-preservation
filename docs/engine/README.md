# Engine Details

Findings from engine analysis that go beyond file format documentation.

| Topic | Description | Docs |
|---|---|---|
| Cheat System | 13 XOR-obfuscated cheat codes built into the engine | [cheats.md](cheats.md) |
| Item Attachment | Vertex-tracking system for positioning held items (swords, shields) on characters. No skeleton — the engine tracks a vertex index per animation frame via RAGR opcode 0 (ShowFrame). All 16 animation opcodes documented. | [attachment.md](attachment.md) |
| SOUP Scripting | SOUP386 virtual machine architecture, bytecode encoding (22 opcodes), value modes, operator tables, threading model, function dispatch (367 functions), and global flag system (369 flags) | [SOUP.md](SOUP.md) |
| Music | Red Book CD tracks per world, track switching, repetition, and the ignored `redbook_volume`. | [music.md](music.md) |
| Positional Sound | Listener, `sound_distance` radius, linear volume curve, triangle-wave pan, 16 channels without stealing, SOUP sound functions, and SURFACE.INI remapping. | [sound.md](sound.md) |
| Sky Renderer | Glide BSI plane, fog-colored background, scrolling inputs, sun configuration, and software-renderer limits. | [sky.md](sky.md) |
| Glide fog and clipping | Scene and sky ranges, fog-table lookup, and encoded RGB blending. | [fog.md](fog.md) |
| Water Waves | Per-frame sine-table vertex displacement on water terrain cells. Radial concentric ripples driven by `world_wave` INI parameters (amplitude, speed, spatial frequency) with runtime console tuning. | [water.md](water.md) |

Player movement units, ground-turn arithmetic, and coverage limits are documented in [player.md](player.md).

[Startup presentation](startup.md) covers the original splash assets, projection, lighting, and screen sequence.

[Book menu](menu.md) covers MENU.INI-driven surfaces, model placement, fonts, selection, and Quit.
