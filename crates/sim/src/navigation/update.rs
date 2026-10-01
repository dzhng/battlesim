//! A side's grid follows what the side learns: each body it comes to place
//! differently is taken off the cells it lay on and laid where the side now
//! believes it stands, and only those cells (and the clearance tiles, the
//! rectangles and the counts that read them) are worked out again. The
//! result is the grid a whole build over the same knowledge gives.
use std::sync::Arc;

use super::base::{body_bucket, body_buckets, settle, Body, Stamp};
use super::cells::{cell_center, cell_of, Cell, NO_BODY};
use super::{NavBase, NavGrid};
use crate::math::{v2, V2};
use crate::world::{Prop, PropId, WorldGeometry};

impl NavGrid {
    /// A side's grid that knows the map as `base` has it, and nothing else.
    pub fn new(base: Arc<NavBase>) -> Self {
        NavGrid {
            nx: base.cells.nx,
            ny: base.cells.ny,
            cells: base.cells.clone(),
            laid: Default::default(),
            laid_in: Default::default(),
            clearance: Default::default(),
            stopping: base.stopping,
            regions: Default::default(),
            slow: base.slow.clone(),
            knowledge: 0,
            relaid: 0,
            work: std::cell::Cell::new(0),
            base,
        }
    }

    /// Take in what the side has learned. `beliefs` names every body it may
    /// now place differently, with the prop as it believes it stands (or
    /// `None`: it plans without it). `cleared` gives the middle of each
    /// patch of forest ground cleared since the grid last looked; `world`
    /// says which ground is cleared now.
    pub fn update(
        &mut self,
        world: &WorldGeometry,
        beliefs: impl Iterator<Item = (PropId, Option<Prop>)>,
        cleared: impl Iterator<Item = V2>,
    ) {
        self.knowledge += 1;
        let base = Arc::clone(&self.base);
        let (nx, ny, r) = (self.nx, self.ny, base.soldier_radius);
        // The cells whose bodies changed, by body bucket.
        let mut relaid: Vec<(usize, usize)> = Vec::new();
        for (id, belief) in beliefs {
            let (old, new) = (self.body(id), belief.as_ref().and_then(Body::of));
            if old == new {
                continue;
            }
            for (body, lays) in [(old, false), (new, true)] {
                let Some(body) = body else { continue };
                let stamp = body.stamp(r, nx, ny);
                relaid.extend(
                    stamp
                        .cells()
                        .map(|(i, j)| (body_bucket(i, j, nx), j * nx + i)),
                );
                let rect = body.rect(r);
                if lays {
                    self.regions.add(&base.regions, rect);
                } else {
                    self.regions.remove(&base.regions, rect);
                }
                if let Some(rank) = body.stopping() {
                    self.stopping[rank] = self.stopping[rank] + u32::from(lays) - u32::from(!lays);
                }
            }
            // The body as the grid holds it for itself: one the map did not
            // lay just so.
            let own = (new != base.body(id)).then_some(new);
            let before = match own {
                Some(body) => self.laid.insert(id, body),
                None => self.laid.remove(&id),
            };
            for (body, lists) in [(before.flatten(), false), (own.flatten(), true)] {
                let Some(body) = body else { continue };
                for bucket in body_buckets(&body.stamp(r, nx, ny), nx) {
                    let held = self.laid_in.entry(bucket).or_default();
                    if lists {
                        held.push(id);
                        continue;
                    }
                    held.retain(|other| *other != id);
                    if held.is_empty() {
                        self.laid_in.remove(&bucket);
                    }
                }
            }
        }
        // Cleared ground is forest no longer, wherever a cell's centre lies
        // on it.
        let patch_m = world.cleared_cell_m();
        for p in cleared {
            let (i0, j0) = cell_of(p - v2(patch_m, patch_m));
            let (i1, j1) = cell_of(p + v2(patch_m, patch_m));
            for j in j0.max(0)..=j1.min(ny as isize - 1) {
                for i in i0.max(0)..=i1.min(nx as isize - 1) {
                    let at = j as usize * nx + i as usize;
                    let c = cell_center(i as usize, j as usize);
                    let old = self.cells[at];
                    if old.forest && world.cleared(c.x, c.y) {
                        let open = Cell {
                            forest: false,
                            ..old
                        };
                        self.slow.replace(at, nx, &old, &open);
                        self.cells.set(at, open);
                        self.relaid += 1;
                    }
                }
            }
        }
        relaid.sort_unstable();
        relaid.dedup();
        let mut stamps: Vec<Stamp> = Vec::new();
        let mut bucket = None;
        for &(k, at) in &relaid {
            if bucket != Some(k) {
                bucket = Some(k);
                stamps.clear();
                let map = base
                    .bodies_in(k)
                    .filter(|(id, _)| !self.laid.contains_key(id))
                    .map(|(_, body)| body);
                let own = self
                    .laid_in
                    .get(&k)
                    .into_iter()
                    .flatten()
                    .filter_map(|id| self.laid[id].as_ref());
                stamps.extend(map.chain(own).map(|body| body.stamp(r, nx, ny)));
            }
            let (i, j) = (at % nx, at / nx);
            let old = self.cells[at];
            let mut cell = Cell {
                heaviest: NO_BODY,
                free: base.bare(at, old.ground),
                ..old
            };
            for stamp in stamps.iter().filter(|stamp| stamp.reaches(i, j)) {
                stamp.lay(&mut cell, i, j);
            }
            if cell.heaviest != old.heaviest {
                self.forget_clearance(at, old.heaviest, cell.heaviest);
            }
            self.cells.set(at, cell);
        }
        let relaid: Vec<usize> = relaid.into_iter().map(|(_, at)| at).collect();
        self.relaid += relaid.len() as u64;
        settle(&mut self.cells, &relaid);
    }

