# PIF program settings

PIF files store Windows launch settings for DOS programs.

`DIGPLAY.PIF` and `SETSOUND.PIF` describe the corresponding sound utilities.
They contain a base header, Windows 386 settings, and a Windows VMM extension.

`DIGPLAY.PIF` launches `DigPlay.EXE` with argument `R212.WAV`.
`SETSOUND.PIF` launches `SETSOUND.EXE` without arguments. These settings belong
to the audio utilities' Windows environment. They do not control sample
playback in a modern game engine. See [Bundled support resources](BUNDLED-RESOURCES.md).

## Overall Structure

```text
[Base header — DOS program, directory, arguments, and display settings]
[PIFEX root — linked extension header referring to the base header]
[Extension records — headers and file-relative payloads]
```

The base header is 369 bytes. A base-only file has no extension chain.
All multibyte integers are little-endian.

## Base header

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| `0x00` | 2 | `[u8; 2]` | unknown_00 | Stored header bytes |
| `0x02` | 30 | `[u8; 30]` | title | Space-padded window title |
| `0x20` | 2 | `u16` | memory_max_kb | Desired conventional memory, in KiB |
| `0x22` | 2 | `u16` | memory_min_kb | Required conventional memory, in KiB |
| `0x24` | 63 | `[u8; 63]` | program | Executable filename |
| `0x63` | 1 | `u8` | flags_63 | Launch flags; bit `0x10` closes the window on exit |
| `0x64` | 1 | `u8` | start_drive | Startup drive byte |
| `0x65` | 64 | `[u8; 64]` | start_directory | Working directory |
| `0xA5` | 64 | `[u8; 64]` | parameters | Program arguments |
| `0xE5` | 1 | `u8` | video_mode | Video mode byte |
| `0xE6` | 1 | `u8` | unknown_e6 | Stored byte |
| `0xE7` | 1 | `u8` | irq_low | Low interrupt vector byte |
| `0xE8` | 1 | `u8` | irq_high | High interrupt vector byte |
| `0xE9` | 1 | `u8` | rows | Screen rows |
| `0xEA` | 1 | `u8` | columns | Screen columns |
| `0xEB` | 1 | `u8` | window_y | Window Y position |
| `0xEC` | 1 | `u8` | window_x | Window X position |
| `0xED` | 2 | `u16` | unknown_ed | Stored word |
| `0xEF` | 64 | `[u8; 64]` | unknown_ef | Stored bytes |
| `0x12F` | 64 | `[u8; 64]` | unknown_12f | Stored bytes |
| `0x16F` | 1 | `u8` | flags_16f | Additional flags |
| `0x170` | 1 | `u8` | flags_170 | Additional flags |

Text fields end at the first NUL or their fixed field boundary. Title padding
is removed when displayed. Bytes after a NUL are outside the displayed string.
Text fields contain legacy character bytes. A Unicode representation depends
on the OEM or Windows code page used to display them.

## Extension chain

Each extension header is 22 bytes. Its offsets are absolute file positions.

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| `+0x00` | 16 | `[u8; 16]` | signature | NUL-terminated extension name |
| `+0x10` | 2 | `u16` | next_offset | Next header position; `0xFFFF` ends the chain |
| `+0x12` | 2 | `u16` | data_offset | Payload position |
| `+0x14` | 2 | `u16` | data_size | Payload size in bytes |

The root header starts at `0x171`. Its signature is `MICROSOFT PIFEX`, its
payload offset is zero, and its payload size is 369. This payload is the base
header. Each later extension has its own signature, payload position, and size.

## Windows 386 settings

The `WINDOWS 386 3.0` extension contains a 104-byte settings record.
Its memory limits and arguments supersede the corresponding base fields for
enhanced-mode execution.

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| `+0x00` | 2 | `u16` | memory_max_kb | Desired conventional memory |
| `+0x02` | 2 | `u16` | memory_min_kb | Required conventional memory |
| `+0x04` | 2 | `u16` | foreground_priority | Foreground scheduling priority |
| `+0x06` | 2 | `u16` | background_priority | Background scheduling priority |
| `+0x08` | 2 | `u16` | ems_max_kb | Expanded memory limit |
| `+0x0A` | 2 | `u16` | ems_min_kb | Required expanded memory |
| `+0x0C` | 2 | `u16` | xms_max_kb | Extended memory limit |
| `+0x0E` | 2 | `u16` | xms_min_kb | Required extended memory |
| `+0x10` | 2 | `u16` | option_flags | Execution options |
| `+0x12` | 2 | `u16` | memory_flags | Memory options |
| `+0x14` | 2 | `u16` | video_flags | Display options |
| `+0x16` | 18 | `[u16; 9]` | hotkey | Hotkey fields |
| `+0x28` | 64 | `[u8; 64]` | parameters | Enhanced-mode arguments |

Memory sizes are in KiB. `0xFFFF` is retained as the stored limit value.
The `WINDOWS VMM 4.0` payload occupies 428 bytes in both shipped utility files.

## External References

- [Wine PIF structures and reader](https://github.com/wine-mirror/wine/blob/wine-1.8/programs/winevdm/winevdm.c)
