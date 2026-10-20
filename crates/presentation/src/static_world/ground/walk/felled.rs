//! Known falling trees: immutable motion envelopes and independent pass selection.
use super::*;
use std::f64::consts::FRAC_PI_2;

/// A tree the side saw fall, its direction and presentation start time.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct FelledTree {
    pub prop: u32,
    pub toward: [f64; 2],
    pub fell_at: f64,
}

#[derive(Default)]
pub(super) struct FelledSpatial {
    ready: bool,
    pub(super) known: HashMap<usize, usize>,
    pub(super) latest: Option<f64>,
    candidates: Vec<usize>,
    pub(super) visited: usize,
    initialized: usize,
    pub(super) reconciled: usize,
    examined: usize,
}

impl SceneryWalk {
    /// Cumulative motion-envelope rows initialized and snapshot rows reconciled,
    /// then standing/fallen membership checks in the last selection. Snapshot
    /// equality and replacement also cost O(F); these are work units, not timings.
    pub fn felled_work(&self) -> [usize; 3] {
        [
            self.felled_spatial.initialized,
            self.felled_spatial.reconciled,
            self.forest.examined + self.felled_spatial.examined,
        ]
    }

    pub(super) fn prepare_felled(&mut self, rules: &FelledRules) {
        if self.felled_spatial.ready {
            return;
        }
        // Replace the existing hierarchy once, retaining its original standing
        // boxes for visibility and LOD. No second index or forest records live on.
        let mut bounds = Vec::with_capacity(self.forest.chunks.len());
        for chunk in &self.forest.chunks {
            let mut b = chunk.bounds;
            for k in 0..self.forest.kinds {
                for i in chunk.start[k]..chunk.end[k] {
                    self.felled_spatial.initialized += 1;
                    let r = self.forest.sorted[k][i];
                    let h = self.forest.heights[k][i] as f64;
                    let reach = self.sizes[k].radius * r[12] as f64;
                    let trunk = self.trunks[k].radius * r[12] as f64;
                    let hinge = rules.stump_height_m;
                    // Shader compression never expands radial components; its
                    // kick is two trunk radii. Include the unscaled-height stump.
                    let span = (h - hinge).max(0.) + 2. * reach;
                    let extent = [
                        span + 2. * trunk,
                        span + 2. * trunk,
                        span + hinge.max(trunk),
                    ];
                    for axis in 0..3 {
                        b[axis] = b[axis].min(r[axis] as f64 - extent[axis]);
                        b[axis + 3] = b[axis + 3].max(r[axis] as f64 + extent[axis]);
                    }
                }
            }
            bounds.push(b);
        }
        self.forest.chunk_index = BoundsIndex::new(bounds);
        self.felled_spatial.ready = true;
    }

