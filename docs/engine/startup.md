# Startup presentation

Glide startup presents the active Glide implementation's 3dfx splash and
`system/STARTUP2.GXA`. The splash is supplied by Glide, rather than by a
Redguard movie file.

Startup plays the Introduction cinematic before loading the introductory game
world. New Game from the book repeats this movie-before-world sequence. The book is not necessarily the first game presentation after the
title. A book opened from a running world uses the in-game entry context
described in [Book menu](menu.md).

The title image is a single 640×480 frame. The original also draws small
version and loading labels along its bottom edge.

## Title labels

The title redraw selects the `[system] gui_low_font` entry from `SYSTEM.INI`.
The GOG configuration points to `fonts\\arialvb.fnt`, a 9-pixel line-height
font. The version label starts at `(10,467)` in RGB `(32,32,32)`. Its format is
`Version %s.%s`; the GOG release displays `Version 1.0.1.0`. Loading
text is right-aligned to `(630,467)` in RGB `(50,0,0)`. The map-registration
stage supplies `Loading Map %d`. Sound preparation supplies `Loading Sound`.

Glyph bitmaps use their FNT left and top offsets. Each active character
advances by its bitmap width plus one pixel. Space additionally advances by
the width of the next glyph, the exclamation mark. Text-width measurement
uses the same advances. These rules follow the title caller, text-width
routine, and glyph-drawing path, independently of the font-file fields.

## Splash variants

The full DOS `glide2x_emu.ovl` contains the older "3Dfx Interactive" logo on a
white shield with blue and yellow borders. DOSBox-X Voodoo emulation runs this
hardware-facing overlay and displays that animation.

The small DOS forwarding `glide2x.ovl` routes Glide calls to a host Windows
wrapper. A wrapper using `3DfxSpl2.dll` displays the later metallic lowercase
3dfx logo with an orange swoosh. The DLL's 1999 branding postdates Redguard's
1998 release. The GOG install includes both paths, so the active Glide
implementation determines the splash. OpenGLide's `NoSplash` and nGlide's
splash setting can also suppress driver startup presentation.

The [DLL resource](../formats/GLIDE-SPLASH-DLL.md) contains authored geometry,
textures and commands for the later animation. The older animation remains
available in the [DOS overlay tables](../formats/GLIDE-SPLASH.md). These assets
are separate from the Smacker New Game introduction.

## Windows splash rendering

The DLL command stream supplies the camera, horizontal FOV, model transforms,
material selection, and positional light. Multiply each model transform by
the camera transform. Negate the resulting position and normal Z components,
then normalize the normal. Projection uses the same scale on both axes,
`W / (2*tan(horizontal_fov/2))`, and centers the viewport. The Glide origin
is lower-left.

Geometry without flag bit 3 uses its authored UV pairs multiplied by 256.
Geometry with that flag intersects the positive normal ray from the transformed
vertex with a sphere of radius 400 centered at the view origin. The far
intersection supplies S/T as `(intersection.xy/400 + 1) * 204.8`.
Texture coordinates are perspective-correct. Textures use bilinear filtering.

For a normalized light direction L and normal N, diffuse is `max(dot(N,L),0)`.
The reflection direction is `normalize(2*N*dot(N,L)-L)`. Specular is the eighth
power of its nonnegative dot product with the normalized view direction.
The view direction is the camera matrix's third column with negated Z.

One lighting contribution uses direction `(1,1,-1)/sqrt(3)`; the other uses
the authored positional light minus the transformed vertex. Beyond a squared
light distance of 4,000,000, the positional contribution is multiplied by
`(4,000,000/distance_squared)^4`. Vertex RGB, in byte units, is:

```text
RGB = clamp(255*positional_RGB*(positional_diffuse + positional_specular)
            + 255*(directional_diffuse + directional_specular) + 16, 0, 255)
```

During full playback, texture RGB multiplies interpolated vertex RGB.
Constant material RGB is used by the single-frame drawing path, which bypasses
textures. That diagnostic path therefore differs from full animation playback.
Triangles cull by winding. Polygon edges are drawn through Glide's antialiased
line path after pending edge groups flush.

The timed-frame command overlays black before presenting. Its alpha is 1 for
frame counters below 24, then `1-(frame-24)/8` for counters 24 through 31.
Frames from counter 32 onward have no overlay. After presenting, both color and
depth are cleared for the next frame. The final untimed redraws precede a fade
of `2*frame_rate` swaps, each waiting two refresh intervals. Fade alpha starts
at zero, then uses the truncated byte value `102*step/(2*frame_rate)`.
The black overlay accumulates separately on the two alternating buffers.

