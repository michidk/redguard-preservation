# CPF — Console preferences

`REDGUARD.CPF` stores the console settings written by `console save`.

## Overall Structure

```
[Signature — three ASCII bytes]
[Console options — eight 32-bit fields]
[Background palette index — one byte]
[Screen limit — one 32-bit field]
```

The file is 40 bytes. All multi-byte fields are little-endian.

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| 0x00 | 3 | `[u8; 3]` | signature | ASCII `2.0`, without a terminator |
| 0x03 | 4 | `u32` | font | Font selector; `console font` cycles zero and one |
| 0x07 | 4 | `u32` | shadow | Written as zero for disabled or one for enabled |
| 0x0B | 4 | `u32` | edit_color | GUI color index for editable text |
| 0x0F | 4 | `u32` | report_color | GUI color index for report text |
| 0x13 | 4 | `u32` | screen_color | GUI color index for screen reports |
| 0x17 | 4 | `u32` | paper_color | GUI color index for the solid background |
| 0x1B | 4 | `i32` | hex_lines | Number of lines in a hexadecimal dump |
| 0x1F | 4 | `u32` | paper_disabled | Zero selects paper artwork; one selects a solid background |
| 0x23 | 1 | `u8` | background | Palette index for the solid background |
| 0x24 | 4 | `i32` | screen_limit | Screen limit with eight fractional bits |

## Loading and commands

The loader requires the signature and reads the fields in file order. It raises
`hex_lines` to one when the stored value is below one. A zero shadow value disables
shadows; a nonzero value retains the existing shadow state.

The color commands cycle their GUI color indices modulo 16. Changing the paper
color also updates the stored background palette index, and is allowed only when
paper artwork is disabled. `console limit` accepts values from zero through 300
and stores the value multiplied by 256.

Initial settings select font zero, disabled shadows, color indices zero, 13,
nine and zero, 16 hexadecimal lines, enabled paper artwork, background palette
index zero and screen limit zero.

See [Developer console](../engine/console.md) for command syntax and presentation.

## External References

- [UESP Redguard Console](https://en.uesp.net/wiki/Redguard:Console)
