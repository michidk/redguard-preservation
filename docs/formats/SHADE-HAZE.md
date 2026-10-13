# Software palette lookup tables

`SHADE.###` and `HAZE.###` files contain palette-index lookup tables for the software renderer.

## Overall Structure

```text
[Lookup rows — 64 consecutive rows of 256 palette indices]
```

Each file contains exactly 16,384 bytes, with no header or embedded palette. The entry for row `r` and input palette index `i` is at byte `r * 256 + i`. Its value is an output index into the world's [COL palette](COL.md). Preserve all rows and indices when importing a table.

These tables accompany the software [TEXTURE banks](TEXTURE.md).

## External References

- [UESP: Redguard file formats](https://en.uesp.net/wiki/Mod:Redguard_File_Formats)
