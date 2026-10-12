# Miles audio configuration

`DIG.INI` and `MDI.INI` store a selected Miles driver and its sound-card I/O parameters.

## Record syntax

Each setting consists of a key, whitespace, and its value. These files do not
use section headers or equals signs. A `DEVICE` value can contain spaces.
Lines beginning with `;` are comments, and blank lines separate records.
The shipped files use CRLF line endings.

```text
DEVICE      Creative Labs SoundBlaster 16, AWE32 or AWE64
DRIVER      SB16.DIG
IO_ADDR     220h
IRQ         7
DMA_8_BIT   1
DMA_16_BIT  5
```

## Settings

| Key | Value syntax | Meaning |
|---|---|---|
| `DEVICE` | Text | Displayed device description. |
| `DRIVER` | Filename | Selected `.DIG` or `.MDI` driver image. |
| `IO_ADDR` | Hexadecimal digits followed by `h` | Sound-card I/O base address. `220h` is `0x220`. |
| `IRQ` | Decimal integer | Interrupt request number. |
| `DMA_8_BIT` | Decimal integer | DMA channel for eight-bit transfers. |
| `DMA_16_BIT` | Decimal integer | DMA channel for sixteen-bit transfers. |

The shipped digital configuration selects `SB16.DIG`. The MIDI configuration
selects `SBLASTER.MDI` and labels it `Creative Labs Sound Blaster or 100%
compatible`. Both store I/O address `220h`, IRQ `7`, eight-bit DMA channel `1`,
and sixteen-bit DMA channel `5`.

## Runtime role

The original engine initializes digital sample playback from `DIG.INI`.
Its MIDI initialization path reads `MDI.INI`, but the shipped engine disables
that path. Background [music](../engine/music.md) uses CD audio.

These values configure DOS sound hardware. A modern audio backend handles its
own device selection. The [driver format](../formats/MILES-DRIVER.md) and
[setup catalog](../formats/BUNDLED-RESOURCES.md#miles-driver-catalog) describe
the accompanying hardware resources.
