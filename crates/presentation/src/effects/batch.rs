//! The instances the effect pass draws, 16 floats each, and the shapes
//! they take; [`EffectFrame`](super::frame::EffectFrame) fills them.
use super::high_air_shadow::GroundShadow;
use super::lights::CastLightList;
use crate::world::Vec3;

/// Floats per instance: a, b, colour, misc (vec4 each).
pub const EFFECT_FLOATS: usize = 16;
/// Instance shapes (misc.x): a streak from a to b (a lit smoke ribbon when
/// its colour carries an opacity), a glow at a (a solid disc when its colour
/// carries an opacity), a flipbook sprite at a, a rotor's blur, a high-air
/// aircraft's shadow on the ground beneath it.
pub const SHAPE_NAMES: [&str; 5] = ["streak", "glow", "flipbook", "rotor", "shadow"];
const STREAK: f64 = 0.0;
const GLOW: f64 = 1.0;
const FLIPBOOK: f64 = 2.0;
pub(super) const ROTOR: f64 = 3.0;
const SHADOW: f64 = 4.0;
/// Flipbook layers in the effect atlas, and each one's frames.
pub(super) const FIRE: f64 = 0.0;
pub(super) const DUST: f64 = 1.0;
pub(super) const FIRE_FRAMES: f64 = 25.0;
pub(super) const DUST_FRAMES: f64 = 64.0;

/// What the effect pass draws this frame, and the lights it casts.
pub struct EffectBatch {
    pub data: Vec<f32>,
    pub count: usize,
    /// Instances that did not fit this frame.
    pub dropped: usize,
    pub lights: CastLightList,
}

impl EffectBatch {
    pub fn new(capacity: usize) -> Self {
        Self {
            data: vec![0.0; capacity * EFFECT_FLOATS],
            count: 0,
            dropped: 0,
            lights: CastLightList::default(),
        }
    }

    /// One instance in the shader's layout (a, b, colour, misc: a vec4
    /// each); dropped when the batch is full.
    pub(super) fn put(&mut self, rows: [[f64; 4]; 4]) {
        let o = self.count * EFFECT_FLOATS;
        if o >= self.data.len() {
            self.dropped += 1;
            return;
        }
        for (d, x) in self.data[o..o + EFFECT_FLOATS]
            .iter_mut()
            .zip(rows.as_flattened())
        {
            *d = *x as f32;
        }
        self.count += 1;
    }

    /// An additive streak from a (`along[0]` of the way from tail to head)
    /// to b: a sharp line, or a soft glow. A negative width is screen pixels.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn streak(
        &mut self,
        a: Vec3,
        b: Vec3,
        width: f64,
        min_px: f64,
        c: Vec3,
        k: f64,
        along: [f64; 2],
        soft: bool,
        zoom_boost: f64,
    ) {
        let misc = match (zoom_boost != 0.0, soft) {
            (true, true) => -zoom_boost,
            (true, false) => zoom_boost,
            (false, soft) => f64::from(u8::from(soft)),
        };
        let shape = [STREAK, along[0], along[1], misc];
        self.put([v4(a, width), v4(b, min_px), v4(times(c, k), 0.0), shape]);
    }

    /// An additive glow of `radius` metres at p, with `rays` of star.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn glow(
        &mut self,
        p: Vec3,
        radius: f64,
        min_px: f64,
        c: Vec3,
        k: f64,
        turn: f64,
        rays: f64,
    ) {
        let shape = [GLOW, 0.0, 0.0, 0.0];
        self.put([
            v4(p, radius),
            [turn, min_px, rays, 0.0],
            v4(times(c, k), 0.0),
            shape,
        ]);
    }

    /// A solid disc at p in `c` at `opacity`, blended over: a round seen as an object.
    pub(super) fn disc(&mut self, p: Vec3, radius: f64, min_px: f64, c: Vec3, opacity: f64) {
        let shape = [GLOW, 0.0, 0.0, 0.0];
        let colour = v4(times(c, opacity), opacity);
        self.put([v4(p, radius), [0.0, min_px, 0.0, 0.0], colour, shape]);
    }

    /// A lit smoke ribbon from a to b, cut square, blended over.
    pub(super) fn ribbon(
        &mut self,
        a: Vec3,
        b: Vec3,
        width: f64,
        min_px: f64,
        albedo: Vec3,
        opacity: f64,
    ) {
        let shape = [STREAK, 1.0, 1.0, 0.0];
        self.put([v4(a, width), v4(b, min_px), v4(albedo, opacity), shape]);
    }

    /// A flipbook sprite at p: `layer`'s (fractional) `frame` times `tint`,
    /// lifted by `emissive`, fading into what it meets over `soft_m`; `lit`:
    /// `tint` is an albedo the sun lights (smoke, dust), else fire's own colour.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn flipbook(
        &mut self,
        p: Vec3,
        radius: f64,
        turn: f64,
        layer: f64,
        frame: f64,
        tint: Vec3,
        opacity: f64,
        emissive: f64,
        soft_m: f64,
        lit: bool,
    ) {
        let shape = [FLIPBOOK, emissive, f64::from(u8::from(lit)), 0.0];
        self.put([
            v4(p, radius),
            [turn, frame, layer, soft_m],
            v4(tint, opacity),
            shape,
        ]);
    }
}

impl EffectBatch {
    /// A high-air aircraft's shadow on the ground beneath it: its plan's
    /// middle and half length, the ground's up and half span, its nose's
    /// way and opacity, its blur and how far off the ground under its
    /// middle a surface may lie and still take it (`band_m`).
    pub(super) fn ground_shadow(&mut self, s: &GroundShadow, band_m: f64) {
        self.put([
            v4(s.centre, s.half[0]),
            v4(s.normal, s.half[1]),
            v4(s.along, s.opacity),
            [SHADOW, s.blur_m, band_m, 0.0],
        ]);
    }
}

/// `v` with `w` as its fourth.
pub(super) fn v4(v: Vec3, w: f64) -> [f64; 4] {
    [v[0], v[1], v[2], w]
}

/// A colour times an intensity.
fn times(c: Vec3, k: f64) -> Vec3 {
    c.map(|x| x * k)
}
