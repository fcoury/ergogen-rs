# Detailed Rust Migration Execution Plan

**Last Updated:** 2025-12-19  
**Status:** In Progress (**M0–M2 complete**, **M3 mostly complete**, **M5 complete**, **M6 groundwork implemented + fixture expansion**, **M7 underway: spec parser/renderer + initial footprint conversion**)  
**Target:** Ergogen v4.x behavior -> Rust implementation (CLI + WASM)

This document is the “what to do next, in what order” execution plan. The higher-level rationale and architectural decisions live in `plans/migrate-ergogen-to-rust.md`.

---

## Current State (This Repo)

- This repo is already a Cargo **workspace** (`Cargo.toml`) with these crates:
  - `crates/ergogen-cli` (bin: `ergogen`) — stub entrypoint for now
  - `crates/ergogen-core` — `Point` transforms + tests (ported from upstream)
  - `crates/ergogen-parser` — YAML parse + units + expr eval + **M2 prepare/preprocess**
  - `crates/ergogen-layout` — **M3 points engine + anchors** + fixture harness
  - `crates/ergogen-geometry`, `crates/ergogen-outline`, `crates/ergogen-export`, `crates/ergogen-pcb` — placeholders for M4+
- Fixtures are pinned and tracked in `fixtures/README.md` (upstream commit recorded).
- Full upstream integration fixtures are imported (`fixtures/upstream/test`); the harness compares **61** reference artifacts with **0 skips** across points/outlines/cases/pcbs/footprints.
- Upstream CLI fixtures are now exercised via `crates/ergogen-cli/tests/cli_suite.rs` using imported inputs from `fixtures/upstream/fixtures/` (logs/errors + 73 reference artifacts including SVG).
- SVG export support (points/outlines) is implemented and validated via the CLI fixture suite.
- SVG export also has focused unit tests in `crates/ergogen-export/tests/svg.rs` (single/multi‑subpath and round‑trip semantics).
- M4 property tests for boolean/offset invariants exist in `ergogen-geometry` + `ergogen-outline` (proptest), including self-intersection, winding, and curved-segment cases.
- Footprint-driven PCB outputs now include injected, MX/Choc asymmetry, TRRS/rotary/promicro/pad/button/chocmini, OLED/RGB (mirrored + non‑mirrored), scrollwheel (reverse + non‑reverse), slider (front/back), and alps/jumper/omron/via fixtures in `fixtures/m6/pcbs` with a dedicated test.
- Default loop passes:
  - `cargo fmt --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test --workspace`

## Immediate Next Steps (From Here)

- Finish M3 follow-ups:
  - Expand fixture assertions beyond `x/y/r` when valuable (meta, tags, flags).
- Continue M4/M5:
  - Build out `ergogen-geometry` using a bulge-aware polyline kernel (CavalierContours) for boolean + offset.
- Continue M6:
  - Expand export coverage beyond the upstream fixtures (SVG + CLI outputs now covered; injected footprint PCB fixture added; remaining: more footprint variants + any missing CLI behaviors).
  - Add regression tests for SVG edge cases if new path primitives are introduced (arcs, curves, non-line entities).

---

## Target Architecture (Workspace)

The migration uses a modular workspace approach to decouple CLI, WASM, and core logic, and to keep geometry/layout work independently testable.

```
ergogen-rs/
├── Cargo.toml                  # workspace root
├── crates/
│   ├── ergogen-core            # common types (Point, Anchor, Units, filters)
│   ├── ergogen-parser          # YAML parsing, preprocessing, expr evaluation
│   ├── ergogen-layout          # points generation (zones/columns/rows)
│   ├── ergogen-geometry        # 2D primitives + boolean/offset ops
│   ├── ergogen-outline         # outline generation driven by geometry
│   ├── ergogen-export          # SVG/DXF/JSCAD writers (and helpers)
│   ├── ergogen-pcb             # footprints + KiCad PCB generation
│   └── ergogen-cli             # binary entry point (clap)
├── ergogen-wasm/               # wasm-bindgen bindings (planned; add when M3+ is stable)
└── fixtures/                   # copied/ported upstream fixtures + golden outputs
```

