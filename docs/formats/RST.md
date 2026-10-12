# RST editor state

`STATE.RST` stores sectioned text editor settings and file-position records.

## Overall Structure

```text
[Section header — bracketed section name]
[Assignments — ordered key=value lines]
[Additional sections — further headers and assignments]
```

The file is text with CRLF line endings. The parser also accepts LF line
endings. There is no binary header or byte-order requirement.

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
record. The parser preserves these values as text and retains section order,
entry order, and duplicate keys.

```ini
[edit-]
screen=80 50
toggles=1 1 0 1 0 0
srch=
file=d:\redguard\sound\dig.ini 1 1 1 7
```

Blank lines and lines beginning with a semicolon are ignored. Semicolons within
values remain part of the value. Assignments require a nonempty key and a
preceding section header. Malformed lines and non-text control bytes are
rejected.

Legacy text bytes map directly to Unicode code points U+0000–U+00FF. The parser
retains the original image without selecting a DOS code page.

## External References

- [INI section and assignment syntax](https://en.wikipedia.org/wiki/INI_file#Format)
