# File formats

Binary layouts for game assets, scripts, saved state, and bundled support resources.
Integers are little-endian unless a reference states otherwise.
For runtime rules, see [engine behavior](../engine/).
Asset locations and renderer versions are listed in [Asset Directories](../#asset-directories).

The [project README](https://github.com/michidk/redguard-preservation/blob/main/README.md) describes CLI conversion commands and the library API.

## Models and worlds

| Format | Extension | Contents |
|---|---|---|
| [Model](models/3d.md) | `.3d` | Static 3D models. |
| [Animated Model](models/3dc.md) | `.3dc` | Animated 3D models (multi-frame). |
| [ROB Archive](ROB.md) | `.rob` | Model archive used by worlds and dungeons. |
| [Map Data](RGM.md) | `.rgm` | Scene sections for objects, scripts, locations, and collision data. |
| [World Geometry](WLD.md) | `.wld` | World geometry and height maps stored in four sections on a 128×128 grid. |
| [Visibility Octree](pvo/format.md) | `.pvo` | Pre-computed visibility octree for level geometry culling. |

## Images, textures, and fonts

| Format | Extension | Contents |
|---|---|---|
| [TEXBSI](TEXBSI.md) | `.###` | Texture container (`TEXBSI.###`); indexed-color images with optional palettes and animation. |
| [Palette](COL.md) | `.col` | 256-color palette files; 776 bytes (8-byte header + 256×RGB). |
| [Font](FNT.md) | `.fnt` | Bitmap glyphs with a 56-byte header and optional palette. |
| [GXA Bitmap](GXA.md) | `.gxa` | Indexed-color bitmap archive used for UI, loading flashes, and sky panoramas. |
| [PCX Image](PCX.md) | `.pcx` | Eight-bit indexed PCX image with a trailing palette. |
| [IFF Bitmap](LBM.md) | `.lbm` | Packed PBM bitmap with ByteRun1 scanlines. |
| [Deluxe Paint Animation](ANM.md) | `.anm` | LPF pages containing indexed bitmap deltas. |
| [Standalone BSI](BSI.md) | `.bsi` | Standalone image chunks for named textures such as skies and suns. |

## Audio and cinematics

| Format | Extension | Contents |
|---|---|---|
| [Sound Effects](SFX.md) | `.sfx` | All game sound effects in a single container (`MAIN.SFX`); 118 raw PCM clips. |
| [Dialogue Audio](RTX.md) | `.rtx` | Dialogue text + voice clip container (`ENGLISH.RTX`) with chunk index footer (`RNAV`); 4866 entries (3933 voice clips, 933 text-only). |
| [Smacker video](SMK.md) | `.smk` | Compressed indexed video, palettes, audio streams, and frame metadata. |

## Scripts and saved state

| Format | Extension | Contents |
|---|---|---|
| [SOUP386.DEF](../engine/SOUPDEF.md) | `.def` | Definition-file format for SOUP callable functions, references/equates, attributes, and global flags. |
| [Save Games](SAVEGAME.md) | `.sav`, `.tsg` | Save metadata, state chunks, and per-map snapshots. |
| [Cheat States](CHT.md) | `.cht` | Cheat persistence file (`REDGUARD.CHT`); 256-byte raw dump of 64 u32 LE cheat state slots. |

## Startup graphics

- [DOS Glide splash tables](GLIDE-SPLASH.md): shield geometry and animation tables in the game overlay.
- [Windows Glide splash resource](GLIDE-SPLASH-DLL.md): the animated logo resource in `3DfxSpl2.dll`.

[Startup presentation](../engine/startup.md) describes how the splash assets are displayed.

## Bundled support files

These resources describe DOS hardware interfaces, setup utilities, controller
presets, and development tools. [Bundled support resources](BUNDLED-RESOURCES.md)
explains their roles alongside the game assets.

| Resource | Description | Format reference |
|---|---|---|
| `.dig`, `.mdi` | Executable Miles DOS digital audio and MIDI hardware drivers. | [Miles drivers](MILES-DRIVER.md) |
| `.pif` | Windows launch settings for DOS audio utilities. | [PIF](PIF.md) |
| `.opl`, `.ad` | Two-operator FM MIDI instrument banks. | [OPL and AD](OPL.md) |
| `.rst` | Sectioned text editor settings and remembered file positions. | [RST](RST.md) |
| `.swg`, `.swx` | Microsoft SideWinder controller preferences. | [SideWinder](SIDEWINDER.md) |
| Extensionless `more` | Historical model filenames and version labels. | [MORE](MORE.md) |
