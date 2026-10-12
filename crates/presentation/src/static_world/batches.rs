//! The static world's batches (18b's `Frame.batches` for what the world
//! places once): each population a view chooses from hands back its record
//! lists by appearance row, tier and pass, each with a print of what it
//! holds. A frame writes a list whose print changed since the renderer last
//! took it, and names every other unchanged (`first` = `NONE`), so the
//! renderer draws what it holds.
use crate::presentation::Frame;
use crate::records::{Batch, NONE};
use crate::seed::js_hypot;
use crate::static_world::walk::{Record, Shadow};
use crate::view::{Camera, View};
use std::collections::HashMap;

/// A batch's pass: drawn into the view, or cast into the sun's cascades.
pub const VIEW_PASS: u32 = 0;
pub const CAST_PASS: u32 = 1;

/// One list a population draws: its appearance row, tier, pass and records,
/// and a print that changes whenever the records do.
pub struct List<'a> {
    pub row: u32,
    pub tier: u32,
    pub pass: u32,
    pub records: &'a [Record],
    pub print: u64,
}

/// Folds `value` into a list's print.
pub fn print(hash: u64, value: u64) -> u64 {
    (hash ^ value).wrapping_mul(0x100000001b3).rotate_left(5)
}

/// Instances placed once that a view chooses from (buildings, scenery).
pub trait Population {
    /// Chooses what `view` draws, and what casts into it under `shadow`;
    /// true when any list changed.
    fn select(&mut self, view: &View, shadow: Option<&Shadow>) -> bool;
    /// Each of its lists.
    fn each_list(&self, visit: &mut dyn FnMut(List));
}

/// The sun the static world casts under.
pub struct Sun {
    /// Azimuth and elevation, rad.
    pub azimuth: f64,
    pub elevation: f64,
    /// The farthest a shadow is received from the eye along the view, m.
    pub far_m: f64,
}

impl Sun {
    /// Where shadows fall for `camera`: metres across the ground per metre of
    /// height, and how far from the eye they are received in any direction
    /// (the cascades' far plane at the view's corner). JS's `Math` functions,
    /// as the page's `sunShadow` took them.
    pub fn shadow(&self, camera: &Camera) -> Shadow {
        let throw = 1.0 / libm::tan(self.elevation);
        let tan_v = libm::tan(camera.fov_y / 2.0);
        let [w, h] = camera.size.map(|v| f64::from(v.max(1)));
        Shadow {
            fall: [
                -libm::cos(self.azimuth) * throw,
                -libm::sin(self.azimuth) * throw,
            ],
            reach: self.far_m * js_hypot(&[1.0, tan_v, tan_v * w / h]),
        }
    }
}

/// What the static batches were last walked for, and what the renderer holds.
#[derive(Default)]
pub struct StaticBatches {
    /// The camera last walked.
    walked: Option<Camera>,
    /// Something besides the camera changed (a fall, a clock): walk again.
    stale: bool,
    /// Per batch key: the print and version the renderer last took.
    sent: HashMap<u32, (u64, u32)>,
}

impl StaticBatches {
    /// The renderer holds nothing (a load, a new renderer): walk and write all.
    pub fn invalidate(&mut self) {
        self.stale = true;
        self.sent.clear();
    }

    /// What the populations draw may have changed: walk again.
    pub fn rewalk(&mut self) {
        self.stale = true;
    }

    /// Walks `populations` for `camera` when it or what they know changed,
    /// and lists their batches in `f`: written when any list changed,
    /// named unchanged otherwise.
    pub fn write(
        &mut self,
        populations: &mut [&mut dyn Population],
        camera: &Camera,
        view: &View,
        sun: Option<&Sun>,
        f: &mut Frame,
    ) {
        if self.stale || self.walked.as_ref() != Some(camera) {
            let shadow = sun.map(|s| s.shadow(camera));
            for p in populations.iter_mut() {
                p.select(view, shadow.as_ref());
            }
            (self.walked, self.stale) = (Some(*camera), false);
        }
        let mut sent = HashMap::with_capacity(self.sent.len());
        for p in populations.iter() {
            p.each_list(&mut |list| {
                let List {
                    row,
                    tier,
                    pass,
                    records,
                    print,
                } = list;
                if records.is_empty() {
                    return;
                }
                let key = (row << 8) | (tier << 4) | pass;
                let held = self.sent.get(&key).filter(|(p, _)| *p == print);
                let version = held.map_or(f.index, |&(_, v)| v);
                f.batches.push(Batch {
                    key,
                    appearance: row,
                    tier,
                    pass,
                    cascades: 0,
                    version,
                    first: if held.is_some() {
                        NONE
                    } else {
                        f.instances.len() as u32
                    },
                    count: records.len() as u32,
                });
                if held.is_none() {
                    f.instances.extend_from_slice(bytemuck::cast_slice(records));
                }
                sent.insert(key, (print, version));
            });
        }
        // A batch left out is the renderer's to drop: it is written anew
        // when it comes back.
        self.sent = sent;
    }
}
