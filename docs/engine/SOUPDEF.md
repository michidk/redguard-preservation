# SOUP386.DEF

Runtime text definition file that declares the SOUP scripting API surface (functions, references, attributes, and flags).

## Overview

`SOUP386.DEF` is loaded by the runtime and used to build script-call metadata at startup. Compiled script bytecode in `RASC`/`.AI` refers to function and flag indices that are resolved using this definition file.

The file is plain text and organized as named sections.

## Section Layout

| Section | Delimiter | Content |
|---|---|---|
| Functions | `[functions]` ... `[refs]` | One callable entry per line: `<type> <name> params <count>` where type is `task` or `function` |
| References | `[refs]` ... `auto` | One reference/equate name per line |
| Attributes | `auto` ... `endauto` | One attribute name per line (maps to per-actor RAAT byte slots) |
| Flags | `[flags]` ... EOF | One flag per line: `<type> <name> <value>[;<comment>]` with types `BOOL`, `NUMBER`, `FLIPFLOP` |

Function index 0 is treated as `NullFunction` in runtime behavior.

## Relationship to RGM Script Data

- `RASC` and standalone `.AI` contain compiled script bytecode. No `.AI` files ship with the game — see [SOUP Scripting](SOUP.md) for details.
- Bytecode function calls encode function IDs (u16 indices).
- Those indices are resolved using the runtime function table built from `[functions]` in `SOUP386.DEF`.
- RAAT attribute bytes are interpreted using names declared in the `auto` ... `endauto` attribute block.

See [RGM.md](../formats/RGM.md) for `RASC`/`RAAT` container details and [SOUP.md](SOUP.md) for script-source boundaries.

## Notes

- Runtime behavior includes a DEF-to-script compatibility/checksum validation path in the engine.
- Some declared functions do not appear in shipped map scripts; declaration in DEF does not imply invocation.

## External References

- [RGUnity/redguard-unity `soupdeffcn_nimpl.cs`](https://github.com/RGUnity/redguard-unity/blob/master/Assets/Scripts/RGFileImport/RGMData/soupdeffcn_nimpl.cs)
- [Dillonn241/redguard-mod-manager `MapDatabase.java`](https://github.com/Dillonn241/redguard-mod-manager/blob/main/src/redguard/MapDatabase.java)
- [UESP `Mod:RGM File Format`](https://en.uesp.net/wiki/Mod:RGM_File_Format)
