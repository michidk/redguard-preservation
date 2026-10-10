# Book menu

The menu renders textured models from `fxart/MENU.ROB` and paints configured text directly into their TEXBSI surfaces.

## Data and scene

[`MENU.INI`](../config/menu-ini.md) supplies ordered labels, texel positions, alignment, selectable/default/grayed flags, texture references, and action IDs. Labels do not identify actions. `import::menu_ini::MenuPage::parse` exposes a requested page also exposes widget output positions/alignment and inclusive slider bounds.

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

## Options and settings pages

Main action 4 opens page 4. Actions 405, 406, and 407 open Display, Sound, and Controls on pages 5, 6, and 7. Actions 500, 600, and 700 return to Options; 400 returns to Main. Escape returns from a settings page to Options and from Options or Movies to Main. Each page retains its selection while the menu stays open.

The Glide executable omits actions 501, 502, and 503 when constructing Display entries. These are Screen Size, Resolution, and Gamma. Subtitles, action 504, establishes the Display selection. Pages 4, 5, and 6 each define only texture slot zero.

Options turns MB_PG01 and MB_PG02 to their last frames. MB_PG01 moves four units farther along Z when passing its middle frame. The settled camera is at the origin. Display uses camera `(336, -128, 0)` and Sound uses `(336, 176, 0)`, retaining those page frames. Controls additionally turns MB_PG03 to its last frame, moving it four units nearer along Z; the camera returns to the origin. Returning reverses those changes. Camera pitch changes during the transition cancel at the settled endpoint.

Left and Right subtract or add one within the installed slider bounds without wrapping. Enter or the primary activation key toggles actions 504, 505, and 603. These preferences map to SYSTEM.INI `[dialog] dialog_print_text`, `[screen] smk_interlace`, and `[dialog] dialog_use_speech`. Sound and Music actions 601 and 602 initialize from `[system] volume` and `redbook_volume`, divided by 12.8 and rounded. Updates multiply the menu value by 12.8 and round to the engine volume. Sound controls range from zero to twenty in the examined install.

Checkboxes use MM_CHECK.GXA frames indexed by the toggle value. Sound sliders use MM_SLIDER.GXA, selecting the rounded product of the value and `(frame_count - 1) / (slider_max - slider_min)`, clamped at the endpoints. Widgets paint at output coordinates; their labels retain normal text without a numeric suffix.

Controls actions 710 through 717 map to up, down, left, right, A, B, C, D. Actions 718 through 724 map to inventory, sword, health, map, log, next, previous. Output names come from KEYS.INI `[defined] key[code]`. While capturing, the book alternates `<prompt>` and `>prompt<` in the selection font. Its prompt buffer initializes to `WAITING` and uses KEYS.INI `[misc] waiting_message` when configured; the examined install displays `HIT KEY`. Key-name output appears in uppercase. Escape cancels capture. Tab, Enter, F1 through F10, F11 and F12 are rejected during capture; Minus and Equal are also rejected by binding validation. Rebinding a used code clears the first conflicting binding. Zero displays the installed name for an empty binding. Mouse/joystick input shares the original code table but requires separate device handling.

Movies action 3 turns MB_PG01 to its last frame with the same four-unit depth change, and uses camera `(336, 0, 0)`. Page 3 action 300 returns to Main; actions 301 through 305 select movie definitions zero through four. MM_MOVIE.GXA supplies the corresponding preview images. The original gates movie entries with runtime unlock flags; the installed text alone does not establish unlock state. MENU.INI `[general] force_movies=1` initializes every movie as unlocked. Playing a cinematic sets its unlock flag. The startup game path plays movie zero, so Introduction is visible at the first settled menu. MM_MOVIE.GXA preview data begins at row 33 of the second Movies page texture.

Configuration lookup matches section/key names without case and returns the first matching occurrence. Settings outside the requested section do not override that section.

These settings and submenu facts were verified from MENU.INI, KEYS.INI, SYSTEM.INI, the three MM_ GXA assets, and the Glide menu initialization, painting, dispatch, traversal, binding-capture, binding-validation, and camera/frame update routines. Engine volume scaling was checked against the stored floating-point constants.
