# Developer console

The Glide console accepts 82 registered commands with short aliases, plus 13 cheat names handled separately.

## Command dispatch and syntax

Press **F12** to open the console. Command names are case-insensitive. Each registered command has exactly the long name and short alias listed below. Other abbreviations are not accepted. The `commands` command, also spelled `?`, lists the 82 command pairs; `oracle` lists the 13 cheat names and their raw state values. There is no literal `cheat` prefix command.

The engine also has a programmatic command entry point. It accepts the long registered names and the cheat names, but does not look up short aliases.

Input is lowercased and normalized before dispatch. A semicolon ends the input; it does not separate commands to execute in sequence. Arguments are parsed by each handler. Spaces and commas are not interchangeable: for example, use `load 1,0,0`, `item add,ID`, `setflag NAME,VALUE`, and `object attr attr_health = 100`. Square brackets in this reference indicate optional arguments and are not typed.

Most numeric handlers accept decimal integers, negative decimal integers, `0x` hexadecimal values, named equates, and named SOUP flag values. An unresolved symbolic value prints a diagnostic and substitutes `1`. The percent-prefixed binary-input path returns zero for ASCII binary digits. Quoted numeric input reads the first four bytes following the quote as a little-endian 32-bit value; it is not general string evaluation. `delay` uses decimal conversion directly.

The console keeps a selected object, a monitored object, a selected rope and a selected light. These are separate selections. An object selected for inspection does not become the monitor target until `object monitor` is used.

## Interactive presentation and editor

The interactive console covers the game viewport with `SYSTEM/PAPER.GXA`.
At widths of at least 640 pixels, its standard font is the FNT named by
`SYSTEM.INI [system] gui_low_font`. The console draws that font with a single
ink color rather than the glyph palette colors. Its header and left text margin
start at the scaled equivalent of eight pixels in a 640 by 480 viewport.
The header identifies the development console and reports memory usage; a
horizontal separator precedes retained output and the command prompt.

Standard settings use palette entry zero for editing and header text. Report
text uses the GUI color nearest to six-bit RGB `(0, 42, 0)` in the active palette.
Palette matching examines entries 1 through 255 in six-bit RGB. An exact match
wins immediately. White entries whose component sum is 189 are otherwise skipped.
For each component, the difference is target minus candidate when nonnegative,
and candidate minus target minus one otherwise. Candidates with all three
differences below nine are compared by the absolute difference of their weighted
RGB sums, using weights 19661, 38666 and 7209. Equal scores select the later entry.
If this pass finds no candidate or its score exceeds 24, a second pass minimizes
the sum of the three component differences, again preferring later ties.
The console has 512 retained lines, with at most 100 characters in each editable
command line. A `>` prompt distinguishes command lines from report lines.

F12 and Escape leave the interactive console. Left and Right move the cursor;
Backspace removes the preceding character and Delete removes the following
character. Typing inserts printable characters at the cursor. Up and Down select
retained lines, including report lines; report lines are not editable. Tab copies
a preceding command into the current editable line. Home moves to the start of
the line, then to the oldest retained line when already at the start. Page Up and
Page Down move through retained output. Enter dispatches an editable command.
The interactive console runs its own input and drawing loop before returning to
the caller's game loop.

Console preferences are stored in [REDGUARD.CPF](../formats/CPF.md).

## Registered commands

Each row is one command. The alias invokes the same handler from the interactive console.

### Console and diagnostics

| Command | Alias | Arguments and behavior |
|---|---|---|
| `commands` | `?` | `commands`. Lists the 82 registered commands and their aliases. The 13 cheat names are listed separately by oracle. |
| `show` | `sh` | `show <topic>`. Reports engine, world, player, object, memory, hardware, sound, and navigation information. Some topics toggle debug displays or write GRID.LOG. See subcommands. |
| `console` | `con` | `console <option>`. Changes console appearance, clears history, exports output, and saves console preferences. |
| `print` | `lpt` | `print page / all`. Sends the visible console page or retained output to a DOS printer. This is printer output, not an on-screen print command. |
| `hex` | `x` | `hex [address / last]`. Displays memory as hexadecimal bytes and characters; omission continues the previous dump. |
| `recover` | `r` | `recover`. Requests recovery from the recoverable error state. Refuses when there is no error or system integrity is considered bad. |
| `break` | `b` | `break`. Enters the engine error path with the critical-drive-error code. Not a general-purpose script breakpoint. |
| `poke` | `p` | `poke address,byte[,byte...]`. Validates an address and byte values, then displays one line of memory. Contains no memory write. Values outside -128 through 255 are rejected. |
| `restart` | `rs` | `restart video / xngine`. Restarts the video subsystem or the rendering engine. Does not start a new game. |
| `delay` | `dl` | `delay [number]`. Sets an artificial delay in the main loop; omission or zero clears it. |

