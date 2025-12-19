# WASM Crate Execution Plan (Ergogen‑RS)

**Created:** 2025-12-19  
**Owner:** Ergogen‑RS  
**Status:** Draft  
**Goal:** Provide a `ergogen-wasm` crate that exposes a stable browser API, including JS‑footprint execution via the host runtime.

---

## Scope

### In Scope

- A new `crates/ergogen-wasm` crate using `wasm-bindgen`.
- JS footprint bridge (host JS executes `*.js` footprints).
- Expose minimal, stable API to run Ergogen in a browser:
  - parse config
  - generate points/outlines/pcb
  - return outputs as strings (SVG, DXF, KiCad PCB, JSCAD as available).
- Build/test harness for wasm in CI (headless or node-based).

### Out of Scope (for initial version)

- UI, editors, or file pickers.
- Full CLI parity in WASM.
- Offline caching of footprints or worker threading (may follow later).

---

## Architecture Overview

### 1) Rust crate: `ergogen-wasm`

**Dependencies**

- `wasm-bindgen`
- `serde-wasm-bindgen`
- core crates: `ergogen-parser`, `ergogen-layout`, `ergogen-outline`, `ergogen-export`, `ergogen-pcb`
- enable `ergogen-pcb` feature `js-footprints-wasm` (disable `js-footprints`)

**Core JS bridge functions (already expected by Rust)**

```js
// host must provide these (global or module export wired into wasm)
function ergogenJsFootprintParams(source) -> object
function ergogenRenderJsFootprint(source, p) -> string
```

**Rust entrypoints (wasm-bindgen)**

- `render_all(config_yaml: &str) -> JsValue`
  - returns `{ pcbs: { name: string }, outlines: { name: string }, cases: {...}, errors: [...] }`
- `render_pcb(config_yaml: &str, pcb_name: &str) -> String`
- `render_outlines(config_yaml: &str, outline_name: &str) -> String`
- `render_svg(config_yaml: &str, outline_name: &str) -> String`
- `render_dxf(config_yaml: &str, outline_name: &str) -> String`

> These mirror existing CLI usage but keep the wasm surface simple and stable.

### 2) Host JS layer

Provide a small JS helper that the wasm module calls into. This should live alongside the wasm bundle (or be re‑exported by the bundler).

**Host functions**

```js
export function ergogenJsFootprintParams(source) { ... }
export function ergogenRenderJsFootprint(source, p) { ... }
```

**Implementation details**

- Use a CommonJS shim:
  ```js
  const module = { exports: {} };
  const exports = module.exports;
  // eval source
  ```
- `ergogenJsFootprintParams` returns `module.exports.params || {}`.
- `ergogenRenderJsFootprint` executes `module.exports.body(p)` and returns the string.
- Throw readable errors with context (path/footprint name if available).

**CSP / `eval` considerations**

- If `eval` is blocked, require host opt‑in via `unsafe-eval` or provide a precompiled footprint mechanism later.
- Document this constraint clearly.

---

## Phased Implementation Plan

### Phase 1 — Crate Skeleton

1. Create `crates/ergogen-wasm` with:
   - `Cargo.toml`
   - `src/lib.rs`
   - minimal `wasm-bindgen` exports
2. Add workspace entry in root `Cargo.toml`.
3. Add `README` in the crate with usage examples.
4. Add a CI job that builds `wasm32-unknown-unknown` (no tests yet).

### Phase 2 — JS Footprint Bridge

1. Wire `ergogen-pcb` to `js-footprints-wasm` in wasm builds.
2. Implement the host JS functions.
3. Add a wasm test (node or headless) that:
   - runs `fixtures/m7/js_footprints/simple.yaml`
   - compares output to the existing golden
4. Document CSP requirements and how to provide the host JS layer.

### Phase 3 — Render APIs

1. Expose `render_pcb`, `render_svg`, `render_dxf`, `render_outlines`.
2. Normalize error handling (convert Rust errors into JS Exceptions with `{ kind, message, path? }`).
3. Add tests:
   - points/outlines basic fixture
   - pcb fixture with a JS footprint (bridge exercised)

### Phase 4 — Packaging

1. Add an npm‑friendly build step (wasm-pack or bundler).
2. Provide a minimal example app that:
   - loads wasm
   - registers host JS functions
   - calls `render_pcb` and prints output

---

## API Design Notes

### Error Propagation

Standardize on:

```ts
type ErgogenError = { kind: string; message: string; path?: string };
```

### Outputs

Return plain strings (SVG/DXF/KiCad/JSCAD) to avoid losing fidelity.

---

## Testing Strategy

- **Unit**: JS host layer tests for `ergogenJsFootprintParams` / `ergogenRenderJsFootprint`.
- **Integration**: wasm test that:
  1. loads the wasm module
  2. registers host JS functions
  3. executes `render_pcb` on `fixtures/m7/js_footprints/simple.yaml`
  4. compares output to the golden file

---

## CI Plan

- Add a wasm build job (cache + `wasm-pack test --node` or `wasm-bindgen-test`).
- Run wasm parity tests on PRs + nightly.
- Keep upstream JS parity job separate (already added).

---

## Deliverables Checklist

- [ ] `crates/ergogen-wasm` exists and builds
- [ ] JS host functions implemented
- [ ] wasm bridge for JS footprints works against fixture
- [ ] wasm integration test in CI
- [ ] minimal example usage documented

---

## Open Questions

1. Should wasm outputs include all outputs by default, or be opt‑in per export?
2. Where should host JS live (crate‑local `js/`, separate package, or user‑provided)?
3. Do we want an eval‑free mode (precompiled footprints) in Phase 2 or later?
