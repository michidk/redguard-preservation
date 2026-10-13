# Engine behavior

Runtime rules for scripts, actors, world rendering, sound, and presentation.
The [file format references](../formats/) describe the stored data;
the [configuration reference](../config/) describes the INI settings.

## Scripting and actors

| Topic | Behavior |
|---|---|
| [SOUP scripting](SOUP.md) | Virtual machine, bytecode, value modes, tasks, function dispatch, and global flags. |
| [SOUP386.DEF](SOUPDEF.md) | Function, attribute, reference, and flag definitions used by scripts. |
| [Actor animation](animation.md) | Model frames, animation requests, loop counts, and placed actor eligibility. |
| [Player movement](player.md) | Coordinate units, turning, collision, and frame timing. |
| [Item attachment](attachment.md) | Vertex tracking for held weapons and shields. |
| [Developer console](console.md) | All 82 registered commands, aliases, subcommands, numeric parsing, and command quirks. |
| [Hidden cheats](cheats.md) | Cheat entry, effects, persistence, and developer controls. |

## World rendering

| Topic | Behavior |
|---|---|
| [Terrain lighting](terrain-lighting.md) | Ambient and directional RGB lighting and terrain normals. |
| [Fog and clipping](fog.md) | Scene and sky ranges, fog tables, and color blending. |
| [Sky](sky.md) | Sky planes, background color, scrolling, and sun settings. |
| [Water waves](water.md) | Terrain vertex displacement and wave settings. |

## Audio and presentation

| Topic | Behavior |
|---|---|
| [Positional sound](sound.md) | Distance attenuation, stereo pan, channels, and surface sound remapping. |
| [Music](music.md) | CD tracks, world selection, switching, and repetition. |
| [Subtitles](subtitles.md) | Dialogue, taunt, and movie text layout and timing. |
| [In-game HUD](hud.md) | Health, items, activation prompts, compass, and draw order. |
| [Book menu](menu.md) | Menu models, text placement, fonts, selection, and actions. |
| [Cinematic playback](movies.md) | Movie selection, skip points, subtitles, and display rules. |
| [Startup presentation](startup.md) | Splash assets, projection, lighting, and screen sequence. |
