# RST editor state

`STATE.RST` stores sectioned text editor settings and file-position records.

Its `file` entries refer to editor documents such as `d:\redguard\sound\dig.ini`.
These are remembered editing positions, separate from game save state and
runtime audio configuration. See [Miles audio configuration](../config/miles-ini.md).

## Overall Structure

```text
[Section header — bracketed section name]
[Assignments — ordered key=value lines]
[Additional sections — further headers and assignments]
```

The file is text with CRLF line endings. There is no binary header or
byte-order requirement.

## Sections and assignments

`STATE.RST` contains these sections:

| Section | Keys |
|---|---|
| `edit-` | `screen`, `toggles`, `srch`, `src`, `rpl`, `file` |
| `brief` | `file` |
| `shared-` | `pmark` |

Section names retain their trailing hyphens. Assignments split at the first
equals sign. Empty values are valid. Values can contain Windows paths,
space-separated fields, and further equals signs, such as `c=0` in a `file`
record. Repeated `file` keys represent separate editor records, so section and
entry order are part of the stored state.

```ini
[edit-]
screen=80 50
toggles=1 1 0 1 0 0
srch=
file=d:\redguard\sound\dig.ini 1 1 1 7
```

Blank lines separate records, and lines beginning with a semicolon are
comments. Semicolons within values remain part of the value. Each assignment
belongs to the preceding section header.

The file contains legacy text bytes. Their displayed characters depend on the
editor's code page.

## External References

- [INI section and assignment syntax](https://en.wikipedia.org/wiki/INI_file#Format)
