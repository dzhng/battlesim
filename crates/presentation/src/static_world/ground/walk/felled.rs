//! Falling-tree records and animation shared by both renderers.
use super::*;
use std::f64::consts::FRAC_PI_2;

/// A tree the side saw fall: its trunk's prop, the way it fell, and the
/// presentation second it began to.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct FelledTree {
    pub prop: u32,
    pub toward: [f64; 2],
    pub fell_at: f64,
}

impl SceneryWalk {
    pub(super) fn select_felled(&mut self, ground: &Ground, view: &View, lod: &[f64; 3]) {
        // What the last walk listed: a view that moves a tree to another
        // tier writes its lists anew.
        let before = std::mem::take(&mut self.felled_lists);
        self.felled_lists = vec![vec![]; before.len()];
        self.stumps.iter_mut().for_each(Vec::clear);
        self.falling = false;
        let rules = &ground.biome.trees.felled;
        let forest = self.scenery.forest.as_chunks::<FLOATS>().0;
        for f in &self.felled {
            let Some(&i) = self.index.get(&f.prop) else {
                continue;
            };
            let p = &forest[i];
            let (tree, kind, h, reach) = instance(p, &self.sizes);
            let (sxy, sz) = (p[4] as f64, p[5] as f64);
            let age = self.clock - f.fell_at;
            self.falling |= age < rules.fall_s + rules.settle_s;
            let t = &self.trunks[kind];
            let pose = fell_pose(
                rules,
                age,
                h as f64,
                reach as f64,
                t.radius * sxy,
                t.crown * sz,
            );
            let mut fall = tree;
            fall[4..8]
                .copy_from_slice(&[f.toward[0], f.toward[1], pose[0], pose[1]].map(|v| v as f32));
            let mut rest = [0.0; 16];
            for (r, v) in rest.iter_mut().zip(&pose[2..]) {
                *r = *v as f32;
            }
            let d = crate::world::length([0, 1, 2].map(|a| tree[a] as f64 - view.eye[a]));
            let list = &mut self.felled_lists[kind * TIERS + tier_for(h as f64, d, view, lod)];
            list.extend([fall, rest]);
            let mut stump = tree;
            stump[11] = 0.0;
            stump[14..16].copy_from_slice(&[1.0, (0.5 * rules.stump_height_m) as f32]);
            self.stumps[kind].push(stump);
        }
        // Casters draw a tier coarser.
        let kinds = self.scenery.kinds.len();
        for k in 0..kinds {
            for t in 0..TIERS {
                let view_list = self.felled_lists[k * TIERS + t].clone();
                self.felled_lists[kinds * TIERS + k * TIERS + caster_tier(t)].extend(view_list);
            }
        }
        if self.felled_lists != before {
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
