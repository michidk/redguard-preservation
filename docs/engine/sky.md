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

The shipped Glide build selects `world_fogrgb[N]` as the RGB color used to
clear the next back buffer after presenting a normal game frame. An omitted
value defaults to `(0, 0, 0)`. The finite sky plane draws over this fill;
uncovered regions retain the fog color, including when the sky is disabled.

Installed samples: world 0 uses `(200, 100, 100)`, world 1 uses
`(200, 190, 250)`, world 6 uses `(57, 49, 49)`, world 27 uses `(0, 0, 10)`,
and world 28 uses `(200, 100, 100)`.

The shipped Glide world-settings reader does not read `world_background[N]`.
WORLD.INI describes these legacy background modes; their software-renderer
implementation and special sky-color derivation remain **Unknown**.

| Value | Behavior |
|---|---|
| `0` | Black |
| `2` | Sky color (derived from the palette) |
| Other | Palette index used as a solid fill color |

### Sky fog

The outdoor frame enables Glide table fog for the sky draw and restores the
scene fog table afterwards. The sky draw does not disable fog. Its 64-entry
table contains zero in entries 0 through 32, increases by eight from 8 through
240 in entries 33 through 62, and ends at 255 in entry 63.

The sky raster supplies reciprocal camera-space depth to Glide. The
[Glide fog lookup](fog.md#fog-lookup) describes table interpolation and RGB
blending. A raw texture projection and correct clear color alone do not
establish the original horizon's appearance.

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
per BIOS timer tick by dividing by 16. The timer runs at
`1193182 / 65536` ticks per second. The initial scrolling direction is selected
randomly from 2048 angle steps. Scroll offsets start at zero, advance with the
shared engine sine/cosine table, and wrap at plus/minus 256 texels. A frame
processes at most 36 elapsed ticks. The plane tilts around a camera-relative axis as described below.

### Plane tilt

The sky is a plane that can be crossed vertically and is visible from either
side. It is not an enclosing cube or sphere.

The initial tilt is 24 of the engine's 2048 angle units, approximately 4.21875
degrees. It is a fixed angle, not an animation rate. The tilt setter masks its
input to 11 bits.

Let `h` be the normalized horizontal camera-forward direction in engine
coordinates. For an unrotated plane point with horizontal position `p`, let
`q = dot(p, h)` and `a = 24 * 6.2831852 / 2048`. Its tilted horizontal position
is `p + (cos(a) - 1) * q * h`, and its relative vertical position is
`sin(a) * q + world_skylevel - camera_y`. Texture coordinates stay attached to
the unrotated corners. The camera-relative tilt is applied before adding the
height, not around the world origin.

Transform samples before adding height, with the camera facing positive Z:
`(-65000, 0, 65000)` becomes approximately
`(-65000, 4781.6963, 64823.8789)`. Facing positive X, the same corner becomes
approximately `(-64823.875, -4781.6963, 65000)`.

### Initial scroll direction

The sky consumes one value from the engine's shared random stream and keeps its
low 11 bits as the angle. The stream updates its unsigned 32-bit state with
`state = state * 1103515245 + 12345`, wrapping at 32 bits; the returned value is
`(state >> 16) & 32767`. Therefore the sky direction is
`(updated_state >> 16) & 2047`. Reproducing a particular session requires the
state immediately before sky initialization or the resulting direction, not
just a world identifier. Other engine users also consume this stream.

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

The billboard center, relative to the camera, is the negative of
`(truncate(16000*sin(angle)), truncate(16000*cos(angle)),
truncate(16000*sin(skew)))`, using `world_sunangle` and `world_sunskew` in the
2048-step engine angle table. It does not use `world_sun` as its position.
The sprite faces the camera and remains the same angular size during camera
translation. Its near/far range is 1/65535 engine units.

Each billboard half-size is the corresponding BSI image dimension multiplied
by `0.5 * (tex_scale / 256) * (world_sunscale / 100)`. A zero or omitted sun
scale defaults to 12800. The installed `SUN001.BSI` is a single 256 by 256
frame with texture scale 256, so the installed sun scale 4000 gives half-size
5120 engine units. Missing sun image names suppress the disc.

The sun texture is bound as eight-bit alpha: its indexed pixel bytes supply
opacity, independently of its palette. Constant encoded RGB comes from
`world_sunimgrgb`, defaulting to `(255,255,200)`. The draw uses source-alpha /
one-minus-source-alpha blending and disables fog for the sun, restoring scene
fog afterwards. Scene geometry drawn later can cover it.

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
