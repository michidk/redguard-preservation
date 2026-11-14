# Demo release

The November 1998 demo restricts play to the island of Stros M'Kai. It ships a
later engine build than the retail release and a reduced data set.

## Package

The downloadable demo (`rgdemo.zip`) contains an InstallShield installer. Its
`RGFX.EXE` sits beside the installer, outside `data1.cab`. The cabinet installs
the Glide data set: `SYSTEM.INI`, `WORLD.INI` and the other configuration files,
`ENGDEMO.RTX`, and the `fxart`, `fonts`, `maps`, `sound`, `soup386` and `system`
directories. It contains no software-renderer assets and no cinematics.
`DEMOMOD.TXT` describes the demo for players.

## Data

| Data | Demo content |
|---|---|
| Maps | `ISLAND.RGM`, `ISLAND.WLD` and `ISLAND.PVO` only, identical to the retail files. Every interior, the catacombs and N'Gasta's island are absent. |
| `WORLD.INI` | World 1 (the island by day) and world 99 (`maps\brennans.rgm`, whose map is not shipped). `start_world=1`, `start_marker=1`. |
| `fxart` | 259 of the 755 retail files, identical to their retail versions. |
| `system` | 37 of the 69 retail files, identical to their retail versions. |
| Dialogue | `ENGDEMO.RTX`: the same 4,866 entries as `ENGLISH.RTX` with the same text except entry `$133`, but only 1,224 voiced entries instead of 3,933. The other entries are string-only. |
| Unchanged | `ITEM.INI`, `MENU.INI`, `KEYS.INI`, `COMBAT.INI`, `REGISTRY.INI`, `SURFACE.INI` and `SOUP386.DEF` are identical to the retail files. |

The island scripts are the retail ones, including the door and boat
transitions to interiors and other worlds. Their target maps are not part of
the demo; the [demo lock](#demo-lock) prevents those transitions.

`SYSTEM.INI` differs from the retail file in these values:

| Section | Key | Demo | Retail |
|---|---|---|---|
| `[screen]` | `smk_interlace` | `1` | `0` |
| `[system]` | `animation_drive` | `h:\` | `D:\` |
| `[system]` | `fast_sound` | `0` | `1` |
| `[system]` | `rtx_filename` | `ENGDEMO.RTX` | `ENGLISH.RTX` |
| `[debug]` | `demo_version` | `1` | absent |
| `[3dmanager]` | `cache_lifetime` | `32` | `64` |
| `[dialog]` | `dialog_print_text` | `0` | `1` |
| `[3dfx]` | `resolution` | `7` (640×480) | `12` (1024×768) |
| `[rtx_patch]` | `bad[0]`, `good[0]` | `D2DC`, `D12C` | absent |

`d2dc` is referenced by the retail `TAVERN.RGM` and exists in neither RTX file.
`d12c` is "I THOUGHT I MIGHT LEND A HAND."

## Executable

The demo `RGFX.EXE` has version `1.0`, build `5.0` in its
[version stamp](engine/startup.md#version-stamp); the CD and GOG executable is
build `1.0`. It runs the retail data as well. Its change notes read:

- "Fixed rope death-reattach bug."
- "Fixed texture binding crash bug on certain object flats."
- "Fixed run of edge fall 'stutter' bug."

The build 1.0 note, "Fixed camera reset bug, (see leaving the catacombs).", is
not repeated; the build 5.0 executable contains that build's code.

### Demo lock

[`[debug] demo_version`](config/system-ini.md#demo-executable-keys) enables the
lock. Without the key it is off, so the demo executable plays the full game with
the retail `SYSTEM.INI`. With it on:

- every world load loads world 1 at marker 1, and a load while world 1 is
  loaded does nothing;
- pending world changes from doors, boats and the console `load` command are
  discarded;
- the player-state update that follows the first world load at startup is
  skipped.

### Gameplay changes

- **Rope attachment while dying.** The rope and hook attachment test also
  requires that the player is not dead. The flag it reads is set when health
  reaches zero and the death animation starts.
- **Ledge falls.** When a fall off a ledge begins, the actor is also moved up
  to 4096 position units in its last movement direction, away from the edge.
- **Object flats.** A flat's texture ID is resolved when the flat is created:
  bits 7–15 select the [TEXBSI](formats/TEXBSI.md) file and bits 0–6 the image.
  An ID that cannot be resolved is a fatal error. The build 1.0 fatal error
  `Bad Texture Index %d` for indices above 30000 is removed.
- **Inventory models.** Item models load their textures at full resolution:
  [`[3dfx] text_scale`](config/system-ini.md#3dfx) is overridden with `1, 1`
  while the item system loads its models and restored afterwards.
- **Animation selection.** Empty animation slots in an actor's group table are
  skipped instead of being read.

### Dialogue changes

- An RTX entry without audio is marked as having no voice when it is played.
- With `[dialog] dialog_use_speech=1`, a string-only entry is shown for
  `characters × 15` units, like a line with speech off. Build 1.0 neither draws
  nor frees such an entry (see [subtitle timing](engine/subtitles.md#timing)).
- A string-only entry is drawn even with `dialog_print_text=0`.
- A missing RTX entry logs `ERROR: GetRtx( %c%c%c%c ), RTX NOT found` to
  `GENERAL.LOG` and plays entry `cbu1` ("DON'T SUPPOSE YOU KNOW WHAT'S GOING
  ON.") instead. Build 1.0 stops with a fatal error. A missing `cbu1` is fatal.
- [`[RTX_PATCH]`](config/system-ini.md#demo-executable-keys) substitutes up to
  15 entries by name.
- **F11** toggles an overlay that shows the current entry as `(%d) RTX %c%c%c%c`.
  Build 1.0 contains the overlay but never calls it.

### Diagnostics

- [`[3DFX] warnings`](config/system-ini.md#demo-executable-keys) enables two
  texture-manager warnings.
- A missing texture archive prints `Missing Texture File: FXART\TEXBSI.%03d`
  before the fatal error.
- `COMBAT.LOG` is reset at startup with the other log files. No code writes to
  it.

## External References

- [DOS Games Archive: `rgdemo.zip`](https://www.dosgamesarchive.com/file/the-elder-scrolls-adventures-redguard/rgdemo/)
- [Internet Archive: The Elder Scrolls Adventures: Redguard Demo](https://archive.org/details/REDGUARDDemo)
