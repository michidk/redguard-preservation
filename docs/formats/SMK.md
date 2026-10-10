# SMK

Smacker files carry compressed indexed video, palette changes, and up to seven audio streams.

## Overall structure

```text
[Fixed header, 104 bytes]
[Frame size and stream flag tables]
[Huffman trees]
[Compressed frame and audio packets]
```

The fixed header uses little-endian fields. `import::smacker::SmackerHeader` reads metadata only; it does not decode compressed packets.

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| 0 | 4 | `[u8; 4]` | signature | SMK2 or SMK4. |
| 4 | 4 | `u32` | width | Source pixel width. |
| 8 | 4 | `u32` | height | Source pixel height. |
| 12 | 4 | `u32` | frames | Authored frame count, excluding an optional ring frame. |
| 16 | 4 | `i32` | timing | Positive values are milliseconds; negative values are hundredths of a millisecond. |
| 20 | 4 | `u32` | flags | Bit 0 adds a ring frame; bits 1 or 2 request doubled display height. |

The examined INTRO.SMK has 640 by 240 source pixels, 6895 authored frames, timing -6666, and doubled display height. FFmpeg independently reports approximately 15.0015 frames per second and stereo 22050 Hz Smacker audio.

## External references

- [FFmpeg Smacker demuxer](https://github.com/FFmpeg/FFmpeg/blob/n7.0.2/libavformat/smacker.c).
- [FFmpeg Smacker video decoder](https://github.com/FFmpeg/FFmpeg/blob/n7.0.2/libavcodec/smacker.c).