### State and scripts

| Command | Alias | Arguments and behavior |
|---|---|---|
| `get` | `g` | `get pattern`. Looks up console variables by name pattern. The Glide build has an empty console-variable table, so no variables match. |
| `set` | `s` | `set pattern,value[,value...]`. Parses assignments to console variables. The same empty table prevents assignments in this build. |
| `getflag` | `gf` | `getflag pattern`. Lists matching SOUP flags, their types, values, and set status. |
| `setflag` | `sf` | `setflag pattern,value[,status]`. Changes matching SOUP flag values and optionally their status. A nonzero status sets the set bit. Zero XORs the metadata byte with 239; it does not clear the set bit. |
| `script` | `sc` | `script log / disable [name] / block`. Writes a script log, suppresses updates for actors using a named script, or advances the script block list. disable without a name clears the suppression; this is not an arbitrary SOUP-expression evaluator. |

### Objects and inventory

| Command | Alias | Arguments and behavior |
|---|---|---|
| `object` | `o` | `object <operation>`. Selects, inspects, edits, monitors, or teleports to objects; also provides an interactive kill mode. Includes attribute assignment and subcommands missing from help. See subcommands. |
| `logplayer` | `pl` | `logplayer off / console / screen`. Disables player diagnostics or toggles their console/screen output. |
| `logmonitor` | `ml` | `logmonitor off / console / screen`. Disables monitored-object diagnostics or toggles their console/screen output. |
| `rope` | `rp` | `rope heap / select / general`. Lists ropes, interactively selects one, or reports the selected rope and its attached objects. |
| `item` | `i` | `item <operation>`. Lists inventory definitions or player items, adds or removes one item, or drops a specified amount. |
| `playerdist` | `pd` | `playerdist`. Reports the distance between the selected console object and the player. |
| `cameradist` | `cd` | `cameradist`. Reports the distance between the selected console object and the camera. |

### World and navigation

| Command | Alias | Arguments and behavior |
|---|---|---|
| `load` | `l` | `load world[,marker][,angle]`. Requests a world load with optional marker and angle. Omitted marker and angle default to zero. |
| `setmarker` | `sm` | `setmarker number`. Saves the current player/camera placement as a user marker in REDGUARD.MRK. |
| `getmarker` | `gm` | `getmarker number`. Restores a user marker for the current world from REDGUARD.MRK. |
| `setnetmark` | `snm` | `setnetmark number`. Saves a user marker in the configured shared marker file. Uses a file path from SYSTEM.INI; it is not a multiplayer command. |
| `getnetmark` | `gnm` | `getnetmark number`. Restores a user marker from that configured shared file. |
| `openputnode` | `op` | `openputnode [filename]`. Opens the PUT-node authoring system, optionally with a file. |
| `saveputnode` | `sp` | `saveputnode [filename]`. Writes the PUT nodes to the supplied filename or the filename used when opened. |
| `closeputnode` | `cp` | `closeputnode`. Closes the PUT-node authoring system. |
| `getmapmarker` | `gmm` | `getmapmarker number`. Moves the player to a marker authored in the current map. |
| `walk` | `w` | `walk [filename]`. Generates a walk map from PUT nodes. Requires at least two nodes and a filename; enters the generation workflow and then exits through the engine shutdown path. |
| `node` | `n` | `node <operation>`. Inspects navigation maps/nodes, toggles diagnostic displays, logs node data, and clears node occupancy. The help-only spelling showinuse is not accepted. |

### Developer tests

| Command | Alias | Arguments and behavior |
|---|---|---|
| `test` | `t` | `test`. Writes and reloads the player object through OBJECT.SAV, then compares its state. This changes live object state and writes files. |
| `test2` | `t2` | `test2`. Writes GRID1.LOG and GRID2.LOG around an internal world save/load test. Also uses GRID.LOG and the internal save/load workflow. |
| `test3` | `t3` | `test3`. Writes and reloads the monitored object through OBJECT.SAV, then compares its state. Requires a monitored object; changes live object state and writes files. |

