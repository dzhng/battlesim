//! Smoke in fog of war's sight: the side's clouds, each at its step of
//! `contract::smoke`'s law, as the terrain march reads them (`fogSmokeStep`
//! adds what `contract::smoke::piece_depth` adds), and which eye maps a
//! change reaches. A cloud steps a handful of times in its life, so a map
//! rebuilds only when a cloud within its reach comes, steps or goes.
use contract::smoke::CloudState;

use crate::records::FogSmoke;

/// A plan-view circle a change reached: its centre and radius.
pub(super) type Circle = ([f64; 2], f64);

/// The clouds the side's sight is cut by, as last set, and whether the
/// renderer holds them.
#[derive(Default)]
pub(super) struct Smoke {
    clouds: Vec<(u32, CloudState)>,
    sent: bool,
}

impl Smoke {
    /// Takes the side's clouds, each `(id, state)`: the circles of every
    /// cloud that came, went or changed state, before and after.
    pub(super) fn set(&mut self, next: &[(u32, CloudState)]) -> Vec<Circle> {
        let mut changed: Vec<Circle> = Vec::new();
        for (from, to) in [(&self.clouds[..], next), (next, &self.clouds[..])] {
            for (id, c) in from {
                if !to.iter().any(|(i, other)| i == id && other == c) {
                    changed.push((c.centre, c.radius_m));
                }
            }
        }
        if !changed.is_empty() {
            self.clouds = next.to_vec();
            self.sent = false;
        }
        changed
    }

    /// The renderer holds nothing: the next upload sends the clouds again.
    pub(super) fn resend(&mut self) {
        self.sent = false;
    }

    /// The circles, when the renderer does not hold them yet.
    pub(super) fn upload(&mut self) -> Option<Vec<FogSmoke>> {
        if self.sent {
            return None;
        }
        self.sent = true;
        Some(
            self.clouds
                .iter()
                .map(|(_, c)| FogSmoke {
                    center: c.centre.map(|v| v as f32),
                    radius: c.radius_m as f32,
                    height: c.height_m as f32,
                    depth_per_m: c.depth_per_m as f32,
                    ..FogSmoke::default()
                })
                .collect(),
        )
    }

    /// How many circles the march reads.
    pub(super) fn count(&self) -> u32 {
        self.clouds.len() as u32
    }
}

/// Whether a map built at `position` out to `reach` meets any of `circles`.
pub(super) fn reaches(circles: &[Circle], position: [f64; 3], reach: f64) -> bool {
    circles.iter().any(|&([x, y], r)| {
        let (dx, dy) = (x - position[0], y - position[1]);
        dx * dx + dy * dy <= (reach + r) * (reach + r)
    })
}
