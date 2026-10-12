# Windows Glide splash resource

`3DfxSpl2.dll` contains the animated metallic 3dfx logo with an orange swoosh in its PE `ANIM` resource.

This is a Glide driver asset, separate from the game's Smacker cinematics and
from the older shield animation in the [DOS overlay](GLIDE-SPLASH.md).
The DLL's version metadata identifies the splash as a 1999 3dfx asset. The
GOG copies with file versions `1.00.00.0004` and `1.00.00.0005` have identical
`ANIM` resource payloads.

## Overall Structure

```text
[PE32 resource directory]
  [ANIM / 101 / language]
    [Version line]
    [Animation frame count and frame rate]
    [Texture records and mip chains]
    [Material groups]
    [Stored geometry slots and records]
    [Animation command stream]
  [PLUG / 107 / language]
    [Static plug image in TGA format]
```

The resource directory supplies the payload's RVA and length. Resource RVAs
map through the PE section table; they are not file offsets. No DLL code needs
to execute to extract the resources. `ANIM` starts with the newline-terminated
ASCII line `3Dfx Splash Version 1.2`, followed by little-endian `u32` frame
count and frame rate. The supplied animation has 90 timed frames at 30 fps.

## Texture records

A `u32` count precedes the textures. Each record has five `u32` fields:
small LOD, large LOD, aspect ratio, Glide format, and an unused serialized
pointer. A `u32` byte length and that many mip-chain bytes follow. The loader
uses the large LOD as both ends of the uploaded range.

The two supplied textures use square RGB565 data, with largest levels of
128×128 and 256×256. Large LOD values 1 and 0 select those dimensions;
small LOD is 8, aspect ratio is 3, and format is 10. Mips descend to 1×1.
RGB565 stores red in bits 11–15, green in bits 5–10, and blue in bits 0–4.

## Materials

A `u32` group count precedes groups. Each group begins with a `u32` material
count, then records of six floats and a `u32` texture index. The first three
floats are RGB factors; the next three are additional material factors. A
texture index of `0xffffffff` means no texture. The supplied animation uses
two groups, each containing one material, referring to textures 0 and 1.

## Stored geometry

A `u32` slot count precedes indexed records. Each starts with a `u32` slot ID,
then flags, triangle count, triangle indices, vertex count, vertices, texture
coordinate count, and texture coordinates. Slot ID `0xffffffff` terminates
the records. The supplied animation fills all 74 slots.

Triangle indices are three `u32` values per triangle. Vertices contain three
position floats and three normal floats. Texture coordinates are pairs of
floats. Only pairs corresponding to existing vertices are assigned; remaining
pairs are consumed and discarded. Geometry flag bit 3 selects generated
sphere-map coordinates instead of these authored coordinates.

## Commands

Commands have a one-byte opcode followed by an unaligned payload. All scalar
payloads are little-endian; matrices contain 16 column-major floats.

| Opcode | Payload | Meaning |
|---|---|---|
| 2 | None | Present a timed frame and advance the frame counter |
| 3 | Two `u32`, matrix | Draw a stored geometry slot; the second ID groups edge processing |
| 4 | Float, matrix | Horizontal field of view in radians and camera transform |
| 5 | None | Fade out and end playback |
| 6 | `u32` | Select a material group |
| 7 | Seven floats | Set directional RGB and position inputs; the fourth float is skipped |
| 8 | Byte | Update the auxiliary toggle |
| 9 | `u32` | Set alpha from the high byte; 0 and `0xff000000` select opaque |
| 11 | None | Start the playback clock and reset frame state |
| 12 | None | Flush pending edges without advancing the clock |
| 13 | None | Swap buffers without advancing the clock |
| 14 | Seven floats | Set positional RGB and light position; the fourth float is skipped |

The supplied resource uses these opcodes. The last timed frame is followed by
two final redraws, a buffer swap, and the fade command. See
[startup presentation](../engine/startup.md) for the rendering rules.

## External References

- [Microsoft PE resource directory](https://learn.microsoft.com/en-us/windows/win32/debug/pe-format#the-rsrc-section)
- [Glide DOS splash renderer](https://github.com/sezero/glide/blob/master/glide2x/sst1/glide/src/gsplash.c)
