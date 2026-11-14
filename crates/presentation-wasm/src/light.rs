//! The map-seed rules a match's light follows, as the native client applies
//! them: the page asks here rather than deriving them a second time.
use wasm_bindgen::prelude::*;

/// The sun azimuth `light` (a `presentation` light look, JSON) takes for
/// `map_seed`: one of its `sun_azimuths`, else its own.
#[wasm_bindgen]
pub fn light_sun_azimuth(light: &str, map_seed: &str) -> Result<f64, JsError> {
    let light = presentation::light::LightSettings::from_value(
        &serde_json::from_str(light).map_err(crate::error)?,
    )
    .map_err(crate::error)?;
    Ok(light
        .with_map_seed(map_seed)
        .map_err(crate::error)?
        .sun_azimuth)
}

/// `map_seed`'s pick among `n` choices (Random lighting's look).
#[wasm_bindgen]
pub fn map_seed_index(map_seed: &str, n: u32) -> Result<u32, JsError> {
    presentation::light::map_seed_index(map_seed, n as usize)
        .map(|i| i as u32)
        .map_err(crate::error)
}