---

## Execution Model

### Milestones (Gated)

Each milestone is “mergeable” on its own: tests passing, clear deliverables, and a measurable equivalence target.

- **M0 — Workspace + Quality Gates**
  - Deliverables:
    - [x] Convert to a Cargo workspace (root `Cargo.toml` with `members`)
    - [x] Create empty crate shells with `lib.rs` and minimal public APIs
    - [x] Set up `cargo fmt`, `cargo clippy`, `cargo test` as the default loop
    - [x] Add a `fixtures/` directory with a README explaining provenance
  - Exit criteria:
    - [x] `cargo fmt --check` passes
    - [x] `cargo clippy --all-targets --all-features -- -D warnings` passes
    - [x] `cargo test --workspace` passes

- **M1 — Parse + Units + Core Types**
  - Goal: parse representative configs and evaluate unit math deterministically.
  - Deliverables:
    - [x] `ergogen-core`: `Point` (+ transforms) and minimal metadata
    - [x] `ergogen-parser`: YAML parse into a deterministic IR used by later stages
    - [x] Units system compatible with Ergogen (`u`, `U`, `cx`, `cy`, plus `$default_*`)
    - [x] Expression evaluation for string math with a controlled environment
  - Exit criteria:
    - [x] Unit expressions match expected values in fixture-driven tests (`fixtures/m1/`, upstream `*___units.json`)
    - [x] A minimal config parses and produces a predictable intermediate representation (IR)

- **M2 — Prepare/Preprocess (Deterministic IR)**
  - Goal: replicate `prepare.js` behavior so downstream algorithms run against a stable, fully-expanded IR.
  - Deliverables:
    - [x] Deterministic ordered IR (`IndexMap`-backed) for canonical JSON snapshots
    - [x] `unnest` (dotted keys → nested maps)
    - [x] Inheritance (`$extends`) with cycle detection + `$unset`
    - [x] Parameterization (`$params` / `$args` / `$skip`) compatible with upstream fixture behavior
    - [x] Canonical JSON snapshot output (`.canonical.json`) for fixture comparison
  - Exit criteria:
    - [x] Snapshot tests for IR (stable canonical JSON)
    - [x] At least 5 upstream configs produce identical IR snapshots run-to-run

  #### M2 end-to-end (how to extend/verify)

  **What M2 produces:** a “canonical” prepared config IR that downstream engines consume. This is the functional equivalent of upstream’s `prepare.js` output for the covered inputs.

  **Key implementation files:**
  - `crates/ergogen-parser/src/value.rs` (deterministic ordered `Value`)
  - `crates/ergogen-parser/src/prepare.rs` (preprocess pipeline: unnest/inherit/parameterize)
  - `crates/ergogen-parser/src/eval.rs` (expr evaluation + implicit multiplication normalization)
  - `crates/ergogen-parser/tests/m2_canonical.rs` (golden snapshot tests)
  - `crates/ergogen-parser/examples/prepare.rs` (CLI helper to print canonical JSON)

  **Pipeline order (matches how downstream stages expect inputs):**
  1. `unnest`: converts dotted keys like `zones.main.columns.a` into nested maps.
  2. `inherit`: applies `$extends` chains (string or array), with cycle detection; deep-merges objects; arrays merge by index; `$unset` deletes keys.
  3. `parameterize`: applies `$params` + `$args` replacements; honors `$skip: true` and removes `$params/$args/$skip` from the output object.

  **How to add a new canonical snapshot fixture:**
  1. Copy the upstream YAML into `fixtures/m2/upstream/<name>.yaml` (record the upstream commit in `fixtures/README.md` if it changed).
  2. Generate the canonical JSON:
     - `cargo run -p ergogen-parser --example prepare -- fixtures/m2/upstream/<name>.yaml > fixtures/m2/upstream/<name>.canonical.json`
  3. Register the new `<name>` in `crates/ergogen-parser/tests/m2_canonical.rs`.
  4. Verify:
     - `cargo test -p ergogen-parser --test m2_canonical`

  **Ordering rule (why snapshots are stable):**
  - Canonical snapshots preserve mapping insertion order from parsing (they are not sorted lexicographically), so diffs are intentional and deterministic.

