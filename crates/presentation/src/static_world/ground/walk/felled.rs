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
    pub(super) dirty: bool,
    index: BoundsIndex,
    ids: Vec<usize>,
    latest: Option<f64>,
    candidates: Vec<usize>,
    pub(super) visited: usize,
}

impl SceneryWalk {
    fn rebuild_felled(&mut self, rules: &FelledRules) {
        let spatial = &mut self.felled_spatial;
        spatial.ids.clear();
        spatial.latest = None;
        let forest = self.scenery.forest.as_chunks::<FLOATS>().0;
        let mut bounds = Vec::new();
        for (id, f) in self.felled.iter().enumerate() {
            let Some(&i) = self.index.get(&f.prop) else {
                continue;
            };
            let p = &forest[i];
            let (_, kind, h, reach) = instance(p, &self.sizes);
            let trunk = self.trunks[kind].radius * p[4] as f64;
            let hinge = rules.stump_height_m;
            // The shader rotates the height above the hinge, compresses radial
            // components (never expands them), and kicks the trunk two radii.
            // Sum the two radial components rather than assuming compression is
            // a rigid rotation. This also encloses the unscaled-height stump.
            let span = (h as f64 - hinge).max(0.) + 2. * reach as f64;
            let xy = span + 2. * trunk;
            let z = span + hinge.max(trunk);
            let [x, y, base] = [p[0], p[1], p[2]].map(f64::from);
            bounds.push([x - xy, y - xy, base - z, x + xy, y + xy, base + z]);
            spatial.ids.push(id);
            spatial.latest = Some(spatial.latest.map_or(f.fell_at, |v| v.max(f.fell_at)));
        }
        spatial.index = BoundsIndex::new(bounds);
        spatial.dirty = false;
    }

    pub(super) fn select_felled(
        &mut self,
        ground: &Ground,
        view: &View,
        shadow: Option<&CasterVolumes>,
        lod: &[f64; 3],
    ) {
        let rules = &ground.biome.trees.felled;
        if self.felled_spatial.dirty {
            self.rebuild_felled(rules);
        }
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
        for pass in 0..=CASCADES {
            spatial.candidates.clear();
            spatial.visited += spatial.index.query(
                |b| {
                    if pass == 0 {
                        view.sees_box(b)
                    } else {
                        shadow.is_some_and(|s| s.mask(b) & (1 << (pass - 1)) != 0)
                    }
                },
                |i| spatial.candidates.push(i),
            );
            spatial.candidates.sort_unstable();
            for &id in &spatial.candidates {
                let f = &self.felled[spatial.ids[id]];
                let p = &forest[self.index[&f.prop]];
                let (mut stump, kind, _, _) = instance(p, &self.sizes);
                stump[11] = 0.;
                stump[14..16].copy_from_slice(&[1., (0.5 * rules.stump_height_m) as f32]);
                self.stumps[pass * kinds + kind].push(stump);
            }
            // Casters merge source tiers into coarser tiers in source-tier order.
            // Preserve that order, including the two tiers that share LAST.
            let tier = |id: usize| {
                let f = &self.felled[spatial.ids[id]];
                let p = &forest[self.index[&f.prop]];
                let (_, _, h, _) = instance(p, &self.sizes);
                let d = crate::world::length([0, 1, 2].map(|a| p[a] as f64 - view.eye[a]));
                tier_for(h as f64, d, view, lod)
            };
            if pass != 0 {
                spatial.candidates.sort_by_cached_key(|&id| (tier(id), id));
            }
            for &id in &spatial.candidates {
                let f = &self.felled[spatial.ids[id]];
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
