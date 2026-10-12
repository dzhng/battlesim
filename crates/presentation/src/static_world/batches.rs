//! The static world's batches (18b's `Frame.batches` for what the world
//! places once): each population a view chooses from hands back its record
//! lists by appearance row, tier and pass. A frame whose camera, or whose
//! side's knowledge, changed what any population draws writes every list;
//! any other frame names the same batches unchanged (`first` = `NONE`), and
//! the renderer draws what it holds.
use crate::presentation::Frame;
use crate::records::{Batch, NONE};
use crate::seed::js_hypot;
use crate::static_world::walk::{Record, Shadow};
use crate::view::{Camera, View};

/// A batch's pass: drawn into the view, or cast into the sun's cascades.
pub const VIEW_PASS: u32 = 0;
pub const CAST_PASS: u32 = 1;

/// One list a population draws: its appearance row, tier, pass and records.
pub struct List<'a> {
    pub row: u32,
    pub tier: u32,
    pub pass: u32,
    pub records: &'a [Record],
}

/// Instances placed once that a view chooses from (buildings, and in time
/// the scenery).
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
        let tan_v = libm::tan(f64::from(camera.fov_y) / 2.0);
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

/// When the static batches were last written.
#[derive(Default)]
pub struct StaticBatches {
    /// The camera last walked.
    walked: Option<Camera>,
    /// Something besides the camera changed (a load, a fall): walk and write.
    stale: bool,
    /// The frame that last wrote them.
    version: u32,
}

impl StaticBatches {
    pub fn invalidate(&mut self) {
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
        let mut changed = false;
        if self.stale || self.walked.as_ref() != Some(camera) {
            let shadow = sun.map(|s| s.shadow(camera));
            for p in populations.iter_mut() {
                changed |= p.select(view, shadow.as_ref());
            }
            changed |= self.stale;
            (self.walked, self.stale) = (Some(*camera), false);
        }
        if changed {
            self.version = f.index;
        }
        let version = self.version;
        for p in populations.iter() {
            p.each_list(&mut |list| {
                let List {
                    row,
                    tier,
                    pass,
                    records,
                } = list;
                if records.is_empty() {
                    return;
                }
                f.batches.push(Batch {
                    key: (row << 8) | (tier << 4) | pass,
                    appearance: row,
                    tier,
                    pass,
                    cascades: 0,
                    version,
                    first: if changed {
                        f.instances.len() as u32
                    } else {
                        NONE
                    },
                    count: records.len() as u32,
                });
                if changed {
                    f.instances.extend_from_slice(bytemuck::cast_slice(records));
                }
            });
        }
    }
}
