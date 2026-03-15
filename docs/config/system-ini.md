# SYSTEM.INI

Primary engine configuration file controlling rendering, gameplay, camera, dialog, debug, and 3D subsystem parameters.

Shipped sample path: `/Redguard/SYSTEM.INI` (e.g. `.../GOG Galaxy/Redguard/Redguard/SYSTEM.INI`).

## File Structure

Standard Windows-style INI file with 10 sections:

1. `[screen]` — display resolution and palette settings
2. `[system]` — core engine paths, audio, physics, and HUD layout
3. `[debug]` — developer diagnostics and logging flags
4. `[game]` — gameplay physics thresholds and interaction radii
5. `[cyrus]` — player character movement and control parameters
6. `[3dmanager]` — 3D object cache and memory budget
7. `[xngine]` — renderer texture memory, clipping planes, and sky behavior
8. `[camera]` — camera rig offsets, distances, and glide factors for each mode
9. `[dialog]` — dialog menu layout and speech settings
10. `[3dfx]` — Glide/GOG-specific resolution and font scaling overrides

---

## `[screen]`

Display mode and palette initialization settings.

| Key | Default Value | Description |
|---|---|---|
| `candle_mode` | `2` | Candle/torch lighting mode index. |
| `colour_bits` | `8` | Color depth in bits per pixel (8 = paletted). |
| `resolution` | `1` | Software renderer resolution index. |
| `Palette_red` | `0` | Red component of the initial palette background color. |
| `Palette_green` | `0` | Green component of the initial palette background color. |
| `Palette_blue` | `0` | Blue component of the initial palette background color. |
| `smk_interlace` | `0` | Smacker video interlace mode. `0` = disabled. |

---

## `[system]`

Core engine paths, audio configuration, physics constants, HUD element positions, and subsystem enable flags.

`world_ini` and `item_ini` point to `WORLD.INI` and `ITEM.INI` respectively, which the engine loads for world and item definitions.

