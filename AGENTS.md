# Agent Knowledge Base

## Project overview

Preserving *The Elder Scrolls Adventures: Redguard* (1998) — reverse-engineered file format specifications, engine documentation, and a Rust CLI for parsing and converting game assets.

Most formats are binary little-endian; ROB includes mixed endianness (some big-endian fields from Saturn-era heritage).

Primary outcomes in this repo:

- parser and converter behavior in `src/`
- verified format documentation in `docs/`
- engine discoveries not yet recorded on UESP or other community wikis (e.g. the hidden cheat system in `docs/engine/cheats.md`)

## Repository map

- Rust source: `src/`
- Importers/parsers: `src/import/`
- GLTF/GLB conversion: `src/gltf/` (`builder.rs`, `primitives.rs`, `terrain.rs`, `texture_cache.rs`)
- FFI native plugin: `src/ffi/` (`mod.rs` for GLB exports, `scene.rs` for mesh data + textures + audio + config, `buffer.rs` for memory management)
- CLI handlers: `src/cli/` (convert subcommands in `src/cli/convert/`)
- Error types: `src/error.rs`
- Format specifications: `docs/formats/`
- Engine discoveries: `docs/engine/` (`cheats.md`, `attachment.md`, `SOUP.md`, `SOUPDEF.md`)
- Configuration docs: `docs/config/`
- Docs index: `docs/README.md`

## Commit conventions