- **M3 — Points/Layout Engine**
  - Goal: port `points.js` + `anchor.js` behavior so point locations match upstream.
  - Deliverables:
    - [x] Anchor resolution (named refs, aggregates, shifts/rotations/orient/rotate, affect/resist)
    - [x] Zones/columns/rows iteration with cumulative transforms
    - [x] `stagger`, `spread`, `splay`, mirroring, `autobind` (covered by upstream `test/points` goldens)
    - [x] Golden test harness for points fixtures (YAML + `*___points.json`/`*__points.json` + `*___units.json`)
    - [~] Expand fixture assertions beyond `x/y/r` when valuable (meta flags/identifiers now covered)
    - [x] Decide equivalence strategy for the upstream `.dxf` reference artifacts (treat as M6 export fixtures; semantic compare)
  - Exit criteria:
    - [x] Point fixtures with golden points JSON match within epsilon (x/y/r)
    - [ ] Remaining gaps/intentional non-equivalences are documented (if any)

  #### M3 end-to-end (how to extend/verify)

  **What M3 produces:** a point map keyed by point name, where each point has `(x, y, r)` and upstream-compatible anchor behavior. This is the minimum substrate needed for outlines/footprints to reference points deterministically.

  **Key implementation files:**
  - `crates/ergogen-layout/src/points.rs` (zones/columns/rows → points)
  - `crates/ergogen-layout/src/anchor.rs` (anchor resolution + aggregates)
  - `crates/ergogen-layout/tests/upstream_points.rs` (fixture harness vs upstream `test/points`)
  - `crates/ergogen-layout/tests/anchor_unit.rs` (focused anchor behavior tests)
  - `crates/ergogen-layout/examples/points.rs` (debug helper for a single YAML)

  **How to add/validate a points fixture:**
  1. Copy upstream inputs into `fixtures/m3/points/<name>.yaml`.
  2. If upstream provides point goldens, keep them alongside the YAML:
     - `fixtures/m3/points/<name>___points.json` (usual)
     - `fixtures/m3/points/<name>__points.json` (upstream oddity; the harness accepts both)
  3. If upstream expects an error, add `fixtures/m3/points/<name>___EXCEPTION.txt` with a substring that must appear in our error message.
  4. Verify:
     - `cargo test -p ergogen-layout --test upstream_points`

  **Debug one fixture locally:**
  - `cargo run -p ergogen-layout --example points -- fixtures/m3/points/<name>.yaml`

  **DXF artifacts in `fixtures/m3/points`:**
  - Files like `*___demo_dxf.dxf` and `*___outlines_*_dxf.dxf` are imported as references and now validated by the upstream suite via **semantic** DXF comparison (normalized entities + epsilon), not byte-for-byte.

