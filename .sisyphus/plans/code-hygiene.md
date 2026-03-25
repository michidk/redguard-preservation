# Code Hygiene Cleanup

## TL;DR

> **Quick Summary**: Behavior-preserving cleanup across 5 areas — eliminate production panics, deduplicate GLTF builder logic, consolidate CLI converter patterns, remove duplicate constants, and break up long parser functions.
>
> **Deliverables**:
> - Zero `unwrap()`/`expect()` in production code paths (5 instances → 0)
> - Deduplicated GLTF accessor builders and mesh-caching logic
> - Consistent palette loading and asset root resolution across CLI converters
> - Single source of truth for `ENGINE_UNIT_SCALE`
> - Shorter, focused parser functions in model3d/parser.rs
>
> **Estimated Effort**: Short
> **Parallel Execution**: YES — 3 waves
> **Critical Path**: Wave 1 (independent fixes) → Wave 2 (dedup/consolidation) → Wave 3 (parser restructure) → Final verification

---

## Context

### Original Request
"How can we make the code cleaner?" — general hygiene pass with no specific trigger.

### Interview Summary
**Key Discussions**:
- User selected 5 of 7 identified cleanup areas (excluded unit tests and "all of the above")
- Error handling: return errors via `?` + `eprintln!`/warning log at point of error
- Commit strategy: one atomic commit per task
- Test strategy: run existing integration tests at end only

**Research Findings**:
- Codebase is already well-organized: clean module structure, no circular deps, no dead code, no TODO/FIXME in source
- `error.rs` is well-designed (34 lines, thiserror) — just needs consistent usage
- Most `unwrap()`/`expect()` calls (39 of 44) are in `#[cfg(test)]` modules — only 5 are in production code
- GLTF builder has 3 near-identical accessor functions and 2 conversion functions sharing ~60 lines of mesh-caching logic
- WLD CLI converter duplicates palette loading and asset root resolution from `convert/mod.rs`

### Gap Analysis
**Corrected from initial assessment**: The explore agent reported ~20 production `unwrap`/`expect` calls, but careful review shows only 5 are in production paths. The rest are in test modules. This makes the unwrap cleanup a quick task rather than a medium one.

---

## Work Objectives

### Core Objective
Improve code cleanliness across 5 areas while preserving all existing behavior. Every change is a pure refactor — no functional changes.

### Concrete Deliverables
- Modified `src/import/rgm/parser.rs` — 3 `expect()` → proper error returns
- Modified `src/gltf/terrain.rs` — 1 `expect()` → proper error propagation
- Modified `src/ffi/mod.rs` — 1 `unwrap()` → proper error handling
- Modified `src/gltf/builder.rs` — 3 accessor functions → 1 generic helper
- Modified `src/gltf/mod.rs` — shared mesh-caching logic extracted to helper
- Modified `src/ffi/scene.rs` — `ENGINE_UNIT_SCALE` imported instead of redefined
- Modified `src/cli/convert/wld.rs` — reuses shared palette loading + asset root resolution
- Modified `src/import/model3d/parser.rs` — long functions broken into named helpers

### Definition of Done
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] `cargo test` passes (all integration tests)
- [ ] `cargo build` succeeds
- [ ] Zero `unwrap()`/`expect()` in non-test code paths (excluding intentional `unwrap_or_default`)

### Must Have
- All existing tests continue to pass unchanged
- No behavioral changes — identical outputs for identical inputs
- Each cleanup area gets its own commit

### Must NOT Have (Guardrails)
- No new features or functionality
- No changes to public API signatures (lib.rs exports, FFI function signatures)
- No changes to test code (`#[cfg(test)]` modules, `tests/` directory)
- No over-abstraction (e.g., don't introduce trait hierarchies for parsers)
- No documentation-only changes
- No dependency additions or removals

---

## Verification Strategy

> **ZERO HUMAN INTERVENTION** — ALL verification is agent-executed. No exceptions.

### Test Decision
- **Infrastructure exists**: YES (`tests/input_fixtures_integration.rs`)
- **Automated tests**: Run existing tests at end (no TDD, no new tests)
- **Framework**: `cargo test`

### QA Policy
Every task includes agent-executed verification. Evidence saved to `.sisyphus/evidence/`.

- **Compilation**: `cargo build` + `cargo clippy --all-targets --all-features -- -D warnings`
- **Tests**: `cargo test` (run in final verification wave)
- **Behavioral**: Spot-check that modified functions still work via compilation + test suite

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Start Immediately — independent quick fixes, MAX PARALLEL):
├── Task 1: Consolidate ENGINE_UNIT_SCALE constant [quick]
├── Task 2: Replace expect() in rgm/parser.rs [quick]
├── Task 3: Replace expect() in terrain.rs [quick]
└── Task 4: Replace unwrap() in ffi/mod.rs [quick]

