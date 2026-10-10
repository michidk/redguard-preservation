# Sky Renderer

Outdoor sky configuration and the Glide renderer's BSI sky plane and sun disc.

## Overview

In the shipped Glide build, the sky uses `world_skyfx[N]` and a
matching `.COL` file. The texture is projected onto a horizontal quad before
scene geometry. A separate billboard draws the sun. The GXA file named by
`world_sky[N]` is not loaded by this renderer's sky setup.

**Unknown:** the software renderer's GXA projection. A GXA panorama must not be
interpreted as an equirectangular map without evidence for that renderer.

## Lifecycle

| Phase | Trigger | Action |
|---|---|---|
| Load | World open | Engine reads per-world sky keys from `WORLD.INI`, then calls the sky system opener with the BSI texture filename |
| Render | Every frame in the main loop | Sky layers are drawn before terrain and scene geometry |
| Close | Session close | Sky textures are released and the system is shut down |

## Background Fill

`world_background[N]` selects these legacy background modes in `WORLD.INI`.
**Unknown:** how the special sky-color mode is derived and whether these modes
affect the Glide sky path.

| Value | Behavior |
|---|---|
| `0` | Black |
| `2` | Sky color (derived from the palette) |
| Other | Palette index used as a solid fill color |

## GXA assets

`world_sky[N]` names a GXA asset in `system/`. These files exist in the install,
but their projection and use by the software renderer are **Unknown**. The
Glide sky is supplied by the BSI asset instead.

## Glide sky plane

The shipped Glide build uses the following geometry and texture mapping.
Coordinates use engine units, with positive Y pointing down.

- The plane is centered horizontally on the camera. Its four relative X/Z
  corners are `(-65000, 65000)`, `(65000, 65000)`, `(65000, -65000)`, and
  `(-65000, -65000)`.
- Its absolute Y is `world_skylevel[N]`. Zero or omitted selects `-3500`.
- `world_skyscale[N]` is the half-span of texture coordinates in texels. Zero
  or omitted selects `2000`. This is not the sun disc's default scale.
- For camera position `(cx, cy, cz)`, the plane's relative height is
  `world_skylevel[N] - cy`. Initial texture-coordinate centers are
  `(0.009 * cx, -0.009 * cz)`.
- For relative point `(x, z)` on the plane and scroll offset `(su, sv)`, the
  texture coordinates in texels are
  `(scale * x / 65000 + 0.009 * cx + su,
  -scale * z / 65000 - 0.009 * cz + sv)`.
  Divide by image width and height for normalized coordinates and repeat the
  texture. The sky draw uses near/far distances of `1` and `65000`.
- The BSI file is a standalone chunk stream, not a TEXBSI archive record.
  Its matching `.COL` file supplies the colors independently of the world's
  terrain/model palette. Sky sampling is opaque, including palette index 0.

Glide renderer initialization selects bilinear minification and
magnification on texture unit 0. The sky texture binding and draw path do not
change the filter mode. **Inferred:** the initial sky uses this bilinear setting;
filter-state interactions with other draw paths have not been compared visually.

The installed samples `SKY888.BSI`, `SKY899.BSI`, `SKYNIT.BSI`, and
`SKYNEC.BSI` each contain one 256 by 256 frame and have matching COL files.
The seven outdoor world entries are 0, 1, 6, 14, 27, 28, and 30.

### Scrolling and rotation

Raw `world_skyspeed[N]` is an unsigned byte converted to texels
per timer tick by dividing by 16. The initial scrolling direction is selected
randomly from 2048 angle steps. Scroll offsets start at zero, advance with the
shared engine sine/cosine table, and wrap at plus/minus 256 texels. A frame
processes at most 36 elapsed ticks. The plane can also tilt around a
camera-relative axis.

**Unknown:** a reproducible initial random state for a particular saved scene,
the rotation controls' complete mapping, and the software-renderer behavior.
A static initial sky frame is useful for checking asset selection and projection;
it does not verify sky movement or sun rendering.

## Global Engine Toggles

The `[xngine]` section of `SYSTEM.INI` provides master controls that apply to all worlds:

| Key | Default | Description |
|---|---|---|
| `sky_disable` | `0` | Disable sky rendering entirely. `0` = enabled. |
| `sky_move` | `1` | Enable sky scrolling. `0` = frozen. |
| `sky_xrotate` | `3` | Global X-axis rotation speed |
| `sky_yrotate` | `40` | Global Y-axis rotation speed |

**Unknown for Glide:** the effect of the per-world `world_sky_xrotate` and
`world_sky_yrotate` keys. Their presence in WORLD.INI does not establish that
the Glide renderer applies them.

## Sun disc

A separate billboard renders the sun as a textured sprite in the sky, independent of the BSI sky plane:

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

Seven outdoor world entries define sky parameters, including world 14. All others are indoor/dungeon locations with no sky.

| World | Location | Sky Features |
|---|---|---|
| 0 | Starting hideout (exterior) | Sunset BSI sky |
| 1 | Stros M'Kai island (daytime) | Daytime BSI sky |
| 6 | Necromancer's Isle | Necromancer BSI sky and rain configuration |
| 14 | Island alternate entry | Daytime sky texture |
| 27 | Island (night variant) | Night BSI sky (`SKYNIT.COL` palette) |
| 28 | Island (sunset variant) | Sunset BSI sky (`sunset.COL` palette) |
| 30 | Palace exterior | Sunset BSI sky (shares island WLD) |

Worlds 1, 27, and 28 share the same `ISLAND.WLD` terrain and PVO node maps. The visual difference is driven by different palettes, BSI sky textures, and lighting parameters — demonstrating that time-of-day in Redguard is implemented as separate world entries rather than dynamic sky transitions.

## External References

- [WORLD.INI documentation](../config/world-ini.md) — per-world sky, sun disc, and background fill keys
- [SYSTEM.INI documentation](../config/system-ini.md#xngine) — global sky engine toggles in the `[xngine]` section
- [UESP: Redguard Console](https://en.uesp.net/wiki/Redguard:Console) — `show world` command displays sky parameters at runtime