- **M4 — Geometry Kernel**
  - Goal: replace MakerJS capabilities needed by outlines with a Rust-native stack.
  - Deliverables:
    - [x] Geometry primitives and path model (bulge-aware polylines via CavalierContours)
    - [~] Boolean ops: union/diff/intersect (fixtures covered; property tests include self-intersection, winding normalization, and order-invariance checks)
    - [~] Offset/expand and fillet/rounding strategy (round + rectangle-only joints landed)
    - [x] Hull algorithms needed by Ergogen (convex + concave hull)
  - Exit criteria:
    - [~] Property tests for boolean/offset stability (bbox/offset invariants plus winding/self-intersection checks; add more robustness over tricky arcs)
    - [ ] Deterministic output normalization for comparisons

  #### M4 end-to-end (recommended approach)

  **Design checkpoint (pick one early and document it):**
  - **Option A: float geometry** end-to-end with careful epsilon handling.
    - Pros: simpler mapping from Ergogen math; no scaling issues.
    - Cons: boolean/offset robustness can be painful.
  - **Option B: scaled-integer kernel** for boolean/offset (convert `(x, y)` to int grid with a scale factor).
    - Pros: tends to be far more robust/deterministic for boolean + offset.
    - Cons: arcs become polylines (unless you carry arc metadata separately); need consistent tolerance + scaling rules.

  **Suggested execution (regardless of option):**
  1. Define the public geometry IR in `crates/ergogen-geometry`:
     - primitives: `Line`, `Arc` (or “polyline-only” if doing scaled-int)
     - `Path` / `Shape` container types with explicit winding + closed/open
  2. Add **normalization utilities first** (so later fixtures are stable):
     - snap-to-epsilon (or snap-to-grid)
     - canonical winding rules
     - deterministic ordering of entities (e.g., by bbox then first vertex)
  3. Implement boolean ops and offset/expand, then add:
     - property tests (quickcheck/proptest) for invariants
     - a small “known-bad” corpus of tricky shapes (near-collinear, tiny radii)
  4. Only then move to M5 outlines: the outline DSL should compile to this geometry IR.

- **M5 — Outlines**
  - Goal: implement `outlines.js` behavior and validate against fixtures.
  - Deliverables:
    - [x] Outline operations (`glue`, combine, expand, subtract, etc.)
      - [x] Minimal rectangle outlines for `fixtures/m5/outlines/basic.yaml`
      - [x] `circle` + `adjust` support (shape-specific vars like `r`)
      - [x] `where:` anchors/aggregates and regex selectors (`/pattern/`)
      - [x] `asym` mirroring + `affect` anchor semantics (via `ergogen-layout` anchors)
      - [x] `bound: true` bind-based rectangle expansion
      - [x] Outline references (`what: outline`, `name:`) + `expand` (rectangle-only for now)
      - [x] `fillet`, `scale`, generalized `expand` (round), `hull`, `path`
    - [x] Fixture-driven tests scaffolding (DXF parse/normalize + semantic compare harness)
      - `crates/ergogen-outline/tests/upstream_outlines.rs`
  - Exit criteria:
    - [x] Outline fixtures pass (semantic equivalence)
      - [x] `basic.yaml` matches `basic___outlines_outline_dxf.dxf`
      - [x] `rectangles.yaml`, `circles.yaml`, `polygons.yaml`, `binding.yaml`, `affect_mirror.yaml`, `expand.yaml`
      - [x] Remaining outline fixtures (`hull.yaml`, `outlines.yaml`, `path.yaml`)

  #### M5 end-to-end (fixtures reality check)

  - Upstream outline fixtures live in the upstream repo under `test/outlines/` and are primarily **DXF goldens** (`*___outlines_*_dxf.dxf`).
  - Practical implication: **M5 validation needs the M6 DXF semantic comparer** (or a parallel “DXF-to-IR” normalizer for tests).
  - Recommended workflow:
    1. Implement outlines as “compile config → geometry IR” in `crates/ergogen-outline` (no string formatting concerns).
    2. In tests, parse upstream DXF goldens into a normalized entity list and compare against our geometry IR normalized to the same representation.
    3. Only after the semantics match, add the actual DXF writer (and then test writer output via the same semantic comparer).

