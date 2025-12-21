# ergogen-wasm

Minimal WASM entrypoint crate for Ergogen‑RS.

## Current API

```js
import init, {
  version,
  render_pcb,
  render_dxf,
  render_svg,
  render_all,
  set_virtual_fs,
  clear_virtual_fs
} from "ergogen_wasm";

await init();
console.log(version());
const pcb = render_pcb(configYaml, "pcb");
const outlineDxf = render_dxf(configYaml, "outline");
const outlineSvg = render_svg(configYaml, "outline");
const outputs = render_all(configYaml);
```

Rendering APIs are available for PCB + outline exports. `render_all` returns the
same shape expected by the web UI (canonical/points/units, demo DXF/SVG, outlines,
cases JSCAD, and PCBs), plus an `errors` array for per-target failures.

### Accepted input formats

All render functions accept a **YAML config string** or **KLE JSON** (auto-detected)
and convert it to the canonical Ergogen config before rendering.

### API surface (v0)

- `version() -> string`
- `set_virtual_fs({ [path: string]: string })`
- `clear_virtual_fs()`
- `render_all(config: string) -> RenderAllOutput`
- `render_pcb(config: string, pcbName: string) -> string`
- `render_dxf(config: string, outlineName: string) -> string`
- `render_svg(config: string, outlineName: string) -> string`
- `render_case_jscad_v2(config: string, caseName: string) -> string`

### Virtual FS (spec footprints)

To resolve `what: spec` footprints without touching the host filesystem, provide a
virtual file map:

```js
set_virtual_fs({
  "footprints/pad.yaml": "name: pad\n...",
  "my_spec.yaml": "name: my_spec\n..."
});
// Call clear_virtual_fs() to reset.
```

## JS Footprints (WASM)

The wasm bridge expects host JS functions to exist:

```js
import {
  installErgogenJsFootprints,
  registerErgogenJsFootprintSource
} from "./js/footprints.js";

installErgogenJsFootprints();
registerErgogenJsFootprintSource("simple.js", "module.exports = { ... }");
```

The host can also provide a global `ergogenJsFootprintSourceLoader(path)` function
to load footprints dynamically (e.g., via `fetch` or a registry map).