### Rendering and visibility

| Command | Alias | Arguments and behavior |
|---|---|---|
| `render` | `rd` | `render on / off / static`. Enables/disables rendering or toggles static-object rendering. |
| `scene` | `sn` | `scene`. Enters an interactive scene inspection mode with information under the pointer. |
| `pvotreeinfo` | `pvoi` | `pvotreeinfo`. Reports the loaded potentially-visible-object tree, lists, and patch count. |
| `pvoaddpatch` | `pvoa` | `pvoaddpatch`. Interactively picks an object to add to the PVO patch list. |
| `pvodeletepatch` | `pvod` | `pvodeletepatch`. Interactively picks an object to remove from the PVO patch list. |
| `pvoonoff` | `pvo` | `pvoonoff`. Toggles the PVO visibility-system switch. Requires a loaded PVO tree. |
| `pvopatchsave` | `pvos` | `pvopatchsave`. Saves the current PVO patches. |
| `pvotreeload` | `pvol` | `pvotreeload filename`. Loads another PVO tree. The command first requires a PVO tree to be loaded already. |

### Lighting editor

| Command | Alias | Arguments and behavior |
|---|---|---|
| `fxlightflag` | `lf` | `fxlightflag`. Activates the light-editing controls and selects a light, or clears/restores the light selection. |
| `fxsavelights` | `sl` | `fxsavelights`. Writes the current world light definitions to its world light file. |
| `fxgolight` | `fxgl` | `fxgolight number`. Moves the player to a numbered light and selects it. Requires lights and the light-editing mode. |
| `fxselect` | `fxsel` | `fxselect`. Interactively selects a light. |
| `fxeditred` | `fxer` | `fxeditred value`. Sets the selected light's red channel. Clamped to -255 through 255. |
| `fxeditgreen` | `fxeg` | `fxeditgreen value`. Sets the selected light's green channel. Clamped to -255 through 255. |
| `fxeditblue` | `fxeb` | `fxeditblue value`. Sets the selected light's blue channel. Clamped to -255 through 255. |
| `fxeditradius` | `fxerad` | `fxeditradius value`. Sets the selected light's radius. Negative input becomes zero; storage is 16-bit. |
| `fxlightmove` | `fxlm` | `fxlightmove value`. Sets the light editor's movement-step value. |
| `fxincstep` | `fxis` | `fxincstep value`. Sets the light editor's increment-step value. |
| `fxmanualx` | `fxmx` | `fxmanualx value`. Sets the selected light's X coordinate. Requires an active light editor and selected light. |
| `fxmanualy` | `fxmy` | `fxmanualy value`. Sets the selected light's Y coordinate. Requires an active light editor and selected light. |
| `fxmanualz` | `fxmz` | `fxmanualz value`. Sets the selected light's Z coordinate. Requires an active light editor and selected light. |
| `fxscriptname` | `fxsn` | `fxscriptname name`. Assigns the selected light's script name. Retains at most eight characters; requires an active light editor and selected light. |

### Sky, lighting and water

