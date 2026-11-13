# Cinematic playback

Movies use the installed MENU.INI definitions and Smacker files, not recorded playback.

`import::menu_movies::parse_menu_movies` reads ordered names, skip points, and subtitle records from [page3]. A subtitle has seven comma-separated fields: an always-display flag, red, green, blue, start frame, stop frame, and text. Text can contain commas. The shipped configuration defines eleven filenames and five browsable cinematics. Names are uppercased and the original filename buffer holds fifteen characters. Original skip tables hold up to twenty frame numbers. Zero terminates a skip table; a single zero defines no skip points. Values after the terminator or twentieth entry are ignored.

Playback numbers frames from one. A new skip input advances to the next configured forward point; after the last point it finishes the movie. Passed skip points are consumed during normal playback. Held input must be released before another skip. Returning from playback restores the menu and retains its selection. Playback marks the cinematic unlocked.

Subtitles begin at the start frame and end before the stop frame. Their always-display flag overrides the subtitle preference. The original wraps text at 35 characters and centers the lines at X 320, ending at Y 470, using the font's line height (26 for REDSEL). Glide playback draws the source's half-height frame over the 640 by 480 viewport. Interlace overlays black odd scanlines. Subtitles use their own copy of `FONTS\REDSEL.FNT` with the value pair `255,180`, not a `SYSTEM.INI` font: the main copy is drawn with alpha 180 and the shadow copy with alpha 140, offset (2, 2) rather than the (1, 1) of [dialogue subtitles](subtitles.md). Glyphs use the selected eight-bit texture-upload palette. At startup this is the unmodified `REDGUARD.COL` palette, including its low two channel bits; the font's six-bit BPAL does not replace that upload palette. The resulting text is gold. Advances, the space rule and centering follow [subtitles](subtitles.md#glyphs). The configured per-record colors are not used.

SYSTEM.INI [system] animation_drive prefixes movie filenames. The GOG configuration uses D:\, mapped to the supplied game.gog CD image. Its data track is MODE1/2352 and the eleven configured movies reside in the ISO 9660 root directory, not an ANIMS subdirectory. `import::cdrom::CdImage` reads root files without sector headers; it also accepts ordinary 2048-byte ISO images. No install data is modified.

## Playback clock and drawing

Movie playback advances its own decoded-frame ordinal; gameplay clock progress is not required while a movie is playing. Each iteration updates the palette, decodes the current frame, advances the decoder to the next frame, draws and presents the decoded image, then waits for the movie clock before continuing. At presentation, the decoder's advanced ordinal equals the one-based frame just drawn.

The wait uses the selected movie audio stream's playback position when a sound track is active. Without an active audio track, it uses a timer deadline and the frame period from the Smacker header. Movie presentation and gameplay updates therefore have separate clock owners.

The large-image Glide path splits source pixels into tiles of at most 256 by 256. Tile origins advance by 256 source pixels. Each destination rectangle extends one pixel beyond its calculated right and bottom edges; movie destination Y coordinates double the source height. Adjacent tiles overlap at their edges. Unused rows and columns of the power-of-two texture backing are filled with palette index zero. Sampling can wrap into this padding; it does not clamp to the populated source rectangle. A single whole-image resize does not reproduce those sampling coordinates.

Glyph drawing temporarily selects its upload palette and restores the movie palette afterwards. A palette sampled only after the complete presentation therefore need not be the palette used for subtitles.

### Glide movie raster

At the 640 by 480 viewport, a 640 by 240 movie uses three 256 by 256
indexed texture backings, populated with source widths 256, 256 and 128.
The source tile origins are 0, 256 and 512. Index zero fills unused backing
texels. The destination rectangles have nominal widths one greater than the
source width and nominal height 481. Rectangle clipping adjusts the source
endpoint proportionally before triangle setup. The final tile is clipped to
the viewport right edge, and every tile is clipped to the viewport bottom.

Triangle setup normalizes the destination edge deltas by the triangle area
before multiplying by source-coordinate deltas. Each operation is stored at
single precision. For an axis-aligned clipped rectangle of destination extent
`W,H` and source endpoint `S,T`, let `inverse_area = 1/(W*H)`. Its texture
gradients are `S * (H * inverse_area)` and `T * (W * inverse_area)`.
Reassociating these products into a single source/destination division can
change texture filter thresholds. The starting source coordinate is anchored
to the submitted vertex, including when it is the rectangle's right edge.

The Voodoo texture raster samples at destination pixel centres and subtracts
half a source texel before bilinear filtering. Its first-generation filter
retains four fractional bits and wraps within the complete power-of-two
backing. Both horizontal interpolation results are floored before the
vertical interpolation is floored. The framebuffer stores RGB565 with the
4 by 4 ordered dither matrix, then presentation expands the stored bits.
These operations precede the final RGB capture format.

Subtitle glyphs use point sampling and index zero transparency. For constant
alpha `a`, the source term is `floor(source*(a+1)/256)` and the destination
term is `floor(destination*(256-a)/256)`. The blend unit reads the stored
RGB565 destination with zero low bits, applies framebuffer dither subtraction,
and adds the independently floored terms before the next dithered RGB565
store. Glyph shadow/main submission order and the selected upload palette
remain significant when glyphs overlap.

## Unlock flags and queued playback

The movie table holds fifteen records, each with an unlock flag (bit 0). The session initializer, which runs at launch and again at New Game, clears every flag and sets it to MENU.INI `[general] force_movies` (bit 0). Playing a movie by index sets that record's flag before playback and sets the stored queued-movie value to -1 afterwards. Save games store the fifteen flag bytes and loading restores them, so unlocks follow the loaded save rather than the process.

On the Movies page, an entry whose action is 302 through 315 (movie 1 through 14) is hidden and unselectable while its flag is clear. Entry 301 (movie 0, the introduction) is always shown.

The stored queued-movie value is played by the main loop after a world loop ends and before the next world is saved to the temporary game and loaded: a value other than -1 plays that movie like any other, with the flag and queue effects above. Launch and New Game also play movie 0 at this point of the loop before the first world loads, and the ending plays movie 4 through the book (see [menu](menu.md)). [`QueueMovie`](SOUP.md#queuemovie) stores or plays an index as described there.

