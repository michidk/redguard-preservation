# Background Music

Redguard's music is Red Book CD audio, played from the game CD through the
Miles Sound System redbook interface. It is not stored in any game file.

## The CD

Track 1 is the data track; tracks 2–7 are audio. The GOG release ships the
CD as a cue sheet `game.ins` and a raw image `game.gog` (2352-byte sectors)
in the directory above `Redguard\`, and its DOSBox configuration mounts it as
drive `D:`.

All 29 shipped `WORLD.INI` entries set `world_redbook`, and every value lies
in 2–7.

## Track selection

When a world loads, if its `world_redbook[N]` is greater than zero and differs
from the track the CD is currently playing, the engine stops the CD, plays
track `world_redbook[N]`, and applies `redbook_volume`. Consequently:

- A world without `world_redbook` (or with a value of zero or less) keeps the
  current track playing.
- Moving between worlds that share a track does not restart it.

A track number below 1 or above the CD's track count is ignored.

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
