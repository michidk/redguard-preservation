# Glide splash tables

The GOG Glide overlay contains authored geometry, animation transforms, and textures for the animated 3dfx splash.

`rgpre::import::glide_splash::parse_glide_splash` extracts data from the player's
`DOSBOX/glide2x_emu.ovl`. It does not execute the overlay. This library API
supports the GOG LE table layout; other overlay versions and the
Windows `3DfxSpl2.dll` animation resource are unsupported.

## Overall Structure

```text
[DOS header and LE executable header]
[LE object table and page map]
[Initialized writable object]
  [Mesh pointers and vertex counts]
  [Face pointers and triangle counts]
  [Authored frame count and transforms]
  [Vertex and face arrays]
  [Texture descriptors, color tables, and mip chains]
```

The extractor reconstructs writable LE objects from their page maps. Table
pointers are relative to their data object. It identifies the supported layout
by its mesh counts and validates all pointer ranges, finite values, triangle
indices, material IDs, and edge flags. It rejects ambiguous matches.

## Mesh tables

There are three meshes, in shield, logo, outer-shield order:

| Mesh | Vertices | Triangles |
|---|---|---|
| Shield | 68 | 100 |
| Logo | 1694 | 1606 |
| Outer shield | 34 | 32 |

The table contains three vertex pointers, three vertex counts, three face
pointers, three face counts, then the frame count. All scalar values are
little-endian 32-bit words. There are 75 authored frames; each contains three
column-major 4×4 float matrices in mesh order.

A vertex occupies 32 bytes:

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| 0 | 12 | `[f32; 3]` | position | Object-space position |
| 12 | 12 | `[f32; 3]` | normal | Object-space normal |
| 24 | 8 | `[f32; 2]` | texture coordinates | Glide S/T coordinates |

A face occupies 20 bytes:

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| 0 | 12 | `[u32; 3]` | vertices | Indices into this mesh's vertex array |
| 12 | 4 | `u32` | material | Material ID, 0 through 4 |
| 16 | 4 | `u32` | edges | Three bits selecting antialiased triangle edges |

## Textures

The three texture blocks begin with width, height, small LOD, large LOD, aspect
ratio, and Glide texture format, as six `u32` values. A 1028-byte color-table
union follows, then the mip-chain byte length and pixel data. The largest mip
is first. The current API returns that mip as RGBA, without embedding assets
in the library.

| Texture | Dimensions | Glide format | Interpretation |
|---|---|---|---|
| Logo color | 64×64 | 1 | YIQ8 with an NCC color table |
| Highlight | 32×32 | 3 | Intensity8 |
| Shadow | 64×32 | 3 | Intensity8 |

Intensity8 produces `(intensity, intensity, intensity, 255)`. It is opaque
grayscale, not an alpha mask.

YIQ8 uses the first 64 bytes of the color-table union: 16 unsigned luma bytes,
then four signed three-channel I vectors and four signed three-channel Q
vectors, each component stored in a `u16` slot as a signed nine-bit value.
Sign extension uses bit 8: values 256 through 511 represent -256 through -1.
For pixel `p`, RGB is the clamped sum of
`Y[p >> 4] + I[(p >> 2) & 3] + Q[p & 3]`; alpha is 255.

See [startup rendering](../engine/startup.md) for transforms, materials, and
screen sequence.

## External References

- [Glide splash data declarations](https://github.com/sezero/glide/blob/master/glide2x/sst1/glide/src/splshdat.c)
- [Glide splash renderer](https://github.com/sezero/glide/blob/master/glide2x/sst1/glide/src/gsplash.c)
- [DOSBox-X Voodoo Glide integration](https://github.com/joncampbell123/dosbox-x/blob/master/src/hardware/voodoo_emu.cpp)
