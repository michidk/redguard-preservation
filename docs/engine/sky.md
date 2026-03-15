# Sky Renderer

Two-layer sky rendering system used by the xngine engine for outdoor environments. Combines a static GXA skybox texture with an optional scrolling BSI texture layer, plus a separate sun disc billboard.

## Overview

The sky system is initialized when a world is loaded and torn down when the session closes. Each frame, the engine renders up to three sky elements in order:

1. **Background fill** — a solid color behind everything
2. **GXA skybox** — a static panoramic texture
3. **BSI scrolling layer** — an animated texture scrolling over the skybox

Only outdoor worlds (those with a [WLD](../formats/WLD.md) terrain mesh) use the sky system. Indoor and dungeon worlds have no sky; their background is a solid fill color.

## Lifecycle

| Phase | Trigger | Action |
|---|---|---|
| Load | World open | Engine reads per-world sky keys from `WORLD.INI`, then calls the sky system opener with the BSI texture filename |
| Render | Every frame in the main loop | Sky layers are drawn before terrain and scene geometry |
| Close | Session close | Sky textures are released and the system is shut down |

## Background Fill

The `world_background[N]` key sets what is drawn behind the sky layers:

| Value | Behavior |
|---|---|
| `0` | Black |
| `2` | Sky color (derived from the palette) |
| Other | Palette index used as a solid fill color |

## GXA Skybox Layer

The `world_sky[N]` key points to a GXA file in the `system/` directory. This is a static panoramic texture that wraps the horizon. Only outdoor worlds define this key.

The GXA skybox provides the base sky appearance — cloud formations, horizon gradient, and sky color. Time-of-day variants (day, sunset, night) are achieved by loading different worlds that share the same terrain but use different GXA textures and palettes.

## BSI Scrolling Layer

The `world_skyfx[N]` key names a BSI texture that scrolls on top of the skybox. Four parameters control its behavior:

| Key | Description | Default |
|---|---|---|
| `world_skyscale[N]` | Size/scale of the scrolling texture | `0x3200` (12800) if 0 or omitted |
| `world_skylevel[N]` | Vertical offset (negative = below horizon) | `0xFFFFF254` (−3500) if 0 or omitted |
| `world_skyspeed[N]` | Scroll speed | 0 |
| `world_sky_xrotate[N]` | Rotation rate around the X axis | 0 |
| `world_sky_yrotate[N]` | Rotation rate around the Y axis | 0 |

The scrolling layer creates the appearance of moving clouds or atmospheric effects. Rotation parameters allow the sky to slowly rotate, used on Necromancer's Isle (world 6) for its unsettling spinning-sky effect.

## Global Engine Toggles

The `[xngine]` section of `SYSTEM.INI` provides master controls that apply to all worlds:

| Key | Default | Description |
|---|---|---|
| `sky_disable` | `0` | Disable sky rendering entirely. `0` = enabled. |
| `sky_move` | `1` | Enable sky scrolling. `0` = frozen. |
| `sky_xrotate` | `3` | Global X-axis rotation speed |
| `sky_yrotate` | `40` | Global Y-axis rotation speed |

Per-world `world_sky_xrotate` / `world_sky_yrotate` values override these globals for that world.

## Sun Disc

A separate billboard renders the sun as a textured sprite in the sky, independent of both sky layers:

| Key | Description |
|---|---|
| `world_sunimg[N]` | BSI texture for the sun disc |
| `world_sunimgrgb[N]` | Tint color (r, g, b) applied to the sun texture |
| `world_sunscale[N]` | Size scale of the sun disc |

The sun disc position is derived from the world's sun direction vector (`world_sun[N]`) and sun angle/skew parameters. It is a visual element only — the lighting system uses the sun direction independently.

## Console Commands

The developer console (F12) exposes runtime sky adjustment:

| Command | Alias | Description |
|---|---|---|
| `fxskyscale <value>` | `skysc` | Set the BSI layer scale |
| `fxskylevel <value>` | `skyl` | Set the BSI layer vertical offset |
| `fxskyspeed <value>` | `skysp` | Set the BSI layer scroll speed |

The `show world` console command displays current sky parameters in the on-screen debug overlay, including the sky texture name, scale, level, and speed.

## World Sky Assignments

Of the 31 shipped worlds, only 6 outdoor worlds define sky parameters. All others are indoor/dungeon locations with no sky.

| World | Location | Sky Features |
|---|---|---|
| 0 | Starting hideout (exterior) | Sunset skybox |
| 1 | Stros M'Kai island (daytime) | Daytime skybox + scrolling clouds |
| 6 | Necromancer's Isle | Skybox + rotating BSI layer + rain weather |
| 27 | Island (night variant) | Night skybox (`nightsky.COL` palette) |
| 28 | Island (sunset variant) | Sunset skybox (`sunset.COL` palette) |
| 30 | Palace exterior | Sunset skybox (shares island WLD) |

Worlds 1, 27, and 28 share the same `ISLAND.WLD` terrain and PVO node maps. The visual difference is entirely driven by different palettes, sky textures, and lighting parameters — demonstrating that time-of-day in Redguard is implemented as separate world entries rather than dynamic sky transitions.

## External References

- [WORLD.INI documentation](../config/world-ini.md) — per-world sky, sun disc, and background fill keys
- [SYSTEM.INI documentation](../config/system-ini.md#xngine) — global sky engine toggles in the `[xngine]` section
- [UESP: Redguard Console](https://en.uesp.net/wiki/Redguard:Console) — `show world` command displays sky parameters at runtime
