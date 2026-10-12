//! The static world's batches (18b's `Frame.batches` for what the world
//! places once): each population a view chooses from hands back its record
//! lists by appearance row, tier and pass, each with a print of what it
//! holds. A frame writes a list whose print changed since the renderer last
//! took it, and names every other unchanged (`first` = `NONE`), so the
//! renderer draws what it holds. A renderer that holds nothing says so with
//! a new generation, and is written every list.
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
    /// The renderer's generation `sent` was written for.
    generation: Option<u32>,
}

impl StaticBatches {
    /// What was sent no longer stands (a load, new populations): walk and
    /// write all.
    pub fn invalidate(&mut self) {
        self.stale = true;
        self.sent.clear();
    }

    /// What the populations draw may have changed: walk again.
    pub fn rewalk(&mut self) {
        self.stale = true;
    }

    /// Walks `populations` for `camera` when it or what they know changed,
    /// and lists their batches in `f`: written when any list changed or the
    /// renderer's `generation` (bumped whenever it holds nothing) is new,
    /// named unchanged otherwise.
    pub fn write(
        &mut self,
        populations: &mut [&mut dyn Population],
        camera: &Camera,
        view: &View,
        sun: Option<&Sun>,
        generation: u32,
        f: &mut Frame,
    ) {
        if self.generation != Some(generation) {
            (self.generation, self.sent) = (Some(generation), HashMap::new());
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// One building's list that never changes.
    struct Still([Record; 1]);

    impl Population for Still {
        fn select(&mut self, _: &View, _: Option<&Shadow>) -> bool {
            false
        }
        fn each_list(&self, visit: &mut dyn FnMut(List)) {
            visit(List {
                row: 3,
                tier: 0,
                pass: VIEW_PASS,
                records: &self.0,
                print: 7,
            });
        }
    }

    /// Frames `still` once for a renderer at `generation`: whether its batch
    /// was written (records follow) rather than named unchanged.
    fn written(statics: &mut StaticBatches, still: &mut Still, generation: u32) -> bool {
        let camera = Camera {
            eye: [0.0, -50.0, 30.0],
            target: [0.0; 3],
            up: [0.0, 0.0, 1.0],
            fov_y: 0.8,
            near: 0.5,
            size: [640, 480],
        };
        let mut f = Frame::default();
        let view = View::new(&camera);
        statics.write(&mut [still], &camera, &view, None, generation, &mut f);
        let [batch] = f.batches[..] else {
            panic!("one batch, not {}", f.batches.len());
        };
        batch.first != NONE && f.instances.len() == 1
    }

    #[test]
    fn a_renderer_holding_nothing_is_written_every_batch_again() {
        let mut statics = StaticBatches::default();
        let mut still = Still([[1.0; 16]]);
        assert!(written(&mut statics, &mut still, 1), "first frame");
        assert!(!written(&mut statics, &mut still, 1), "held: unchanged");
        assert!(written(&mut statics, &mut still, 2), "new generation");
        assert!(!written(&mut statics, &mut still, 2), "held again");
    }
}
