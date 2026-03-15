# MENU.INI

Menu system layout configuration defining page structure, text placement, textures, selectable items, and embedded movie (Smacker) playback definitions.

Shipped path: `MENU.INI` (game root directory, 42 KB, 961 lines).

## Sections

### `[general]`

Global menu system flags:

| Key | Default | Description |
|---|---|---|
| `preload_sprites` | `1` | Preload menu sprite textures at startup. |
| `hi_res_menu` | `1` | Use high-resolution menu rendering. |
| `ignore_autosave` | `0` | Skip auto-save slot handling. |
| `force_movies` | `0` | Force movie playback (skip user-skip). |

### `[pageN]` (pages 0–7)

Each page section defines the visual layout and interactive elements for one menu screen. 8 pages are defined in the shipped file.

#### Page assignments

| Page | Menu Screen |
|---|---|
| 0 | Main Menu (Save, Load, New, Options, Movies, Continue, Quit) |
| 1 | Save Game |
| 2 | Load Game |
| 3 | Movies (cinematic list with Smacker playback) |
| 4 | Options (Sound Volume, Music Volume, Text toggle, Speech toggle) |
| 5 | Confirm New Game |
| 6 | Confirm Overwrite Save |
| 7 | Key Binding Options |

#### Texture fields

Each page uses up to 2 background textures sourced from TEXBSI archives:

| Key | Description |
|---|---|
| `texture_set[0]`, `texture_set[1]` | Texture set number (TEXBSI archive id). |
| `texture_index[0]`, `texture_index[1]` | Index within the texture set. |

All shipped pages use texture set `289`.

#### Per-element fields

Elements are indexed sequentially within each page (e.g. `text_x[0]`, `text_x[1]`, etc.):

| Key | Description |
|---|---|
| `text_x[N]` | X position of the text element. |
| `text_y[N]` | Y position of the text element. |
| `text[N]` | Display text string. |
| `justify[N]` | Text alignment: `0` = left, `1` = center, `2` = right. |
| `selectable[N]` | `1` if the user can select/interact with this element. |
| `default[N]` | `1` if this element is selected when the page first opens. |
| `texture[N]` | Which background texture to place text on (`0` or `1`). |
| `action[N]` | Action id dispatched when selected (program-defined). |
| `grayed[N]` | `1` to display in the grayed-out font. |
| `output_x[N]` | X position for special output (key name text, slider art). |
| `output_y[N]` | Y position for special output. |
| `slider_min[N]` | Minimum value for slider elements. |
| `slider_max[N]` | Maximum value for slider elements. |

#### Movie fields (page 3)

Page 3 defines cinematic movie entries with timed subtitle overlays:

| Key | Description |
|---|---|
| `movie_name[N]` | Smacker (`.SMK`) filename in the `anims` directory. |
| `movie_keys[N]` | Comma-separated key frames for user fast-forward. |
| `movieM_text[N]` | Subtitle overlay: `red,green,blue,start_frame,stop_frame,text`. |

The shipped file defines 2 movies: `INTRO.SMK` (game intro, 88 subtitle lines) and `OUTRO.SMK` (ending, 34 subtitle lines).

#### Action id ranges

Action ids follow a convention per page:

| Range | Meaning |
|---|---|
| `-1` | Continue / resume game. |
| `-2` | Quit game. |
| 1–10 | Main menu navigation (Save=1, Load=2, Movies=3, Options=4, New=10). |
| 100–102 | Save game page actions. |
| 200–202 | Load game page actions. |
| 300–302 | Movie page actions. |
| 400–411 | Options page actions (volume sliders, toggles). |
| 500–501 | Confirm new game actions. |
| 600–601 | Confirm overwrite actions. |
| 700–724 | Key binding page actions. |

## External References

- [UESP: Mod:Redguard File Formats](https://en.uesp.net/wiki/Mod:Redguard_File_Formats)
