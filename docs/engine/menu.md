# Book menu

The menu renders textured models from `fxart/MENU.ROB` and paints configured text directly into their TEXBSI surfaces.

## Data and scene

[`MENU.INI`](../config/menu-ini.md) supplies ordered labels, texel positions, alignment, selectable/default/grayed flags, texture references, and action IDs. Labels do not identify actions. `import::menu_ini::MenuPage::parse` exposes a requested page and ignores fields outside its text/layout contract.

The model scene uses these placements in original engine coordinates, with positive Y down. Each model turns half a revolution around Y. The default view faces positive Z at the origin.

| Model | X | Y | Z |
|---|---:|---:|---:|
| MENUA001 | 384 | 28 | 396 |
| MB_PG01 | 384 | 28 | 390 |
| MB_TABLE | 150 | -72 | 550 |
| MB_PG02 | 384 | 28 | 392 |
| MB_PG03 | 384 | 28 | 394 |

All four book/page models have nine uncompressed frames. The settled main menu uses frame zero. The in-game entry context moves the cover from the last frame to zero; the page models remain at zero. In that context the camera begins at `(408, 0, 0)`, moves to `(172, 204, -152)` with a pitch change of -116 angle units, holds that position, then moves to the origin with a +116 pitch change. A revolution is 2048 angle units. The startup entry context keeps frame zero and moves the camera from `(-4, -100, 240)` to the origin instead.

The Glide viewport has horizontal half-field tangent `0.788`. At 640 by 480, screen coordinates for camera-space `(x, y, z)` are `(320 + 320*x/(0.788*z), 240 + 320*y/(0.788*z))`. Texture UVs follow the model format's cumulative 1/16-texel coordinates. `REDGUARD.COL` is the initial palette; game/save context can replace the image on the main page with a gameplay snapshot. That snapshot is runtime content, not static book artwork.

## Text

At width 640 the menu uses `FONTS/FONTNORM.FNT` for normal and grayed text, and `FONTS/FONTSEL.FNT` for the current selection. At other widths it can select the smaller `FONTNORS.FNT` and `FONTSELS.FNT` glyphs. `hi_res_menu` affects that font-switch path. The `HINORMAL`/`HISELECT` files are not these book-menu fonts.

Each enabled glyph advances by its width plus one texel. Space also adds the width of the next glyph. Center/right alignment subtracts half/all of that accumulated width before painting; glyph left/top offsets still apply. Zero pixels are transparent. In the normal font, the normal text blitter adds five to each nonzero palette index. For each nonzero glyph pixel, grayed text adds four to the existing destination page index, preserving the paper shading. Selected text copies the selected-font indices. Nonselectable text uses the low four bits plus 144. These styles come from the menu font brightness/blitter settings, not the FNT palette alone.

## Selection and actions

Entries form a circular ordered list. Up selects the previous eligible entry, Down selects the next. Eligibility requires selectable, not grayed, and a nonempty label. The first configured default establishes selection. Disabling the selected entry moves it to the next entry. Startup and in-game contexts apply additional Save/Continue restrictions; a menu with no resumable game cannot activate Continue. Enter or the configured primary action activates the selection. The engine's held-direction repeat delay comes from `SYSTEM.INI` `[dialog] menu_traverse_delay`, in BIOS timer ticks.

Action `-2` begins a closing-book/camera sequence and exits the game. It does not ask for confirmation. Action `-1` resumes; positive IDs dispatch the configured submenu or game function. Those functions must be implemented separately from rendering their labels.

## Evidence and limits

Verified against the installed MENU.INI, all six MENU.ROB segments, TEXBSI banks 288/289, the FNT files, and the menu setup, text, traversal, action, camera, and viewport routines. The model frame arrays have full signed-triplet spacing in every book frame. Independent parser probes verified the four nine-frame models and their bounded open/closed geometry.

Lighting inherits engine/environment state. This document does not establish a context-independent light rig or a replacement for the runtime gameplay snapshot. Mouse menu selection and menu audio remain outside this investigation.
