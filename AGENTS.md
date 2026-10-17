# Agent Knowledge Base

## Project overview

Preserving *The Elder Scrolls Adventures: Redguard* (1998) — reverse-engineered file format specifications, engine documentation, and a Rust CLI for parsing and converting game assets.

The repository ships four things from one Cargo package (`redguard-preservation`):

- **Docs** in `docs/`, published with mdBook at <https://michidk.github.io/redguard-preservation/>. This is the main product: finished reference material for formats, engine behavior, and INI settings.
- **CLI** `rgpre` (`src/main.rs`) with `read`, `scan`, and `convert` commands.
- **Rust library** `rgpre` (`src/lib.rs`), which holds all parsing and conversion logic. The CLI is a thin layer over it.
- **Native plugin**: the same library built as a `cdylib` with a C API (`src/ffi/`), used by Unity-style consumers.

[Cyrus](https://github.com/michidk/cyrus), a Rust reimplementation of Redguard, depends on the crate, so public library APIs and documented behavior are load-bearing.

The repo contains no game data apart from small synthetic files in `tests/fixtures/`. Anything that needs a real install reads the `REDGUARD_DIR` environment variable and skips itself when it is unset.

Most formats are binary little-endian; ROB includes mixed endianness (some big-endian fields from Saturn-era heritage).

Primary outcomes in this repo:

- parser and converter behavior in `src/`
- format reference documentation in `docs/`
- engine discoveries not yet recorded on UESP or other community wikis (e.g. the hidden cheat system in `docs/engine/cheats.md`)

## Repository map

- Rust source: `src/` (`main.rs` CLI entry, `lib.rs` library root, `opts.rs` clap definitions, `geometry.rs` coordinate conventions)
- Importers/parsers: `src/import/` (one module per format or INI file; `model3d/`, `rgm/`, `rob/`, and `bsi/` are directories)
- GLTF/GLB conversion: `src/gltf/` (`builder.rs`, `primitives.rs`, `terrain.rs`, `texture_cache.rs`)
- FFI native plugin: `src/ffi/` (`mod.rs` for GLB exports, `scene.rs` for mesh data + textures + audio + config, `world.rs` for world handles, `types.rs` for `#[repr(C)]` structs, `buffer.rs` for memory management, `rgpre.h` for the C header, `README.md` for the API reference)
- CLI handlers: `src/cli/` (`read.rs`, `scan.rs`, `filetype.rs` for auto-detection; convert subcommands in `src/cli/convert/`)
- Error types: `src/error.rs`
- Tests: `tests/` (integration tests; sample inputs in `tests/fixtures/`)
- Examples: `examples/` (developer tools that run against a game install via `REDGUARD_DIR`)
- Format specifications: `docs/formats/`
- Engine behavior: `docs/engine/` (index in `docs/engine/README.md`; includes `cheats.md`, `attachment.md`, `SOUP.md`, `SOUPDEF.md`, rendering, sound, menu, and HUD references)
- Configuration docs: `docs/config/`
- Docs index: `docs/README.md`; mdBook navigation: `docs/SUMMARY.md`; mdBook config and theme: `docs/book.toml`, `docs/theme/`
- Release notes config: `cliff.toml`; CI and publishing: `.github/workflows/`
- Root `README.md`: the public entry point for the CLI, library, and FFI (see [README conventions](#readme-conventions))

## Commit conventions

This project uses [Conventional Commits](https://www.conventionalcommits.org/). Every commit message must follow:

```
type(scope): description
```

**Types:** `feat`, `fix`, `refactor`, `perf`, `docs`, `style`, `test`, `build`, `ci`, `chore`

**Scopes** (optional, use when it clarifies): a format (`wld`, `rgm`, `rob`, `3d`, `texbsi`, `sfx`, `rtx`, `fnt`, `col`, `pvo`, `cht`, `keys`), a code area (`ffi`, `cli`, `gltf`, `import`), or, for docs, the subsystem described (`player`, `sound`, `menu`, `soup`, `animation`, `render`, `startup`, `combat`, `items`, `hud`, `config`). Prefer an existing scope from `git log` over inventing a new one.

Pull request titles are checked by CI against the same format. `cliff.toml` groups release notes by type (`feat`, `fix`, `perf`, `refactor`, `docs`, `style`, `test`) and skips `chore`.

**Examples:**
- `feat(ffi): add scene-data FFI with pre-transformed mesh output`
- `fix(gltf): collapse nested if blocks for clippy 1.94`
- `perf: parallelize mesh primitive building with rayon`
- `docs: document FFI native plugin API in README`
- `ci: add FFI library assets to release workflow`

## Working style for agents

- Verify claims from code, game data, or sample files before publishing them.
- Treat `docs/` as finished reference material, not a research log.
- Keep verification methods, evidence sections, debugger traces, runtime observations, test vectors, measurement logs, research history, open questions, coverage notes, and implementation status out of published docs. Put that material in the pull request, commit message, or untracked research notes.
- State established behavior directly. Write "The field is zero in all shipped files," not "Verified across all shipped files."
- Include counts and statistics only when they are stable, implementation-relevant properties of the format or engine. State them as direct facts.
- Remove unresolved hypotheses instead of publishing them. Use neutral names such as `field_xx` or `unknown_xx` when a field's meaning is not established.
- Keep edits focused and consistent with local style.

## Development checks

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

Tests use `tests/fixtures/` and pass without the game. To also run the install-backed tests and examples, set `REDGUARD_DIR` to a Redguard install directory.

When changing anything exported from `src/ffi/`, update `src/ffi/rgpre.h` by hand; it is not generated (see the checklist in `src/ffi/README.md`).

The docs site is built with `mdbook build docs` (`cargo install mdbook --locked`) only when `main` is deployed, not on pull requests. After editing `docs/`, check that links resolve and new pages are in `docs/SUMMARY.md`; build locally if you changed structure.

## README conventions

The root `README.md` is read first by people who may not know Redguard or this project. Keep this order: what the project is and what you need (your own game copy), install and a first working session, CLI reference, game editions, Rust library, native plugin, documentation map, then contributor material.

- Lead with tasks and working commands; put dense API detail after the quick start.
- Keep the convert table in step with `rgpre convert --help`. When a subcommand or flag changes, update the table and run the example commands.
- Keep format and engine detail in `docs/` and link to it. Do not duplicate specs in the README.
- Give every code example paths that exist in a real install, and say which edition (`fxart/` Glide or `3dart/` software) it assumes.

## Documentation conventions (`docs/`)

- **Game context** (game name/year/global notes) belongs in `docs/README.md` only.
- **Version-to-directory mapping** belongs in `docs/README.md` Asset Directories.
- Each format doc starts with a **one-line format-specific intro**.
- **Overall Structure** code blocks use bracketed outline style: `[Component — brief description]`. No inventory totals or right-aligned byte columns.
- Field tables use `Offset | Size | Type | Name | Description`.
- Add `Endian` only where mixed endianness is relevant.
- In field tables, **always backtick Type column values** (`` `u32` ``, `` `[u8; 4]` ``, `` `i16` ``). Use `—` (no backticks) when no type applies.
- Do not put endianness annotations inline in the Type column (e.g. `` `u32` (LE) ``). Use a separate `Endian` column or state endianness above the table.
- Include stable, implementation-relevant counts and statistics as direct facts.
- Cross-link related formats (for example `3DC` -> `3D`, `ROB` -> `3D`).
- End each format doc with `## External References`.
- In references, link to the **exact file/folder page**, not generic repo root links.
- A new doc page must be added to `docs/SUMMARY.md` (the mdBook navigation) and to the index table of its section (`docs/formats/README.md`, `docs/engine/README.md`, or `docs/config/README.md`).

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
