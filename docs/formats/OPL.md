# OPL instrument banks

OPL files store Miles FM synthesis timbres indexed by MIDI patch and bank.

The sound directory contains `sample.opl` and `Fatman.opl`. These banks provide
two-operator instrument settings for FM MIDI playback. See also
[Miles DOS drivers](MILES-DRIVER.md).

## Overall Structure

```text
[Instrument index — patch and bank identifiers with file-relative offsets]
[Index terminator — FF FF]
[Timbre records — two-operator register settings]
```

All multibyte integers are little-endian. There is no file signature or
instrument-count field.

## Instrument index

Each entry is six bytes.

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| `+0x00` | 1 | `u8` | patch | MIDI patch identifier |
| `+0x01` | 1 | `u8` | bank | Bank identifier |
| `+0x02` | 4 | `u32` | offset | Absolute file offset of the timbre record |

The index ends when both identifiers are `0xFF`. This terminator is two bytes
and has no offset field. Timbre offsets must point beyond the complete index.
Entries may share a timbre offset; record ordering need not match index ordering.

## Timbre record

The two-operator record is 14 bytes, including its size word.

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| `+0x00` | 2 | `u16` | size | Record size, 14 |
| `+0x02` | 1 | `i8` | transposition | Melodic pitch adjustment or percussion note |
| `+0x03` | 1 | `u8` | modulator.reg_20 | Operator characteristics and multiplier |
| `+0x04` | 1 | `u8` | modulator.reg_40 | Key scale level and total level |
| `+0x05` | 1 | `u8` | modulator.reg_60 | Attack and decay rates |
| `+0x06` | 1 | `u8` | modulator.reg_80 | Sustain level and release rate |
| `+0x07` | 1 | `u8` | modulator.reg_e0 | Waveform selection |
| `+0x08` | 1 | `u8` | reg_c0 | Channel feedback and connection settings |
| `+0x09` | 1 | `u8` | carrier.reg_20 | Operator characteristics and multiplier |
| `+0x0A` | 1 | `u8` | carrier.reg_40 | Key scale level and total level |
| `+0x0B` | 1 | `u8` | carrier.reg_60 | Attack and decay rates |
| `+0x0C` | 1 | `u8` | carrier.reg_80 | Sustain level and release rate |
| `+0x0D` | 1 | `u8` | carrier.reg_e0 | Waveform selection |

For melodic channels, `transposition` adjusts the incoming note. On the
percussion channel it supplies the note directly.

Register values are stored bytes, before playback adjusts pitch, volume,
or channel assignment. The parser accepts the 14-byte format and rejects other
timbre sizes. Parsing does not synthesize audio.

## External References

- [ScummVM Miles AdLib instrument reader](https://github.com/scummvm/scummvm/blob/master/audio/miles_adlib.cpp)
