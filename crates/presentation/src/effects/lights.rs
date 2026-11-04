//! Cast lights: the light combat throws on what is round it
//! (the shared renderer's `lighting.wgsl` owns shading). Effects offer burning lights each frame
//! (a flash, a motor, a tracer, a burst, a fire), each with its cause; of
//! those reaching into the view, the world's uniform takes every one while
//! they fit, else the `CAST_LIGHTS_MAX` of most weight as the camera sees them.
use std::sync::Arc;

use crate::view::View;
use crate::world::Vec3;

/// Lights the world's uniform holds.
pub const CAST_LIGHTS_MAX: usize = 24;
/// Candidates the effects may offer in one frame; past it a light is dropped.
pub const CAST_LIGHT_CANDIDATES: usize = 1024;
/// Floats per candidate: x, y, z, radius, r, g, b (colour × intensity), 0.
pub const CAST_LIGHT_FLOATS: usize = 8;
/// The packed uniform's floats: a header (count, wrap, 0, 0), then per light
/// its position and 1/radius², and its colour and 0.
pub const CAST_LIGHTS_FLOATS: usize = 4 + 8 * CAST_LIGHTS_MAX;

/// The lights burning this frame, as the effects offer them.
pub struct CastLightList {
    pub data: Vec<f32>,
    pub count: usize,
    /// Lights that did not fit this frame.
    pub dropped: usize,
    /// How far round a surface every light reaches past facing it, 0..1.
    pub wrap: f64,
    /// What cast each light (`flash:<kind>`, `round:<kind>`, `impact:<hit>`,
    /// `blast`, `fire:<kind>`): for probes and captures, not the GPU.
    pub causes: Vec<Arc<str>>,
}

impl Default for CastLightList {
    fn default() -> Self {
        Self {
            data: vec![0.0; CAST_LIGHT_CANDIDATES * CAST_LIGHT_FLOATS],
            count: 0,
            dropped: 0,
            wrap: 0.0,
            causes: Vec::with_capacity(CAST_LIGHT_CANDIDATES),
        }
    }
}

impl CastLightList {
    pub fn clear(&mut self, wrap: f64) {
        self.count = 0;
        self.dropped = 0;
        self.wrap = wrap;
        self.causes.clear();
    }

    /// Offer a light at `at` of `radius` metres, `color` × `intensity`;
    /// nothing when it is dark or reaches nothing.
    pub fn offer(&mut self, at: Vec3, radius: f64, color: Vec3, intensity: f64, cause: &Arc<str>) {
        if !(intensity > 0.0 && radius > 0.0) {
            return;
        }
        if self.count >= CAST_LIGHT_CANDIDATES {
            self.dropped += 1;
            return;
        }
        let o = self.count * CAST_LIGHT_FLOATS;
        let row = [
            at[0],
            at[1],
            at[2],
            radius,
            color[0] * intensity,
            color[1] * intensity,
            color[2] * intensity,
            0.0,
        ];
        for (d, v) in self.data[o..o + CAST_LIGHT_FLOATS].iter_mut().zip(row) {
            *d = v as f32;
        }
        self.causes.push(cause.clone());
        self.count += 1;
    }
}

/// The world's cast lights for one view, packed (`packCastLights`).
pub struct CastLights {
    pub packed: [f32; CAST_LIGHTS_FLOATS],
    order: Vec<usize>,
    weight: Vec<f32>,
}

impl Default for CastLights {
    fn default() -> Self {
        Self {
            packed: [0.0; CAST_LIGHTS_FLOATS],
            order: Vec::with_capacity(CAST_LIGHT_CANDIDATES),
            weight: vec![0.0; CAST_LIGHT_CANDIDATES],
        }
    }
}

impl CastLights {
    /// Packs every light of `list` reaching into `view`'s sides while they
    /// fit, else the `CAST_LIGHTS_MAX` of most weight for a camera on its
    /// target; returns how many.
    pub fn pack(&mut self, list: &CastLightList, view: &View) -> usize {
        let v = &list.data;
        self.order.clear();
        for i in 0..list.count {
            let o = i * CAST_LIGHT_FLOATS;
            let centre = [v[o], v[o + 1], v[o + 2]].map(f64::from);
            if view.sees_sphere(centre, f64::from(v[o + 3])) {
                self.order.push(i);
            }
        }
        if self.order.len() > CAST_LIGHTS_MAX {
            for &i in &self.order {
                self.weight[i] = weight(&v[i * CAST_LIGHT_FLOATS..], view.target, view.distance);
            }
            // Stable, by float32 weight, so equal candidates retain their order.
            let w = &self.weight;
            self.order.sort_by(|&a, &b| w[b].total_cmp(&w[a]));
        }
        let kept = self.order.len().min(CAST_LIGHTS_MAX);
        let out = &mut self.packed;
        out[..4].copy_from_slice(&[kept as f32, list.wrap as f32, 0.0, 0.0]);
        for (k, &i) in self.order[..kept].iter().enumerate() {
            let (o, w) = (i * CAST_LIGHT_FLOATS, 4 + k * 8);
            let r = f64::from(v[o + 3]);
            out[w..w + 3].copy_from_slice(&v[o..o + 3]);
            out[w + 3] = (1.0 / (r * r)) as f32;
            out[w + 4..w + 7].copy_from_slice(&v[o + 4..o + 7]);
            out[w + 7] = 0.0;
        }
        kept
    }
}