| Key | Default Value | Description |
|---|---|---|
| `game_bitmap` | `system\powerup.gxa` | Path to the startup/powerup UI bitmap (GXA format). |
| `pointers` | `system\pointers.bmp` | Path to the cursor sprite sheet. |
| `system_font` | `fonts\redguard.fnt` | Path to the primary system font. |
| `icon_font` | `fonts\arialvs.fnt` | Path to the icon/small font. |
| `gui_font` | `fonts\arialbg.fnt` | Path to the main GUI font. |
| `gui_low_font` | `fonts\arialvb.fnt` | Path to the low-resolution GUI font. |
| `animation_drive` | `D:\` | Drive letter used for animation streaming. |
| `back_texture` | `0` | Background texture index. `0` = none. |
| `volume` | `255` | Master sound volume (0..255). |
| `sound` | `1` | Sound system enabled. `1` = on. |
| `fast_sound` | `1` | Fast sound mixing mode. `1` = on. |
| `fidelity` | `0` | Sound fidelity level. |
| `redbook` | `on` | CD audio (Red Book) playback enabled. |
| `redbook_volume` | `200` | CD audio volume level. |
| `sound_distance` | `64` | Maximum distance at which sounds are audible. |
| `post_collide_height` | `50` | Post-collision step height for ground snapping. |
| `hpost_collide_height` | `50` | Post-collision step height for hanging/climbing. |
| `pre_validate` | `no` | Pre-validate collision geometry on load. |
| `normal_frame_rate` | `12` | Target frame rate for normal gameplay. |
| `use_smooth_fps` | `yes` | Enable frame-rate smoothing. |
| `use_smooth_divisor` | `yes` | Enable frame-rate smoothing divisor. |
| `min_frame_rate` | `6` | Minimum allowed frame rate. |
| `max_frame_rate` | `300` | Maximum allowed frame rate. |
| `jump_time` | `6` | Duration of a jump in frames. |
| `jump_height` | `80` | Jump height in engine units. |
| `sphere_object_scale` | `256` | Scale factor for sphere collision objects. |
| `standing_height` | `-2` | Vertical offset for the player standing position. |
| `disable_text` | `yes` | Disable on-screen text rendering. |
| `disable_debug_text` | `yes` | Disable debug text overlay. |
| `normal_ink` | `1` | Normal ink/outline rendering mode. |
| `slide_range` | `-60000` | Slide detection range threshold. |
| `statics_load` | `yes` | Load static objects. |
| `flats_load` | `yes` | Load flat/billboard objects. |
| `objects_load` | `yes` | Load dynamic objects. |
| `lights_load` | `yes` | Load light objects. |
| `ropes_load` | `yes` | Load rope objects. |
| `task_system` | `yes` | Enable the task/AI system. |
| `animation_system` | `yes` | Enable the animation system. |
| `floating_point_physics` | `yes` | Use floating-point physics calculations. |
| `rtx_filename` | `ENGLISH.RTX` | Path to the dialogue/voice container file. |
| `world_ini` | `WORLD.INI` | Path to the world definitions INI. |
| `item_ini` | `ITEM.INI` | Path to the item definitions INI. |
| `start_fade` | `on` | Fade in on game start. |
| `compass_xco` | `546` | Compass HUD element X coordinate. |
| `compass_yco` | `396` | Compass HUD element Y coordinate. |
| `candle_xco` | `12` | Candle HUD element X coordinate. |
| `candle_yco` | `8` | Candle HUD element Y coordinate. |
| `logbook_xco` | `540` | Logbook HUD element X coordinate. |
| `logbook_yco` | `20` | Logbook HUD element Y coordinate. |
| `pickup_xco` | `12` | Pickup prompt HUD element X coordinate. |
| `pickup_yco` | `386` | Pickup prompt HUD element Y coordinate. |
| `pickup_text_yco` | `436` | Pickup text HUD element Y coordinate. |
| `game_xco1` | `576` | Game UI element 1 X coordinate. |
| `game_yco1` | `6` | Game UI element 1 Y coordinate. |
| `game_xco2` | `576` | Game UI element 2 X coordinate. |
| `game_yco2` | `96` | Game UI element 2 Y coordinate. |
| `lock_windows` | `no` | Lock window position/size. |
| `disable_drive_check` | `yes` | Skip CD drive presence check on startup. |
| `disable_cpu_check` | `yes` | Skip CPU speed check on startup. |
| `disable_svga_check` | `yes` | Skip SVGA capability check on startup. |
| `report_machine` | `no` | Log machine hardware info on startup. |
| `max_active_objects` | `32` | Maximum number of simultaneously active objects. |
| `max_effects` | `32` | Maximum number of simultaneous particle effects. |
| `max_particles` | `512` | Maximum number of simultaneous particles. |
| `max_remap_objects` | `64` | Maximum number of palette-remapped objects. |
| `disable_effects` | `no` | Disable particle effects. |

---

## `[debug]`

Developer diagnostics, logging, and display flags. Most are disabled in the shipped build.

`network_marker_file=g:\PROJECTS\REDGUARD\DEMO\NETWORK.MRK` is a build-time artifact: a hardcoded developer machine path left in the shipped file.

| Key | Default Value | Description |
|---|---|---|
| `final_version` | `1` | Marks this as a final/release build. Suppresses some developer output. |
| `enable_logs` | `0` | Enable runtime log file writing. |
| `map_log` | `no` | Log map loading events. |
| `console_error` | `yes` | Print errors to the console. |
| `software_interrupt` | `yes` | Enable software interrupt handling. |
| `software_break` | `yes` | Enable software breakpoint handling. |
| `attempt_recover` | `no` | Attempt to recover from errors rather than aborting. |
| `object_log` | `no` | Log object system events. |
| `video_log` | `no` | Log video/renderer events. |
| `node_marker` | `no` | Display navigation node markers. |
| `task_debug` | `no` | Enable task/AI debug output. |
| `memory_manager` | `0` | Memory manager debug level. |
| `monitor_object` | _(empty)_ | Name of an object to monitor for debug output. |
| `display_masters` | `no` | Display master object markers. |
| `display_slaves` | `no` | Display slave object markers. |
| `display_edges` | `no` | Display edge/collision markers. |
| `ignore_errors` | `yes` | Continue running on non-fatal errors. |
| `disable_family` | `no` | Disable object family grouping. |
| `disable_master_slaves` | `no` | Disable master/slave object relationships. |
| `disable_slaves` | `no` | Disable slave objects. |
| `family_log` | `no` | Log object family events. |
| `script_log` | `no` | Log script execution events. |
| `show_manager` | `no` | Display the object manager overlay. |
| `display_node_map` | `no` | Display the navigation node map. |
| `display_nodes` | `no` | Display individual navigation nodes. |
| `display_markers` | `no` | Display world markers. |
| `network_marker_file` | `g:\PROJECTS\REDGUARD\DEMO\NETWORK.MRK` | Path to the network marker file. Build-time developer path; not used in the shipped game. |
| `object_system_log` | `no` | Log object system events. |
| `convert_static_angles` | `no` | Convert static object angles on load. |
| `def_checksum` | `yes` | Verify SOUP386.DEF checksum on load. |
| `render_log` | `no` | Log renderer events. |
| `idebug` | `0` | Interactive debug level. |
| `idebug_refresh` | `no` | Refresh interactive debug display each frame. |
| `memory_monitor` | `no` | Enable memory usage monitoring. |

---

## `[game]`

Gameplay physics thresholds, fall damage, and interaction radii.

| Key | Default Value | Description |
|---|---|---|
| `fall_bounce_height` | `100` | Fall height (engine units) below which the player bounces without damage. |
| `fall_death_height` | `568` | Fall height at which the player dies. |
| `fall_hurt_height` | `300` | Fall height at which the player takes damage. |
| `fall_hurt_zap` | `10` | Damage amount applied at `fall_hurt_height`. |
| `slide_hurt_height` | `1024` | Slide distance at which the player takes damage. |
| `slide_hurt_zap` | `10` | Damage amount applied at `slide_hurt_height`. |
| `rope_jump_add` | `18` | Velocity added when jumping from a rope. |
| `rope_attach_angle` | `512` | Angle threshold for rope attachment (engine angle units). |
| `slide_speed` | `36` | Player slide speed in engine units per frame. |
| `rtx_pickup_override_time` | `24` | Duration (frames) of the pickup text override display. |
| `swim_depth` | `40` | Depth threshold for switching to swim mode. |
| `player_dead_time` | `3` | Time (seconds) before respawn after death. |
| `player_fall_dead_time` | `12` | Time (frames) before death is registered after a fatal fall. |
| `old_combat` | `off` | Use legacy combat system. |
| `lineup_distance` | `128` | Distance at which enemies line up for combat. |
| `dialog_radius` | `512` | Radius within which NPCs can initiate dialog. |
| `combat_sphere_scale` | `200` | Scale factor for combat hit sphere. |

---

## `[cyrus]`

Player character (Cyrus) movement, control, and camera-follow parameters.

| Key | Default Value | Description |
|---|---|---|
| `sheath_sword_delay` | `6` | Frames before the sword auto-sheathes after combat. |
| `auto_defend` | `1` | Enable automatic defense. `1` = on. |
| `walk_mode` | `1` | Walk mode index. |
| `turn_speed` | `12` | Base turning speed. |
| `turn_max_speed` | `48` | Maximum turning speed. |
| `mouse_turn` | `0` | Mouse-driven turning. `0` = disabled. |
| `joy_tolerance` | `60` | Joystick dead-zone tolerance. |
| `camera_distance` | `200` | Default camera follow distance. |
| `tap_time` | `12` | Maximum frames between taps for a double-tap input. |
| `poly_push_units` | `16` | Distance (engine units) the player is pushed out of geometry on collision. |
| `auto_grab` | `off` | Automatically grab ledges and ropes. |
| `smooth_post_min` | `2` | Minimum smoothing steps for post-collision position. |
| `smooth_post_max` | `10` | Maximum smoothing steps for post-collision position. |
| `smooth_post_divisor` | `2` | Divisor for post-collision position smoothing. |

---

## `[3dmanager]`

3D object cache, memory budgets, and manager behavior.

| Key | Default Value | Description |
|---|---|---|
| `buffer_kbytes` | `800` | Size of the 3D streaming buffer in kilobytes. |
| `heap_kbytes` | `22000` | Size of the 3D object heap in kilobytes. |
| `max_objects` | `255` | Maximum number of 3D objects loaded simultaneously. |
| `compress` | `1` | Enable compressed object loading. `1` = on. |
| `save_compressed` | `0` | Save objects in compressed form. `0` = off. |
| `cache_objects` | `yes` | Cache loaded 3D objects in memory. |
| `cache_lifetime` | `64` | Number of frames a cached object is retained after last use. |
| `dummy_manager` | `0` | Use a dummy (no-op) 3D manager. `0` = off. |
| `in_view_enabled` | `yes` | Enable in-view culling for 3D objects. |
| `shutdown_mode` | `yes` | Perform full shutdown cleanup on exit. |

---

## `[xngine]`

Renderer memory budgets, clipping planes, perspective settings, and sky behavior.

| Key | Default Value | Description |
|---|---|---|
| `texture_kbytes` | `12000` | Texture memory budget in kilobytes. |
| `gfx_kbytes` | `256` | General graphics buffer size in kilobytes. |
| `front_plane` | `7` | Near clipping plane distance. |
| `back_plane` | `3800` | Far clipping plane distance. |
| `perspective_low_x` | `190` | Perspective correction X extent at low detail. |
| `perspective_low_y` | `190` | Perspective correction Y extent at low detail. |
| `perspective_med_x` | `400` | Perspective correction X extent at medium detail. |
| `perspective_med_y` | `400` | Perspective correction Y extent at medium detail. |
| `perspective_high_x` | `800` | Perspective correction X extent at high detail. |
| `perspective_high_y` | `800` | Perspective correction Y extent at high detail. |
| `perspective_ultra_x` | `800` | Perspective correction X extent at ultra detail. |
| `perspective_ultra_y` | `800` | Perspective correction Y extent at ultra detail. |
| `detail` | `8` | Renderer detail level. |
| `ambient_light` | `32` | Global ambient light level (0..255). |
| `screen_scale` | `256` | Screen-space scale factor. |
| `haze_depth` | `768` | Distance at which atmospheric haze begins. |
| `sky_disable` | `0` | Disable sky rendering. `0` = sky enabled. |
| `sky_move` | `1` | Enable sky scrolling. `1` = on. |
| `sky_xrotate` | `3` | Sky X-axis rotation speed. |
| `sky_yrotate` | `40` | Sky Y-axis rotation speed. |
| `game_detail` | `2` | In-game detail preset index. |
| `exclusion` | `0` | Exclusion zone rendering mode. `0` = off. |

---

## `[camera]`

Camera rig configuration for all gameplay modes: normal follow, combat, hanging, rope, and debug. Coordinates and angles use engine units. Floating-point glide factors control camera lag.

| Key | Default Value | Description |
|---|---|---|
| `static_rope_threshold` | `5` | Velocity threshold below which a rope is considered static. |
| `obstacle_size` | `20` | Radius used for camera obstacle avoidance. |
| `camera_size` | `10` | Camera collision sphere radius. |
| `camera_scape_size` | `5` | Camera collision sphere radius in scape/exterior areas. |
| `target_offset_x` | `-6000` | Target point X offset from the player. |
| `target_offset_y` | `-14000` | Target point Y offset from the player. |
| `target_offset_z` | `0` | Target point Z offset from the player. |
| `offset_pos_x` | `0` | Camera position X offset. |
| `offset_pos_y` | `-14000` | Camera position Y offset. |
| `offset_pos_z` | `0` | Camera position Z offset. |
| `offset_angle_x` | `0` | Camera angle X offset. |
| `offset_angle_y` | `0` | Camera angle Y offset. |
| `offset_angle_z` | `0` | Camera angle Z offset. |
| `camera_distance` | `250` | Default follow camera distance. |
| `camera_min_distance` | `120` | Minimum allowed camera distance. |
| `camera_right_pos` | `5000` | Camera right-side position limit. |
| `camera_left_pos` | `5000` | Camera left-side position limit. |
| `camera_combat_angle_offset_x` | `134` | Combat camera X angle offset. |
| `camera_combat_angle_offset_y` | `245` | Combat camera Y angle offset. |
| `camera_combat_angle_offset_z` | `0` | Combat camera Z angle offset. |
| `camera_combat_distance` | `260` | Camera distance in combat mode. |
| `camera_hang_angle_offset_x` | `256` | Hanging camera X angle offset. |
| `camera_hang_angle_offset_y` | `0` | Hanging camera Y angle offset. |
| `camera_hang_angle_offset_z` | `0` | Hanging camera Z angle offset. |
| `camera_hang_distance` | `250` | Camera distance in hanging mode. |
| `camera_rope_max_vel` | `18000` | Maximum camera velocity when following a rope. |
| `camera_rope_angle_offset_x` | `96` | Rope camera X angle offset. |
| `camera_rope_angle_offset_y` | `128` | Rope camera Y angle offset. |
| `camera_rope_angle_offset_z` | `0` | Rope camera Z angle offset. |
| `camera_rope_distance` | `300` | Camera distance in rope mode. |
| `camera_rope_above_angle_offset_x` | `96` | Rope-above camera X angle offset. |
| `camera_rope_above_angle_offset_y` | `128` | Rope-above camera Y angle offset. |
| `camera_rope_above_angle_offset_z` | `0` | Rope-above camera Z angle offset. |
| `camera_rope_above_distance` | `300` | Camera distance in rope-above mode. |
| `camera_rope_above_aim_x` | `-512` | Rope-above camera aim X offset. |
| `camera_rope_above_aim_y` | `0` | Rope-above camera aim Y offset. |
| `camera_rope_above_aim_z` | `0` | Rope-above camera aim Z offset. |
| `camera_rope_below_angle_offset_x` | `96` | Rope-below camera X angle offset. |
| `camera_rope_below_angle_offset_y` | `128` | Rope-below camera Y angle offset. |
| `camera_rope_below_angle_offset_z` | `0` | Rope-below camera Z angle offset. |
| `camera_rope_below_distance` | `300` | Camera distance in rope-below mode. |
| `camera_rope_below_aim_x` | `512` | Rope-below camera aim X offset. |
| `camera_rope_below_aim_y` | `0` | Rope-below camera aim Y offset. |
| `camera_rope_below_aim_z` | `0` | Rope-below camera aim Z offset. |
| `camera_debug_angle_offset_x` | `128` | Debug camera X angle offset. |
| `camera_debug_angle_offset_y` | `256` | Debug camera Y angle offset. |
| `camera_debug_angle_offset_z` | `0` | Debug camera Z angle offset. |
| `camera_debug_distance` | `300` | Camera distance in debug mode. |
| `max_x_angle` | `1692` | Maximum camera X angle (engine angle units). |
| `min_x_angle` | `400` | Minimum camera X angle (engine angle units). |
| `max_vel` | `8000` | Maximum camera velocity. |
| `max_y_vel` | `5000` | Maximum camera Y-axis velocity. |
| `max_acc` | `5000` | Maximum camera acceleration. |
| `player_control_x_inc` | `70` | Player-controlled camera X increment per frame. |
| `player_control_y_inc` | `-70` | Player-controlled camera Y increment per frame. |
| `glide_x` | `0.9` | Camera position X glide (lag) factor. |
| `glide_y` | `0.2` | Camera position Y glide (lag) factor. |
| `glide_z` | `0.9` | Camera position Z glide (lag) factor. |
| `glide_angle_x` | `0.7` | Camera angle X glide (lag) factor. |
| `glide_angle_y` | `0.1` | Camera angle Y glide (lag) factor. |
| `glide_angle_z` | `0.7` | Camera angle Z glide (lag) factor. |
| `cam_prox_up` | `70.0` | Camera proximity upward adjustment distance. |

---

## `[dialog]`

Dialog menu layout, line limits, and speech settings.

| Key | Default Value | Description |
|---|---|---|
| `menu_start_x` | `35` | Dialog menu X start position in screen pixels. |
| `menu_start_y` | `25` | Dialog menu Y start position in screen pixels. |
| `dialog_max_menu_items` | `20` | Maximum number of items in a dialog menu. |
| `dialog_max_dialog_lines` | `20` | Maximum number of lines in a dialog text block. |
| `dialog_max_text_width` | `500` | Maximum width of dialog text in pixels. |
| `dialog_print_text` | `1` | Display dialog as on-screen text. `1` = on. |
| `dialog_use_speech` | `1` | Play voiced speech audio during dialog. `1` = on. |
| `dialog_max_distance` | `1600` | Maximum distance at which dialog audio is played. |
| `menu_traverse_delay` | `4` | Frames of delay between menu cursor movements. |

---

## `[3dfx]`

Glide renderer overrides, active in the GOG release which runs via the Glide/3dfx code path. These settings take precedence over the software-renderer equivalents where applicable.

| Key | Default Value | Description |
|---|---|---|
| `resolution` | `12` | Glide renderer resolution index. |
| `text_scale` | `1, 1` | Text scaling factors (X, Y) for the Glide renderer. |
| `anim_text_scale` | `1, 1` | Animated text scaling factors (X, Y) for the Glide renderer. |
| `font_sel` | `255,255` | Font selection color values (foreground, background) for selected menu items. |
| `font_norm` | `125,255` | Font color values (foreground, background) for normal menu items. |
| `font_used` | `125,128` | Font color values (foreground, background) for used/visited menu items. |

---

## External References

- [UESP: Mod:SYSTEM.INI Info](https://en.uesp.net/wiki/Mod:SYSTEM.INI_Info)
