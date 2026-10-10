# Subtitles

Spoken dialogue, combat taunts and movies draw their text in
`FONTS\REDSEL.FNT` on a virtual 640×480 screen, which the Glide renderer
scales to the output size. Facts below come from engine analysis; those
marked *observed* were also checked against the running original (DOSBox-X,
Glide).

## Glyphs

- Advance per character: glyph width + 1. A space additionally adds the width
  of `!`, so a REDSEL space advances 11 pixels. Disabled glyphs advance 0 and
  are not drawn.
- A glyph is drawn at (pen x + `offset_left`, line top + `offset_top`).
  Palette index 0 is transparent.
- Each glyph is drawn twice: first a shadow copy offset by (1, 1) for dialogue
  and (2, 2) for movies, then the main copy.
- Three in-memory copies of REDSEL differ only in a value pair `A,B`:
  NPC lines use `SYSTEM.INI [3dfx] font_sel` (shipped `255,255`), Cyrus' lines
  `font_norm` (`125,255`), movies a fixed `255,180`. The main copy is drawn
  with alpha `B`, the shadow copy with alpha `A × 200 / 256` (integer).
- *Observed* (NPC line, `font_sel`): main glyphs are opaque in the font's own
  palette colours (BPAL entries are 6-bit; index 195 `39,3,14` shows as the
  crimson edge, index 81 `60,59,11` as yellow). The shadow is the glyph shape
  in black, blended over the scene at alpha 199/255.
- A line is centred on x = 320: it starts at `320 + trunc(−width / 2)`.
- Subtitles ignore `[system] disable_text`.

## Dialogue

The text is the RTX entry's own string: the label of a voiced entry, the text
of a string-only entry. Script functions `RTX`, `rtxAnim` and `RTXpAnim`
(speaker: the scripted actor), `RTXp` (speaker: Cyrus) and combat taunts
create subtitles.

### Slots

`[dialog] dialog_max_dialog_lines` is the number of subtitle slots, at least
16 (shipped 20), not a line limit. Slot 0 belongs to Cyrus (`RTXp`) and is
overwritten unconditionally; other speakers take the lowest free slot, and no
subtitle is created when none is free. Only one subtitle is drawn per frame:
the lowest active slot.

### Wrapping

Width limit `W = max([dialog] dialog_max_text_width, 150)` (shipped 500),
measured with REDSEL advances. The text is split into tokens, each an optional
leading space plus a word. A token joins the line while `running + token <
W` (strict); otherwise a line break replaces its leading space and the running
width restarts at the token's width *including* the dropped space.

### Position

With `breaks` line breaks, the block height is `26 × (max(breaks, 1) + 1)`
and the first line's top is `470 − height`; each next line is 26 lower. One
line therefore sits at y 418, two lines at 418 and 444.

*Observed*: "YOU TWO DANCE VERY WELL, BUT, ALAS --" (one line) has glyph pixels
spanning x 73–565, y 423–439, matching these rules with REDSEL's metrics.

### Timing

- `dialog_use_speech = 1`: a voiced line shows while its clip plays and is
  freed when the clip stops. A string-only entry is neither drawn nor freed.
- `dialog_use_speech = 0`: the voice is stopped. The line starts with
  `remaining = clip seconds × 256` (voiced) or `characters × 15`
  (string-only) and loses 21 per game tick; it is freed once `remaining ≤ 0`.
- `dialog_print_text = 0`: slots keep the same timing (script waits are
  unchanged) but nothing is drawn.
- Script `RTX` lines from a speaker farther than `dialog_max_distance` (1600)
  are not started.

## Movies

MENU.INI `movieM_text[N]` lines (see [MENU.INI](../config/menu-ini.md)) are
frame-locked to the Smacker file: a line is visible on frames `start_frame`
to `stop_frame − 1`, one line at a time, in file order. It is drawn when
`dialog_print_text ≠ 0` or its leading flag is non-zero (the 12 flagged lines
are in INTRO.SMK). The stored colour is parsed but not used; every line uses
the movie copy.

Wrapping counts characters: a line breaks at the first space at or after its
35th character (0-based), and the space is removed. With `n` lines the first
top is `470 − 26 × n`, so the last line always sits at y 444. The skip key
jumps to the next `movie_keys` frame and clears the current line.

## Open questions

- Colour of Cyrus' lines and of movie lines (the alpha reading is observed
  for the NPC copy only).
- The game tick rate behind the text-only countdown (12 per second is
  inferred from the timing accumulator).
- Which input clears all subtitles, whether `ambientrtx` draws text, and
  how inventory voice lines show theirs.
