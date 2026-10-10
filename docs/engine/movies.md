# Cinematic playback

Movies use the installed MENU.INI definitions and Smacker files, not recorded playback.

`import::menu_movies::parse_menu_movies` reads ordered names, skip points, and subtitle records from [page3]. A subtitle has seven comma-separated fields: an always-display flag, red, green, blue, start frame, stop frame, and text. Text can contain commas. The examined install defines eleven filenames and five browsable cinematics. Names are uppercased and the original filename buffer holds fifteen characters. Original skip tables hold up to twenty frame numbers.

Playback numbers frames from one. A new skip input advances to the next configured forward point; after the last point it finishes the movie. Passed skip points are consumed during normal playback. Held input must be released before another skip. Returning from playback restores the menu and retains its selection. Playback marks the cinematic unlocked.

Subtitles begin at the start frame and end before the stop frame. Their always-display flag overrides the subtitle preference. The original wraps text at 35 characters and centers the lines at X 320, ending at Y 470, using the current low-resolution GUI font's line height. The examined Glide playback draws the source's half-height frame over the 640 by 480 viewport. Interlace overlays black odd scanlines. The configuration contains per-subtitle colors, but this investigation does not establish that the Glide text renderer applies those colors.

SYSTEM.INI [system] animation_drive prefixes movie filenames. The examined GOG configuration uses D:\, mapped to the supplied game.gog CD image. Its data track is MODE1/2352 and the eleven configured movies reside in the ISO 9660 root directory, not an ANIMS subdirectory. `import::cdrom::CdImage` reads root files without sector headers; it also accepts ordinary 2048-byte ISO images. No install data is modified.

These rules were verified from the installed configuration, the CD directory and INTRO header, and the movie-definition initialization, playback, subtitle and Glide presentation routines. Exact texture filtering, sound volume policy, other input devices, and software-renderer differences remain outside this investigation.