This project uses [Conventional Commits](https://www.conventionalcommits.org/). Every commit message must follow:

```
type(scope): description
```

**Types:** `feat`, `fix`, `refactor`, `perf`, `docs`, `style`, `test`, `build`, `ci`, `chore`

**Scopes** (optional, use when it clarifies): `ffi`, `cli`, `gltf`, `import`, `wld`, `rgm`, `rob`, `3d`, `texbsi`, `sfx`, `rtx`, `fnt`, `col`, `pvo`, `cht`

**Examples:**
- `feat(ffi): add scene-data FFI with pre-transformed mesh output`
- `fix(gltf): collapse nested if blocks for clippy 1.94`
- `perf: parallelize mesh primitive building with rayon`
- `docs: document FFI native plugin API in README`
- `ci: add FFI library assets to release workflow`

Release notes are auto-generated from these prefixes via `cliff.toml`.

## Working style for agents

- Verify claims from code or sample files before writing docs.
- Use neutral field names (`field_xx`, `unknown_xx`) when meaning is unproven.
- Do not promote hypotheses to facts.
- Keep edits focused and consistent with local style.

## Development checks

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

## Documentation conventions (`docs/`)

- **Game context** (game name/year/global notes) belongs in `docs/README.md` only.
- **Version-to-directory mapping** belongs in `docs/README.md` Asset Directories.
- Each format doc starts with a **one-line format-specific intro**.
- **Overall Structure** code blocks use bracketed outline style: `[Component — brief description]`. No inventory totals or right-aligned byte columns.
- Field tables use `Offset | Size | Type | Name | Description`.
- Add `Endian` only where mixed endianness is relevant.
- In field tables, **always backtick Type column values** (`` `u32` ``, `` `[u8; 4]` ``, `` `i16` ``). Use `—` (no backticks) when no type applies.
- Do not put endianness annotations inline in the Type column (e.g. `` `u32` (LE) ``). Use a separate `Endian` column or state endianness above the table.
- Include **verified counts/statistics** where known.
- Cross-link related formats (for example `3DC` -> `3D`, `ROB` -> `3D`).
- Sections describing **this project's CLI/converter behavior** (export notes, JSON sidecar output, parser quirks) go under `## Redguard Preservation CLI`, placed just before `## External References`. Demote subsections accordingly.
- End each format doc with `## External References`.
- In references, link to the **exact file/folder page**, not generic repo root links.

## Contributing to RGUnity

The [RGUnity/redguard-unity](https://github.com/RGUnity/redguard-unity) project is a Unity-based Redguard reimplementation. We contribute engine-correct fixes back to them via PRs from a personal fork.

### Setup (per session — `/tmp/` is ephemeral)

```bash
git clone https://github.com/RGUnity/redguard-unity.git /tmp/redguard-unity
cd /tmp/redguard-unity
git remote add fork https://github.com/michidk/redguard-unity.git
git fetch fork
```

Remotes after setup:
- `origin` → `RGUnity/redguard-unity` (upstream, default branch: `master`)
- `fork` → `michidk/redguard-unity` (personal fork, push target)

### Workflow

1. **Create a branch** off `master`:
   ```bash
   git checkout -b fix/descriptive-name origin/master
   ```
2. **Make changes** — the C# source is under `Assets/Scripts/RGFileImport/`.
3. **Push to fork**:
   ```bash
   git push fork fix/descriptive-name
   ```
4. **Create PR** via the GitHub MCP tool (`gh` CLI is not installed):
   ```
   mcp_github_create_pull_request(
     owner="RGUnity", repo="redguard-unity",
     title="...", head="michidk:fix/descriptive-name", base="master",
     body="..."
   )
   ```

### Key files in RGUnity

| File | What it does |
|------|-------------|
| `RGWLDFile.cs` | WLD terrain parsing (height table, grid scale, offsets) |
| `RG3DFile.cs` | 3D/3DC model importer |
| `RGROBFile.cs` | ROB segment handling |
| `RGRGMFile.cs` | RGM scene parsing |
| `ModelLoader.cs` | Object positioning (rope Y step, scale) |

All under `Assets/Scripts/RGFileImport/RGGFXImport/` (or `RGMData/` for script-related files).

### PR conventions

- Reference this project as source: `Source: https://github.com/michidk/redguard-preservation`
- Link to published docs where applicable (e.g. `https://michidk.github.io/redguard-preservation/formats/WLD.html#height-lookup-table`)
- Decompilation details (function addresses, binary offsets) are fine in RGUnity PR descriptions — the "no decomp references" rule applies only to `docs/` in *this* repo.
- Keep PRs focused: one logical fix per PR.

### Existing PR

- [#59 — fix terrain constants](https://github.com/RGUnity/redguard-unity/pull/59): grid scale 12.8, zero offsets, 128-entry height lookup table (open)

## External references

### Code references

| Reference | Scope |
|----------|-------|
| [uesp/redguard-3dfiletest `Redguard3dFile.cpp`](https://github.com/uesp/redguard-3dfiletest/blob/master/3DFileTest/Common/Redguard3dFile.cpp) | v2.6/v2.7 model header and decode behavior |
| [uesp/redguard-3dfiletest `RedguardTexBsiFile.cpp`](https://github.com/uesp/redguard-3dfiletest/blob/master/3DFileTest/Common/RedguardTexBsiFile.cpp) | TEXBSI record parsing |
| [RGUnity/redguard-unity `RG3DFile.cs`](https://github.com/RGUnity/redguard-unity/blob/master/Assets/Scripts/RGFileImport/RGGFXImport/RG3DFile.cs) | 3D/3DC importer behavior |
| [RGUnity/redguard-unity `RGROBFile.cs`](https://github.com/RGUnity/redguard-unity/blob/master/Assets/Scripts/RGFileImport/RGGFXImport/RGROBFile.cs) | ROB segment handling |
| [RGUnity/redguard-unity `RGRGMFile.cs`](https://github.com/RGUnity/redguard-unity/blob/master/Assets/Scripts/RGFileImport/RGGFXImport/RGRGMFile.cs) | RGM scene parsing |
| [RGUnity/redguard-unity `RGRGMScriptStore.cs`](https://github.com/RGUnity/redguard-unity/blob/master/Assets/Scripts/RGFileImport/RGMData/RGRGMScriptStore.cs) | RASC bytecode interpreter, flags table (369 entries) |
| [RGUnity/redguard-unity `soupdeffcn_nimpl.cs`](https://github.com/RGUnity/redguard-unity/blob/master/Assets/Scripts/RGFileImport/RGMData/soupdeffcn_nimpl.cs) | SOUP function ID-to-name table (367 functions) |
| [RGUnity/redguard-unity `RGWLDFile.cs`](https://github.com/RGUnity/redguard-unity/blob/master/Assets/Scripts/RGFileImport/RGGFXImport/RGWLDFile.cs) | WLD parsing |
| [Dillonn241/redguard-mod-manager `ScriptReader.java`](https://github.com/Dillonn241/redguard-mod-manager/blob/main/src/redguard/ScriptReader.java) | RASC bytecode disassembler |
| [Dillonn241/redguard-mod-manager `ScriptParser.java`](https://github.com/Dillonn241/redguard-mod-manager/blob/main/src/redguard/ScriptParser.java) | RASC bytecode assembler (round-trip verified) |
| [Dillonn241/redguard-mod-manager `MapFile.java`](https://github.com/Dillonn241/redguard-mod-manager/blob/main/src/redguard/MapFile.java) | RGM section reader/writer |
| [Dillonn241/redguard-mod-manager `MapHeader.java`](https://github.com/Dillonn241/redguard-mod-manager/blob/main/src/redguard/MapHeader.java) | RAHD record parser with verified offsets |
| [Dillonn241/redguard-mod-manager `MapDatabase.java`](https://github.com/Dillonn241/redguard-mod-manager/blob/main/src/redguard/MapDatabase.java) | SOUP386.DEF parser |

### UESP pages

| Page | Relevance |
|------|-----------|
| [Mod:Redguard File Formats](https://en.uesp.net/wiki/Mod:Redguard_File_Formats) | Master format overview |
| [Mod:Model Files](https://en.uesp.net/wiki/Mod:Model_Files) | 3D/3DC format notes |
| [User:Daveh/Redguard File Formats](https://en.uesp.net/wiki/User:Daveh/Redguard_File_Formats) | Extended reverse-engineering notes |
| [Redguard:Glide Differences](https://en.uesp.net/wiki/Redguard:Glide_Differences) | Software vs Glide renderer behavior |
