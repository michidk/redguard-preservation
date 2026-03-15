# REGISTRY.INI

File-system abstraction configuration controlling archive (`.ZAP`) lookup, 32-bit file access, and registry path resolution.

Shipped path: `REGISTRY.INI` (game root directory, 302 bytes).

The registry subsystem configured by this file is non-functional in the shipped game. The console command `show registry` reports the registry system is not open, and altering this file has no observable effect at runtime.

## Sections

### `[registry]`

Archive-to-path mapping entries. Each line maps a `.ZAP` archive file to a file glob pattern:

```
3DART\OBJECTS.ZAP   3DART\*.3D
SYSTEM\GXA.ZAP      SYSTEM\*.GXA
INI.ZAP              *.INI
```

The shipped file also contains a commented-out entry: `SOUP386\SOUP386.ZAP  SOUP386\*.ai`.

These entries would allow the engine to look up files inside `.ZAP` archives instead of (or in addition to) loose files on disk. In practice, the GOG release does not ship any `.ZAP` archive files — all assets are loose.

### `[file32]`

File access configuration flags:

| Key | Default | Description |
|---|---|---|
| `use_registry` | `false` | Enable the Windows registry path lookup subsystem. |
| `use_32bit` | `true` | Use 32-bit file I/O (DOS/4GW extended file access). |
| `archive_file_first` | `false` | Check `.ZAP` archives before loose files. |
| `local_path` | `true` | Use local (relative) file paths. |
| `ignore_registry_errors` | `false` | Suppress errors from registry path lookup failures. |
| `registry_log` | `false` | Enable logging of registry system operations. |
| `advanced_exist` | `true` | Use advanced file-existence checking. |

## External References

- [UESP: Redguard:Console](https://en.uesp.net/wiki/Redguard:Console) (documents `show registry` command)