- **M6 — Exports (SVG/DXF/JSCAD)**
  - Goal: make outputs consumable by real tools.
  - Deliverables:
    - [~] SVG export sufficient for previewing outlines/plates (lines + arcs + polylines)
    - [x] DXF export suitable for laser cutters
      - [x] DXF parser + normalizer + semantic comparer
      - [x] DXF writer for normalized entities (+ roundtrip tests against upstream goldens)
      - [x] DXF writer from geometry IR (lines/arcs/circles from bulge polylines; full outline fixture suite)
    - [ ] JSCAD generation (or a staged alternative)
  - Exit criteria:
    - [ ] At least one “reference keyboard” opens/renders correctly in downstream tools

  **Reference output helper:**
  - `cargo run -p ergogen-export --example reference_outputs -- fixtures/upstream/fixtures/big.yaml`
    writes DXF/SVG/JSCAD/KiCad outputs into `target/reference-outputs/big/{outlines,cases,pcbs}/` for manual tool checks.
  - `cargo test -p ergogen-export --test reference_outputs`
    verifies the generated outputs against goldens in `fixtures/m6/reference_outputs/big/`.

  #### DXF fixture strategy (for M6)

  - Treat upstream DXF files as **semantic fixtures**:
    - parse both DXFs (upstream + ours) into a normalized list of entities
    - compare entity types + geometry within epsilon (ignore headers, ordering, comments)
  - If we choose a polyline-only kernel (M4 option B), standardize on:
    - a single arc discretization tolerance (documented)
    - matching discretization rules in tests so comparisons don’t drift

  **Planned Rust implementation location:**
  - `crates/ergogen-export/src/dxf.rs` (DXF parse + normalize + semantic compare)
  - `crates/ergogen-export/tests/m3_points_dxf.rs` (smoke test: parse/normalize imported DXFs)
  - `crates/ergogen-export/tests/m5_outlines_dxf.rs` (smoke test: parse/normalize imported upstream outline DXF goldens)
  - `crates/ergogen-export/tests/m5_outlines_golden_semantic.rs` (harness: semantic-compare goldens to themselves)
  - `crates/ergogen-export/tests/m5_outlines_generated_vs_golden.rs` (ignored: compare generated DXF vs upstream goldens)

- **M7 — Footprints + PCB**
  - Goal: support Ergogen’s PCB generation with a Rust-friendly footprint system.
  - Deliverables:
    - [~] Declarative footprint schema + loader + templating/interpolation (parser + resolver in place)
    - [~] Spec-backed footprint rendering (pad primitive via `what: spec`)
    - [~] Spec search paths + caching + additional primitives (line/circle/thru-hole pad/arc/rect/text) + kicad8 fp_rect
    - [ ] Conversion of built-in footprints (`mx`, `choc`, `diode`, etc.)
    - [ ] KiCad PCB writer (S-expression templates or direct emitter)
  - Exit criteria:
    - [ ] Generated `.kicad_pcb` opens in KiCad without errors (target versions specified in plan)

- **M8 — CLI + Bundles + WASM**
  - Goal: ship user-facing entry points.
  - Deliverables:
    - [ ] `ergogen-cli` parity flags (start with the subset needed for fixtures)
    - [ ] Bundle I/O and output directory handling
    - [ ] `ergogen-wasm` bindings exposing a stable API surface
  - Exit criteria:
    - [ ] End-to-end CLI test(s) cover parsing → points → outlines → export
    - [ ] WASM build produces outputs for at least one fixture (even if not fully optimized)

---

## Work Breakdown (PR-Sized)

Guideline: keep PRs small and verifiable; “one subsystem + its tests” per PR.

- Parser PRs should land with fixtures + snapshot tests immediately.
- Layout PRs should land with at least one golden fixture per feature (anchors, mirror, autobind).
- Geometry PRs should land with normalization helpers (winding, epsilon snapping) so later comparisons are stable.

---

## Equivalence Rules (So We Don’t Bike-Shed)

**Must match:**
- Point coordinates and rotations within a documented epsilon
- Shape topology / connectivity for outlines (same resulting regions)
- Net connectivity for PCBs (logical net assignments)

**May differ (allowed):**
- Floating formatting (e.g., `14` vs `14.000000`)
- Ordering of SVG/DXF entities
- Whitespace and comments

---

## Risk Register (Execution-Focused)

- **Geometry edge cases** (self-intersections, tiny radii, near-collinear segments)
  - Mitigation: normalization + property tests + “known-bad” corpus early (M4)
- **Anchor cycles / recursion** (infinite loops)
  - Mitigation: explicit cycle detection with error reporting in M3
- **Fixture drift / provenance**
  - Mitigation: pin upstream fixture versions and record the source commit in `fixtures/README.md`

---

## Default Dev Loop

- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