Wave 2 (After Wave 1 — deduplication and consolidation):
├── Task 5: Deduplicate GLTF builder accessor functions [deep]
├── Task 6: Extract shared mesh-caching logic from gltf/mod.rs [deep]
└── Task 7: Consolidate CLI WLD converter to reuse shared helpers [quick]

Wave 3 (After Wave 2 — parser function restructuring):
├── Task 8: Break up parse_face_data into smaller helpers [quick]
└── Task 9: Break up parse_3d_file into smaller helpers [quick]

Wave FINAL (After ALL tasks):
├── Task F1: Plan compliance audit (oracle)
├── Task F2: Code quality review (unspecified-high)
├── Task F3: Real manual QA (unspecified-high)
└── Task F4: Scope fidelity check (deep)
-> Present results -> Get explicit user okay
```

### Dependency Matrix

| Task | Depends On | Blocks |
|------|-----------|--------|
| 1 | — | 5, 6 |
| 2 | — | — |
| 3 | — | — |
| 4 | — | — |
| 5 | 1 | — |
| 6 | 1 | — |
| 7 | — | — |
| 8 | — | 9 |
| 9 | 8 | — |

### Agent Dispatch Summary

- **Wave 1**: **4 tasks** — T1 → `quick`, T2 → `quick`, T3 → `quick`, T4 → `quick`
- **Wave 2**: **3 tasks** — T5 → `deep`, T6 → `deep`, T7 → `quick`
- **Wave 3**: **2 tasks** — T8 → `quick`, T9 → `quick`
- **FINAL**: **4 tasks** — F1 → `oracle`, F2 → `unspecified-high`, F3 → `unspecified-high`, F4 → `deep`

---

## TODOs

- [x] 1. Consolidate `ENGINE_UNIT_SCALE` to single definition

  **What to do**:
  - `ENGINE_UNIT_SCALE` (value `20.0`) is defined identically in two places:
    - `src/gltf/mod.rs:29` — `const ENGINE_UNIT_SCALE: f32 = 20.0;`
    - `src/ffi/scene.rs:25` — `const ENGINE_UNIT_SCALE: f32 = 20.0;`
  - Make `ENGINE_UNIT_SCALE` in `src/gltf/mod.rs` a `pub(crate)` const (it's already used by `terrain.rs` and `primitives.rs` via `super::ENGINE_UNIT_SCALE`)
  - In `src/ffi/scene.rs`, remove the local definition and import via `crate::gltf::ENGINE_UNIT_SCALE`
  - Verify all 4 usage sites in `gltf/` (terrain.rs:5,7,200 and primitives.rs:7,167-169) and both in `ffi/scene.rs:137-139` still compile

  **Must NOT do**:
  - Don't create a new `constants.rs` module — overkill for one constant
  - Don't change the value (20.0)
  - Don't change any public API

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 2, 3, 4)
  - **Blocks**: Tasks 5, 6 (they modify files that import this constant)
  - **Blocked By**: None

  **References**:

  **Pattern References**:
  - `src/gltf/mod.rs:29` — Current definition: `const ENGINE_UNIT_SCALE: f32 = 20.0;`
  - `src/ffi/scene.rs:25` — Duplicate definition to remove

  **API/Type References**:
  - `src/gltf/terrain.rs:5` — Imports as `use super::{ENGINE_UNIT_SCALE, ...}` — will keep working
  - `src/gltf/primitives.rs:7` — Imports as `use super::ENGINE_UNIT_SCALE;` — will keep working
  - `src/ffi/scene.rs:137-139` — Uses `ENGINE_UNIT_SCALE` for coordinate scaling

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: Constant consolidation compiles correctly
    Tool: Bash
    Steps:
      1. Run `cargo build 2>&1`
      2. Assert exit code 0
      3. Run `grep -rn 'const ENGINE_UNIT_SCALE' src/` and assert exactly 1 result
    Expected Result: Build succeeds; exactly one definition of ENGINE_UNIT_SCALE in codebase
    Evidence: .sisyphus/evidence/task-1-constant-consolidation.txt

  Scenario: Value unchanged in output
    Tool: Bash
    Steps:
      1. Run `grep -n 'ENGINE_UNIT_SCALE' src/gltf/mod.rs` — should show definition as 20.0
      2. Run `grep -n 'ENGINE_UNIT_SCALE' src/ffi/scene.rs` — should show usage, not definition
    Expected Result: Single definition with value 20.0; scene.rs uses import only
    Evidence: .sisyphus/evidence/task-1-constant-value-check.txt
  ```

  **Commit**: YES
  - Message: `refactor: consolidate ENGINE_UNIT_SCALE to single definition`
  - Files: `src/gltf/mod.rs`, `src/ffi/scene.rs`
  - Pre-commit: `cargo build`

