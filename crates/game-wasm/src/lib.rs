//! Thin WASM boundary over `sim`: commands in, side-filtered observations out.
use wasm_bindgen::prelude::*;

/// Build identity, so the browser can report which simulation it loaded.
#[wasm_bindgen]
pub fn build_id() -> String {
    format!("game-wasm {}", env!("CARGO_PKG_VERSION"))
}