    /// The body this grid has laid for prop `id`.
    fn body(&self, id: PropId) -> Option<Body> {
        match self.laid.get(&id) {
            Some(own) => *own,
            None => self.base.body(id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::navigation::regions::Regions;
    use crate::navigation::{Mobility, Mover, PUSH_CLASSES};
    use crate::rng::Rng;
    use contract::map::{MapDefinition, MoverClass, PropDefinition};
    use contract::scenario::{PushClass, Rules};

    const SOLDIER_M: f64 = 0.3;

    /// A small map with everything a grid reads: a hill, water under a
    /// bridge, a road, a wood, a building and loose bodies of every weight.
    fn world(rules: &Rules) -> WorldGeometry {
        let map: MapDefinition = serde_json::from_value(serde_json::json!({
            "size": [320, 256], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
            "relief": [{ "kind": "ridge", "center": [250, 200], "radius_m": 60, "peak_m": 30 }],
            "water": [{ "rect": [150, 0, 12, 256], "bed_z": -2, "surface_z": -0.5 }],
            "bridges": [{ "deck": "bridge_deck", "center": [156, 100], "half_extents": [12, 4],
                "yaw": 0, "deck_z": 0.1, "thickness_m": 0.8 }],
            "surfaces": [{ "kind": "road", "shape": { "kind": "stroke",
                "points": [[4, 100], [150, 100], [300, 60]], "width_m": 8 } }],
            "forests": [{ "shape": { "kind": "polygon",
                "ring": [[20, 120], [130, 120], [130, 230], [20, 230]] } }],
            "props": [
                { "kind": "ruin", "center": [60, 60], "yaw": 0.3, "half_extents": [6, 5, 3] },
                { "kind": "wall", "center": [220, 120], "yaw": 1.1, "half_extents": [0.4, 14, 1] },
                { "kind": "crate", "center": [100, 90], "yaw": 0.2, "half_extents": [1, 1, 1] },
                { "kind": "tooth", "center": [200, 96], "yaw": 0, "half_extents": [0.6, 0.6, 0.6] }
            ]
        }))
        .expect("a map");
        WorldGeometry::new(&map, rules)
    }

    /// What a side of this test believes: the bodies it places somewhere
    /// (learned, or the map's where it last saw them), and the map's bodies
    /// it believes gone.
    #[derive(Default)]
    struct Belief {
        placed: std::collections::BTreeMap<PropId, Prop>,
        gone: std::collections::BTreeSet<PropId>,
    }

    impl Belief {
        fn of(&self, world: &WorldGeometry, authored: PropId, id: PropId) -> Option<Prop> {
            if self.gone.contains(&id) {
                return None;
            }
            self.placed
                .get(&id)
                .cloned()
                .or_else(|| world.prop(id).filter(|p| p.id < authored).cloned())
        }
    }

    /// Every field of two grids over the same knowledge agrees, and so does
    /// every answer a search reads from them.
    fn assert_same(step: &str, kept: &NavGrid, fresh: &NavGrid) {
        assert_eq!((kept.nx, kept.ny), (fresh.nx, fresh.ny));
        for at in 0..kept.nx * kept.ny {
            assert_eq!(kept.cells[at], fresh.cells[at], "{step}: cell {at}");
        }
        assert_eq!(kept.stopping, fresh.stopping, "{step}: stopping ranks");
        assert_eq!(kept.slow.forest, fresh.slow.forest, "{step}: forest tiles");
        assert_eq!(kept.slow.steep, fresh.slow.steep, "{step}: steep tiles");
        for bucket in 0..Regions::buckets(&kept.base.regions) {
            assert!(
                kept.regions.seen_in(&kept.base.regions, bucket)
                    == fresh.regions.seen_in(&fresh.base.regions, bucket),
                "{step}: rectangles of bucket {bucket}"
            );
        }
        for push in PushClass::ALL {
            let vehicle = Mobility {
                off_road_mps: 6.0,
                road_mps: 12.0,
                forest_multiplier: 0.4,
                half_width_m: 1.8,
                class: MoverClass::Vehicle,
                push,
                drive: None,
            };
            let squad = Mobility {
                class: MoverClass::Infantry,
                half_width_m: 0.5,
                ..vehicle
            };
            for at in 0..kept.nx * kept.ny {
                assert_eq!(
                    kept.clearance_at(push, at),
                    fresh.clearance_at(push, at),
                    "{step}: {push:?} clearance at cell {at}"
                );
                for m in [&vehicle, &squad] {
                    assert_eq!(
                        kept.fits(at, Mover::free(m)),
                        fresh.fits(at, Mover::free(m)),
                        "{step}: {:?} {push:?} fits cell {at}",
                        m.class
                    );
                    assert_eq!(
                        kept.uniform_stencil(at, Mover::free(m)),
                        fresh.uniform_stencil(at, Mover::free(m)),
                        "{step}: {:?} {push:?} open ground round cell {at}",
                        m.class
                    );
                }
            }
        }
    }

    #[test]
    fn a_grid_updated_body_by_body_is_the_grid_built_whole_from_the_same_knowledge() {
        let rules: Rules = serde_json::from_value(crate::fixtures::village()).expect("rules");
        let mut world = world(&rules);
        let authored = world.props().count() as PropId;
        let base = Arc::new(NavBase::build(&world, world.props(), SOLDIER_M));
        let mut kept = NavGrid::new(base);
        let mut belief = Belief::default();
        let mut rng = Rng::new(7);
        let mut taken = 0;
        // Read every clearance tile before any change, so a tile left
        // standing when a body under it changes shows.
        assert_same("built", &kept, &NavGrid::new(Arc::clone(&kept.base)));
        let kinds = [
            "crate",
            "tooth",
            "light_wreck",
            "medium_wreck",
            "heavy_wreck",
            "wall",
        ];
        for step in 0..160 {
            let mut draw = |n: usize| (rng.unit() * n as f64) as usize % n;
            let ids: Vec<PropId> = (0..world.obstacle_revision() as PropId + authored)
                .filter(|&id| belief.of(&world, authored, id).is_some())
                .collect();
            let known = ids[draw(ids.len())];
            let at = [4.0 + draw(312) as f64 + 0.37, 4.0 + draw(248) as f64 + 0.61];
            let mut touched = vec![known];
            let what = match draw(5) {
                // A body appears and the side learns of it.
                0 => {
                    let half = [0.5 + draw(4) as f64, 0.5 + draw(3) as f64, 1.0];
                    let id = world.add_prop(&PropDefinition {
                        kind: kinds[draw(kinds.len())].into(),
                        center: at,
                        yaw: draw(7) as f64 * 0.45,
                        half_extents: half,
                        base_z: None,
                    });
                    belief.placed.insert(id, world.prop(id).cloned().unwrap());
                    touched.push(id);
                    "appears"
                }
                // A body the side knows is gone, and the side sees it go.
                1 => {
                    belief.placed.remove(&known);
                    belief.gone.insert(known);
                    "removed"
                }
                // A body is shoved and the side sees where it came to rest.
                2 => {
                    let mut moved = belief.of(&world, authored, known).unwrap();
                    moved.center = v2(at[0], at[1]);
                    moved.yaw += 0.3;
                    belief.placed.insert(known, moved);
                    "moved"
                }
                // A tree falls to a shell: its ground is cleared and the
                // side sees it gone.
                3 => {
                    let tree = world
                        .props()
                        .filter(|p| p.forest_tree && !belief.gone.contains(&p.id))
                        .map(|p| (p.id, p.center))
                        .nth(draw(40));
                    if let Some((id, center)) = tree {
                        world.knock_down(id);
                        world.clear_spot(center, rules.forests.rule.trunk_spacing_m);
                        belief.gone.insert(id);
                        touched.push(id);
                    }
                    "destroyed"
                }
                // A body the side thought gone turns out to stand after all.
                _ => {
                    if let Some(&id) = belief.gone.iter().nth(draw(belief.gone.len().max(1))) {
                        if let Some(stands) = world.prop(id).cloned() {
                            belief.gone.remove(&id);
                            belief.placed.insert(id, stands);
                            touched.push(id);
                        }
                    }
                    "relearned"
                }
            };
            let beliefs: Vec<_> = touched
                .iter()
                .map(|&id| (id, belief.of(&world, authored, id)))
                .collect();
            let cleared: Vec<V2> = world.cleared_since(taken).collect();
            taken += cleared.len();
            kept.update(&world, beliefs.into_iter(), cleared.into_iter());

            let known: Vec<Prop> = (0..world.obstacle_revision() as PropId + authored)
                .filter_map(|id| belief.of(&world, authored, id))
                .collect();
            let fresh = NavGrid::new(Arc::new(NavBase::build(&world, known.iter(), SOLDIER_M)));
            assert_same(&format!("step {step} ({what})"), &kept, &fresh);
        }
        assert!(PUSH_CLASSES > 1 && !belief.gone.is_empty() && taken > 0);
    }
}
