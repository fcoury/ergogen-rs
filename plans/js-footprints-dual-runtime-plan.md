# JS Footprints Dual Runtime Plan (Boa + Browser Bridge)

**Created:** 2025-12-19  
**Owner:** Ergogen-RS  
**Status:** Draft  
**Goal:** Support JS footprint execution in native CLI **and** WASM/browser builds without forcing a JS engine into WASM.

---

## Problem Statement

Ergogen projects often rely on JS-based footprints (`*.js`). We need to support them:
- **Native/CLI** builds (Rust binary).
- **WASM/browser** builds (running in a host JS environment).

Embedding a JS engine in WASM is heavy and incompatible with CSP in many contexts. The plan is:
- **Native:** embed Boa as the JS engine.
- **WASM:** use a JS bridge that executes footprints in the browser’s JS engine.

---

## Architecture Overview

### 1) Common Runtime Contract

We define a single **runtime contract** for the footprint execution environment. Both implementations (Boa + browser bridge) must match.

Required JS module contract:
```js
module.exports = {
  params: { /* footprint param spec */ },
  body: (p) => string
}
```

Required `p` object shape:
- **Scalar fields**
  - `p.at` (string): `(at x y r)`
  - `p.r` / `p.rot` (number): rotation in degrees
  - `p.ref` (string)
  - `p.ref_hide` (string): `hide` or `""`
  - `p.side` (string): `"F"` or `"B"`
- **Param fields** as provided by the user (strings, numbers, booleans, arrays)
- **Net fields**: for net params, each is an object:
  - `name` (string)
  - `index` (number)
  - `str` (string): `(net <idx> "<name>")`
- **Functions**
  - `p.xy(x, y)` → `"x y"` rotated & translated
  - `p.eaxy(x, y)` → same as `xy` (Ergogen’s e-xy)
  - `p.local_net(id)` → net object (name is `ref_id`, e.g. `MCU1_24`)
  - `p.global_net(name)` → net object by name

All string formatting (number precision, sign, etc.) must match the Rust KiCad emitter.

---

## Native/CLI Runtime (Boa)

### Components
- **`js_footprints.rs`** (new): host for Boa evaluation
- **`JsFootprintModule`**
  - Load JS source
  - Inject CommonJS shim:
    ```js
    globalThis.module = { exports: {} };
    globalThis.exports = module.exports;
    ```
  - Evaluate JS
  - Extract `module.exports.params` and `module.exports.body`

### Param Binding
Parse `params` object into Rust:
- Primitive defaults: string/number/bool/array
- Net params: `{ type: "net", value: "GND" | undefined }`
- If `value` is `undefined`, treat as required

### Execution
Build `p` object in Rust with the runtime contract:
- map params → `p` fields
- add `p.at`, `p.r`, `p.ref`, `p.ref_hide`, `p.side`
- attach `p.xy`, `p.eaxy`, `p.local_net`, `p.global_net`
- call `body(p)` and return string

---

## WASM Runtime (Browser Bridge)

### JS Bridge (Host)
Provide a JS helper (bundled with WASM):
```ts
renderJsFootprint(source: string, p: object): string
```

Responsibilities:
- Provide CommonJS shim
- `eval` or `Function` compile
- Call `module.exports.body(p)`

### WASM Side
Expose a wasm-bindgen import:
```rust
#[wasm_bindgen]
extern "C" {
    fn render_js_footprint(source: &str, p: JsValue) -> String;
}
```

Rust builds `p` (same contract) and calls the JS host.

### CSP Considerations
If CSP forbids `eval`, the host must:
- precompile JS footprints at load time using permitted mechanisms, or
- require the user to allow `unsafe-eval`.

We should document this constraint.

---

## Implementation Plan

### Phase 1 — Core Runtime Contract
1. **Define Rust structs** for JS param parsing and runtime contract:
   - `JsFootprintParamSpec`
   - `JsFootprintModule`
2. **Build `p` constructor** shared by both runtimes.
3. **Add unit tests** for:
   - param parsing (`net`, `number`, `bool`, `string`, `array`)
   - `p.xy` / `p.eaxy` rotation + translation
   - `p.local_net` naming (`ref_id`) and net indexing

### Phase 2 — Boa Execution (CLI)
4. Add optional `boa_engine` dependency under `js-footprints` feature.
5. Implement `eval_js_footprint` to:
   - load JS source
   - shim CommonJS
   - extract `params` + `body`
6. Integrate into `render_footprint`:
   - detect `*.js` or `what` mapped to JS file
   - invoke Boa evaluation
7. Add CLI integration tests:
   - minimal fixture (mounting hole, diode)
   - compare against existing `knuckles/ergogen/output/pcbs/*.kicad_pcb`

### Phase 3 — WASM Bridge
8. Add wasm-bindgen bridge:
   - `render_js_footprint(source, p)` in JS host
   - `extern "C"` import in Rust
9. Add WASM tests (or integration harness) that:
   - load JS footprint
   - compare to CLI output for same inputs
10. Document CSP requirements.

### Phase 4 — Footprint Search & Config Integration
11. Extend footprint search path rules:
   - `pcbs.<pcb>.footprints_search_paths`
   - project-local `ergogen/footprints/`
12. Add config validation:
   - If JS footprint used and `js-footprints` feature missing, return a clear error.

---

## Validation Strategy

- **Unit Tests**: `p` construction + param parsing
- **Spec Parity Tests**: still run for YAML specs
- **JS Parity Tests**:
  - Run `knuckles` fixtures in Rust
  - Compare `*.kicad_pcb` to upstream Ergogen outputs
- **WASM Parity Tests**:
  - Compare WASM output to CLI output on same fixture set

---

## Risks & Mitigations

- **CSP blocks eval** in browser:
  - Document requirement
  - Provide alternative: prebundle footprints or allow user upload with eval
- **Runtime drift between CLI and WASM**:
  - Share one canonical `p` builder in Rust and serialize for JS
  - Add golden parity tests between CLI and WASM
- **Performance** (large JS footprints):
  - Cache compiled modules in both runtimes
  - Avoid re-parsing for each footprint instance

---

## Success Criteria

- `knuckles/ergogen/config.yaml` renders successfully in:
  - Native CLI (Boa)
  - Browser (JS bridge)
- JS footprints match upstream outputs within existing golden parity harness.
  
