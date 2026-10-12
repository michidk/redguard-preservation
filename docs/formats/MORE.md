# MORE model information report

The extensionless `more` file is a text report of model filenames and exporter versions from Hugh's Marvelous 3D Information Program.

## Overall Structure

```text
[Program heading — tool name and version]
[Copyright line — publisher notice]
[Blank line — heading separator]
[Model entries — filename and version separated by a tab]
[Blank line — footer separator]
[Footer — declared number of files]
```

The report uses ASCII text and CRLF line endings. Its heading identifies
version `2.2` of the information program.

## Model Entries

Each entry contains a `.3D` filename, one tab, and a version label. The
label consists of `v`, a decimal major version, a period, and a decimal
minor version. Entries retain their original order.

```text
XBUCKETE.3D	v2.7
TV_BTTL3.3D	v2.7
XWANTED.3D	v4.0
```

The report ends with a decimal count followed by ` files found.`.
The shipped report lists 65 models and ends with `65 files found.`.

These are report-time version labels. They can differ from the versions in
the installed [3D model headers](models/3d.md). For example, the report lists
`XBUCKETE.3D` as `v2.7`, while the installed file has a `v5.0` header.
The report is not a model archive and contains no geometry or textures.

The related `fxart.txt` and `fxart3dc.txt` files are directory listings rather
than version reports. They record historical model sizes and timestamps. See
[Bundled support resources](BUNDLED-RESOURCES.md#directory-reports).

## External References

- [UESP model file formats](https://en.uesp.net/wiki/Mod:Model_Files)