- [x] 2. Replace `expect()` calls in `rgm/parser.rs` with error propagation

  **What to do**:
  - Replace 3 `expect()` calls in production code with proper error returns + warning log:
    - Line 73: `trailing_bytes.try_into().expect("trailing must be 2 bytes")` — This is inside a nom parser returning `IResult`. Since `take(2usize)` guarantees 2 bytes, the expect can't fail, but replace it with a nom error for consistency: use `let trailing: [u8; 2] = trailing_bytes.try_into().map_err(|_| nom::Err::Failure(nom::error::Error::new(input, nom::error::ErrorKind::Count)))?;` or similar pattern
    - Line 140: `.try_into().expect("MPOB script name must be 9 bytes")` — Same pattern, `take(9usize)` guarantees size
    - Line 144: `.try_into().expect("MPOB model name must be 9 bytes")` — Same pattern
  - Add `eprintln!` or `log::warn!` before returning the error (user preference: return error + log warning)
  - NOTE: These expects are "logically safe" since nom's `take(N)` guarantees the slice size. The cleanup is for consistency and to eliminate ALL production `expect()` calls.

  **Must NOT do**:
  - Don't modify test code in `rgm/tests.rs`
  - Don't change any struct definitions or public API
  - Don't change the parsing logic — only error handling style

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 3, 4)
  - **Blocks**: None
  - **Blocked By**: None

  **References**:

  **Pattern References**:
  - `src/import/rgm/parser.rs:73` — `let trailing: [u8; 2] = trailing_bytes.try_into().expect("trailing must be 2 bytes");`
  - `src/import/rgm/parser.rs:138-144` — Two `try_into().expect()` calls for fixed-size arrays
  - `src/import/rgm/parser.rs:22-34` — `section_header` function shows the established nom error pattern in this file

  **API/Type References**:
  - `src/import/rgm/parser.rs:52-87` — `mps_record` function (context for line 73 change)
  - `src/import/rgm/parser.rs:130-204` — `mpob_record` function (context for lines 140, 144)

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: No expect() in production rgm/parser.rs
    Tool: Bash
    Steps:
      1. Run `cargo build 2>&1` — assert exit code 0
      2. Extract non-test code from src/import/rgm/parser.rs (lines before #[cfg(test)]) and grep for `.expect(`
      3. Assert zero matches
    Expected Result: Build succeeds; zero expect() calls in production code of rgm/parser.rs
    Evidence: .sisyphus/evidence/task-2-rgm-expect-audit.txt

  Scenario: Parser still handles valid input
    Tool: Bash
    Steps:
      1. Run `cargo test` — assert all tests pass (existing tests exercise these parsers)
    Expected Result: All tests pass
    Evidence: .sisyphus/evidence/task-2-test-pass.txt
  ```

  **Commit**: YES
  - Message: `refactor(rgm): replace expect() with proper error propagation`
  - Files: `src/import/rgm/parser.rs`
  - Pre-commit: `cargo build`

- [x] 3. Replace `expect()` in `terrain.rs` with error propagation

  **What to do**:
  - Replace 1 `expect()` call in production code:
    - `src/gltf/terrain.rs:71`: `f32::from(u16::try_from(value).expect("terrain grid coordinate exceeds u16::MAX"))` inside `scaled_grid_coordinate(value: usize) -> f32`
  - This function currently returns `f32` (no Result). Change signature to return `Result<f32>` and propagate the error via `?` with a descriptive `Error::Conversion` message
  - Update all call sites of `scaled_grid_coordinate` (lines 78, 80 in `terrain_position`) to propagate the error — this means `terrain_position` also needs to return `Result<[f32; 3]>`
  - Chain the Result propagation up through callers: check `build_wld_unrolled_primitives` which already returns `Result<Vec<UnrolledPrimitive>>`
  - Add `eprintln!` or `log::warn!` before returning the error

  **Must NOT do**:
  - Don't change the WLD_HEIGHT_TABLE or WLD_SIZE_SCALE constants
  - Don't change the terrain generation logic
  - Don't modify test code

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 2, 4)
  - **Blocks**: None
  - **Blocked By**: None

  **References**:

  **Pattern References**:
  - `src/gltf/terrain.rs:69-73` — Current `scaled_grid_coordinate` function with expect()
  - `src/gltf/terrain.rs:76-82` — `terrain_position` function that calls it
  - `src/gltf/terrain.rs:84-` — `build_wld_unrolled_primitives` (already returns `Result<...>`, check how it calls terrain_position)
  - `src/error.rs` — Error::Conversion variant for the error message

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: No expect() in terrain.rs
    Tool: Bash
    Steps:
      1. Run `cargo build 2>&1` — assert exit code 0
      2. Run `grep -n '.expect(' src/gltf/terrain.rs` — assert zero matches
    Expected Result: Build succeeds; zero expect() in terrain.rs
    Evidence: .sisyphus/evidence/task-3-terrain-expect-audit.txt
  ```

  **Commit**: YES
  - Message: `refactor(gltf): replace expect() with error propagation in terrain`
  - Files: `src/gltf/terrain.rs`
  - Pre-commit: `cargo build`

- [x] 4. Replace `unwrap()` in `ffi/mod.rs` with proper error handling

  **What to do**:
  - Replace 1 `unwrap()` call in production code:
    - `src/ffi/mod.rs:167`: `f(map.get_mut(assets_dir).unwrap())` inside `with_texture_cache`
  - The `unwrap()` is logically safe because line 164 inserts the key if missing, so the `get_mut` on line 167 will always succeed. However, replace with explicit handling for consistency:
    - Use `.ok_or_else(|| crate::error::Error::Parse("texture cache map entry missing after insert".into()))?`
  - Add `eprintln!` or `log::warn!` before returning the error

  **Must NOT do**:
  - Don't change any FFI function signatures or `#[unsafe(no_mangle)]` functions
  - Don't change the `ByteBuffer` or `run_on_large_stack` logic
  - Don't modify unsafe blocks

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 2, 3)
  - **Blocks**: None
  - **Blocked By**: None

  **References**:

  **Pattern References**:
  - `src/ffi/mod.rs:150-168` — `with_texture_cache` function containing the unwrap
  - `src/ffi/mod.rs:155` — Shows existing error pattern in same function: `.map_err(|e| crate::error::Error::Parse(format!(...)))?`
  - `src/error.rs` — Error::Parse variant for the error message

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: No unwrap() in production ffi/mod.rs
    Tool: Bash
    Steps:
      1. Run `cargo build 2>&1` — assert exit code 0
      2. Run `grep -n '.unwrap()' src/ffi/mod.rs` — assert zero matches
    Expected Result: Build succeeds; zero unwrap() in ffi/mod.rs
    Evidence: .sisyphus/evidence/task-4-ffi-unwrap-audit.txt
  ```

  **Commit**: YES
  - Message: `refactor(ffi): replace unwrap() with proper error handling`
  - Files: `src/ffi/mod.rs`
  - Pre-commit: `cargo build`

- [x] 5. Deduplicate GLTF builder accessor functions into generic helper

  **What to do**:
  - `src/gltf/builder.rs` has 3 nearly identical accessor-building functions:
    - `push_vec3_accessor` (lines 96-153): serializes `&[[f32; 3]]`, creates View + Accessor with Type::Vec3, supports min/max
    - `push_vec2_accessor` (lines 155-194): serializes `&[[f32; 2]]`, creates View + Accessor with Type::Vec2, no min/max
    - `push_index_accessor` (lines 196-234): serializes `&[u32]`, creates View + Accessor with Type::Scalar + ComponentType::U32, Target::ElementArrayBuffer
  - All three follow the same pattern: align buffer → serialize to le bytes → push View → push Accessor → return index
  - Create a private generic helper method that handles the common View + Accessor creation:
    ```rust
    fn push_accessor_raw(
        &mut self,
        bytes: &[u8],
        count: usize,
        component_type: ComponentType,
        type_: Type,
        target: Target,
        min: Option<Value>,
        max: Option<Value>,
    ) -> usize
    ```
  - Rewrite the three functions to serialize their specific data types to bytes, then delegate to `push_accessor_raw`
  - The serialization differs per type (f32×3, f32×2, u32), so keep thin wrappers that handle the byte conversion but share the View/Accessor creation

  **Must NOT do**:
  - Don't change the public interface of GltfBuilder (all three methods stay with same signatures)
  - Don't change the GLB output format
  - Don't use `bytemuck` or add dependencies — keep the explicit serialization
  - Don't touch `push_blob_buffer_view` (line 236) — it's different (no accessor, no target)

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Refactoring a central builder with careful type handling; needs thorough understanding
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 6, 7)
  - **Blocks**: None
  - **Blocked By**: Task 1 (modifies gltf module's constant visibility)

  **References**:

  **Pattern References**:
  - `src/gltf/builder.rs:96-153` — `push_vec3_accessor` (longest variant, has min/max)
  - `src/gltf/builder.rs:155-194` — `push_vec2_accessor` (simpler, no min/max)
  - `src/gltf/builder.rs:196-234` — `push_index_accessor` (different ComponentType and Target)
  - `src/gltf/builder.rs:91-94` — `align_buffer` helper (used by all three)
  - `src/gltf/builder.rs:236-250` — `push_blob_buffer_view` (do NOT touch — different purpose)

  **API/Type References**:
  - `gltf_json::accessor::{Accessor, ComponentType, GenericComponentType, Type}` — accessor types
  - `gltf_json::buffer::{Target, View}` — buffer view types
  - `json::validation::{Checked, USize64}` — wrapper types used in View/Accessor construction

  **WHY Each Reference Matters**:
  - Lines 96-234 are the three functions to refactor — study them to identify the shared pattern vs. type-specific differences
  - The key differences are: data type (f32/u32), component count (3/2/1), ComponentType (F32/U32), Type (Vec3/Vec2/Scalar), Target (ArrayBuffer/ElementArrayBuffer), and min/max support

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: Builder compiles and accessor count reduced
    Tool: Bash
    Steps:
      1. Run `cargo build 2>&1` — assert exit code 0
      2. Run `cargo clippy --all-targets --all-features -- -D warnings 2>&1` — assert exit code 0
      3. Count accessor-related methods: `grep -c 'fn push_.*_accessor\|fn push_accessor_raw' src/gltf/builder.rs`
      4. Assert the generic helper exists and the three original functions are simplified wrappers
    Expected Result: Build + clippy pass; builder has a generic core helper
    Evidence: .sisyphus/evidence/task-5-builder-dedup.txt

  Scenario: GLB output unchanged (behavioral equivalence)
    Tool: Bash
    Steps:
      1. Run `cargo test 2>&1` — assert all tests pass
    Expected Result: All integration tests pass (they test GLB conversion end-to-end)
    Evidence: .sisyphus/evidence/task-5-test-pass.txt
  ```

  **Commit**: YES
  - Message: `refactor(gltf): deduplicate accessor builder into generic helper`
  - Files: `src/gltf/builder.rs`
  - Pre-commit: `cargo build`

