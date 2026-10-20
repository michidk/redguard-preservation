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

All four book/page models have nine uncompressed frames. The settled main menu uses frame zero. Entry context `1` moves the cover from its last authored frame to zero, with a one-update turn delay; the page models remain at zero. Its camera begins at `(408, 0, 0)`, moves to `(172, 204, -152)` with a pitch change of -116 angle units, holds that position, then moves to the origin with a +116 pitch change. All three requests use fractional step `0.15`. A revolution is 2048 angle units. Other main-page entry contexts, including death context `3`, keep frame zero and move the camera from `(-4, -100, 240)` to the origin with fractional step `0.15`. Contexts `5` through `9` instead enter Save, Load, Display, Sound and Controls respectively through their page handlers.

The Glide viewport has horizontal half-field tangent `0.788`. At 640 by 480, screen coordinates for camera-space `(x, y, z)` are `(320 + 320*x/(0.788*z), 240 + 320*y/(0.788*z))`. Texture UVs follow the model format's cumulative 1/16-texel coordinates. `REDGUARD.COL` is the initial palette; game/save context can replace the image on the main page with a gameplay snapshot. That snapshot is runtime content, not static book artwork.

Book surfaces are single-sided. With positive screen Y down, visible triangles have negative signed area; the opposite winding is culled. Each main, Movies, Options, and Controls page surface has sixteen projected triangles. Turning the page reverses their winding and exposes its other surface, rather than a mirrored copy of the previous page.

## Text

At width 640 the menu uses `FONTS/FONTNORM.FNT` for normal and grayed text, and `FONTS/FONTSEL.FNT` for the current selection. At other widths it can select the smaller `FONTNORS.FNT` and `FONTSELS.FNT` glyphs. `hi_res_menu` affects that font-switch path. The `HINORMAL`/`HISELECT` files are not these book-menu fonts.

Each enabled glyph advances by its width plus one texel. Space also adds the width of the next glyph. Center/right alignment subtracts half/all of that accumulated width before painting; glyph left/top offsets still apply. Zero pixels are transparent. In the normal font, the normal text blitter adds five to each nonzero palette index. For each nonzero glyph pixel, grayed text adds four to the existing destination page index, preserving the paper shading. Selected text copies the selected-font indices. Nonselectable text uses the low four bits plus 144. These styles come from the menu font brightness/blitter settings, not the FNT palette alone.

## Selection and actions

Entries form a circular ordered list. Up selects the previous eligible entry, Down selects the next. Eligibility requires selectable, not grayed, and a nonempty label. The first configured default establishes selection. Disabling the selected entry moves it to the next entry. Startup and in-game contexts apply additional Save/Continue restrictions; a menu with no resumable game cannot activate Continue. Enter or the configured primary action activates the selection. The engine's held-direction repeat delay comes from `SYSTEM.INI` `[dialog] menu_traverse_delay`, in BIOS timer ticks.

Action `-2` begins a closing-book/camera sequence and exits the game. It does not ask for confirmation. Action `-1` resumes; positive IDs dispatch the configured submenu or game function. Those functions must be implemented separately from rendering their labels.

