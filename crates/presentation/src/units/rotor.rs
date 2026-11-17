//! Rotor speed from the pose the side knows, independent of drawing or sound.
/// At or below this speed the authored blades replace the shutter effect.
/// Kept in step with `BLUR_CROSSOVER` in packages/scene-assets/src/articulation.ts.
pub const BLUR_CROSSOVER: f64 = 0.2;

/// Full in flight; on landing, ease down after touchdown; on packing, ease up
/// over the whole pack. A parked aircraft without deployment is shut down.
pub fn rotor_speed(
    grounded: bool,
    progress: Option<f64>,
    target: Option<&str>,
    drop_seconds: f64,
    seconds: f64,
) -> f64 {
    if !grounded {
        return 1.0;
    }
    let Some(progress) = progress else { return 0.0 };
    let u = if target == Some("packed") {
        1.0 - progress
    } else {
        let touchdown = drop_seconds / seconds;
        1.0 - (progress - touchdown) / (1.0 - touchdown)
    }
    .clamp(0.0, 1.0);
    u * u * (3.0 - 2.0 * u)
}