- [x] 6. Extract shared mesh-caching logic from `gltf/mod.rs`

  **What to do**:
  - `src/gltf/mod.rs` has two functions with nearly identical mesh-caching logic (~60 lines each):
    - `convert_positioned_models_to_gltf` (lines 123-183): builds unique_source_models HashMap, parallel unrolled_cache, mesh_instance_cache loop
    - `convert_wld_scene_to_gltf` (lines 244-304): identical mesh-caching logic
  - Extract the shared logic into a private helper function, e.g.:
    ```rust
    fn add_positioned_models_to_builder(
        builder: &mut GltfBuilder,
        positioned_models: &[PositionedModel],
        palette: Option<&Palette>,
        texture_cache_available: bool,
    )
    ```
  - This helper handles: unique source model collection, parallel unrolled primitive building, mesh instance caching, node creation with transforms
  - Both `convert_positioned_models_to_gltf` and `convert_wld_scene_to_gltf` then call this shared helper
  - The WLD function additionally handles terrain mesh before calling the shared positioned model logic

  **Must NOT do**:
  - Don't change any public function signatures (`convert_models_to_gltf`, `convert_positioned_models_to_gltf`, `convert_wld_scene_to_gltf` keep their signatures)
  - Don't change the light handling logic in `convert_positioned_models_to_gltf` (lines 185-208)
  - Don't change the terrain mesh handling in `convert_wld_scene_to_gltf` (lines 231-242)

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Central conversion logic with complex ownership patterns (mutable builder + shared references); needs careful Rust borrow checker handling
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 5, 7)
  - **Blocks**: None
  - **Blocked By**: Task 1 (modifies gltf module's constant visibility)

  **References**:

  **Pattern References**:
  - `src/gltf/mod.rs:123-183` — First copy of mesh-caching logic (in `convert_positioned_models_to_gltf`)
  - `src/gltf/mod.rs:244-304` — Second copy (in `convert_wld_scene_to_gltf`) — nearly line-for-line identical
  - `src/gltf/mod.rs:61-75` — Simpler variant in `convert_models_to_gltf` (no caching, just builds meshes) — do NOT merge with this, it's different enough

  **API/Type References**:
  - `crate::import::rgm::PositionedModel` — has `source_id: Option<String>`, `model: Model3DFile`, `model_name: String`, `transform: [f32; 16]`
  - `src/gltf/primitives.rs` — `build_unrolled_primitives` used by both duplicate blocks
  - `src/gltf/builder.rs` — `GltfBuilder::append_mesh` and `GltfBuilder::add_node` called in the caching loop

  **WHY Each Reference Matters**:
  - Compare lines 123-183 with 244-304 side-by-side to see the identical pattern
  - The PositionedModel struct fields determine the helper function's parameter signature
  - The builder's mutable methods constrain how ownership flows through the extraction

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: Mesh caching logic deduplicated
    Tool: Bash
    Steps:
      1. Run `cargo build 2>&1` — assert exit code 0
      2. Verify the shared helper exists: `grep -n 'fn add_positioned_models_to_builder\|fn build_positioned_model_nodes' src/gltf/mod.rs`
      3. Verify unique_source_models/mesh_instance_cache logic appears only once (in the helper)
    Expected Result: Build succeeds; mesh-caching logic exists in exactly one place
    Evidence: .sisyphus/evidence/task-6-mesh-cache-dedup.txt

  Scenario: Conversion output unchanged
    Tool: Bash
    Steps:
      1. Run `cargo test 2>&1` — assert all tests pass
    Expected Result: All integration tests pass
    Evidence: .sisyphus/evidence/task-6-test-pass.txt
  ```

  **Commit**: YES
  - Message: `refactor(gltf): extract shared mesh-caching logic`
  - Files: `src/gltf/mod.rs`
  - Pre-commit: `cargo build`

- [x] 7. Consolidate CLI WLD converter to reuse shared palette/asset helpers

  **What to do**:
  - `src/cli/convert/wld.rs` duplicates two patterns from `src/cli/convert/mod.rs`:
    1. **Asset root resolution** (wld.rs lines 36-41 vs mod.rs lines 27-33):
       ```rust
       // wld.rs (duplicate)
       let asset_root = args.assets.clone()
           .or_else(|| args.asset_path.clone())
           .or_else(|| args.asset_dir.clone())
           .unwrap_or_else(|| resolve_asset_root_from_input(&args.file));
       ```
       This is identical to `resolve_asset_root` in mod.rs. Use the existing function.
    2. **Palette loading** (wld.rs lines 43-49 vs mod.rs lines 77-89):
       ```rust
       // wld.rs (duplicate)
       let palette = match args.palette.as_ref() {
           Some(path) => { ... Palette::parse ... }
           None => auto_resolve_palette(&asset_root, &args.file, FileType::Wld)?,
       };
       ```
       This is identical to `load_palette` in mod.rs. Use the existing function.
  - Make `resolve_asset_root` and `load_palette` in `mod.rs` `pub(crate)` (or `pub(super)`) so `wld.rs` can use them
  - Replace the duplicated logic in `wld.rs` with calls to those shared functions

  **Must NOT do**:
  - Don't change the WLD-specific logic (TEXBSI ID handling, terrain toggle, RGM companion lookup)
  - Don't change `texbsi.rs`'s palette handling — it intentionally doesn't auto-resolve (requires explicit `--palette` flag)
  - Don't change function behavior, only call site

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 5, 6)
  - **Blocks**: None
  - **Blocked By**: None

  **References**:

  **Pattern References**:
  - `src/cli/convert/mod.rs:27-33` — `resolve_asset_root` function (source of truth)
  - `src/cli/convert/mod.rs:77-89` — `load_palette` function (source of truth)
  - `src/cli/convert/wld.rs:36-41` — Duplicated asset root resolution
  - `src/cli/convert/wld.rs:43-49` — Duplicated palette loading

  **WHY Each Reference Matters**:
  - Compare mod.rs functions with wld.rs duplicates to confirm they're identical
  - mod.rs functions may need visibility change from private to `pub(super)`

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: WLD converter uses shared helpers
    Tool: Bash
    Steps:
      1. Run `cargo build 2>&1` — assert exit code 0
      2. Verify wld.rs no longer has inline asset_root resolution: `grep -c 'or_else.*asset_path\|or_else.*asset_dir' src/cli/convert/wld.rs` — assert 0
      3. Verify wld.rs no longer has inline palette loading: `grep -c 'Palette::parse' src/cli/convert/wld.rs` — assert 0
    Expected Result: Build succeeds; wld.rs delegates to shared functions
    Evidence: .sisyphus/evidence/task-7-cli-consolidation.txt
  ```

  **Commit**: YES
  - Message: `refactor(cli): reuse shared palette loading in WLD converter`
  - Files: `src/cli/convert/wld.rs`, `src/cli/convert/mod.rs`
  - Pre-commit: `cargo build`

- [x] 8. Break up `parse_face_data` into focused helpers

  **What to do**:
  - `src/import/model3d/parser.rs` function `parse_face_data` (lines 91-173, 82 lines) handles multiple concerns in one function:
    1. Parse vertex count (line 97)
    2. Parse texture data with version branching (lines 100-116)
    3. Parse unused field (line 118)
    4. Call texture data decoder (lines 120-121)
    5. Per-vertex loop with accumulated UV deltas (lines 123-162)
    6. Construct result (lines 164-172)
  - Extract the per-vertex parsing loop (lines 123-162) into a separate function:
    ```rust
    fn parse_face_vertices<'a>(
        input: &'a [u8],
        vertex_count: u8,
        version: &ModelVersion,
    ) -> IResult<&'a [u8], Vec<FaceVertex>>
    ```
  - Extract the version-dependent texture header parsing (lines 100-116) into:
    ```rust
    fn parse_face_texture_header<'a>(
        input: &'a [u8],
        version: &ModelVersion,
    ) -> IResult<&'a [u8], (u32, u8)>
    ```
  - This reduces `parse_face_data` to ~20 lines: call texture header parser → call texture decoder → call vertex parser → construct result

  **Must NOT do**:
  - Don't change the parsing logic or field ordering
  - Don't change `FaceData` or `FaceVertex` struct definitions
  - Don't change test code in the `#[cfg(test)]` module at the bottom of the file
  - Don't touch `parse_3d_file` (that's Task 9)

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 3, sequential with Task 9
  - **Blocks**: Task 9 (same file)
  - **Blocked By**: None

  **References**:

  **Pattern References**:
  - `src/import/model3d/parser.rs:91-173` — Current `parse_face_data` function (full listing)
  - `src/import/model3d/parser.rs:100-116` — Version-branching texture header parsing (extract target)
  - `src/import/model3d/parser.rs:123-162` — Per-vertex loop with UV accumulation (extract target)

  **API/Type References**:
  - `FaceData` struct — return type (has `vertex_count`, `tex_hi`, `texture_data`, `face_vertices`)
  - `FaceVertex` struct — vertex struct (has `vertex_index`, `u`, `v`)
  - `ModelVersion` enum — version variants: V26, V27, V40, V50, Unknown
  - `parse_texture_data_value` (line 177) — called between the two extracted sections

  **Test References**:
  - `src/import/model3d/parser.rs:577-646` — Existing tests that call `parse_face_data` directly — must still pass unchanged

  **WHY Each Reference Matters**:
  - Lines 100-116 and 123-162 are the two extraction targets — study their inputs/outputs to design helper signatures
  - The test module calls `parse_face_data` directly, so the public signature must not change

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: parse_face_data is shorter and helpers exist
    Tool: Bash
    Steps:
      1. Run `cargo build 2>&1` — assert exit code 0
      2. Verify helper functions exist: `grep -n 'fn parse_face_vertices\|fn parse_face_texture_header' src/import/model3d/parser.rs`
      3. Verify parse_face_data is significantly shorter (rough line count)
    Expected Result: Build succeeds; two new helper functions exist; parse_face_data is ~20-30 lines
    Evidence: .sisyphus/evidence/task-8-face-data-split.txt

  Scenario: Existing face data tests pass
    Tool: Bash
    Steps:
      1. Run `cargo test parse_face 2>&1` — assert pass
    Expected Result: All parse_face_data tests pass unchanged
    Evidence: .sisyphus/evidence/task-8-test-pass.txt
  ```

  **Commit**: YES
  - Message: `refactor(3d): break up parse_face_data into focused helpers`
  - Files: `src/import/model3d/parser.rs`
  - Pre-commit: `cargo build`

- [x] 9. Break up `parse_3d_file` into focused helpers

  **What to do**:
  - `src/import/model3d/parser.rs` function `parse_3d_file` (lines 514-575, 61 lines) orchestrates the full model parse:
    1. Logging first bytes (lines 515-518)
    2. Parse header + version (lines 519-520)
    3. Compute adjusted offsets based on version (lines 527-531)
    4. Parse 7 different sections (lines 533-540)
    5. Convert normal indices (lines 544-558)
    6. Construct result (lines 562-574)
  - Extract the normal index conversion (lines 544-558) into:
    ```rust
    fn convert_normal_indices(raw_indices: Vec<u32>, adjusted_offset_normals: u32) -> Vec<u32>
    ```
  - Extract the offset computation (lines 527-531) into:
    ```rust
    fn adjusted_offsets(header: &Model3DHeader) -> (u32, u32)
    ```
  - This reduces `parse_3d_file` to ~35 lines: header → offsets → parse sections → convert normals → construct

  **Must NOT do**:
  - Don't change the parsing logic or section ordering
  - Don't change `Model3DFile` struct definition
  - Don't change test code
  - Don't touch `parse_face_data` (that was Task 8)

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 3, after Task 8
  - **Blocks**: None
  - **Blocked By**: Task 8 (same file)

  **References**:

  **Pattern References**:
  - `src/import/model3d/parser.rs:514-575` — Current `parse_3d_file` function (full listing)
  - `src/import/model3d/parser.rs:527-531` — Offset computation logic (extract target)
  - `src/import/model3d/parser.rs:544-558` — Normal index conversion (extract target)

  **API/Type References**:
  - `Model3DFile` struct — return type
  - `Model3DHeader` — header struct with `is_v27_or_earlier()`, `total_face_vertices`, `offset_vertex_normals`, `offset_vertex_coords`
  - Various `parse_*_section` functions called at lines 533-540

  **WHY Each Reference Matters**:
  - Lines 527-531 and 544-558 are the two extraction targets
  - Model3DHeader fields determine the offset computation helper's interface

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: parse_3d_file is shorter and helpers exist
    Tool: Bash
    Steps:
      1. Run `cargo build 2>&1` — assert exit code 0
      2. Verify helper functions: `grep -n 'fn convert_normal_indices\|fn adjusted_offsets' src/import/model3d/parser.rs`
    Expected Result: Build succeeds; extraction helpers exist
    Evidence: .sisyphus/evidence/task-9-3d-file-split.txt

  Scenario: Full test suite passes
    Tool: Bash
    Steps:
      1. Run `cargo test 2>&1` — assert all pass
    Expected Result: All tests pass (integration tests exercise full 3D file parsing)
    Evidence: .sisyphus/evidence/task-9-test-pass.txt
  ```

  **Commit**: YES
  - Message: `refactor(3d): break up parse_3d_file into focused helpers`
  - Files: `src/import/model3d/parser.rs`
  - Pre-commit: `cargo build`

