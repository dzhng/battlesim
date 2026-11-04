//! The static world's batches (`Frame.batches` for what the world
//! places once): each population a view chooses from hands back its record
//! lists by appearance row, tier and pass, each with a print of what it
//! holds. A frame writes a list whose print changed since the renderer last
//! took it, and names every other unchanged (`first` = `NONE`), so the
//! renderer draws what it holds. A renderer that holds nothing says so with
//! a new generation, and is written every list.
use crate::presentation::Frame;
use crate::records::{Batch, CAST_PASS, NONE};
use crate::shadow::CasterVolumes;
use crate::static_world::walk::Record;
use crate::view::{Camera, View};
use std::collections::HashMap;

/// One list a population draws: its appearance row, tier and pass, the tier
/// it casts with and how fog takes it (`Batch.caster`, `Batch.fog`), its
/// records, and a print that changes whenever the records do.
pub struct List<'a> {
    pub row: u32,
    pub tier: u32,
    pub pass: u32,
    pub caster: u32,
    pub cascade_mask: u32,
    pub fog: u32,
    pub records: &'a [Record],
    pub print: u64,
}

impl List<'_> {
    /// The tier a list of `pass` at `tier` casts with when its population
    /// casts by caster lists of its own: a caster list its tier, a view
    /// list none.
    pub fn own_caster(pass: u32, tier: u32) -> u32 {
        if pass == CAST_PASS {
            tier
        } else {
            NONE
        }
    }
}

/// Where a print starts: never zero, so a first mark of 0 still moves it.
pub const PRINT_BASIS: u64 = 0xcbf2_9ce4_8422_2325;

/// Folds `value` into a list's print.
pub fn print(hash: u64, value: u64) -> u64 {
    (hash ^ value).wrapping_mul(0x100000001b3).rotate_left(5)
}

/// Instances placed once that a view chooses from (buildings, scenery).
pub trait Population {
    /// Chooses what `view` draws, and what casts into it under `shadow`;
    /// true when any list changed.
    fn select(&mut self, view: &View, shadow: Option<&CasterVolumes>) -> bool;
    /// Each of its lists.
    fn each_list(&self, visit: &mut dyn FnMut(List));
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

