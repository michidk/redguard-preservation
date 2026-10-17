# Miles DOS Drivers (.DIG, .MDI)

`.DIG` and `.MDI` files are Miles Sound System hardware drivers with a shared VDI header and 16-bit x86 code.

`.DIG` drivers provide digital sample playback. `.MDI` drivers provide MIDI output or synthesis, including FM synthesis.
These are executable driver images, not audio containers such as [SFX](SFX.md) or [RTX](RTX.md).
The image is loaded into DOS conventional memory and called through interrupt `66h`.
It has no DOS `MZ` executable header or relocation table.

## Overall Structure

```text
[VDI header — driver identity, runtime configuration, interrupt entry, scratch space]
[Optional device name — 80-byte name block in newer headers]
[Driver body — callback tables, hardware data, and 16-bit machine code]
```

All multibyte values are little-endian. Header offsets are relative to the beginning of the file.
The driver body has no common fixed layout. Its dispatcher offset comes from the header, rather than a fixed code position.

## VDI Header

The common header is `0xBA` bytes. Revisions `0x112` and later add an 80-byte device name, extending the header to `0x10A` bytes.
`AUDIODRV.DIG` stores revision zero but includes this name block.
`JAMMER.DIG` and `SNDSCAPE.DIG` use revision `0x111`; `SNDSCAPE.MDI` uses `0x100`. These three images omit the name block.

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| `0x00` | 8 | `[u8; 8]` | id | `AIL3DIG\x1A` for digital audio or `AIL3MDI\x1A` for MIDI. The loader compares the first seven bytes without case sensitivity. |
| `0x08` | 4 | `u32` | driver_version | Driver interface revision. Independent of the version of the host Miles library. |
| `0x0C` | 4 | `u32` | common_io_configurations | Packed DOS pointer to the driver's default I/O configuration records. |
| `0x10` | 2 | `u16` | num_io_configurations | Number of default I/O configuration records. |
| `0x12` | 4 | `u32` | environment_string | Packed DOS pointer to the name of an environment variable used for hardware configuration. |
| `0x16` | 24 | — | io | Current hardware I/O parameters, described below. |
| `0x2E` | 2 | `i16` | service_rate | Background service frequency in Hz. A non-positive value disables periodic service. |
| `0x30` | 2 | `u16` | busy | Non-zero while the driver is servicing a request. |
| `0x32` | 2 | `u16` | driver_number | Runtime slot assigned by the loader. |
| `0x34` | 2 | `u16` | this_isr | File-relative offset of the driver's interrupt `66h` dispatcher. |
| `0x36` | 4 | `u32` | prev_isr | Packed DOS pointer to the previous interrupt `66h` handler. |
| `0x3A` | 128 | `[u8; 128]` | scratch | Shared workspace, including the hardware environment string passed to the driver. |
| `0xBA` | 80, if present | `[u8; 80]` | dev_name | NUL-terminated ASCII device name, followed by padding. |

The default-configuration pointer, configuration count, environment pointer, I/O parameters, and service rate are zero in the shipped images.
Driver initialization populates configuration fields in memory. A file reader cannot recover the driver's default hardware settings by reading these zeros.

Packed DOS pointers contain a 16-bit offset in the low word and a 16-bit segment in the high word.
Their linear address is `segment × 16 + offset`. They are runtime addresses, not file offsets.
`this_isr` is the exception: it is a near offset within the loaded driver image.
The loader saves the previous interrupt vector in `prev_isr` and installs the dispatcher into the interrupt chain.

### I/O Parameters

The `io` record occupies 24 bytes. Offsets in this table are relative to that record.
A value of `-1` means that a hardware parameter is unspecified.

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| `0x00` | 2 | `i16` | io_address | Sound-card I/O base address. |
| `0x02` | 2 | `i16` | irq | Interrupt request number. |
| `0x04` | 2 | `i16` | dma_8_bit | DMA channel for 8-bit transfers. |
| `0x06` | 2 | `i16` | dma_16_bit | DMA channel for 16-bit transfers. |
| `0x08` | 16 | `[i32; 4]` | reserved | Driver-specific parameters. |

## Driver Calls

The function ID is passed in register `AX`. The driver number in `BX` selects the installed image.
The dispatcher invokes a driver callback and returns through the interrupt interface.
Callback tables and their positions are private to each image.

| ID | Name | Purpose |
|---|---|---|
| `0x300` | `DRV_INIT` | Initialize the driver interface and publish configuration information. |
| `0x301` | `DRV_GET_INFO` | Return pointers to driver description and state tables. |
| `0x302` | `DRV_SERVE` | Perform background service. |
| `0x303` | `DRV_PARSE_ENV` | Parse hardware parameters from the shared scratch workspace. |
| `0x304` | `DRV_VERIFY_IO` | Detect or validate hardware configuration. |
| `0x305` | `DRV_INIT_DEV` | Initialize the hardware device. |
| `0x306` | `DRV_SHUTDOWN_DEV` | Shut down the hardware device. |

Digital playback and MIDI drivers add type-specific operations in the `0x400` and `0x500` ranges, respectively.

## Hardware Drivers

| File | Interface | Hardware |
|---|---|---|
| `ESFM.MDI` | MIDI synthesis | ESS ES1688, ES1788, and ES1888 enhanced FM audio. |
| `IWAV.DIG` | Digital audio | InterWave audio processors. |
| `IWAV.MDI` | MIDI synthesis | InterWave General MIDI. |
| `JAMMER.DIG` | Digital audio | New Media Corporation WaveJammer PCMCIA sound cards. |
| `MDIMAXI.MDI` | MIDI | Maxi Sound 64 or compatible hardware. |

Hardware selection and installation messages are separate from the binary header.
`MSSDRVR.LST` supplies selection rules and device labels, including a label for `JAMMER.DIG`, whose header has no name block.
`DIG.INI` and `MDI.INI` select the digital and MIDI driver filenames and their I/O parameters. See the [configuration overview](../config/).
Background [music](../engine/music.md) uses CD audio, separate from these MIDI synthesis drivers.

## Use by the Game

The game initializes digital audio from `DIG.INI`. The shipped configuration
selects `SB16.DIG` for SoundBlaster sample playback, including sound effects,
dialogue, and movie audio. The other DIG images provide alternative hardware
drivers.

The Miles library also contains a MIDI initialization path that reads
`MDI.INI`. The shipped engine disables that path at startup. Its MDI drivers
and [OPL and AD instrument banks](OPL.md) remain bundled MIDI resources,
separate from the CD audio used for background music.

The [PIF files](PIF.md) configure the bundled `DIGPLAY.EXE` and `SETSOUND.EXE`
utilities. They are Windows launch settings, not sound drivers or audio data.

A reimplementation with modern audio APIs uses the decoded sample data and
engine playback rules. The DOS driver images describe the original hardware
interface and are independent of those recordings. The whitespace-separated
configuration syntax is described in [Miles audio configuration](../config/miles-ini.md).
The accompanying setup messages and hardware-selection rules are described in
[Bundled support resources](BUNDLED-RESOURCES.md#miles-driver-catalog).

## External References

- [Syndicate Wars `mssal.h`, VDI header and driver call definitions](https://github.com/swfans/swars/blob/master/bfsoundlib/include/mssal.h)
- [Syndicate Wars `ail.c`, Miles driver installation interface](https://github.com/swfans/swars/blob/master/bfsoundlib/src/ail.c)
