# Browser Example

This example shows how to run the wasm bundle in a browser with JS footprints enabled.

## Build

```bash
wasm-pack build crates/ergogen-wasm --target web --out-dir crates/ergogen-wasm/examples/pkg
```

## Run

```bash
python3 -m http.server 8000 -d crates/ergogen-wasm/examples
```

Then open `http://localhost:8000` in a browser.
