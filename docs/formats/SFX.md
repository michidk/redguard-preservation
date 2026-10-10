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
| 0x0C | 1 | u8 | base_volume | Always 64 (all 118 effects). Base volume (Miles 0..127 scale) of every start: the caller's volume is added and the sum clamped to 0..127. Positional callers pass the distance volume relative to this base; `FlatSound(e, 0, 0)` plays at 64. See [positional sound](../engine/sound.md). |
| 0x0D | 1 | u8 | loop_flag | 0 = play once. Non-zero enables looping; 0xFF loops forever, any other value is passed as the Miles loop count (see Loop Behavior). |
| 0x0E | 4 | u32 | loop_offset | Byte offset into PCM data for loop restart point (always 0) |
| 0x12 | 4 | u32 | loop_end | Sample count before looping (always 0xFFFFFFFF) |
| 0x16 | 4 | u32 | data_length | Byte count of raw PCM data following this header |
| 0x1A | 1 | u8 | reserved_1a | Padding between header and PCM data. Always 0. |
| 0x1B | var | `[u8]` | pcm_data | Raw PCM audio: u8 samples for 8-bit, i16 LE samples for 16-bit |

### Loop Behavior

From static analysis of the engine's sample start (not yet observed in the running game):

- `loop_flag = 0`: play once (non-looping effects)
- `loop_flag = 0xFF`: Miles loop count 0, i.e. loop until stopped (ambient loops like fire, water, wind)
- any other non-zero value: passed unchanged as the Miles loop count. The only shipped case is effect 117 (the snake charmer tune), flag `0xE1`, which therefore plays 225 times rather than indefinitely. Whether this is audible in practice has not been checked.

When looping is enabled the engine also passes `loop_offset` and `loop_end` as the Miles loop block.

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
