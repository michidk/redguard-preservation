# Redguard Preservation

Reference documentation for *The Elder Scrolls Adventures: Redguard* (1998) file formats, engine behavior, and undocumented features. The documentation covers the GOG release's Glide renderer and notes known original-CD differences.

## Reference sections

- [Engine behavior](engine/): scripting, actor animation, movement, rendering, audio, menus, and hidden cheats.
- [File formats](formats/): binary layouts for models, maps, textures, sound, saved state, and bundled support files.
- [Configuration](config/): INI settings for worlds, combat, items, input, rendering, and menus.

## Starting points

| Task | References |
|---|---|
| Load a world | [RGM scenes](formats/RGM.md), [WLD terrain](formats/WLD.md), [WORLD.INI](config/world-ini.md), and [PVO visibility](formats/pvo/format.md) |
| Reproduce actor behavior | [SOUP scripting](engine/SOUP.md), [animation playback](engine/animation.md), [player movement](engine/player.md), and [item attachment](engine/attachment.md) |
| Reproduce the scene renderer | [Terrain lighting](engine/terrain-lighting.md), [fog and clipping](engine/fog.md), [sky](engine/sky.md), and [water](engine/water.md) |
| Reproduce menus and presentation | [Book menu](engine/menu.md), [HUD](engine/hud.md), [subtitles](engine/subtitles.md), and [cinematics](engine/movies.md) |
| Inspect or convert assets | [CLI usage and library API](https://github.com/michidk/redguard-preservation/blob/main/README.md) |

## Installation contents

### Main Files

| File | Size | Description |
|---|---|---|
| `REDGUARD.EXE` | 980 K | Main game executable |
| `RGFX.EXE` | 1.9 M | Glide (3dfx) renderer executable |
| `DOS4GW.EXE` | 260 K | DOS/4GW protected-mode extender |
| `3DfxSpl2.dll` | 1.1 M | 3dfx Splash/Glide support library |
| `glide2x.dll` | 1.3 M | Glide 2.x runtime |
| `ENGLISH.RTX` | 177 M | Dialogue text + voice audio container (largest file in the game) |
| `REDGUARD.SWG` / `redguard.swx` | 18,610 bytes each | [SideWinder Gamepad and Freestyle Pro controller profiles](formats/SIDEWINDER.md) |
| `OBJECT.SAV` | 4 K | Object state persistence |
| `*.INI` | — | Configuration: `COMBAT`, `ITEM`, `KEYS`, `MENU`, `REGISTRY`, `surface`, `SYSTEM`, `WORLD` |
| `*.LOG` | — | Runtime log files (BITMAP, CAMERA, COMBAT, ERROR, EXIT, GENERAL, GRID, MAINLOOP, MENU, OBJECT, PATH, RAI, SAVEFILE, STARTUP, TESTMAPS) |
| `*.TXT` (3 files) | — | `BETHESDA.TXT`, `CREDITS.TXT`, `ReadMe.TXT` |

### Asset Directories

| Directory | Size | Files | Contents |
|---|---|---|---|
| `fonts/` | 604 K | 29 | 29 `.FNT` bitmap font files (Arial variants, HI/LO menu fonts, Redguard-styled fonts) |
| `fxart/` | 102 M | 755 | Glide-version 3D assets: 204 `.3DC`, 65 `.3D`, 31 `.ROB`, 27 `.COL`, 415 `TEXBSI.xxx` textures, `FOG.INI`, [`more` model information report](formats/MORE.md) |
| `maps/` | 12 M | 45 | 27 `.RGM` scene files, 5 `.PVO` visibility octrees, 4 `.WLD` terrain files, 9 `.TSG` trigger-state files |
| `sound/` | 5.3 M | 49 | `MAIN.SFX` (all 118 sound effects), `R212.WAV`, [Miles Sound System drivers](formats/MILES-DRIVER.md) (`.mdi`, `.dig`), [OPL and AD instrument banks](formats/OPL.md), [PIF program settings](formats/PIF.md), [RST editor state](formats/RST.md), audio configs |
| `soup386/` | 48 K | 1 | `SOUP386.DEF` — script function/flag definitions for the SOUP engine |
| `system/` | 18 M | 69 | 65 `.GXA` UI graphics (menus, inventory, compass, maps, skies), `gui.anm`, `gui.lbm`, `pointers.bmp`, `SKY_61.PCX` |
| `SAVEGAME/` | 79 M | 594 | 17 save slots (`SAVEGAME.000`–`016`), each containing per-map `.TSG` trigger-state snapshots + `LOGBOOK.TXT` |

## Asset and support-file roles

Models, textures, fonts, UI images, maps, dialogue, sound effects, movie data,
and CD music supply the content of a faithful reimplementation. Hardware
configuration, audio-utility launch settings, SideWinder presets, and tool
reports describe the original environment. Their layouts remain part of the
format references in [Bundled support resources](formats/BUNDLED-RESOURCES.md).

The cursor file `system/pointers.bmp` contains a GXA archive despite its
extension. `fxart/fxart.txt` and `fxart/fxart3dc.txt` are historical directory
listings, and `fxart/LOGFILE.TXT` records collision-sphere generation.
[The support-file reference](formats/BUNDLED-RESOURCES.md) describes these
files and the Miles setup catalog `sound/MSSDRVR.LST`.

[Save-game containers](formats/SAVEGAME.md) describe `SAVEGAME.SAV` and the
per-world `.TSG` snapshots. A slot's `LOGBOOK.TXT` is journal content.

## GOG vs Original CD Differences

Redguard shipped with two parallel sets of 3D model assets for different renderers:

| Directory | Renderer | Model versions | Description |
|---|---|---|---|
| `3dart/`  | Software | v2.6, v2.7 | Original software-rendered assets. 120 v2.6 .3DC + 52 v2.7 .3D + 27 v2.7 .3DC. Also contains `art_pal.col` (the software renderer palette). |
| `fxart/`  | Glide (3dfx) / GOG | v4.0, v5.0 | Glide-accelerated assets used by the GOG release. 204 v4.0 .3DC + 26 v4.0 .3D + 39 v5.0 .3D. 415 TEXBSI texture files, 27 COL palette files. |

The two directories contain the same models re-exported for different renderers. The Glide versions (v4.0/v5.0) have a cleaner header layout and different texture encoding. See [3D — v2.6/v2.7 Header Differences](formats/models/3d.md#v26v27-header-differences) for details.

> **Note**: The GOG distribution contains `fxart/` only. The software-renderer `3dart/` directory shipped on the original CD but is not present in the GOG release.
