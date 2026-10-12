# Bundled support resources

Bundled support files contain hardware configuration, controller presets, audio test data, and development reports.

## Runtime roles

| Resource | Original role | Reference |
|---|---|---|
| `.DIG` | Executable DOS drivers for digital sound hardware. `DIG.INI` selects the driver used for sample playback. | [Miles drivers](MILES-DRIVER.md) |
| `.MDI` | Executable DOS MIDI drivers. The shipped engine disables MIDI initialization. | [Miles drivers](MILES-DRIVER.md) |
| `.OPL`, `.AD` | FM MIDI instrument banks. These do not supply the CD background music. | [Instrument banks](OPL.md) |
| `DIG.INI`, `MDI.INI` | Driver selection and sound-card I/O parameters. | [Miles configuration](../config/miles-ini.md) |
| `MSSDRVR.LST` | Driver selection criteria and installation messages for audio setup. | [Driver catalog](#miles-driver-catalog) |
| `.PIF` | Windows launch settings for the bundled DOS audio utilities. | [PIF settings](PIF.md) |
| `STATE.RST` | Text editor settings and remembered file positions. | [Editor state](RST.md) |
| `.SWG`, `.SWX` | Microsoft SideWinder controller preferences. | [Controller profiles](SIDEWINDER.md) |
| `fxart/more` | Model filenames and version labels from Hugh's 3D information tool. | [Model report](MORE.md) |
| `fxart.txt`, `fxart3dc.txt` | Historical directory listings with model filenames, timestamps, and sizes. | [Directory reports](#directory-reports) |
| `fxart/LOGFILE.TXT` | Collision-sphere generation tool output. | [Collision tool report](#collision-tool-report) |
| `sound/R212.WAV` | PCM sample selected by the `DIGPLAY.PIF` audio test. | [Audio test sample](#audio-test-sample) |

A reimplementation using modern audio APIs plays samples from [SFX](SFX.md),
[RTX](RTX.md), and movie streams, and uses the [CD audio tracks](../engine/music.md)
for background music. DOS driver images supply the original hardware interface.
They do not supply the game's sound effects, dialogue, or music recordings.

Controller presets describe one hardware setup. Game action bindings are
separate settings in [KEYS.INI](../config/keys-ini.md). Historical reports record
tool output at the time of generation. Current model headers and archive data
define the installed assets.

## Miles driver catalog

`MSSDRVR.LST` is a line-oriented text file. Sections use a driver filename
in brackets, such as `[iwav.dig]`. Lines beginning with `;` are comments.
Dot-prefixed directives define selection conditions and setup behavior.

| Directive | Meaning |
|---|---|
| `.E NAME` | Environment variable `NAME` must be present. |
| `.NE NAME` | Environment variable `NAME` must be absent. |
| `.C TEXT` | `CONFIG.SYS` must contain `TEXT`. |
| `.NC TEXT` | `CONFIG.SYS` must not contain `TEXT`. |
| `.A TEXT` | `AUTOEXEC.BAT` must contain `TEXT`. |
| `.NA TEXT` | `AUTOEXEC.BAT` must not contain `TEXT`. |
| `.ND` | The entry cannot be selected as the default. |
| `.T1` | If the BLASTER `T` parameter is present, its value must be 1 or 3. |
| `.T2` | The BLASTER `T` parameter must be present and equal to 2, 4, or 5. |
| `.T3` | The BLASTER `T` parameter must be present and at least 6. |
| `.IOB` | Derive I/O defaults from the BLASTER environment variable. |
| `.IOD` | Use the first default I/O configuration from the driver. |
| `.ION` | Disable manual I/O configuration. |
| `.IOA` | Configure only the I/O address. Appears at the end of the I/O options. |
| `.IOL` | Suppress the high-DMA prompt. Appears at the end of the I/O options. |
| `.NAME` | The following line supplies a displayed device name. |
| `.SPECIAL_TEXT` | Begin the installation message block. |
| `.END_SPECIAL_TEXT` | End the installation message block. |

Message blocks retain their line breaks. Device labels and setup criteria are
separate from the executable driver's VDI header. For example, the catalog
supplies the WaveJammer device label for `JAMMER.DIG`, whose stored header has
no device-name block.

## Directory reports

`fxart.txt` and `fxart3dc.txt` contain DOS directory output. The heading records
the drive label and the original `E:\PROJECTS\REDGUARD\demo\fxart` path.
Each entry records a date, a time, a decimal file size with optional comma
grouping, and a filename. The footer records totals and free disk space.

The listings describe the files at the time of the report. Their timestamps,
sizes, and totals are historical metadata, not current asset requirements.

## Collision tool report

`fxart/LOGFILE.TXT` begins with `Collision Sphere Generation`, version `1.5`,
and identifies Alphacom Systems Limited as the tool author. Its output contains
`Processing (N) file : NAME` messages and runs of progress dots. Messages can
follow one another on the same line.

The report records tool activity. Collision data resides in the corresponding
[model files](models/3d.md).

## Audio test sample

`R212.WAV` is a RIFF/WAVE PCM file with one channel, eight-bit samples, and a
sample rate of 11,025 Hz. `DIGPLAY.PIF` selects `R212.WAV` as the argument to
`DIGPLAY.EXE`. The recording is an audio utility test input.

## Cursor asset

`system/pointers.bmp` contains a [GXA](GXA.md) bitmap archive with 30 frames.
Its first chunk is `BMHD`; the filename's `.bmp` extension does not describe
the container. This is a cursor image asset, separate from the controller
profiles and setup files.

## External References

- [Miles DOS driver interfaces](https://github.com/swfans/swars/blob/master/bfsoundlib/include/mssal.h)
- [Microsoft RIFF structure](https://learn.microsoft.com/en-us/windows/win32/xaudio2/resource-interchange-file-format--riff-)
