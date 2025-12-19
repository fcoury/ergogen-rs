# ergogen-wasm

Minimal WASM entrypoint crate for Ergogen‑RS.

## Current API

```js
import init, { version } from "ergogen_wasm";

await init();
console.log(version());
```

This is a skeleton; rendering APIs and JS‑footprint bridging will be added in Phase 2/3.