/// A light's weight for the cut: its strength over its reach, dimmed by its
/// distance from the camera's focus in camera distances (a light far off
/// lights few pixels).
fn weight(v: &[f32], focus: Vec3, distance: f64) -> f32 {
    let v: [f64; 7] = std::array::from_fn(|k| f64::from(v[k]));
    let strength = v[4].max(v[5]).max(v[6]) * v[3] * v[3];
    let (dx, dy, dz) = (v[0] - focus[0], v[1] - focus[1], v[2] - focus[2]);
    let far = (dx * dx + dy * dy + dz * dz) / (distance * distance).max(1.0);
    (strength / (1.0 + far)) as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::Camera;

    const WHITE: Vec3 = [1.0; 3];

    /// A camera 65 m from the origin, looking north and down.
    fn view() -> View {
        View::new(&Camera {
            eye: [0.0, -43.0, 48.7],
            target: [0.0; 3],
            up: [0.0, 0.0, 1.0],
            fov_y: 0.8,
            near: 1.0,
            size: [1920, 1080],
        })
    }

    fn offer(list: &mut CastLightList, at: Vec3, radius: f64, color: Vec3, k: f64) {
        list.offer(at, radius, color, k, &Arc::from("round:rifle"));
    }

    /// The packed lights' x and reach.
    fn packed(lights: &CastLights, kept: usize) -> Vec<(f32, f32)> {
        let row = |k: usize| &lights.packed[4 + k * 8..12 + k * 8];
        (0..kept)
            .map(|k| (row(k)[0], 1.0 / row(k)[3].sqrt()))
            .collect()
    }

    #[test]
    fn every_light_in_view_is_packed_while_they_fit_with_the_wrap() {
        let mut list = CastLightList::default();
        list.clear(0.5);
        offer(&mut list, [1.0, 2.0, 3.0], 4.0, [1.0, 0.5, 0.25], 8.0);
        offer(&mut list, [9.0, 0.0, 0.0], 2.0, WHITE, 1.0);
        // Dark, or reaching nothing: not a light.
        offer(&mut list, [9.0, 0.0, 0.0], 2.0, WHITE, 0.0);
        offer(&mut list, [9.0, 0.0, 0.0], 0.0, WHITE, 5.0);
        let mut lights = CastLights::default();
        assert_eq!(lights.pack(&list, &view()), 2);
        assert_eq!(lights.packed[..2], [2.0, 0.5]);
        assert_eq!(lights.packed[8..11], [8.0, 4.0, 2.0]);
        assert_eq!(packed(&lights, 2)[1].0, 9.0);
    }

    #[test]
    fn a_light_whose_reach_never_enters_the_view_is_left_out() {
        let mut list = CastLightList::default();
        // Far past the view's side, and past it but with a pool reaching in.
        offer(&mut list, [400.0, 0.0, 1.0], 16.0, WHITE, 40.0);
        offer(&mut list, [80.0, 0.0, 1.0], 60.0, WHITE, 40.0);
        let mut lights = CastLights::default();
        assert_eq!(lights.pack(&list, &view()), 1);
        assert_eq!(packed(&lights, 1)[0].0, 80.0);
    }

    #[test]
    fn over_the_cap_the_lights_kept_are_those_the_camera_sees_strongest() {
        let mut list = CastLightList::default();
        // A crowd of faint tracers round the focus, a tank's flash among them.
        for i in 0..CAST_LIGHTS_MAX + 20 {
            offer(&mut list, [i as f64 * 0.1, 0.0, 1.0], 2.0, WHITE, 1.0);
        }
        offer(&mut list, [3.0, 0.0, 2.0], 16.0, WHITE, 40.0);
        let mut lights = CastLights::default();
        assert_eq!(lights.pack(&list, &view()), CAST_LIGHTS_MAX);
        let kept = packed(&lights, CAST_LIGHTS_MAX);
        // The flash first; then the equal tracers in the order offered.
        assert_eq!(kept[0].0, 3.0);
        assert!(kept[1..].windows(2).all(|w| w[0].0 < w[1].0));
    }

    #[test]
    fn a_full_candidate_list_drops_what_does_not_fit_and_counts_it() {
        let mut list = CastLightList::default();
        for _ in 0..CAST_LIGHT_CANDIDATES + 2 {
            offer(&mut list, [0.0; 3], 1.0, WHITE, 1.0);
        }
        assert_eq!((list.count, list.dropped), (CAST_LIGHT_CANDIDATES, 2));
    }
}