    pub(super) fn select_felled(
        &mut self,
        ground: &Ground,
        view: &View,
        shadow: Option<&CasterVolumes>,
        lod: &[f64; 3],
    ) {
        let rules = &ground.biome.trees.felled;
        self.prepare_felled(rules);
        // Offscreen animation advances from the clock without walking its trees.
        self.falling = self
            .felled_spatial
            .latest
            .is_some_and(|at| self.clock - at < rules.fall_s + rules.settle_s);
        let before = std::mem::replace(
            &mut self.felled_lists,
            vec![vec![]; self.scenery.kinds.len() * TIERS * (1 + CASCADES)],
        );
        let before_stumps = std::mem::replace(
            &mut self.stumps,
            vec![vec![]; self.scenery.kinds.len() * (1 + CASCADES)],
        );
        let kinds = self.scenery.kinds.len();
        let forest = self.scenery.forest.as_chunks::<FLOATS>().0;
        let spatial = &mut self.felled_spatial;
        spatial.visited = 0;
        spatial.examined = 0;
        for pass in 0..=CASCADES {
            spatial.candidates.clear();
            spatial.visited += self.forest.chunk_index.query(
                |b| {
                    if pass == 0 {
                        view.sees_box(b)
                    } else {
                        shadow.is_some_and(|s| s.mask(b) & (1 << (pass - 1)) != 0)
                    }
                },
                |c| {
                    let chunk = &self.forest.chunks[c];
                    for k in 0..self.forest.kinds {
                        for i in chunk.start[k]..chunk.end[k] {
                            spatial.examined += 1;
                            if let Some(&id) = spatial.known.get(&self.forest.sources[k][i]) {
                                spatial.candidates.push(id);
                            }
                        }
                    }
                },
            );
            spatial.candidates.sort_unstable();
            for &id in &spatial.candidates {
                let f = &self.felled[id];
                let p = &forest[self.index[&f.prop]];
                let (mut stump, kind, _, _) = instance(p, &self.sizes);
                stump[11] = 0.;
                stump[14..16].copy_from_slice(&[1., (0.5 * rules.stump_height_m) as f32]);
                self.stumps[pass * kinds + kind].push(stump);
            }
            // Casters merge source tiers into coarser tiers in source-tier order.
            // Preserve that order, including the two tiers that share LAST.
            let tier = |id: usize| {
                let f = &self.felled[id];
                let p = &forest[self.index[&f.prop]];
                let (_, _, h, _) = instance(p, &self.sizes);
                let d = crate::world::length([0, 1, 2].map(|a| p[a] as f64 - view.eye[a]));
                tier_for(h as f64, d, view, lod)
            };
            if pass != 0 {
                spatial.candidates.sort_by_cached_key(|&id| (tier(id), id));
            }
            for &id in &spatial.candidates {
                let f = &self.felled[id];
                let p = &forest[self.index[&f.prop]];
                let (mut fall, kind, h, reach) = instance(p, &self.sizes);
                let t = &self.trunks[kind];
                let pose = fell_pose(
                    rules,
                    self.clock - f.fell_at,
                    h as f64,
                    reach as f64,
                    t.radius * p[4] as f64,
                    t.crown * p[5] as f64,
                );
                fall[4..8].copy_from_slice(
                    &[f.toward[0], f.toward[1], pose[0], pose[1]].map(|v| v as f32),
                );
                let mut rest = [0.; 16];
                for (r, v) in rest.iter_mut().zip(&pose[2..]) {
                    *r = *v as f32;
                }
                let t = tier(id);
                let t = if pass == 0 { t } else { caster_tier(t) };
                self.felled_lists[pass * kinds * TIERS + kind * TIERS + t].extend([fall, rest]);
            }
        }
        if self.felled_lists != before || self.stumps != before_stumps {
            self.felled_print += 1;
        }
    }
}

/// A felled tree's pose `age` seconds after it began to fall, `height` tall
/// with a crown `reach` round its trunk, trunk radius `trunk` and crown base
/// `crown` (scaled): its tip angle and hinge, then the press up-and-down and
/// across of what lies past its trunk, the trunk, how far it has landed and
/// the crown base.
pub(super) fn fell_pose(
    rules: &FelledRules,
    age: f64,
    height: f64,
    reach: f64,
    trunk: f64,
    crown: f64,
) -> [f64; 7] {
    let hinge = rules.stump_height_m;
    let length = height - hinge;
    let rest = FRAC_PI_2 + libm::asin((trunk / length.max(1e-3)).min(1.0));
    let angle = if age <= 0.0 {
        0.0
    } else if age < rules.fall_s {
        rest * (age / rules.fall_s).powi(2)
    } else {
        let u = if rules.settle_s > 0.0 {
            (age - rules.fall_s) / rules.settle_s
        } else {
            1.0
        };
        if u >= 1.0 {
            rest
        } else {
            rest - rules.settle_deg * std::f64::consts::PI / 180.0
                * libm::sin(std::f64::consts::PI * u)
                * (1.0 - u)
        }
    };
    let u = ((angle / rest - 0.5) / 0.5).clamp(0.0, 1.0);
    let landed = u * u * (3.0 - 2.0 * u);
    let axis = trunk * (1.0 - ((crown - hinge) / length).clamp(0.0, 1.0));
    let squash = ((rules.rest_height_m - axis) / reach.max(1e-3)).clamp(0.02, 1.0);
    [
        angle,
        hinge,
        1.0 + (squash - 1.0) * landed,
        1.0 + (rules.rest_spread - 1.0) * landed,
        trunk,
        landed,
        crown,
    ]
}
