# Background Music

Redguard's music is Red Book CD audio, played from the game CD through the
Miles Sound System redbook interface. It is not stored in any game file.

## The CD

Track 1 is the data track; tracks 2–7 are audio. The GOG release ships the
CD as a cue sheet `game.ins` and a raw image `game.gog` (2352-byte sectors)
in the directory above `Redguard\`, and its DOSBox configuration mounts it as
drive `D:`.

The audio designation in the cue sheet does not guarantee that every byte in
the range is audio. The image packaged by the GOG `2.0.0.5` installer includes
scrambled Mode 1 data-sector bytes at the beginning of track 2 before its audio
silence. The scrambled region preceding track 2 overlaps the end of the plain
data region and is displaced relative to the sector addresses in its headers.
That displacement carries data beyond the cue sheet's audio start. The
data-to-audio transition is not aligned to a 2352-byte sector boundary.
Interpreting the overlapping data as 44.1 kHz signed 16-bit stereo PCM produces
a brief loud noise; the bytes are CD-ROM sector content, not recorded music.

All 29 shipped `WORLD.INI` entries set `world_redbook`, and every value lies
in 2–7.

## Track selection

Loading a world has two CD steps, at different points:

1. At the start of the load, before any world data is read, if `world_redbook[N]`
   is not `-1` and differs from the track the CD is currently playing, the
   engine stops the CD. The loading screen is therefore silent.
2. After the world is loaded, if `world_redbook[N]` is greater than zero and
   differs from the track the CD is currently playing, the engine plays track
   `world_redbook[N]` and applies `redbook_volume`.

Consequently:

- A world without `world_redbook` (or with `-1`) keeps the current track
  playing.
- A value of zero, or any negative value other than `-1`, stops the CD and
  starts nothing.
- Moving between worlds that share a track does not restart it.

A track number below 1 or above the CD's track count is ignored.

## Book menu and movies

Opening the book menu plays track 2 from its start and applies `redbook_volume`.
The startup splash and title play no CD audio.

Every movie, whether the introduction, a browsed Movies page entry, or a movie
queued before the next world, runs the same sequence: sound effects stop, the
CD pauses if it is playing, the movie plays, sound effects stop again, and the
CD resumes if it is paused. The CD continues from where it paused. Movie audio
comes from the Smacker stream, never from the CD.

New Game plays the introduction movie and then loads world 0. The book menu's
track 2 pauses for the movie and resumes when it ends; world 0's
`world_redbook` is 7, which differs from 2, so the world load then stops the CD
and track 7 starts only after the world is loaded. No CD audio plays during the
load.

## Repetition

Periodically from the main loop, the engine compares the CD's current track
with the last track it requested. When they differ — the requested track has
ended and the CD has stopped — it plays the requested track again. World music
therefore repeats indefinitely, with a short pause between repetitions.

## Volume

`redbook_volume` from `SYSTEM.INI` is passed unscaled to the redbook volume
call, which accepts only 0–127 and ignores any other value. The shipped value,
`200`, is therefore ignored and the CD plays at its existing (default) level.

`redbook` (`SYSTEM.INI [system]`, shipped `on`) gates the redbook system;
when it is disabled the CD is never opened and no music plays.

## Related

- [WORLD.INI](../config/world-ini.md) — `world_redbook[N]`
- [SYSTEM.INI](../config/system-ini.md) — `redbook`, `redbook_volume`
