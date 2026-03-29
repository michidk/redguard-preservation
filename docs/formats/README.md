# File Formats Overview

Binary, little-endian file formats.

| File Type | Extension(s) | Parser | Output | Docs | Description |
|---|---|---|---|---|---|
| Sound Effects | .sfx | yes | `.wav` files (directory extract) | [SFX.md](SFX.md) | All game sound effects in a single container (`MAIN.SFX`); 118 raw PCM clips. |
| Dialogue Audio | .rtx | yes | `.wav` files + `index.json` (directory extract) | [RTX.md](RTX.md) | Dialogue text + voice clip container (`ENGLISH.RTX`) with chunk index footer (`RNAV`); 4866 entries (3933 voice clips, 933 text-only). |
| Audio | .ogg | — | — | [Ogg Vorbis spec](https://xiph.org/vorbis/doc/Vorbis_I_spec.html) | Ogg Vorbis format used for game audio. |
| TEXBSI | .### | yes | `.png` files (directory extract) + metadata `.json` | [TEXBSI.md](TEXBSI.md) | Texture container (`TEXBSI.###`); indexed-color images with optional palettes and animation. |
| Palette | .col | yes | swatch `.png` + palette `.json` | [COL.md](COL.md) | 256-color palette files; 776 bytes (8-byte header + 256×RGB). |
| Font | .fnt | yes | `.png` + BMFont `.fnt` + glyph `.json`, or `.ttf` | [FNT.md](FNT.md) | Font graphics files—56-byte header + optional palette data. |
| GXA Bitmap | .gxa | yes | `.png` files (directory extract) + metadata `.json` | [GXA.md](GXA.md) | Indexed-color bitmap archive used for UI, loading flashes, and sky panoramas. |
| Model | .3d | yes | `.glb` | [3D](models/3d.md) | Static 3D models. |
| Animated Model | .3dc | yes | `.glb` | [3DC](models/3dc.md) | Animated 3D models (multi-frame). |
| ROB Archive | .rob | yes | `.glb` | [ROB.md](ROB.md) | Contains world/dungeon model data; used within maps. |
| Map Data | .rgm | yes | output directory: `.glb` + scene/actors/navigation/tables `.json` + per-actor `.soup`/`.json` scripts | [RGM.md](RGM.md) | Game map files containing sections for objects, scripts, locations, collisions, etc. |
| World Geometry | .wld | yes | `.glb` + metadata `.json`, or map `.png` set | [WLD.md](WLD.md) | World geometry/height-map data with 4 sections and 128×128 maps; supports terrain GLB export (and companion RGM merge). |
| Visibility Octree | .pvo | yes | `.json` | [PVO](pvo/format.md) | Pre-computed visibility octree for level geometry culling. |
| Cheat States | .cht | yes | — | [CHT.md](CHT.md) | Cheat persistence file (`REDGUARD.CHT`); 256-byte raw dump of 64 u32 LE cheat state slots. |
| SOUP386.DEF | .def | yes | — | [SOUPDEF.md](../engine/SOUPDEF.md) | Definition-file format for SOUP callable functions, references/equates, attributes, and global flags. |