---

## Final Verification Wave (MANDATORY — after ALL implementation tasks)

> 4 review agents run in PARALLEL. ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.

- [ ] F1. **Plan Compliance Audit** — `oracle`
  Read the plan end-to-end. For each "Must Have": verify implementation exists (read file, run command). For each "Must NOT Have": search codebase for forbidden patterns — reject with file:line if found. Check evidence files exist in .sisyphus/evidence/. Compare deliverables against plan.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`

- [ ] F2. **Code Quality Review** — `unspecified-high`
  Run `cargo clippy --all-targets --all-features -- -D warnings` + `cargo test`. Review all changed files for: `as any` equivalents, empty catches, commented-out code, unused imports. Check AI slop: excessive comments, over-abstraction, generic names.
  Output: `Build [PASS/FAIL] | Clippy [PASS/FAIL] | Tests [N pass/N fail] | Files [N clean/N issues] | VERDICT`

- [ ] F3. **Real Manual QA** — `unspecified-high`
  Start from clean state. Run `cargo build` + `cargo test` + `cargo clippy --all-targets --all-features -- -D warnings`. Verify zero production `unwrap()`/`expect()` by grepping non-test code. Verify `ENGINE_UNIT_SCALE` has exactly one definition. Save to `.sisyphus/evidence/final-qa/`.
  Output: `Build [PASS/FAIL] | Tests [N/N pass] | Clippy [PASS/FAIL] | Unwrap audit [PASS/FAIL] | VERDICT`

- [ ] F4. **Scope Fidelity Check** — `deep`
  For each task: read "What to do", read actual diff (git log/diff). Verify 1:1 — everything in spec was built (no missing), nothing beyond spec was built (no creep). Check "Must NOT do" compliance. Detect cross-task contamination. Flag unaccounted changes.
  Output: `Tasks [N/N compliant] | Contamination [CLEAN/N issues] | Unaccounted [CLEAN/N files] | VERDICT`

---

## Commit Strategy

| Task | Commit Message | Files |
|------|---------------|-------|
| 1 | `refactor: consolidate ENGINE_UNIT_SCALE to single definition` | `src/gltf/mod.rs`, `src/ffi/scene.rs`, `src/lib.rs` (if needed) |
| 2 | `refactor(rgm): replace expect() with proper error propagation` | `src/import/rgm/parser.rs` |
| 3 | `refactor(gltf): replace expect() with error propagation in terrain` | `src/gltf/terrain.rs` |
| 4 | `refactor(ffi): replace unwrap() with proper error handling` | `src/ffi/mod.rs` |
| 5 | `refactor(gltf): deduplicate accessor builder into generic helper` | `src/gltf/builder.rs` |
| 6 | `refactor(gltf): extract shared mesh-caching logic` | `src/gltf/mod.rs` |
| 7 | `refactor(cli): reuse shared palette loading in WLD converter` | `src/cli/convert/wld.rs` |
| 8 | `refactor(3d): break up parse_face_data into focused helpers` | `src/import/model3d/parser.rs` |
| 9 | `refactor(3d): break up parse_3d_file into focused helpers` | `src/import/model3d/parser.rs` |

---

## Success Criteria

### Verification Commands
```bash
cargo build                                                    # Expected: compiles cleanly
cargo clippy --all-targets --all-features -- -D warnings       # Expected: zero warnings
cargo test                                                     # Expected: all tests pass
```

### Final Checklist
- [ ] All "Must Have" present
- [ ] All "Must NOT Have" absent
- [ ] All tests pass
- [ ] Zero production unwrap/expect (outside #[cfg(test)])
- [ ] ENGINE_UNIT_SCALE defined exactly once
- [ ] 9 atomic commits (one per task)
