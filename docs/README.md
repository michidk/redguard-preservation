# Redguard Preservation — Documentation

Documented findings from reverse-engineering *The Elder Scrolls Adventures: Redguard* (1998) — file formats, engine behavior, and undocumented features. Analysis is primarily based on the GOG release (Glide renderer), with original-CD differences noted where known.

## GOG Version Contents

### Main Files

| File | Size | Description |
|---|---|---|
| `REDGUARD.EXE` | 980 K | Main game executable |
| `RGFX.EXE` | 1.9 M | Glide (3dfx) renderer executable |
| `DOS4GW.EXE` | 260 K | DOS/4GW protected-mode extender |
| `3DfxSpl2.dll` | 1.1 M | 3dfx Splash/Glide support library |
| `glide2x.dll` | 1.3 M | Glide 2.x runtime |
| `ENGLISH.RTX` | 177 M | Dialogue text + voice audio container (largest file in the game) |
| `REDGUARD.SWG` / `redguard.swx` | 20 K each | Swap/workspace files |
| `OBJECT.SAV` | 4 K | Object state persistence |
| `*.INI` (7 files) | — | Configuration: `COMBAT`, `ITEM`, `KEYS`, `MENU`, `REGISTRY`, `surface`, `SYSTEM`, `WORLD` |
| `*.LOG` (14 files) | — | Runtime log files (BITMAP, CAMERA, COMBAT, ERROR, EXIT, GENERAL, GRID, MAINLOOP, MENU, OBJECT, PATH, RAI, SAVEFILE, STARTUP, TESTMAPS) |
| `*.TXT` (3 files) | — | `BETHESDA.TXT`, `CREDITS.TXT`, `ReadMe.TXT` |

### Asset Directories

| Directory | Size | Files | Contents |
|---|---|---|---|
| `fonts/` | 604 K | 29 | 29 `.FNT` bitmap font files (Arial variants, HI/LO menu fonts, Redguard-styled fonts) |
| `fxart/` | 102 M | 755 | Glide-version 3D assets: 204 `.3DC`, 65 `.3D`, 31 `.ROB`, 27 `.COL`, 415 `TEXBSI.xxx` textures, `FOG.INI`, `more` (build manifest from Hugh's 3D tool) |
| `maps/` | 12 M | 45 | 27 `.RGM` scene files, 5 `.PVO` visibility octrees, 4 `.WLD` terrain files, 9 `.TSG` trigger-state files |
| `sound/` | 5.3 M | 49 | `MAIN.SFX` (all 118 sound effects), `R212.WAV`, Miles Sound System drivers (`.mdi`, `.dig`), `STATE.RST`, audio configs |
| `soup386/` | 48 K | 1 | `SOUP386.DEF` — script function/flag definitions for the SOUP engine |
| `system/` | 18 M | 69 | 65 `.GXA` UI graphics (menus, inventory, compass, maps, skies), `gui.anm`, `gui.lbm`, `pointers.bmp`, `SKY_61.PCX` |
| `SAVEGAME/` | 79 M | 594 | 17 save slots (`SAVEGAME.000`–`016`), each containing per-map `.TSG` trigger-state snapshots + `LOGBOOK.TXT` |

## GOG vs Original CD Differences

Redguard shipped with two parallel sets of 3D model assets for different renderers:

| Directory | Renderer | Model versions | Description |
|---|---|---|---|
| `3dart/`  | Software | v2.6, v2.7 | Original software-rendered assets. 120 v2.6 .3DC + 52 v2.7 .3D + 27 v2.7 .3DC. Also contains `art_pal.col` (the software renderer palette). |
| `fxart/`  | Glide (3dfx) / GOG | v4.0, v5.0 | Glide-accelerated assets used by the GOG release. 204 v4.0 .3DC + 26 v4.0 .3D + 39 v5.0 .3D. 415 TEXBSI texture files, 27 COL palette files. |

The two directories contain the same models re-exported for different renderers. The Glide versions (v4.0/v5.0) have a cleaner header layout and different texture encoding. See [3D — v2.6/v2.7 Header Differences](formats/models/3d.md#v26v27-header-differences) for details.

> **Note**: The GOG distribution contains `fxart/` only. The software-renderer `3dart/` directory shipped on the original CD but is not present in the GOG release.
