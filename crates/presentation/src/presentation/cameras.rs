//! The frame's cameras: its eye's, and the sun's cascades fitted to it,
//! each on the frame's clock and in the ground's wind, so whatever moves in
//! the wind moves alike in every pass that draws it, shadows included; and
//! the wave field, which follows the eye (`crate::wave`); and where sound
//! listens for it.
use super::*;
use crate::effects::frame::WaveFrame;
use crate::sound::frame::Listener;
use crate::sound::tables::ListenerPlacement;

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
    f.camera.waves = crate::wave::area(camera);
    f.shadow = shadow.as_mut().map(|fit| fit.frame(camera));
    for caster in f.shadow.iter_mut().flat_map(|s| s.casters.iter_mut()) {
        caster.time = f.camera.time;
        caster.wind = f.camera.wind;
    }
}

impl Presentation {
    /// Where sound listens for the last frame's camera, as `placement` puts
    /// it; none before a frame.
    pub fn listener(&self, placement: ListenerPlacement) -> Option<Listener> {
        let c = self.camera.as_ref()?;
        let up = c.up.map(|v| v as f32);
        Some(Listener::of_camera(c.eye, c.target, up, placement))
    }

    /// The wave field's frame for the last frame's clock and camera: its
    /// steps since the one before and the wakes they splat
    /// (`EffectFrame::waves`); none before a battle and a frame.
    pub fn waves(&mut self) -> Option<WaveFrame> {
        let camera = self.camera?;
        let clock = self.frame.clock;
        Some(self.loaded.as_mut()?.effects.waves(clock, &camera))
    }
}