| Command | Alias | Arguments and behavior |
|---|---|---|
| `fxtexturelist` | `fxl` | `fxtexturelist start`. Lists up to 40 texture entries starting at an index. A starting index is required despite the brackets in the built-in usage text. |
| `fxshowtexture` | `fxsh` | `fxshowtexture [number]`. Shows a texture and its metadata; omission disables the texture preview. |
| `fxsunangle` | `sang` | `fxsunangle value`. Sets the sun angle. Clamped to 0 through 2048. |
| `fxsunskew` | `sskew` | `fxsunskew value`. Sets the sun skew. Clamped to 0 through 2048. |
| `fxsunscale` | `sscale` | `fxsunscale value`. Sets the sun-image scale. |
| `fxsunred` | `sred` | `fxsunred value`. Sets the sun lighting red channel. Clamped to -255 through 255. |
| `fxsungreen` | `sgre` | `fxsungreen value`. Sets the sun lighting green channel. Clamped to -255 through 255. |
| `fxsunblue` | `sblu` | `fxsunblue value`. Sets the sun lighting blue channel. Clamped to -255 through 255. |
| `fxsunbrightness` | `sbri` | `fxsunbrightness value`. Sets sun-light brightness. Stored as a signed 16-bit value. |
| `fxsunimgred` | `sired` | `fxsunimgred value`. Sets the sun image red channel. Clamped to -255 through 255. |
| `fxsunimggreen` | `sigre` | `fxsunimggreen value`. Sets the sun image green channel. Clamped to -255 through 255. |
| `fxsunimgblue` | `siblu` | `fxsunimgblue value`. Sets the sun image blue channel. Clamped to -255 through 255. |
| `fxskyscale` | `skysc` | `fxskyscale value`. Sets the current world's sky scale. |
| `fxskylevel` | `skyl` | `fxskylevel value`. Sets the current world's sky level. |
| `fxskyspeed` | `skysp` | `fxskyspeed value`. Sets the current world's sky scrolling speed. |
| `fxfogred` | `fred` | `fxfogred value`. Sets the fog red channel. Clamped to -255 through 255. |
| `fxfoggreen` | `fgre` | `fxfoggreen value`. Sets the fog green channel. Clamped to -255 through 255. |
| `fxfogblue` | `fblu` | `fxfogblue value`. Sets the fog blue channel. Clamped to -255 through 255. |
| `fxambientred` | `ared` | `fxambientred value`. Sets the ambient light red channel. Clamped to -255 through 255. |
| `fxambientgreen` | `agre` | `fxambientgreen value`. Sets the ambient light green channel. Clamped to -255 through 255. |
| `fxambientblue` | `ablu` | `fxambientblue value`. Sets the ambient light blue channel. Clamped to -255 through 255. |
| `fxwaveamp` | `wamp` | `fxwaveamp value`. Sets the current world's wave amplitude. |
| `fxwavespeed` | `wsp` | `fxwavespeed value`. Sets the current world's wave speed. |
| `fxwavefreq` | `wfrq` | `fxwavefreq value`. Sets the current world's wave frequency. |

## Subcommands

Comma-separated names in the first column denote separate alternatives unless the row shows an argument form such as `add,id`.

### show

| Subcommand or argument form | Behavior |
|---|---|
| `memory, free` | Memory and heap reports. |
| `world` | Current world configuration, including sky, lighting and wave settings. |
| `video` | Video subsystem report. |
| `objects` | Object and model totals. |
| `player` | Player position, animation, combat and item state. |
| `xngine` | Camera and renderer settings. |
| `ai` | Loaded AI script files. |
| `3d` | Loaded ROB models. |
| `library` | Object-library entries. |
| `registry` | Resource registry, archives and open-file information. |
| `grid [number / all]` | Grid contents; the all form also writes GRID.LOG. |
| `map` | Grid/object information with GRID.LOG output. |
| `sound` | Sound-system state. |
| `group number, user number` | Members of a group or user group, numbered 1 through 255. |
| `active, list, activelist` | Object-chain reports. |
| `manager [handle]` | Model-cache report, optionally for a handle. |
| `machine` | Machine and DOS environment information. |
| `rain` | Rain-system state. |
| `remap` | Objects in the remap list. |
| `marker [flats]` | Toggles marker display; flats selects the flat-marker display. |
| `all` | Runs the main collection of diagnostic reports. |

### console

| Subcommand or argument form | Behavior |
|---|---|
| `clear` | Clears console history. |
| `write` | Writes retained output to CONSOLE.LOG. |
| `font` | Switches the console font choice. |
| `shadow` | Toggles text shadow. |
| `save` | Saves console preferences to REDGUARD.CPF. |
| `col0, col1, col2, col3` | Separate subcommands that cycle edit, report, on-screen and paper colours respectively. col3 requires paper to be off. |
| `hex,number` | Sets the number of lines in a hex dump. |
| `limit,number` | Sets the screen limit, from 0 through 300. |
| `paper` | Toggles the paper background. |
| `show` | Reports console preferences. |

### object

| Subcommand or argument form | Behavior |
|---|---|
| `select` | Interactively picks an object. |
| `closest` | Selects the closest eligible object to the player. |
| `player` | Selects the player. |
| `master number` | Selects a master object, numbered 1 through 255. |
| `search script-pattern` | Finds objects by script name; one match is selected automatically. |
| `address, "name"` | Alternative selection forms using a numeric address or a library object name. |
| `general, quick, coord` | General state, a short status report, or coordinates. |
| `anim` | Reports the current animation frame. |
| `size` | Reports object size information. |
| `combat, vcollide` | Combat state or vertex collision spheres. |
| `matrix, locals, locations, strings, script, family` | Separate reports for the corresponding object data. |
| `attr` | Lists named attributes and their values. |
| `attr name = value` | Assigns an attribute byte; the equals sign is optional. |
| `monitor` | Makes the selected object the monitor target, or clears it if already monitored. |
| `goto` | Moves the player to the selected object. |
| `die` | Enters interactive object-killing mode. |
| `zap amount` | The supplied-amount form reports a missing argument. The handler has a reversed presence check. |