## DOS overlay splash

The [Glide splash tables](../formats/GLIDE-SPLASH.md) contain the geometry,
textures, and authored transforms. No executable code needs to run to extract
or display the animation.

The full animation renders authored frames 1 through 74. Frame zero is not
part of that playback. Each mesh's position uses its column-major affine
matrix; its normal uses the linear part without translation or normalization.

With a viewport of width W and height H, a transformed point `(x,y,z)` maps to:

```text
screen_x = (x/z) * 0.75W + W/2
screen_y = (y/z) * H + H/2
```

The splash uses a lower-left origin. Texture coordinates are perspective
correct, with 256 S units across the longest texture dimension. A 2:1 texture
uses 128 T units. The renderer enables bilinear filtering and selects the
nearest mip level.

Vertex lighting uses `L = (-0.57735, -0.57735, -0.57735)` and
`brightness = (dot(L, transformed_normal) + 1) / 2`.

| Material | Appearance |
|---|---|
| 0 | Logo color texture multiplied by vertex brightness; additive highlight pass |
| 1 | Highlight texture sampled using the transformed normal; vertex brightness does not modulate the sampled color |
| 2 | Vertex brightness multiplied by RGB `(10,75,120)/255` |
| 3 | Vertex brightness, equal RGB channels |
| 4 | Vertex brightness multiplied by RGB `(248,204,0)/255` |

Highlight S/T coordinates are `(128 + 128*normal.x, 128 + 128*normal.y)`.
Material 1's intensity-factor table contains 0.125 in each channel, but the
texture-only color combiner bypasses that factor during full animation.

The outer shield and shield render first without depth writes. A projected
shadow multiplies their existing colors, with fog disabled for that pass. The
logo then renders with depth testing and writes enabled; its material-0
highlight adds to the existing logo color. Triangles use winding culling and
face-selected edge antialiasing.

The fade is black fog on geometry. With 75 frames, fade-in spans the first 22
frames: before frame 22, visibility is the truncated 8-bit value of
`255*(frame+1)/22`, divided by 255. After frame 60, visibility is the truncated
8-bit value of `255*(75-frame)/15`, divided by 255. Other frames have full
visibility. The background remains black.

## Projected shadow

The shadow quad's object-space corners are `(-280,0,-160)`, `(-280,0,150)`,
`(280,0,150)`, and `(280,0,-160)`, with S/T pairs `(10.5,127.5)`, `(10.5,0.5)`,
`(255,0.5)`, and `(255,127.5)`. Transform these corners with the logo matrix.
The light is `(5,300,shield_translation_z - 1500)`; project the light-to-corner
lines onto `z = shield_translation_z - 26`. Screen projection is the same as
for geometry. Two triangles cover the quad, submitted in both windings.

S and T are stored multiplied by the projected point's reciprocal Z. The
per-TMU reciprocal-W field additionally contains the light-to-projected-point
distance divided by Z.

## External references

- [Glide splash renderer](https://github.com/sezero/glide/blob/master/glide2x/sst1/glide/src/gsplash.c)
- [Glide splash data declarations](https://github.com/sezero/glide/blob/master/glide2x/sst1/glide/src/splshdat.c)

## World loading screen

World entry uses the destination's `WORLD.INI` `world_flash_filename` GXA.
An archive with several frames supplies a randomly selected background; a
single-frame archive uses its only frame. The full-screen picture uses its
own palette. New-game entry uses `start_world` and `start_marker` from the
`[world]` section.

At the 640 by 480 logical viewport, the progress bar starts at `(32,448)`,
is 576 pixels wide and 6 pixels high. Its background is black and its fill
is VGA RGB `(63,0,0)`. Fill width is the nearest-integer product of the bar
width and the completed fraction. The displayed integer percentage also
rounds to nearest. Percentage updates advance monotonically; a zero-percent
update does not draw the bar or label.

The percentage label uses `FONTS/REDSEL.FNT` at `(32,451)`, with foreground
alpha 255 and shadow alpha 180. The ordinary label is `%d%%`; debug output
can include world indices before it. These are world-entry loading visuals,
separate from the startup title's small bottom-edge labels. World loading
does not display a right-aligned status message or an asset filename.

New-game entry fades the loading screen in from black, fades it out to black
after loading, and then fades the first world view in from black. Each fade
step changes a black overlay by 12 out of 256.
