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
build `1.0`. Build 5.0 contains all of build 1.0 and runs the retail data as
well. Its change notes read:

- "Fixed rope death-reattach bug."
- "Fixed texture binding crash bug on certain object flats."
- "Fixed run of edge fall 'stutter' bug."

The build 1.0 note, "Fixed camera reset bug, (see leaving the catacombs).", is
not repeated.

### Overview

| Change | Kind | In change notes | Applies to retail data |
|---|---|---|---|
| [Rope attachment while dying](#rope-attachment-while-dying) | Bug fix | Yes | Yes |
| [Ledge-fall stutter](#ledge-fall-stutter) | Bug fix | Yes | Yes |
| [Object-flat textures](#object-flat-textures) | Bug fix | Yes | Yes |
| [Animation slots](#animation-slots) | Bug fix | No | Yes |
| [String-only dialogue with speech on](#string-only-dialogue-with-speech-on) | Bug fix | No | Yes |
| [Missing dialogue entries](#missing-dialogue-entries) | Improvement | No | Yes |
| [Inventory model textures](#inventory-model-textures) | Improvement | No | Yes |
| [String-only dialogue with subtitles off](#string-only-dialogue-with-subtitles-off) | Demo support | No | Yes |
| [`[RTX_PATCH]` substitutions](#rtx_patch-substitutions) | Demo support | No | Only when configured |
| [Demo lock](#demo-lock) | Demo restriction | No | Only when `demo_version` is set |
| [Diagnostics](#diagnostics) | Diagnostic | No | Yes |

Bug fixes correct build 1.0 behavior that is wrong in the retail game.
Improvements change working behavior. Demo support exists for the demo's
text-only dialogue file, and the demo restriction confines play to the island.

### Bug fixes

#### Rope attachment while dying

The rope and hook attachment test also requires that the player is not dead.
The flag it reads is set when health reaches zero and the death animation
starts. In build 1.0 a dying player can still attach to a rope.

#### Ledge-fall stutter

When a fall off a ledge begins, the actor is also moved up to 4096 position
units in its last movement direction, away from the edge.

#### Object-flat textures

A flat's texture ID is resolved when the flat is created: bits 7–15 select the
[TEXBSI](formats/TEXBSI.md) file and bits 0–6 the image. An ID that cannot be
resolved is a fatal error. A texture group that is not yet loaded is loaded at
full resolution. Build 1.0 stores only the raw ID and leaves the resolved index
at zero; its flat renderer stops with the fatal error `Bad Texture Index %d` when
the resolved index is zero or above 30000. Build 5.0 removes that check.

#### Animation slots

Animation selection scans the actor's animation group table. In the ordered
scan, a slot whose group byte is zero no longer matches, so a request for group
zero selects nothing. The random pick skips a slot whose entry pointer is null
instead of reading through it.

#### String-only dialogue with speech on

An RTX entry without audio is marked as having no voice when it is played.
With `[dialog] dialog_use_speech=1`, such an entry is then shown for
`characters × 15` units, the same timing as a line with speech off. Build 1.0
neither draws nor frees a string-only entry while speech is on (see
[subtitle timing](engine/subtitles.md#timing)). `ENGLISH.RTX` contains 933
string-only entries.

### Improvements

#### Missing dialogue entries

A missing RTX entry logs `ERROR: GetRtx( %c%c%c%c ), RTX NOT found` to
`GENERAL.LOG` and plays entry `cbu1` ("DON'T SUPPOSE YOU KNOW WHAT'S GOING
ON.") instead. Build 1.0 stops with a fatal error. A missing `cbu1` is still
fatal.

#### Inventory model textures

Item models load their textures at full resolution:
[`[3dfx] text_scale`](config/system-ini.md#3dfx) is overridden with `1, 1`
while the item system loads its models and restored afterwards.

### Demo support

#### String-only dialogue with subtitles off

A string-only entry is drawn even with `dialog_print_text=0`, so its text
remains readable when it has no voice. Voiced entries follow the setting.

#### `[RTX_PATCH]` substitutions

[`[RTX_PATCH]`](config/system-ini.md#demo-executable-keys) substitutes up to 15
entries by name. The section is absent from the retail `SYSTEM.INI`.

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

### Diagnostics

- **F11** toggles an overlay that shows the current dialogue entry as
  `(%d) RTX %c%c%c%c`. Build 1.0 contains the overlay but never calls it.
- [`[3DFX] warnings`](config/system-ini.md#demo-executable-keys) enables two
  texture-manager warnings.
- A missing texture archive prints `Missing Texture File: FXART\TEXBSI.%03d`
  before the fatal error.
- `COMBAT.LOG` is reset at startup with the other log files. No code writes to
  it.

## External References

- [DOS Games Archive: `rgdemo.zip`](https://www.dosgamesarchive.com/file/the-elder-scrolls-adventures-redguard/rgdemo/)
- [Internet Archive: The Elder Scrolls Adventures: Redguard Demo](https://archive.org/details/REDGUARDDemo)
