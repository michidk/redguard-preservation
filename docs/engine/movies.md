# Cinematic playback

Movies use the installed MENU.INI definitions and Smacker files, not recorded playback.

`import::menu_movies::parse_menu_movies` reads ordered names, skip points, and subtitle records from [page3]. A subtitle has seven comma-separated fields: an always-display flag, red, green, blue, start frame, stop frame, and text. Text can contain commas. The shipped configuration defines eleven filenames and five browsable cinematics. Names are uppercased and the original filename buffer holds fifteen characters. Original skip tables hold up to twenty frame numbers. Zero terminates a skip table; a single zero defines no skip points. Values after the terminator or twentieth entry are ignored.

Playback numbers frames from one. A new skip input advances to the next configured forward point; after the last point it finishes the movie. Passed skip points are consumed during normal playback. Held input must be released before another skip. Returning from playback restores the menu and retains its selection. Playback marks the cinematic unlocked.

Subtitles begin at the start frame and end before the stop frame. Their always-display flag overrides the subtitle preference. The original wraps text at 35 characters and centers the lines at X 320, ending at Y 470, using the font's line height (26 for REDSEL). Glide playback draws the source's half-height frame over the 640 by 480 viewport. Interlace overlays black odd scanlines. Subtitles use their own copy of `FONTS\REDSEL.FNT` with the value pair `255,180`, not a `SYSTEM.INI` font: the main copy is drawn with alpha 180 and the shadow copy with alpha 140, offset (2, 2) rather than the (1, 1) of [dialogue subtitles](subtitles.md). Glyphs keep the font's own palette colors, so the text is gold. Advances, the space rule and centering follow [subtitles](subtitles.md#glyphs). The configured per-record colors are not used.

SYSTEM.INI [system] animation_drive prefixes movie filenames. The GOG configuration uses D:\, mapped to the supplied game.gog CD image. Its data track is MODE1/2352 and the eleven configured movies reside in the ISO 9660 root directory, not an ANIMS subdirectory. `import::cdrom::CdImage` reads root files without sector headers; it also accepts ordinary 2048-byte ISO images. No install data is modified.

## Unlock flags and queued playback

The movie table holds fifteen records, each with an unlock flag (bit 0). The session initializer, which runs at launch and again at New Game, clears every flag and sets it to MENU.INI `[general] force_movies` (bit 0). Playing a movie by index sets that record's flag before playback and sets the stored queued-movie value to -1 afterwards. Save games store the fifteen flag bytes and loading restores them, so unlocks follow the loaded save rather than the process.

On the Movies page, an entry whose action is 302 through 315 (movie 1 through 14) is hidden and unselectable while its flag is clear. Entry 301 (movie 0, the introduction) is always shown.

The stored queued-movie value is played by the main loop after a world loop ends and before the next world is saved to the temporary game and loaded: a value other than -1 plays that movie like any other, with the flag and queue effects above. Launch and New Game also play movie 0 at this point of the loop before the first world loads, and the ending plays movie 4 through the book (see [menu](menu.md)). [`QueueMovie`](SOUP.md#queuemovie) stores or plays an index as described there.

