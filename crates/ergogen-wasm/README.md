# ergogen-wasm

Minimal WASM entrypoint crate for Ergogen‑RS.

## Current API

```js
import init, { version, render_pcb, render_dxf, render_svg, render_all } from "ergogen_wasm";

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
