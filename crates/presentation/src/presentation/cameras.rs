//! The frame's cameras: its eye's, and the sun's cascades fitted to it,
//! each on the frame's clock and in the ground's wind, so whatever moves in
//! the wind moves alike in every pass that draws it, shadows included.
use super::*;

/// Fills `f`'s camera and sun shadow for `camera` at `time`.
pub(super) fn cameras(
    f: &mut Frame,
    view: &View,
    shadow: &mut Option<ShadowFit>,
    ground: &Option<Ground>,
    camera: &Camera,
    time: f64,
) {
    f.camera = view.camera(camera, time);
    f.camera.wind = [ground
        .as_ref()
        .map(|g| g.biome.wind.record())
        .unwrap_or_default()];
    f.shadow = shadow.as_mut().map(|fit| fit.frame(camera));
    for caster in f.shadow.iter_mut().flat_map(|s| s.casters.iter_mut()) {
        caster.time = f.camera.time;
        caster.wind = f.camera.wind;
    }
}
