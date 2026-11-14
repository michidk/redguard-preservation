# SYSTEM.INI

Primary engine configuration file controlling rendering, gameplay, camera, dialog, debug, and 3D subsystem parameters.

Shipped sample path: `/Redguard/SYSTEM.INI` (e.g. `.../GOG Galaxy/Redguard/Redguard/SYSTEM.INI`).

Settings use `key=value` records grouped by section. Semicolons introduce
comments. Switches use `0`/`1`, `no`/`yes`, `off`/`on`, or `false`/`true`. Most numeric fields
contain decimal integers, including negative position offsets and thresholds.
Camera glide factors and `cam_prox_up` contain decimal floating-point values.
The `[3dfx]` scale and font-color settings contain comma-separated integer pairs.

The integer configuration reader maps `true`, `yes`, and `on` to 1, and
`false`, `no`, and `off` to 0. Missing or empty values produce 0. Other values
use signed decimal conversion; numeric values are not normalized to booleans.
In particular, `[debug] disable_family` clears actor family links only when its
value is exactly 1. See [actor initialization](../engine/SOUP.md#initial-attribute-copy-and-defaults).

The descriptions below describe the Glide executable, `RGFX.EXE`. Several keys
in the shipped file are never read by it, and several are read but have no
effect. Those keys are marked **Not read** and **No effect**. A few keys the
engine reads are absent from the shipped file; they are listed at the end of
their section with the behavior of the missing value.

Map, world, and model distances are in engine world units. Angles use 2048
units per full turn. "Q8" values are world units multiplied by 256. "Ticks" are
nominal game ticks; see [frame scale](../engine/player.md#fps-derived-frame-scale).

## File Structure

Standard Windows-style INI file with 10 sections:

1. `[screen]` — display and movie settings
2. `[system]` — core engine paths, audio, physics, object pools, and HUD layout
3. `[debug]` — developer diagnostics and logging flags
4. `[game]` — falls, slides, ropes, and interaction distances
5. `[cyrus]` — player turning and input parameters
6. `[3dmanager]` — 3D object cache and memory budget
7. `[xngine]` — clipping planes, ambient light, and sky switch
8. `[camera]` — camera rig offsets, distances, and glide factors for each mode
9. `[dialog]` — dialogue menu layout, subtitles, and speech settings
10. `[3dfx]` — Glide resolution, texture reduction, and text colors

---

## `[screen]`

| Key | Default Value | Description |
|---|---|---|
| `candle_mode` | `2` | Candle HUD visibility: `0` = only while the show timer runs, `1` = always, `2` = always with the health number. Any other nonzero value behaves as `1`. F8 cycles 0→1→2 and writes the value back. See [candle](../engine/hud.md#visibility). |
| `colour_bits` | `8` | **Not read.** |
| `resolution` | `1` | Software-renderer mode index (`0`/`3`/`4` = 320×200, `1` = 640×480, `2` = 320×400; out-of-range values become `0`). It does not change the display in the Glide executable, whose display mode comes from [`[3dfx] resolution`](#3dfx). |
| `Palette_red` | `0` | **No effect.** Palette brightness offset of the software renderer's Gamma option; the Glide executable never creates that option. |
| `Palette_green` | `0` | **Not read.** Written only by the software renderer's Gamma option. |
| `Palette_blue` | `0` | **Not read.** Written only by the software renderer's Gamma option. |
| `smk_interlace` | `0` | Movie interlace effect: nonzero draws a black line over every odd output scanline of each movie frame. `0` = off. The menu's interlace option toggles it. See [movies](../engine/movies.md). |

Keys read but absent from the shipped file: `use_uvblib`, `linear_copy_mode`,
and `linear_interlace`. All three have **no effect**.

---

## `[system]`

`world_ini` and `item_ini` point to `WORLD.INI` and `ITEM.INI` respectively, which the engine loads for world and item definitions.

### Files and fonts

| Key | Default Value | Description |
|---|---|---|
| `game_bitmap` | `system\powerup.gxa` | GXA for the two HUD [power-up slots](../engine/hud.md#power-up-slots) (frame 0 gauntlet, frame 1 shield). The path is stored in save games and restored when one is loaded. |
| `pointers` | `system\pointers.bmp` | Cursor [GXA bitmap archive](../formats/GXA.md), stored with a BMP extension. A pixel of color index 16 marks each cursor's hotspot. Loaded only by a developer mode enabled from the command line. |
| `system_font` | `fonts\redguard.fnt` | **Not read.** |
| `icon_font` | `fonts\arialvs.fnt` | **Not read.** |
| `gui_font` | `fonts\arialbg.fnt` | **Not read.** |
| `gui_low_font` | `fonts\arialvb.fnt` | Font loaded at startup as font-table entries 1 and 2 (see [reader style table](../engine/hud.md#reader-style-table)). A missing file stops startup. |
| `animation_drive` | `D:\` | CD drive path. Movie filenames are prefixed with it (a `\` is added when missing), and its first letter selects the drive opened for [CD music](../engine/music.md). Without it, movies load from the bare filename and no CD music plays. |
| `back_texture` | `0` | **Not read.** |
| `rtx_filename` | `ENGLISH.RTX` | [RTX](../formats/RTX.md) dialogue file (voice and text). Without it, the first dialogue line stops the game with an error. |
| `world_ini` | `WORLD.INI` | INI file holding the `[world]` section ([WORLD.INI](world-ini.md)). Absent means `SYSTEM.INI`. |
| `item_ini` | `ITEM.INI` | INI file holding the `[ITEMS]` section ([ITEM.INI](item-ini.md)). Absent means `SYSTEM.INI`. |

### Sound and music

| Key | Default Value | Description |
|---|---|---|
| `volume` | `255` | Master sound-effect volume: every sample volume is scaled by `volume / 256`. Missing means silence. The menu Sound slider sets it. |
| `sound` | `1` | Enables sound. `0` skips sound startup entirely: no effects, voices, or CD music, and subtitles are forced on with speech off. The `nosound` command-line option also disables it. |
| `fast_sound` | `1` | `0` sets three extra Miles Sound System driver preferences before the driver opens (interrupt lock on, 50 ms latency, 250 Hz service rate). Nonzero leaves the Miles defaults. Output is 22050 Hz 16-bit stereo either way. |
| `fidelity` | `0` | Load-time quality reduction for sound effects and voice clips. See [Sound fidelity](#sound-fidelity). |
| `redbook` | `on` | Enables [CD music](../engine/music.md). Requires `sound` on and `animation_drive` naming the CD drive. |
| `redbook_volume` | `200` | CD music volume, clamped to 0–255 and halved to the Miles 0–127 range; the shipped `200` plays at 100 of 127. Applied when each CD track starts. The menu Music slider sets it. See [music volume](../engine/music.md#volume). |
| `sound_distance` | `64` | Sound radius: `sound_distance × 36` world units (shipped 2304); see [positional sound](../engine/sound.md#radius-and-divisor). Values of 3 or less make the divisor zero. |

#### Sound fidelity

`fidelity` converts every sample as it loads, both the `MAIN.SFX` effect bank
and [RTX](../formats/RTX.md) voice clips. Conversion keeps every other sample
frame to halve the rate and keeps the high byte to turn 16-bit samples into
8-bit ones. No filtering is applied.

| Value | 16-bit stereo | 16-bit mono | 8-bit stereo | 8-bit mono |
|---|---|---|---|---|
| `0` | unchanged | unchanged | unchanged | unchanged |
| `1` | half rate | half rate | unchanged | unchanged |
| `2` | 8-bit stereo | 8-bit mono | unchanged | unchanged |
| `3` | 8-bit stereo, half rate | 8-bit mono, half rate | half rate | half rate |
| `4` | 8-bit mono (left channel), half rate | 8-bit mono, half rate | mono (left channel), half rate | half rate |

Other values leave samples unchanged. Halving the rate turns a 22050 Hz sample
into 11025 Hz and an 11025 Hz sample into 5512 Hz.

### Frame rate and movement physics

| Key | Default Value | Description |
|---|---|---|
| `normal_frame_rate` | `12` | Nominal game tick rate. The per-frame scale is `normal_frame_rate × 256 / fps`, so game time advances `normal_frame_rate` ticks per second regardless of frame rate. `0` means 12. See [frame scale](../engine/player.md#fps-derived-frame-scale). |
| `use_smooth_fps` | `yes` | When on, the frame scale uses a smoothed FPS value instead of the raw measured FPS. Each update moves the smoothed value one step toward the measured value, or half the difference for gaps of 5 or more. |
| `use_smooth_divisor` | `yes` | When on, the frame scale steps toward its new target over several frames instead of jumping to it. |
| `min_frame_rate` | `6` | Lowest FPS used in the frame-scale division. Below this rate game time slows down rather than taking larger steps; the shipped value caps each frame at two nominal ticks. |
| `max_frame_rate` | `300` | Highest FPS used in the frame-scale division. It does not cap rendering. `0` means 36. |
| `jump_time` | `6` | Default for actor attribute `attr_jump_time` when the actor's value is zero or negative (low byte used). Values below 1 mean 6. |
| `jump_height` | `80` | Default for actor attribute `attr_jump_height` when the actor's value is zero. The engine stores bits 8–15 of this value, so the shipped `80` (and the fallback 80 for values below 1) leaves the attribute at zero. |
| `post_collide_height` | `50` | Vertical probe height in map units for actor collision: forward-obstacle probes, step and support checks, body-shape sphere layout, and the falling-speed limit. See [player movement](../engine/player.md#forward-obstacle-wrapper-and-motion-transition). |
| `hpost_collide_height` | `50` | **No effect.** |
| `pre_validate` | `no` | When on, actors outside living states 27–32 skip the forward obstacle probes during movement sub-steps while the measured frame rate is at least 6. |
| `sphere_object_scale` | `256` | Collision-size scale in 1/256 units, applied at load to static-model X/Z bounds and actor collision-sphere radii (radius minimum 1). `256` leaves sizes unchanged; values below 1 mean 256. |
| `standing_height` | `-2` | Vertical offset in map units added when an actor is placed on ground support or a ledge. The shipped `-2` sinks actors two units into their support. |
| `slide_range` | `-60000` | Wall-slide threshold for the dot product of the normalized movement and the contact normal. Contacts above it slide along the wall; at or below it the actor stops. See [body-contact response](../engine/player.md#horizontal-body-contact-response). |
| `floating_point_physics` | `yes` | **No effect.** |

### Map loading and engine subsystems

| Key | Default Value | Description |
|---|---|---|
| `statics_load` | `yes` | When off, map loading skips `MPSO` static objects. |
| `flats_load` | `yes` | When off, map loading skips `MPSF` flat objects. |
| `objects_load` | `yes` | When off, map loading skips all `MPOB` records: actors, markers, and object local lights. |
| `lights_load` | `yes` | When off, map loading skips `MPSL` light records. |
| `ropes_load` | `yes` | When off, map loading builds no `MPRP` ropes. |
| `task_system` | `yes` | Initial value of the `task` cheat: on runs actor entry, task, and main script stages. See [cheats](../engine/cheats.md) and [SOUP update gates](../engine/SOUP.md#ordinary-update-gates). |
| `animation_system` | `yes` | Initial value of the `animation` cheat: on runs actor animation playback and animation-gated combat timing. See [cheats](../engine/cheats.md). |
| `start_fade` | `on` | Whether the first world entered at startup without the book menu (first run with no autosave) fades the loading screen out and the world in from black. `0` cuts directly. Later world loads always fade. |
| `max_active_objects` | `32` | Reusable actor-pool capacity, clamped to at least 10. |
| `max_effects` | `32` | Capacity of the [environmental effect pool](../engine/SOUP.md#environmental-effect-animation). Negative values become 0; 0 disables the pool. |
| `max_particles` | `512` | Capacity of the particle array used by `ScriptParticles`. `0` or negative disables particles. |
| `max_remap_objects` | `64` | Capacity of the list of moved actors waiting to be re-filed in the map grid (console `show remap`), clamped to at least 32. Exceeding it raises runtime error 196. |
| `disable_effects` | `no` | When on, the [environmental effect pool](../engine/SOUP.md#environmental-effect-animation) is not created. |

### Text and startup checks

| Key | Default Value | Description |
|---|---|---|
| `disable_text` | `yes` | Initial state of the engine's formatted-text suppression flag. While set, diagnostic text printers draw nothing. Menus, loading progress, and other screens clear the flag around their own text, and HUD prompts and subtitles ignore it. |
| `disable_debug_text` | `yes` | Initial state of queued debug-text suppression. The shipped `yes` hides queued debug text. |
| `normal_ink` | `1` | **No effect.** |
| `lock_windows` | `no` | **Not read.** |
| `disable_drive_check` | `yes` | When off, startup enumerates drive letters for the console `show machine` report. It does not gate startup. The `nodrive` command-line option forces it on. |
| `disable_cpu_check` | `yes` | When off, startup detects the CPU type for the `show machine` report. It does not gate startup. The `nocpu` option forces it on. |
| `disable_svga_check` | `yes` | When off, startup identifies the SVGA chipset for the `show machine` report; otherwise the report shows "SVGA TEST DISABLED". The `nosvga` option forces it on. |
| `report_machine` | `no` | When on, startup runs the console command `show machine`. The console opens for it only when `disable_debug_text` is off. |

### HUD layout

| Key | Default Value | Description |
|---|---|---|
| `compass_xco` | `546` | [Compass](../engine/hud.md#compass) top-left X. |
| `compass_yco` | `396` | [Compass](../engine/hud.md#compass) top-left Y. |
| `candle_xco` | `12` | [Candle](../engine/hud.md#candle) top-left X. |
| `candle_yco` | `8` | [Candle](../engine/hud.md#candle) top-left Y. |
| `logbook_xco` | `540` | [Logbook icon](../engine/hud.md#logbook-icon) top-left X. |
| `logbook_yco` | `20` | [Logbook icon](../engine/hud.md#logbook-icon) top-left Y. |
| `pickup_xco` | `12` | [Active item](../engine/hud.md#active-item) icon top-left X, and start X of the [prompt text](../engine/hud.md#prompt-text). |
| `pickup_yco` | `386` | [Active item](../engine/hud.md#active-item) icon top-left Y. |
| `pickup_text_yco` | `436` | Top Y of the HUD [prompt text](../engine/hud.md#prompt-text) line. |
| `game_xco1` | `576` | [Power-up slot](../engine/hud.md#power-up-slots) 1 X. |
| `game_yco1` | `6` | [Power-up slot](../engine/hud.md#power-up-slots) 1 Y. |
| `game_xco2` | `576` | [Power-up slot](../engine/hud.md#power-up-slots) 2 X. |
| `game_yco2` | `96` | [Power-up slot](../engine/hud.md#power-up-slots) 2 Y. |

Key read but absent from the shipped file: `archive_file_first`. `1` opens a
loose file in preference to an archived copy; `0` (the missing value) searches
`.ZAP` archives first and falls back to the loose file. It has no effect on an
install without `.ZAP` archives.

---

## `[debug]`

Developer diagnostics, logging, and display flags. Most are disabled in the shipped build.

| Key | Default Value | Description |
|---|---|---|
| `final_version` | `1` | Selects release behavior. `0` selects developer mode, which also enables the queued developer text used by other `[debug]` displays. See [Developer mode](#developer-mode). |
| `enable_logs` | `0` | Log output level. `0` writes no logs, `1` appends engine messages to `*.LOG` files in the game directory, `2`–`3` echo them to the console only, and `4`–`5` do both; `3` and `5` also open the console at startup. A fatal error switches logging to `1`. |
| `map_log` | `no` | **No effect.** |
| `console_error` | `yes` | `1` prints the fatal-error report in the in-game console before the game shuts down. Any other value leaves only the report printed after exit. |
| `software_interrupt` | `yes` | Enables the engine's debug-break checks during logging and input waits: a developer key combination triggers a debugger breakpoint, and a pending break request ends or interrupts the session. Required by `software_break`. |
| `software_break` | `yes` | `1` lets a break request from the keyboard handler exit the game during startup, before the first world starts. The engine clears it when gameplay begins. |
| `attempt_recover` | `no` | Nonzero makes fatal errors non-fatal: the engine shows the error report in the console and keeps running instead of exiting. Also forces `console_error` on. |
| `object_log` | `no` | `1` restarts `OBJECT.LOG` each time a world loop begins. Requires nonzero `enable_logs`. |
| `video_log` | `no` | **Not read.** |
| `node_marker` | `no` | Initial value of the `nodemarker` cheat slot, which has no effect. See [cheats](../engine/cheats.md). |
| `task_debug` | `no` | Nonzero turns invalid script operations (unknown opcodes, out-of-range function numbers, failed lookups) into fatal errors; `1` also range-checks property access by index. |
| `memory_manager` | `0` | Heap-tracking mode. `0` is off, `1` counts allocated bytes, and `2` or higher also records each block so that freeing an unknown pointer is a fatal error. Nonzero modes add memory statistics to error reports. |
| `monitor_object` | _(empty)_ | Name of an actor to monitor. The matching actor is selected for the console's per-frame animation-state readout and for fatal-error reports. |
| `display_masters` | `no` | **No effect.** |
| `display_slaves` | `no` | **No effect.** |
| `display_edges` | `no` | Nonzero prints the player's ledge-detection candidates and results as developer text. Visible only in developer mode. |
| `ignore_errors` | `yes` | **No effect.** The `ie` command-line option sets the same unused value. |
| `disable_family` | `no` | `1` clears every actor's master/slave attributes at creation. Any nonzero value also skips building the world's family tree at world load. |
| `disable_master_slaves` | `no` | Nonzero excludes actors that are both master and slave from object updates. |
| `disable_slaves` | `no` | Nonzero excludes slave actors that are not also masters from object updates. |
| `family_log` | `no` | `1` writes each world's actor family tree to `FAMILY.LOG` at world load. Requires nonzero `enable_logs` and `disable_family=0`. |
| `script_log` | `no` | **No effect.** The console `log` command writes `SCRIPT.LOG`. |
| `show_manager` | `no` | **No effect.** |
| `display_node_map` | `no` | Nonzero draws a marker sprite above every navigation node of the loaded node maps. |
| `display_nodes` | `no` | Nonzero draws marker sprites at each navigating actor's movement target and current navigation node. |
| `display_markers` | `no` | Nonzero draws a marker sprite at every enabled world marker (RGM `MPMK`). |
| `network_marker_file` | `g:\PROJECTS\REDGUARD\DEMO\NETWORK.MRK` | **No effect.** The shipped value is a leftover developer path. |
| `object_system_log` | `no` | **No effect.** |
| `convert_static_angles` | `no` | Nonzero forces `[3dmanager] dummy_manager` on. It performs no angle conversion. |
| `def_checksum` | `yes` | Nonzero makes world loading require the RGM's stored SOUP386.DEF checksum (the byte sum of the file) to match the installed [SOUP386.DEF](../engine/SOUPDEF.md); a mismatch is a fatal error. |
| `render_log` | `no` | Nonzero logs the draw list of one rendered frame, then switches itself off. Works regardless of `enable_logs`. |
| `idebug` | `0` | Nonzero sends log output to a remote network monitor session instead of log files. |
| `idebug_refresh` | `no` | With `idebug`, nonzero makes each log message wait for the remote monitor to acknowledge it. |
| `memory_monitor` | `no` | Nonzero shows an on-screen free-memory readout and a scrolling graph of heap usage changes. It also forces on-screen text on. |

Key read but absent from the shipped file: `camera_log`. Nonzero restarts
`CAMERA.LOG` when the camera system starts, if `enable_logs` is nonzero. No
camera messages are written to it.

---

## `[game]`

| Key | Default Value | Description |
|---|---|---|
| `fall_bounce_height` | `100` | Minimum fall distance in map units before a falling actor that overlaps an actor with nonzero `attr_bounce_power` tries to land on it. Shorter falls use the ordinary overlap response. |
| `fall_death_height` | `568` | Fall distance in map units above which landing is fatal (health change -256). Landings on surfaces 1 through 4 count as zero distance. |
| `fall_hurt_height` | `300` | Fall distance in map units above which landing hurts: animation group 73 and a health change of `-fall_hurt_zap`. |
| `fall_hurt_zap` | `10` | Health removed by a landing that exceeds `fall_hurt_height` but not `fall_death_height`. |
| `slide_hurt_height` | `1024` | Accumulated slide distance in map units above which ending a slide hurts the actor (knock-back, group 73, `-slide_hurt_zap` health). |
| `slide_hurt_zap` | `10` | Health removed when a slide longer than `slide_hurt_height` ends. |
| `rope_jump_add` | `18` | Base speed of a jump off a rope, in map units. A swinging rope adds its node speed on top. |
| `rope_attach_angle` | `512` | Largest heading difference at which the player can grab a directional rope; masked to 0–1023, and 0 means 256. |
| `slide_speed` | `36` | Slope-slide speed: each update moves `slide_speed / 2` map units at nominal frame scale. Values below 8 become 8. |
| `rtx_pickup_override_time` | `24` | Ticks for which holding the previous/next item key suppresses an [activation prompt](../engine/hud.md#activation-prompt) and shows the item icon. |
| `swim_depth` | `40` | Model Y offset in map units for animation groups 141 through 143 on the non-inhibited surface-2/3/4 [model-position path](../engine/SOUP.md#current-model-presentation). Zero is replaced with 40. |
| `player_dead_time` | `3` | **No effect.** |
| `player_fall_dead_time` | `12` | Falling-death timeout input. Clamped to at least 3, then multiplied by 12 nominal ticks; the [world loop](../engine/SOUP.md#world-loop-death-exit-ordering) exits only after the elapsed value exceeds that limit. |
| `old_combat` | `off` | **Not read.** |
| `lineup_distance` | `128` | Distance in map units at which `PlayerLineUp` switches from walking the player toward the target to aligning in place. |
| `dialog_radius` | `512` | Player-proximity radius in map units for walk-route navigation: while the exclusion switch is on, route nodes closer than this to the player are not used. |
| `combat_sphere_scale` | `200` | Radius scale in 1/256 units for the secondary (fighting) collision shape: each sphere radius becomes the larger of two opposite bounds extents times this value divided by 256. 0 means 230. |

---

## `[cyrus]`

| Key | Default Value | Description |
|---|---|---|
| `sheath_sword_delay` | `6` | **No effect.** Read at startup (`0` becomes 6) but not used. |
| `auto_defend` | `1` | **No effect** on combat. F7 toggles the value, shows the RTX line `AUTO-DEFEND MODE ON` or `AUTO-DEFEND MODE OFF` (`#don`/`#dof`), and writes it back to this file. |
| `walk_mode` | `0` | **No effect** on movement. Tab toggles the value, shows the RTX line `WALK MODE ON` or `WALK MODE OFF` (`#won`/`#wof`), and writes it back to this file. |
| `turn_speed` | `12` | Turn acceleration: added to the held-direction turn rate each input update while the rate is below `turn_max_speed`. |
| `turn_max_speed` | `48` | Turn rate limit in heading units per update at nominal frame scale. Acceleration stops once the rate reaches it. |
| `mouse_turn` | `0` | Nonzero enables mouse turning: horizontal mouse motion plays the turn animation groups (14/15, or 16/17 with a burning torch). Only the low byte is used. |
| `joy_tolerance` | `60` | Joystick dead zone: an axis registers as left/right/up/down only when its reading is more than this many units from the calibrated center. |
| `camera_distance` | `200` | **Not read.** The camera uses [`[camera] camera_distance`](#camera). |
| `tap_time` | `12` | Multi-tap window: a press counts as a repeated tap only when it follows a release within this many updates at nominal frame scale (minimum 6). 0 means 3. |
| `poly_push_units` | `16` | Minimum separation distance in map units used when pushing an actor out of face geometry. Values below 4 become 4. |
| `auto_grab` | `off` | `1`/`on` makes the forward-obstacle response try the ledge-grab path automatically; any other value disables it. |
| `smooth_post_min` | `2` | **No effect.** |
| `smooth_post_max` | `10` | **No effect.** |
| `smooth_post_divisor` | `2` | **No effect.** |

---

## `[3dmanager]`

| Key | Default Value | Description |
|---|---|---|
| `buffer_kbytes` | `800` | 3D manager workspace size in kilobytes as shown in the developer console. No allocation uses it. Values below 100 become 100. |
| `heap_kbytes` | `22000` | 3D object manager heap size in kilobytes. Values below 500 become 500. |
| `max_objects` | `255` | Number of 3D object slots in the object manager and its cache table. Values below 10 become 10. |
| `compress` | `1` | **Not read.** |
| `save_compressed` | `0` | **Not read.** |
| `cache_objects` | `yes` | When on, loaded 3D objects stay in a slot cache until they have been unused for `cache_lifetime` main-loop iterations. Off disables the cache. |
| `cache_lifetime` | `64` | Number of main-loop iterations an unused cached 3D object is kept before it is released. |
| `dummy_manager` | `0` | `1` replaces the 3D object manager with a stub that loads no model data. The `dm` command-line option forces it on. |
| `in_view_enabled` | `yes` | **No effect.** |
| `shutdown_mode` | `yes` | **Not read.** |

---

## `[xngine]`

| Key | Default Value | Description |
|---|---|---|
| `texture_kbytes` | `12000` | **No effect.** Shown only in the developer console header as the texture heap size. |
| `gfx_kbytes` | `256` | **No effect.** |
| `front_plane` | `7` | Near clipping distance of the scene. See [fog and clipping](../engine/fog.md#clipping-distances). |
| `back_plane` | `3800` | Far clipping distance of the scene; a nonzero `world_back_plane[N]` replaces it for that world. See [fog and clipping](../engine/fog.md#clipping-distances). |
| `perspective_low_x` … `perspective_ultra_y` | `190`–`800` | **Not read.** All eight `perspective_*` keys. |
| `detail` | `8` | **No effect.** |
| `ambient_light` | `32` | **No effect.** Initial environmental-effect brightness, replaced by `world_ambientfx[N]` on every world load. |
| `screen_scale` | `256` | **No effect.** Shown only by the console `show xngine` report. |
| `haze_depth` | `768` | **No effect.** Scene fog comes from [FOG.INI](fog-ini.md). |
| `sky_disable` | `0` | `1` disables the [sky](../engine/sky.md) in every world; any other value leaves it enabled. |
| `sky_move` | `1` | **No effect.** |
| `sky_xrotate` | `3` | **No effect.** |
| `sky_yrotate` | `40` | **No effect.** |
| `game_detail` | `2` | **No effect.** Shown only by the console `show xngine` report. |
| `exclusion` | `0` | **Not read.** |

---

## `[camera]`

Camera rig configuration for the normal, combat, hanging, rope, and debug cameras.
Each camera orbits a pivot point near the player at its own distance and angles.
See the [camera sections of SOUP](../engine/SOUP.md#camera-marks-and-the-render-camera-step)
for the runtime behavior.

### Pivot, target, and normal camera

| Key | Default Value | Description |
|---|---|---|
| `target_offset_x` / `_y` / `_z` | `-6000` / `-14000` / `0` | Camera target point relative to the player, in Q8 units in the player's rotated frame. It is the look-at point and the start of the camera's obstacle test. Applies to the normal and combat cameras; the normal camera uses only Y. |
| `offset_pos_x` / `_y` / `_z` | `0` / `-14000` / `0` | Camera pivot offset from the player in Q8 units, rotated with the player. The camera sits `distance` behind this point; the shipped Y puts it about 55 units above the player origin. |
| `offset_angle_x` / `_y` / `_z` | `0` / `0` / `0` | Normal camera orbit angles (pitch, yaw, roll) relative to the player. `0` places the camera straight behind the player. |
| `camera_distance` | `250` | Normal camera distance behind the pivot, in world units. |
| `camera_min_distance` | `120` | When an obstacle pulls the normal or combat camera closer than this, the camera rises so that its height above the player scales by the ratio of minimum to actual distance. |
| `camera_left_pos` / `camera_right_pos` | `5000` / `5000` | **No effect.** |

### Mode cameras

All angle triples are pitch, yaw, and roll.

| Key | Default Value | Description |
|---|---|---|
| `camera_combat_angle_offset_x` / `_y` / `_z` | `134` / `245` / `0` | Combat camera orbit angles relative to the player, used while the player holds a weapon. |
| `camera_combat_distance` | `260` | Combat camera distance in world units. |
| `camera_hang_angle_offset_x` / `_y` / `_z` | `256` / `0` / `0` | Hanging (ledge) camera orbit angles relative to the player. |
| `camera_hang_distance` | `250` | Hanging camera distance in world units. |
| `static_rope_threshold` | `5` | Ropes whose maximum swing amplitude (`MPRP` `swing`) is below this value are static. On a static rope the camera uses the rope-above settings while the player holds a link in the upper two thirds and the rope-below settings otherwise. Other ropes use the rope settings. |
| `camera_rope_max_vel` | `18000` | Movement limit of the swinging-rope camera, in the units of `max_vel`. `0` means `max_vel`. |
| `camera_rope_angle_offset_x` / `_y` / `_z` | `96` / `128` / `0` | Swinging-rope camera orbit angles. The pitch is absolute (the player's pitch is ignored); yaw and roll are relative to the player. |
| `camera_rope_distance` | `300` | Swinging-rope camera distance in world units. |
| `camera_rope_above_angle_offset_x` / `_y` / `_z` | `96` / `128` / `0` | Static-rope camera orbit angles for the upper part of the rope, relative to the aim angles. |
| `camera_rope_above_distance` | `300` | Upper static-rope camera distance in world units. |
| `camera_rope_above_aim_x` / `_y` / `_z` | `-512` / `0` / `0` | Fixed angles that the upper static-rope camera uses in place of the player's orientation. `-512` pitches the view a quarter turn from level. |
| `camera_rope_below_angle_offset_x` / `_y` / `_z` | `96` / `128` / `0` | Static-rope camera orbit angles for the lower part of the rope, relative to the aim angles. |
| `camera_rope_below_distance` | `300` | Lower static-rope camera distance in world units. |
| `camera_rope_below_aim_x` / `_y` / `_z` | `512` / `0` / `0` | Fixed angles that the lower static-rope camera uses in place of the player's orientation. |
| `camera_debug_angle_offset_x` / `_y` / `_z` | `128` / `256` / `0` | Debug camera orbit angles. All three values are stored into the pitch, so only `camera_debug_angle_offset_z` takes effect (as pitch); yaw and roll come from `offset_angle_y` and `offset_angle_z`. |
| `camera_debug_distance` | `300` | Debug camera distance in world units. |

### Collision, limits, and smoothing

| Key | Default Value | Description |
|---|---|---|
| `obstacle_size` | `20` | Only models whose bounding radius is strictly larger than this can block the camera's line of sight. See [camera collision](../engine/SOUP.md#camera-collision-candidate-radius). |
| `camera_size` | `10` | Camera body radius in world units. On contact the camera is pushed this far along the contact normal. See [camera collision response](../engine/SOUP.md#camera-collision-response). |
| `camera_scape_size` | `5` | Minimum camera height above WLD terrain, in steps of 4 world units (shipped 20 units). It has no effect on maps without terrain. |
| `max_x_angle` / `min_x_angle` | `1692` / `400` | Pitch limits for the look key: the look pitch offset never enters the band between `min_x_angle` and `max_x_angle`, so the shipped values allow -356 to +400 units around level. The automatic cameras ignore them. |
| `max_vel` | `8000` | Maximum camera movement per frame step in Q8 units (shipped about 31 units), scaled by frame time. See [movement limiting](../engine/SOUP.md#camera-movement-limiting). |
| `max_y_vel` | `5000` | Maximum vertical camera movement per frame step in Q8 units (shipped about 20 units). The upward limit is scaled by frame time; the downward limit is not. |
| `max_acc` | `5000` | Camera acceleration: each frame step may exceed the previous one by this many Q8 units, up to `max_vel`. |
| `player_control_x_inc` / `player_control_y_inc` | `70` / `-70` | Pitch (`x`) and yaw (`y`) change per frame while the look key is held and a direction key is pressed. |
| `glide_x` / `glide_y` / `glide_z` | `0.9` / `0.2` / `0.9` | Fraction of the remaining distance that the rendered camera moves toward its target each frame on each axis. `1.0` means no lag. See [render camera step](../engine/SOUP.md#camera-marks-and-the-render-camera-step). |
| `glide_angle_x` / `glide_angle_y` / `glide_angle_z` | `0.7` / `0.1` / `0.7` | The same fraction for the rendered camera angles. `glide_angle_x` applies to yaw, `glide_angle_y` to pitch, and `glide_angle_z` to roll. |
| `cam_prox_up` | `70.0` | **No effect.** |

Keys read but absent from the shipped file:

- `camera_combat_max_vel`, `camera_hang_max_vel`, `camera_rope_above_max_vel`,
  `camera_rope_below_max_vel`, `camera_debug_max_vel`: movement limit for that
  camera, in the units of `max_vel`. Missing or `0` means `max_vel`.
- `fudge_factor`, `max_angular_velocity`: **no effect**.

---

## `[dialog]`

| Key | Default Value | Description |
|---|---|---|
| `menu_start_x` | `35` | Left edge of the dialogue topic menu, in text-screen pixels. |
| `menu_start_y` | `25` | Top of the first dialogue topic-menu entry, in text-screen pixels; each later entry is one font line lower. |
| `dialog_max_menu_items` | `20` | Capacity of the dialogue topic menu, at least 300. The shipped `20` therefore gives 300 entries. |
| `dialog_max_dialog_lines` | `20` | Number of [subtitle](../engine/subtitles.md) slots (minimum 16), not a line limit. |
| `dialog_max_text_width` | `500` | Subtitle wrap width in 640-wide screen pixels (minimum 150). |
| `dialog_print_text` | `1` | `0` hides dialogue subtitles without changing their timing. The book's Subtitles checkbox edits this value. |
| `dialog_use_speech` | `1` | `1` plays dialogue speech and shows each line for the length of its clip; `0` silences speech and times lines from their text length. |
| `dialog_max_distance` | `1600` | Scripted RTX lines from a speaker farther than this distance (in `PlayerDistance` units) are not started. |
| `menu_traverse_delay` | `4` | Delay in BIOS timer ticks (18.2 per second) before a held direction key moves the selection again, in the dialogue topic menu and the book menu. `0` means one tick. |

---

## `[3dfx]`

Glide-specific display, texture, and text settings.

| Key | Default Value | Description |
|---|---|---|
| `resolution` | `12` | Glide display mode: `3` = 512×384, `7` = 640×480, `8` = 800×600, `9` = 960×720, `12` = 1024×768, `13` = 1280×1024, `14` = 1600×1200 (the engine's own size table uses 1600×1280 for this mode). Other values are not supported. If the mode cannot be opened, the game falls back to 640×480. |
| `text_scale` | `1, 1` | Texture reduction factors for 256-texel and 128-texel textures (`256 factor, 128 factor`). `1` keeps full resolution; a missing key means `4, 2`. Sky textures are never reduced. |
| `anim_text_scale` | `1, 1` | **No effect.** |
| `font_sel` | `255,255` | Color scale and alpha (`A,B`, 0–255) for the REDSEL text style used by NPC subtitles, HUD text, and the selected topic-menu entry. |
| `font_norm` | `125,255` | Color scale and alpha for the normal REDSEL style, used for Cyrus's subtitles and unselected topic-menu entries. A missing key copies `font_sel`. |
| `font_used` | `125,128` | Color scale and alpha for topic-menu entries already chosen. A missing key copies `font_sel`. |

---

## Developer mode

`[debug] final_version=0` selects developer behavior. The `fv` command-line
option forces release behavior and takes precedence over `nfv`, which forces
developer behavior. Either option overrides the INI value.

In developer mode, startup does not present the book menu and enters the
starting world directly. In a world, Escape waits for its release and then
exits the game instead of opening the book. Developer mode also enables the
camera position/angle diagnostic keys and other developer display paths.

## External References

- [UESP: Mod:SYSTEM.INI Info](https://en.uesp.net/wiki/Mod:SYSTEM.INI_Info)
