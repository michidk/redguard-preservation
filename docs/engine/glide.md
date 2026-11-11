# Glide interface

`RGFX.EXE`, the Glide build of the engine, renders through the 3dfx Glide 2
library and contains no Glide implementation of its own.

## Library binding

The executable reaches Glide through the DOS overlay interface. It carries one
five-byte stub per Glide 2 entry point, 124 in all, and a parallel table of the
decorated entry-point names, such as `_GRBUFFERSWAP@4`. The first call through
any stub loads the `glide2x` overlay; each stub binds its entry point by name
when it is first called. The 124 names are the complete public Glide 2.4x
interface.

The game never asks Glide for its version. Any Glide 2 overlay that exports the
called entry points can run it. The game CD contains no Glide implementation,
so the player's installed 3dfx driver supplied the overlay.

## Implementations in the GOG release

| File | Implementation |
|---|---|
| `DOSBOX/glide2x_emu.ovl` | 3dfx Glide 2.42 DOS overlay with Voodoo Graphics (SST-1) and Voodoo Rush (SST-96) initialization. It drives the hardware directly and runs on emulated Voodoo hardware. |
| `DOSBOX/glide2x.ovl` | Small forwarding overlay that passes Glide calls to the host through DOSBox's Glide port. |
| `DOSBOX/Glide2x.dll` | OpenGLide 0.09rc9 host wrapper, configured by `DOSBOX/glide.ini`. |
| `Redguard/glide2x.dll` | nGlide host wrapper. |

[Startup presentation](startup.md#splash-variants) describes which path the GOG
launch uses and which splash each one shows.

## Called entry points

The executable calls 38 of the 124 entry points.

| Group | Entry points |
|---|---|
| Startup and shutdown | `grGlideInit`, `grSstQueryHardware`, `grSstSelect`, `grSstWinOpen`, `grGammaCorrectionValue`, `grGlideShutdown` |
| Buffers | `grRenderBuffer`, `grBufferClear`, `grBufferSwap`, `grColorMask`, `grLfbLock`, `grLfbUnlock` |
| Depth | `grDepthBufferMode`, `grDepthBufferFunction`, `grDepthMask`, `grDepthBiasLevel` |
| Textures | `grTexSource`, `grTexDownloadMipMap`, `grTexDownloadTable`, `grTexMinAddress`, `grTexMaxAddress`, `grTexFilterMode`, `grTexClampMode`, `grTexMipMapMode`, `guTexCombineFunction` |
| Color and alpha | `guColorCombineFunction`, `guAlphaSource`, `grConstantColorValue`, `grAlphaBlendFunction`, `grChromakeyMode`, `grChromakeyValue`, `grDitherMode` |
| Fog | `grFogMode`, `grFogTable`, `grFogColorValue` |
| Drawing | `grDrawTriangle`, `grDrawLine`, `grDrawPoint` |

The game draws only independent triangles, lines and points. It uses none of
Glide's antialiased, polygon or clipping draw calls, and none of the
`guFogGenerate*` helpers: it builds its own [fog tables](fog.md#fog-lookup).
It sets texture and color combination only through the `gu` convenience
functions, never through `grTexCombine`, `grColorCombine` or `grAlphaCombine`.
Glide's texture memory manager, 3DF loader, culling, clip window and alpha test
are unused. Linear framebuffer access uses `grLfbLock` and `grLfbUnlock`, never the region
read and write calls.

## Window and initial state

Opening the Glide window requires `grSstQueryHardware` to report at least one
board; otherwise Glide startup fails. The engine selects board 0 and opens the
window at the configured resolution with these parameters:

| Parameter | Value |
|---|---|
| Refresh | 60 Hz |
| Color format | ARGB |
| Origin | Upper left |
| Color buffers | 2 |
| Auxiliary buffers | 1 |

It then sets this state:

| State | Value |
|---|---|
| Render buffer | Back buffer |
| Depth buffer | W-buffer, `GR_CMP_LESS`, depth writes enabled |
| Texture clamp (TMU 0) | Wrap in S and T |
| Mipmapping (TMU 0) | Disabled, no LOD blending |
| Texture filter (TMU 0) | Bilinear minification and magnification |
| Texture combine (TMU 0) | Decal |
| Color combine | Texture times iterated RGB |
| Alpha source | Constant-color alpha |
| Dither | 4 by 4 |
| Fog | Disabled |

Each drawing path changes this state for its own draws. The
[shading overview](shading.md#detailed-rules) links the per-path rules, and the
[HUD](hud.md) lists the state its overlay sets and restores.

## External References

- [Glide 2 API constants](https://github.com/sezero/glide/blob/master/glide2x/sst1/glide/src/glide.h)
- [Glide 2 video modes and refresh rates](https://github.com/sezero/glide/blob/master/glide2x/sst1/init/sst1vid.h)
- [Glide 2 state functions](https://github.com/sezero/glide/blob/master/glide2x/sst1/glide/src/gglide.c)
- [Glide 2 convenience functions](https://github.com/sezero/glide/blob/master/glide2x/sst1/glide/src/gu.c)
