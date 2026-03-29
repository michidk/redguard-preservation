# SFX Sound Effects File Format

Single-file container for all game sound effects, stored as `MAIN.SFX` in the `SOUND` directory. Does not include voice clips (those are in `ENGLISH.RTX`).

## Overall Structure

```
FXHD section (44 bytes)
FXDT section (variable)
"END " (4 bytes)
```

Effects are stored sequentially with no offset table. The game references effects by their 0-based index in the file.

## FXHD (Header Section)

44 bytes total. Section size word is big-endian; remaining fields are little-endian.

| Offset | Size | Type | Endian | Name | Description |
|---|---|---|---|---|---|
| 0x00 | 4 | `[u8; 4]` | — | tag | Always `FXHD` |
| 0x04 | 4 | `u32` | BE | section_size | Payload size excluding this field and the tag |
| 0x08 | 32 | `[u8; 32]` | — | description | ASCII string, set by internal tool "SoupFX" |
| 0x28 | 4 | `u32` | LE | effect_count | Number of sound effects (118 in `MAIN.SFX`) |

## FXDT (Data Section)

Begins with a 4-byte ASCII tag `FXDT`, followed by a big-endian u32 section size (excluding itself and the tag), followed immediately by sequential effect records.

### Effect Record

27-byte header followed by raw PCM audio data.

All fields little-endian unless noted.

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| 0x00 | 4 | u32 | type_id | Audio type: 0 = 8-bit mono, 1 = 16-bit mono, 2 = 8-bit stereo (unused), 3 = 16-bit stereo |
| 0x04 | 4 | u32 | bit_depth | 0 = 8-bit, 1 = 16-bit |
| 0x08 | 4 | u32 | sample_rate | Always 11025 or 22050 Hz |
| 0x0C | 1 | u8 | unused_0c | Always 64. Runtime behavior is driven by the surrounding 26-byte header block (`0x00`–`0x19`), with no separate per-field behavior for this byte. Likely a vestigial default volume value (64/127 ≈ 50% on the Miles Sound System scale). |
| 0x0D | 1 | i8 | loop_flag | 0 = no loop, non-zero = enable looping. The engine checks only `!= 0`; values -1 (0xFF) and -31 (0xE1) are functionally identical. |
| 0x0E | 4 | u32 | loop_offset | Byte offset into PCM data for loop restart point (always 0) |
| 0x12 | 4 | u32 | loop_end | Sample count before looping (always 0xFFFFFFFF) |
| 0x16 | 4 | u32 | data_length | Byte count of raw PCM data following this header |
| 0x1A | 1 | u8 | reserved_1a | Padding between header and PCM data. Always 0. |
| 0x1B | var | `[u8]` | pcm_data | Raw PCM audio: u8 samples for 8-bit, i16 LE samples for 16-bit |

### Loop Behavior

The engine checks only whether `loop_flag` is non-zero — the specific value is not interpreted.

- `loop_flag = 0`: play once (non-looping effects)
- `loop_flag = -1 (0xFF)`: enable looping (used for ambient loops like fire, water, wind)
- `loop_flag = -31 (0xE1)`: enable looping (functionally identical to -1; only used on effect 117, the snake charmer tune)

`loop_offset` and `loop_end` appear to be unused features — always `loop_offset = 0` and `loop_end = 0xFFFFFFFF`.

### Runtime Effect Structure

The engine allocates a 34-byte (0x22) runtime structure per effect, reading 26 bytes (0x00–0x19) from the file. The remaining 8 bytes are computed at runtime:

| Struct Offset | Size | Source | Contents |
|---|---|---|---|
| 0x00–0x19 | 26 | File | Header fields (type_id through data_length) |
| 0x1A–0x1D | 4 | Runtime | Pointer to allocated PCM data buffer |
| 0x1E–0x21 | 4 | Runtime | Computed duration value: `(data_length << 8) / (sample_rate × bytes_per_sample × channels)` |

## Related Formats

- [RTX](RTX.md) — dialogue audio container. Uses the same 27-byte audio header structure (offsets 0x00–0x1A) as SFX effect records. SFX stores sound effects; RTX stores voice clips.

## External References

- [UESP: Mod:SFX File](https://en.uesp.net/wiki/Mod:SFX_File)
