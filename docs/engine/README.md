# Engine Details

Findings from engine analysis that go beyond file format documentation.

| Topic | Description | Docs |
|---|---|---|
| Cheat System | 13 XOR-obfuscated cheat codes built into the engine | [cheats.md](cheats.md) |
| Item Attachment | Vertex-tracking system for positioning held items (swords, shields) on characters. No skeleton — the engine tracks a vertex index per animation frame via RAGR opcode 0 (ShowFrame). All 16 animation opcodes documented. | [attachment.md](attachment.md) |
| SOUP Scripting | SOUP386 virtual machine architecture, bytecode encoding (22 opcodes), value modes, operator tables, threading model, function dispatch (367 functions), and global flag system (369 flags) | [SOUP.md](SOUP.md) |
| Sky Renderer | Two-layer sky system: static GXA skybox + scrolling BSI texture, sun disc billboard, per-world configuration, and runtime console commands. | [sky.md](sky.md) |
| Water Waves | Per-frame sine-table vertex displacement on water terrain cells. Radial concentric ripples driven by `world_wave` INI parameters (amplitude, speed, spatial frequency) with runtime console tuning. | [water.md](water.md) |
