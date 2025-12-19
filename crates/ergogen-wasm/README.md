# ergogen-wasm

Minimal WASM entrypoint crate for Ergogen‑RS.

## Current API

```js
import init, { version } from "ergogen_wasm";

await init();
console.log(version());
```

This is a skeleton; rendering APIs are still in progress.

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
