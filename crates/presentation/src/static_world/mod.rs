//! The static world resolved once per map for every renderer:
//! the buildings resolved against the template art library, the walk that
//! chooses what each view draws of them, the state a side's knowledge stands
//! each in, and the batches a frame lists them in; the ground, its tables and
//! its scenery; the map's props and what the side knows of them. The
//! simulation never reads art.
pub mod batches;
pub mod buildings;
pub mod fallen;
#[cfg(all(feature = "files", not(target_arch = "wasm32")))]
pub mod files;
pub mod ground;
pub mod index;
pub mod library;
pub mod props;
pub mod walk;

/// `srgbToLinear` (`math/color`): an art colour channel as the GPU reads it.
pub fn srgb_to_linear(c: f64) -> f64 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        libm::pow((c + 0.055) / 1.055, 2.4)
    }
}
