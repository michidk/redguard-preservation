# BSI texture file format

Standalone image chunks used by named textures such as sky and sun images.

## Overall structure

Unlike a [TEXBSI archive](TEXBSI.md), a standalone BSI has no 9-byte image name
or record-size envelope. It begins directly with tagged chunks:

```
[IFHD or BSIF — image marker]
[BHDR — image header]
[CMAP — optional embedded palette]
[DATA — indexed pixels or animation rows]
[END — terminator]
```

Chunk headers and BHDR fields follow [TEXBSI](TEXBSI.md#subrecord-structure).
`IFHD` alone does not imply multiple frames. **Verified:** the four installed
sky textures have `IFHD`, a frame count of 1, and raw 256 by 256 indexed pixels.
The Glide sky uses the matching external COL palette, including index 0 as
an opaque color. General sprite decoding treats index 0 as transparent.

## External References

- [TEXBSI chunk and pixel specifications](TEXBSI.md)
- [Glide sky texture selection and projection](../engine/sky.md)