    /// Republish currently held lists from their authoritative populations
    /// without reframing dynamic motion, UI, effects or camera selection.
    pub fn materialize(
        &mut self,
        populations: &[&mut dyn Population],
        frame: &mut Frame,
        generation: u32,
    ) {
        let missing: HashMap<_, _> = frame
            .batches
            .iter()
            .enumerate()
            .filter(|(_, batch)| batch.first == NONE)
            .map(|(index, batch)| (batch.key, index))
            .collect();
        for population in populations {
            population.each_list(&mut |list| {
                let key = crate::view::batch_key(list.row, list.tier, list.pass, list.cascade_mask);
                if let Some(&index) = missing.get(&key) {
                    frame.batches[index].first = frame.instances.len() as u32;
                    frame
                        .instances
                        .extend_from_slice(bytemuck::cast_slice(list.records));
                }
            });
        }
        // The recovered frame is complete now. The next ordinary frame still
        // writes every list, so a renderer may install before or after it.
        self.invalidate();
        self.generation = Some(generation);
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
        generation: u32,
        f: &mut Frame,
    ) {
        if self.generation != Some(generation) {
            (self.generation, self.sent) = (Some(generation), HashMap::new());
        }
        if self.stale || self.walked.as_ref() != Some(camera) {
            let shadow = f.shadow.as_ref().map(|s| CasterVolumes::new(&s.casters));
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
                    caster,
                    cascade_mask,
                    fog,
                    records,
                    print: listed,
                } = list;
                if records.is_empty() {
                    return;
                }
                let key = crate::view::batch_key(row, tier, pass, cascade_mask);
                // A list named unchanged is exactly what was last written:
                // never one of another length under a colliding print. The
                // population's print is mixed first, so a small counter and
                // a length never cancel.
                let written = print(print(PRINT_BASIS, listed), records.len() as u64);
                let held = self.sent.get(&key).filter(|(p, _)| *p == written);
                let version = held.map_or(f.index, |&(_, v)| v);
                f.batches.push(Batch {
                    key,
                    appearance: row,
                    tier,
                    pass,
                    caster,
                    cascade_mask,
                    fog,
                    version,
                    first: if held.is_some() {
                        NONE
                    } else {
                        f.instances.len() as u32
                    },
                    count: records.len() as u32,
                    pad: [0; 2],
                });
                if held.is_none() {
                    f.instances.extend_from_slice(bytemuck::cast_slice(records));
                }
                sent.insert(key, (written, version));
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
    use crate::records::{FOG_FACES, VIEW_PASS};

    /// One building's list under a fixed print.
    struct Still(Vec<Record>);

    impl Population for Still {
        fn select(&mut self, _: &View, _: Option<&CasterVolumes>) -> bool {
            false
        }
        fn each_list(&self, visit: &mut dyn FnMut(List)) {
            visit(List {
                row: 3,
                tier: 0,
                pass: VIEW_PASS,
                caster: NONE,
                cascade_mask: 0,
                fog: FOG_FACES,
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
        statics.write(&mut [still], &camera, &view, generation, &mut f);
        let [batch] = f.batches[..] else {
            panic!("one batch, not {}", f.batches.len());
        };
        batch.first != NONE && f.instances.len() == still.0.len()
    }

    #[test]
    fn a_renderer_holding_nothing_is_written_every_batch_again() {
        let mut statics = StaticBatches::default();
        let mut still = Still(vec![[1.0; 16]]);
        assert!(written(&mut statics, &mut still, 1), "first frame");
        assert!(!written(&mut statics, &mut still, 1), "held: unchanged");
        assert!(written(&mut statics, &mut still, 2), "new generation");
        assert!(!written(&mut statics, &mut still, 2), "held again");
    }

    #[test]
    fn a_batch_named_unchanged_is_exactly_what_was_last_written() {
        // A population whose print does not see its list grow: the batch is
        // still written, never named unchanged with another count.
        let mut statics = StaticBatches::default();
        let mut still = Still(vec![[1.0; 16]]);
        assert!(written(&mut statics, &mut still, 1), "first frame");
        still.0.push([2.0; 16]);
        assert!(written(&mut statics, &mut still, 1), "a longer list");
        assert!(!written(&mut statics, &mut still, 1), "held");
    }

    /// Two lists sharing one counter as their print (as the buildings do).
    struct Counted {
        lists: [Vec<Record>; 2],
        changes: u64,
    }

    impl Population for Counted {
        fn select(&mut self, _: &View, _: Option<&CasterVolumes>) -> bool {
            false
        }
        fn each_list(&self, visit: &mut dyn FnMut(List)) {
            for (tier, records) in self.lists.iter().enumerate() {
                visit(List {
                    row: 3,
                    tier: tier as u32,
                    pass: VIEW_PASS,
                    caster: NONE,
                    cascade_mask: 0,
                    fog: FOG_FACES,
                    records,
                    print: self.changes,
                });
            }
        }
    }

    #[test]
    fn a_change_writes_every_list_of_a_population_that_prints_by_a_counter() {
        // Counter and length must not cancel: 1 with 2 records, then 2
        // with 1 record, is a change.
        let camera = Camera {
            eye: [0.0, -50.0, 30.0],
            target: [0.0; 3],
            up: [0.0, 0.0, 1.0],
            fov_y: 0.8,
            near: 0.5,
            size: [640, 480],
        };
        let view = View::new(&camera);
        let mut statics = StaticBatches::default();
        let mut walk = |pop: &mut Counted| {
            let mut f = Frame::default();
            statics.write(&mut [pop], &camera, &view, 1, &mut f);
            f.batches
                .iter()
                .map(|b| b.first != NONE)
                .collect::<Vec<_>>()
        };
        let mut pop = Counted {
            lists: [vec![[1.0; 16]; 2], vec![[1.0; 16]; 3]],
            changes: 1,
        };
        assert_eq!(walk(&mut pop), [true, true]);
        pop.lists[0].pop();
        pop.changes = 2;
        assert_eq!(
            walk(&mut pop),
            [true, true],
            "every list of the change is written"
        );
    }

    #[test]
    fn cascade_keys_do_not_alias_other_tiers_passes_or_appearance_rows() {
        let mut keys = std::collections::HashSet::new();
        for row in [0, 1, 2, (1 << 20) - 1] {
            for tier in 0..5 {
                for pass in 0..4 {
                    for mask in [0, 1, 2, 4, 8, crate::shadow::ALL_CASCADES] {
                        assert!(keys.insert(crate::view::batch_key(row, tier, pass, mask)));
                    }
                }
            }
        }
    }
}
