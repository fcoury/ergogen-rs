use wasm_bindgen::prelude::wasm_bindgen;

/// Returns the current crate version. Used as a minimal wasm smoke export.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