Action 10 starts New Game. Before loading world zero at marker zero, the engine
calls `LockoutPlayer(0)`, clearing the retained lockout and applying its
[actor and HUD cleanup](hud.md#shared-rules). The returned New Game transition resets game state, plays movie zero
(the Introduction), then loads world zero at marker zero. Movie playback
finishes or follows its configured skip points before map loading begins.

`WonGame` ends the world loop with a won result (see
[session requests](SOUP.md#session-requests-loadworld-wongame-pushplayer-and-popplayer)).
The engine then runs the ending instead of showing a menu page: sound effects
stop, the CD pauses, movie 4 plays under the [movie rules](movies.md) (`WIN.SMK`
in the installed MENU.INI, with its configured skip point), and the credits
scroll plays. The credits stop the CD when they end or when Escape is pressed.
When `CREDITS.TXT` cannot be loaded the credits are skipped and the paused
world track resumes. F10 in a world plays the same credits scroll without the
movie.

The menu inherits the current engine lighting and environment state. The default
book menu has no pointer hover or click selection. Pointer coordinates do not
determine the selected entry. Mouse buttons and motion have input codes in
KEYS.INI, but the shipped menu bindings use keyboard and joystick logical
actions. Controls key capture accepts the left and right mouse button codes on
release.

The main page disables both Save and Continue while the selected player's
[death override](animation.md#dead-actor-default-selection) is active. This
restriction uses the retained death override, not a direct health comparison.
The installed main-page entries are addressed by ordered index: Save is entry
zero and Continue is entry five. Disabling an entry sets its grayed flag and
moves selection away from it when necessary.

The startup entry context (`1`) also disables Save. Entry context `2` disables
Continue. These context checks run independently of the selected-player death
restriction.

Escape is processed after the key is released. On the main page, the selected
player's death override also prevents Escape from resuming gameplay. This
restriction applies to leaving the main page; Escape can still return from a
submenu to its parent while the death override is active.

## Gameplay book shortcuts

With `SYSTEM.INI` `[debug] final_version` enabled, gameplay samples fixed book
shortcuts in this order:

| Key | Destination page |
| --- | --- |
| F2 | Save, page 1 |
| F3 | Load, page 2 |
| F4 | Display, page 5 |
| F5 | Sound, page 6 |
| F6 | Controls, page 7 |

Escape takes priority and requests the main page. Otherwise, the shortcut
sampler chooses the first held key in the table and clears that key's state.
It returns no request when none is held. The retained request is consumed near
the start of the following gameplay update. Escape enters the ordinary in-game
book context; the five shortcuts enter their destination pages directly.

## Save and load pages

Existing saves are ordered by the filesystem modification time of `SAVEGAME.SAV`, newest first. Equal timestamps retain ascending save-directory number order.

Main action 1 opens Save on page 1. Its camera moves directly to `(336, 0, 0)` with fractional step `0.15`, without turning a page. Action 100 returns directly to the origin. Main action 2 opens Load on page 2 using the three-part camera sweep to the origin and a forward MB_PG01 turn delayed by four updates. Action 200 reverses that turn and repeats the sweep to the origin.

Save displays `<NEW SAVE GAME>` as its first selectable row on the first list page, followed by eight existing saves. Subsequent list pages contain nine existing saves. Load displays nine existing saves per list page. Rows are addressed by their ordered entry index and list-page offset, not by action alone: existing Save rows share action 102, and Load rows share action 202. Missing saves produce empty, disabled rows. Slot descriptions are copied into menu labels with a limit of forty characters.

Action 101 asks for `Game Description` before creating a save. Action 102 opens the same input dialog with the existing description before overwriting that save. The input accepts at most 64 characters, requires nonempty text to confirm, and permits Escape to cancel. New manual saves use the first free directory number from 2 through 299. The Save list excludes the slot numbered zero; Load may include it. Delete on an existing save opens a `DELETE GAME?` confirmation that includes its description.

Newly typed ASCII lowercase letters become uppercase; prefilled text retains its case. Without Shift, Backspace deletes the preceding character and Enter confirms nonempty text. With Shift, Backspace and Enter append character codes 13 and 11 respectively, without deleting or confirming.

The description display wraps a copy of the edited text. It scans for spaces starting at zero-based character 13, excluding the final character. Each encountered space becomes a newline, and scanning resumes fourteen characters after that replacement. This preparation uses character positions without measuring font width; the edited value retains its spaces.

The delete confirmation presents `Yes` first and `No` second, with `Yes` initially selected. Enter confirms the current selection before simultaneous directional selection is applied.

The last-game confirmation presents `Yes` first and `No` second, initially
selecting `Yes`. The configured `KEY_UP` action selects `Yes`; `KEY_DOWN`
selects `No`. If both directions are active, Up takes priority. Enter or the
configured `KEY_D` action confirms the current selection before a simultaneous
direction can change it. The dialog waits until both confirmation inputs are
released before returning. Escape has no separate cancellation path.

If the retained book entry context is three, the confirmation's first display
waits 72 BIOS timer ticks, then drains held `KEY_D` and Enter input before
accepting a choice. Later redraws in that dialog do not repeat this delay.
Other retained contexts skip this entry delay.

Centered dialog text starts at `x + trunc((available_width - text_width) / 2)`, where `trunc` rounds toward zero. A zero available width centers the measured text around `x`.

The reserved quick-save slot is zero, with description `Quick Save Game`.
The reserved automatic-save slot is one, with description `Redguard AutoSave`.
When Quit closes the menu outside the startup context, the menu prepares this
automatic save. With the selected-player death override active, preparation
removes the reserved slot's `SAVEGAME.SAV` instead of writing it. Otherwise it
calls the save writer only when the retained gameplay-thumbnail buffer exists.
This path runs before the menu returns to its caller. Startup Quit skips it.
Quick Save reads scan code 67, F9, directly rather than an action binding. It
clears that key state when latching a request near the end of the world loop.
With `[debug] final_version` enabled, the following loop consumes the pending
request before ordinary world processing, calls Quick Save, then clears the
request. Menu requests and the Quick Save request are latched independently, so
both can be pending together. The next loop handles the menu request first. If
the menu returns without ending the world loop, Quick Save is consumed afterward.

Before preparing a slot, the save writer requires at least 10 MiB of available
disk space. Below that threshold it displays the failure message and returns
without preparing the slot or writing `SAVEGAME.SAV`. Equality passes the check.

### Retained last-save selection

The retained last-save selection starts absent. Admitting the startup autosave
selects slot one and copies its `Redguard AutoSave` description. Admission requires
final-version mode, `ignore_autosave` equal to zero, an existing file and an accepted
save header. Rejecting admission leaves the previous last-save selection and its
description unchanged.

After the disk-space check passes, the save wrapper prepares the destination path,
removes the previous save file, then retains the requested slot and a copy of its
description before calling the save writer. The selection does not depend on the
writer's return value. The insufficient-space path leaves the previous selection
unchanged.

Selecting an existing Load row retains its slot and a copy of its description,
then publishes the pending load selection before the menu's exit presentation.
The selected list index is the row's ordered index minus one plus the current
list-page offset. A missing list record leaves both retained and pending selections
unchanged. The retained selection supplies the
[death completion prompt](SOUP.md#death-completion-dispatch); consuming a pending
load clears that pending selection independently.

The main page and the second surface of each save/load page receive runtime gameplay snapshots. The image begins at texel row 33, spans the 256-texel surface width and is 192 rows high. Save's new-save row shows the current gameplay snapshot; existing rows show their stored snapshot. Load previews the selected save, and selecting it also replaces the main-page snapshot before leaving the menu.

## Options and settings pages

Main action 4 opens page 4. Actions 405, 406, and 407 open Display, Sound, and Controls on pages 5, 6, and 7. Actions 500, 600, and 700 return to Options; 400 returns to Main. Escape returns from a settings page to Options and from Options or Movies to Main. Each page retains its selection while the menu stays open.

The Glide executable omits actions 501, 502, and 503 when constructing Display entries. These are Screen Size, Resolution, and Gamma. Subtitles, action 504, establishes the Display selection. Pages 4, 5, and 6 each define only texture slot zero.

Options turns MB_PG01 and MB_PG02 to their last frames. MB_PG01 moves four units farther along Z when passing its middle frame. The settled camera is at the origin. Display uses camera `(336, -128, 0)` and Sound uses `(336, 176, 0)`, retaining those page frames. Controls additionally turns MB_PG03 to its last frame, moving it four units nearer along Z; the camera returns to the origin. Returning reverses those changes. Camera pitch changes during the transition cancel at the settled endpoint.

Left and Right subtract or add one within the installed slider bounds without wrapping. Enter or the primary activation key toggles actions 504, 505, and 603. These preferences map to SYSTEM.INI `[dialog] dialog_print_text`, `[screen] smk_interlace`, and `[dialog] dialog_use_speech`. Sound and Music actions 601 and 602 initialize from `[system] volume` and `redbook_volume`, divided by 12.8 and truncated toward zero. Updates multiply the menu value by 12.8 and truncate to the engine volume. The shipped configuration gives Sound a range from zero to twenty and these engine volumes:

| Slider | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15 | 16 | 17 | 18 | 19 | 20 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Volume | 0 | 12 | 25 | 38 | 51 | 64 | 76 | 89 | 102 | 115 | 128 | 140 | 153 | 166 | 179 | 192 | 204 | 217 | 230 | 243 | 256 |

The shipped `volume=255` opens at 19, not 20, and `redbook_volume=200` at 15. Because both directions truncate, most values reopen one notch lower (slider 1 writes 12, which reads back as 0). The Sound volume becomes the master volume of [positional sound](sound.md#starting-a-sound); the Music value is described in [music](music.md).

Checkboxes use MM_CHECK.GXA frames indexed by the toggle value. Sound sliders use MM_SLIDE.GXA, selecting the rounded product of the value and `(frame_count - 1) / (slider_max - slider_min)`, clamped at the endpoints. Widgets paint at output coordinates; their labels retain normal text without a numeric suffix.

Controls actions 710 through 717 map to up, down, left, right, A, B, C, D. Actions 718 through 724 map to inventory, sword, health, map, log, next, previous. Output names come from KEYS.INI `[defined] key[code]`. While capturing, the book alternates `<prompt>` and `>prompt<` in the selection font. Its prompt buffer initializes to `WAITING` and uses KEYS.INI `[misc] waiting_message` when configured; the shipped configuration displays `HIT KEY`. Key names and the configured prompt are uppercased and limited to fifteen characters before display. Escape cancels capture. Tab, Enter, F1 through F10, F11 and F12 are rejected during capture; Minus and Equal are also rejected by binding validation. Rebinding a used code clears the first conflicting binding. Zero displays the installed name for an empty binding. A binding name is drawn glyph by glyph from the value column at x=120 of the 256-pixel page surface and stops at the first glyph whose pen position is 247 or more, so `NUM MINUS`, `LEFT MOUSE` and `RIGHT MOUSE` display as `NUM MINU`, `LEFT MOUS` and `RIGHT MOU`. A glyph that starts at pen 246 is drawn whole, and narrower names such as fifteen `i` characters fit entirely. Mouse/joystick input shares the original code table but requires separate device handling.

Movies action 3 turns MB_PG01 to its last frame with the same four-unit depth change, and uses camera `(336, 0, 0)`. Page 3 action 300 returns to Main; actions 301 through 305 select movie definitions zero through four. MM_MOVIE.GXA supplies the corresponding preview images. Runtime unlock flags gate movie entries. MENU.INI `[general] force_movies=1` initializes every movie as unlocked. Playing a cinematic sets its unlock flag. The startup game path plays movie zero, so Introduction is visible at the first settled menu. MM_MOVIE.GXA preview data begins at row 33 of the second Movies page texture. The preview copies every indexed pixel into the page, including zero; its black pixels are opaque, unlike transparent widget pixels.

Configuration lookup matches section/key names without case and returns the first matching occurrence. Settings outside the requested section do not override that section.

## Frame and camera transitions

Page turns use the authored frame arrays, not an independent hinge rotation. A turn request supplies an initial frame, a target frame, and an update delay. The delay counts down first; at zero it selects the initial frame. Later menu updates move one integer frame toward the target. There is no geometric interpolation between those authored frames in this path. Forward turns and the Quit cover turn target the model's frame count, one past its last authored frame. That final forward step adds an update but does not change the displayed pose. A later reverse turn starts from that index, so its frame stepping takes one update more than the number of authored frames minus one. Placing a page directly on a submenu, without a turn, sets the last authored frame instead.

MB_PG01 changes Z by +4 when a forward turn reaches frame four, and by -4 when a reverse turn reaches frame four. MB_PG03 applies the opposite depth changes. These shifts happen at the middle-frame crossing, not at the final frame. Returning to Main reverses the appropriate page turns. Cover motion and delayed page-turn starts dispatch the effects listed below.

The startup camera move to the origin uses a fractional step of `0.15`.

Camera requests form an ordered queue. Each request specifies a target position, angle changes, and a fractional step. Position interpolates linearly from the previous settled position; angles interpolate from their previous values by the supplied changes. The first rendered sample uses the request's fractional step, rather than zero. Each update samples the current fraction, then adds that same step. If the incremented value is greater than one, that update replaces the sample with the settled target and advances to the next request. Camera coordinates and angle samples truncate toward zero to engine integers before rendering.

The submenu requests use these camera paths. Each row's targets execute in order. Angle values are changes in the first camera angle, in 2048 units per revolution; the other two angles do not change.

| Navigation | Ordered camera targets | Angle changes | Fractional steps |
|---|---|---|---|
| Main to Movies | `(192, 284, -96)`, `(192, 284, -96)`, `(336, 0, 0)` | `-152`, `0`, `+152` | `0.15`, `0.12`, `0.15` |
| Main to Options, Movies or Options to Main, Options to Controls, Controls to Options | `(192, 284, -96)`, `(192, 284, -96)`, `(0, 0, 0)` | `-152`, `0`, `+152` | `0.15`, `0.12`, `0.15` |
| Options to Display | `(336, -128, 0)` | `0` | `0.15` |
| Options to Sound | `(336, 176, 0)` | `0` | `0.15` |
| Display or Sound to Options | `(0, 0, 0)` | `0` | `0.15` |
| Quit | `(232, 284, -96)`, `(232, 284, -96)`, `(408, 0, 0)` | `-152`, `0`, `+152` | `0.15`, `0.12`, `0.15` |

The repeated target holds the position while the request fraction advances. Main to Movies and its return delay the first page turn by four menu updates. Main to Options delays MB_PG01 by three and MB_PG02 by five; its return delays them by five and three respectively. Options to Controls and back delay MB_PG03 by four. Quit delays the cover turn by two. Outside the startup context, Quit then fades the closed book to black before the menu returns. Each fade update with a nonzero [whole nominal-tick increment](player.md#whole-nominal-ticks) redraws the book, clamping each page frame to its last authored frame, and raises a black overlay by 12 times that increment, out of 256, until the overlay is opaque. Startup-context Quit returns without this fade. Normal menu direction, activation, and Escape handling wait until the camera queue and page turns finish, including the update that settles their final targets.

For these paths, only the first camera angle changes. Subtract the camera position from a world point to obtain `(x, y, z)`, then apply the view rotation before viewport projection:

```text
angle = (first_angle & 2047) * 6.2831852 / 2048
view_x = x
view_y = y * cos(angle) - z * sin(angle)
view_z = y * sin(angle) + z * cos(angle)
```

The sine and cosine samples use the shared [rotation table](water.md#wave-lookup-table), with cosine starting one quarter revolution after sine. The coordinates keep positive Y down. A negative first angle rotates positive-depth points downward on the screen.

## Sound cues

Book motion dispatches these zero-based `MAIN.SFX` effect indices:

| Model | Event | Effect |
|---|---|---:|
| MENUA001 | Forward turn reaches frame 7 | 65 |
| MENUA001 | Reverse turn reaches frame 0 | 64 |
| MB_PG01 | Nonzero turn delay counts down to zero and selects the initial frame | 66 |
| MB_PG02 | Nonzero turn delay counts down to zero and selects the initial frame | 67 |
| MB_PG03 | Nonzero turn delay counts down to zero and selects the initial frame | 68 |

Page cues occur in either direction, once at delayed start, before later frame stepping. A turn without a nonzero delay does not dispatch that page cue. MB_TABLE has no turn cue. The short main-page camera entry, including death context `3`, leaves the cover at frame zero and does not dispatch a cover cue. Context `1` requests the delayed cover turn. Display and Sound camera moves do not turn a page.

These are [FlatSound](sound.md#soup-functions) requests: centered pan 64, zero volume offset, zero rate offset, and no owner or positional attenuation. Playback uses the effect header's base volume and sample rate with the saved master volume.

Successful directional selection or widget adjustment plays effect 13. Accepted control rebinding also plays effect 13; rejected capture plays effect 18.