### item

| Subcommand or argument form | Behavior |
|---|---|
| `player` | Lists item types with a nonzero player count. |
| `show[,id]` | Lists all item definitions or one item. |
| `add,id` | Adds one item to the player. |
| `sub,id` | Removes one item from the player. |
| `drop,id,amount` | Drops the requested amount. |

### node

| Subcommand or argument form | Behavior |
|---|---|
| `maps` | Lists loaded node maps. |
| `monitor` | Toggles node-marker display for the monitored object. |
| `monitor dnode` | Reports the monitored object's destination node. |
| `monitor node` | Reports the monitored object's current node. |
| `monitor log` | Toggles node logging for the monitored object. |
| `shownode` | Toggles object-node markers. |
| `showmap` | Toggles node-map display. |
| `player` | Toggles player-node and route display. |
| `log map-id node-number` | Writes the chosen node report to NODE.LOG. |
| `inuse` | Reports repeated node-occupancy IDs. |
| `sweep` | Clears node occupancy throughout the loaded maps. |

### script

| Subcommand or argument form | Behavior |
|---|---|
| `log` | Opens SCRIPT.LOG, enables script logging, and requests a return from the console. |
| `disable [name]` | Suppresses the actor update path for the matching script basename. Omission clears it; basenames must be one through eight characters. |
| `block` | Advances to the next entry in the engine's script-block list. |

### rope

| Subcommand or argument form | Behavior |
|---|---|
| `heap` | Reports all ropes. |
| `select` | Interactively picks a rope. |
| `general` | Reports the selected rope and its attached objects. |

### restart

| Subcommand or argument form | Behavior |
|---|---|
| `video` | Restarts the video subsystem. |
| `xngine` | Restarts the rendering engine. |

### render

| Subcommand or argument form | Behavior |
|---|---|
| `on, off` | Enables or disables rendering. |
| `static` | Toggles static-object rendering. |

### logplayer

| Subcommand or argument form | Behavior |
|---|---|
| `off` | Clears both logging destinations. |
| `console, screen` | Separately toggles each output destination. |

### logmonitor

| Subcommand or argument form | Behavior |
|---|---|
| `off` | Clears both logging destinations. |
| `console, screen` | Separately toggles each output destination. |

## Cheat names

These 13 names are also accepted as console commands: `oracle`, `nodemarker`, `moonraker`, `task`, `animation`, `magiccarpet`, `savecheats`, `drevil`, `drno`, `goldfinger`, `neversaydie`, `oddjob`, and `yeahbaby`.

`oracle` and `savecheats` perform actions. The remaining names accept a toggle or `name = value`. Their numeric activation conditions differ. See [Cheat system](cheats.md) for the effects and per-cheat rules.

## Differences between help and command behavior

- `show` accepts `rain`, `free`, `map`, `list` and `activelist` even though its usage list omits them.
- `object anim` and `object size` are accepted but omitted from the object usage list.
- The node help lists `node showinuse`, but the handler does not accept that spelling. `node inuse` is accepted.
- `fxtexturelist` requires an index despite the brackets in its usage message. `fxshowtexture` without an index turns its preview off.
- `poke` has no memory-write operation. Successful byte validation is followed by a one-line hex display.
- `get` and `set` use an empty console-variable table in the Glide build. SOUP flag access through `getflag` and `setflag` is separate and uses names loaded from SOUP386.DEF.
- `object zap` tests argument presence backwards. Supplying an amount produces its missing-argument message; omitting it reaches decimal conversion of the empty token.
- The optional zero status in `setflag` XORs the flag metadata with `239`, leaving the set bit unchanged and flipping the other bits. A nonzero status sets the set bit.

## External references

- [Redguard Console on UESP](https://en.uesp.net/wiki/Redguard:Console)
- [Cheat system](cheats.md)
- [SOUP scripting](SOUP.md)
