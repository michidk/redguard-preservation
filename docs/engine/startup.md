# Startup presentation

The GOG DOS Glide startup presents the animated 3dfx splash, `system/STARTUP2.GXA`, then the book-style main menu.

The New Game introduction is separate from this startup sequence. The title
image is a single 640×480 frame. The original also draws small version and
loading labels along its bottom edge.

## Title labels

The title redraw selects the `[system] gui_low_font` entry from `SYSTEM.INI`.
The GOG configuration points to `fonts\\arialvb.fnt`, a 9-pixel line-height
font. The version label starts at `(10,467)` in RGB `(32,32,32)`. Its format is
`Version %s.%s`; the observed GOG release displays `Version 1.0.1.0`. Loading
text is right-aligned to `(630,467)` in RGB `(50,0,0)`. The map-registration
stage supplies `Loading Map %d`.

Glyph bitmaps use their FNT left and top offsets. Each active character
advances by its bitmap width plus one pixel. Space additionally advances by
the width of the next glyph, the exclamation mark. Text-width measurement
uses the same advances. These rules follow the title caller, text-width
routine, and glyph-drawing path, independently of the font-file fields.

## 3dfx splash

The [Glide splash tables](../formats/GLIDE-SPLASH.md) contain the geometry,
textures, and authored transforms. The data arrays were verified against the
installed overlay; the overlay's drawing path was inspected independently of
the published Glide implementation. No executable code needs to run to extract
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
distance divided by Z. The effective use of that extra field by the installed
single-TMU path still needs verification. Hardware edge coverage and pixel
quantization also remain open before claiming pixel-exact rendering.

## External references

- [Glide splash renderer](https://github.com/sezero/glide/blob/master/glide2x/sst1/glide/src/gsplash.c)
- [Glide splash data declarations](https://github.com/sezero/glide/blob/master/glide2x/sst1/glide/src/splshdat.c)
