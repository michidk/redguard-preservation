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

## GOG vs original CD differences

The English CD installer contains software and Glide resources. GOG selects the
Glide distribution and omits the software-specific asset directory and executable.
The game data, renderer choice, configuration defaults, and host packaging
are separate parts of edition compatibility.

### Executables and installation layout

| Resource | CD installer payload | GOG distribution |
|---|---|---|
| Software engine | `RG.EXE` beside the CD installer; uses `3dart/` | Omitted |
| Glide engine | `RGFX.EXE` beside the CD installer; uses `fxart/` | `Redguard/RGFX.EXE` |
| Windows launcher | `REDGUARD.EXE` in the installer cabinet | `Redguard/REDGUARD.EXE` |
| Installed resources | `DATA1.CAB`, including both renderer asset sets | Expanded `Redguard/` directory with Glide assets |
| Cinematics | `.SMK` files in the CD data directory | Data track of `game.gog`, described by `game.ins` |
| Music | Red Book audio tracks on the CD | Audio tracks in the same `game.gog` image |
| Host runtime | DOS/4GW and the selected renderer's hardware support | DOSBox configuration and Glide support libraries |

`RGFX.EXE` is byte-identical between the English CD payload and GOG. The Windows
`REDGUARD.EXE` launchers differ; they are not the DOS rendering executables.
The CD places the DOS executables outside `DATA1.CAB`, so extracting that cabinet
alone does not produce a runnable installation.

### Renderer-specific assets

| Directory | Renderer | Model versions | Resources |
|---|---|---|---|
| `3dart/` | Software | v2.6, v2.7 | 120 v2.6 `.3DC`, 52 v2.7 `.3D`, and 27 v2.7 `.3DC`; ROB archives, `TEXTURE.###` textures, COL palettes, and `SHADE.###` / `HAZE.###` tables. |
| `fxart/` | Glide | v4.0, v5.0 | 204 v4.0 `.3DC`, 26 v4.0 `.3D`, and 39 v5.0 `.3D`; ROB archives, `TEXBSI.###` textures, COL palettes, and standalone BSI images. |

GOG's `fxart/` contents are byte-identical to the English CD installer payload.
GOG does not include `3dart/`.

The directories overlap in model names, but are not interchangeable copies.
Glide includes additional standalone models; software includes additional ROB
archives. Even matching names can have different geometry and animation data:
`TIRBA001.3DC` has different face and frame counts, and `NCROCK.3D` has different
face counts. Resolve resources within the chosen renderer's asset set rather
than substituting a same-named file from the other set.

Both model generations use renderer-specific headers and texture references.
See [3D model versions](formats/models/3d.md#v26v27-header-differences).
Software texture banks are named `TEXTURE.###`, not `TEXBSI.###`; changing the
filename prefix does not establish format compatibility.

`3dart/` contains 18 COL palettes, including `art_pal.col`, world palettes,
and sky palettes. It does not use `art_pal.col` exclusively. Matching palette
names can differ between renderers: `NECRO2.COL` and `REDGUARD.COL` have different
contents. The Glide set contains 27 populated COL palettes and an empty
`CLDTEST.COL` placeholder. See [COL palettes](formats/COL.md).

### Shared content

The English CD installer payload and GOG share byte-identical:

- RGM scenes and their embedded scripts, WLD terrain, and PVO visibility data;
- `SOUP386.DEF`, `WORLD.INI`, `COMBAT.INI`, `ITEM.INI`, `MENU.INI`, `KEYS.INI`,
  `REGISTRY.INI`, and `surface.ini`;
- `ENGLISH.RTX` dialogue and speech, `MAIN.SFX`, and the other sound resources
  except the configured `dig.ini` and `mdi.ini` device selections;
- fonts and system artwork, including title, menu, HUD, and sky images.

Renderer-specific model and texture differences therefore coexist with shared
authored scenes, scripts, and presentation resources.

### Configuration defaults

GOG changes these values relative to the English CD installer's `SYSTEM.INI`:

| Section and key | CD installer default | GOG default |
|---|---|---|
| `[screen] smk_interlace` | `1` | `0` |
| `[system] fast_sound` | `0` | `1` |
| `[3dmanager] cache_lifetime` | `32` | `64` |
| `[dialog] dialog_print_text` | `0` | `1` |
| `[3dfx] resolution` | `7` | `12` |

These are stored settings, not differences in the shared game scripts.
`animation_drive` differs only in drive-letter capitalization, `d:\` versus
`D:\`. The software `[screen] resolution` remains `1` in both configurations.
See [SYSTEM.INI](config/system-ini.md) for field meanings and renderer overrides.

The CD cabinet's `sound/dig.ini` and `sound/mdi.ini` contain setup comments
without device selections. GOG supplies Sound Blaster device and driver settings.
Hardware configuration belongs to the installation environment; it does not
replace the game's Red Book music tracks.

### Cinematics, disc audio, and support files

The CD Smacker movies are byte-identical to those in GOG's data track,
including their embedded audio. GOG's virtual disc omits the CD's installer
cabinet, DOS engines, setup files, and bundled hardware-driver installers.
Its ISO9660 filenames `FINDRE~1.EXE` and `REDGUA~1.ICO` refer to the same
contents as the CD's `FindRedguard.exe` and `RedguardCD.ICO`.

An extracted CD data directory contains movies and installation files, but
not the Red Book audio tracks. Mounting that directory as a CD-ROM cannot
reconstruct the missing music tracks. A music-capable disc source must retain
the audio tracks and their numbering; separately distributed soundtrack files
are not a substitute for the disc's track layout. See [background music](engine/music.md).

GOG adds host Glide libraries and DOSBox launch configuration. The selected
Glide implementation determines the splash variant independently of the shared
Glide executable. See [startup presentation](engine/startup.md#splash-variants).
