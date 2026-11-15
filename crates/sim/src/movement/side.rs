//! What one side may plan with ([`SideGeometry`]): the authored map plus
//! the bodies its units have run into or seen moved, each where the side
//! last saw it, and the planning grid built from that knowledge.
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;

use crate::math::V2;
use crate::navigation::{NavBase, NavGrid};
use crate::world::{Prop, PropId, PropIndex, WorldGeometry, PROP_BUCKET_M};

use super::changes::{span, Change};

/// Where a side last saw a body stand (L1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Seen {
    pub center: V2,
    pub yaw: f64,
    pub base_z: f64,
}

/// What one side may plan with: the authored map plus the dynamic obstacles
/// its units have encountered, each where the side last saw it. Hidden
/// changes never reach this.
#[derive(Clone)]
pub struct SideGeometry {
    /// Every body this side places somewhere other than where it truly
    /// stands, or learned after setup: each learned body where it was last
    /// seen, and each authored body shoved out of sight where it stood
    /// before (L1). Any other authored body stands where it is.
    pub seen: BTreeMap<PropId, Seen>,
    /// Bodies gone out of this side's sight (trees knocked down, props
    /// destroyed): it plans around them, and draws them, until it sees the
    /// ground where they stood (Q16, Q17, L1).
    standing: BTreeMap<PropId, Prop>,
    /// The same remembered snapshots, indexed for sight queries.
    standing_index: PropIndex,
    pub revision: u64,
    /// Where each of the latest revisions changed the side's plan, oldest
    /// first, so a squad re-resolves only for changes within its reach.
    pub(super) changes: VecDeque<Change>,
    /// The side's planning grid as of revision `grid_revision`: the map's
    /// grid, with what the side had learned by then.
    grid: NavGrid,
    grid_revision: u64,
    /// The bodies the side may place otherwise than its grid has them.
    stale: BTreeSet<PropId>,
    /// How many of the world's cleared ground cells the grid has taken in.
    cleared_taken: usize,
    /// Route searches run for this side (the no-per-frame-search contract).
    pub searches: u64,
    /// Where this side's aircraft may fly, and the side and world revisions
    /// it was built at; built when an aircraft first asks.
    air: Option<(u64, u64, Arc<crate::navigation::air::AirGrid>)>,
}

/// A change farther than this from a unit's route cannot put a body in
/// its way: the widest clearance a footprint is judged by, and a cell more.
pub(super) const ROUTE_RECHECK_REACH_M: f64 = 24.0;

impl SideGeometry {
    /// The scratch world owns a fresh clearance journal; its cursor starts there.
    pub(crate) fn planning_snapshot(&self) -> Self {
        let mut snapshot = self.clone();
        snapshot.cleared_taken = 0;
        snapshot
    }

    /// A side that knows the map as `base` has it, and nothing else.
    pub fn new(base: Arc<NavBase>) -> Self {
        let [width, depth] = base.extent();
        SideGeometry {
            seen: BTreeMap::new(),
            standing: BTreeMap::new(),
            standing_index: PropIndex::new(width, depth, PROP_BUCKET_M),
            revision: 0,
            changes: VecDeque::new(),
            grid: NavGrid::new(base),
            grid_revision: 0,
            stale: BTreeSet::new(),
            cleared_taken: 0,
            searches: 0,
            air: None,
        }
    }

    /// The side's air grid: walls are the bodies it believes stand taller
    /// than the obstacle height, where it believes they stand (D32).
    pub fn air_grid(
        &mut self,
        world: &WorldGeometry,
        rules: &contract::scenario::Rules,
        authored: PropId,
    ) -> Arc<crate::navigation::air::AirGrid> {
        use crate::navigation::air::{walls_aircraft, AirGrid};
        let stamp = (self.revision, world.obstacle_revision());
        if let Some((side, at, grid)) = &self.air {
            if (*side, *at) == stamp {
                return grid.clone();
            }
        }
        let walls: Vec<Prop> = world
            .props()
            .chain(self.standing.values())
            .filter(|p| walls_aircraft(world, rules, p))
            .filter_map(|p| self.belief(p, authored))
            .filter(|p| walls_aircraft(world, rules, p))
            .collect();
        let grid = Arc::new(AirGrid::build(world, walls.iter()));
        self.air = Some((stamp.0, stamp.1, grid.clone()));
        grid
    }

    /// Cells the side's grid has worked out again as the side learned.
    pub fn cells_relaid(&self) -> u64 {
        self.grid.relaid()
    }

    /// The world changed these props (added, moved, removed or made known
    /// to all): what the side believes of each may have changed with it.
    pub fn touch(&mut self, props: &[PropId]) {
        self.stale.extend(props);
    }

    /// The side sees or touches `prop` where it stands now. A body new to it
    /// is learned; one it knows elsewhere is re-learned once it has moved
    /// more than `relearn_m` from where it was seen, or has come to rest
    /// (`resting`), so a body being shoved is not re-learned every tick
    /// (Q13). Each learning bumps the side's revision (L2).
    pub fn learn(&mut self, prop: &Prop, authored: PropId, relearn_m: f64, resting: bool) {
        let now = Seen {
            center: prop.center,
            yaw: prop.yaw,
            base_z: prop.base_z,
        };
        match self.seen.get(&prop.id) {
            None if prop.id < authored => return, // it stands where the map put it
            None => {}
            Some(was) if *was == now => return,
            Some(was) if !resting && (was.center - now.center).within_radius(relearn_m) => return,
            Some(_) => {}
        }
        let was = self.seen.insert(prop.id, now).map(|w| w.center);
        self.stale.insert(prop.id);
        self.changed(prop.id, span(prop, was));
    }

    /// `prop` is about to be shoved: an authored body the side has not seen
    /// move stays, for it, where it stands now (L1).
    pub fn before_move(&mut self, prop: &Prop, authored: PropId) {
        if prop.id < authored {
            self.seen.entry(prop.id).or_insert(Seen {
                center: prop.center,
                yaw: prop.yaw,
                base_z: prop.base_z,
            });
            self.stale.insert(prop.id);
        }
    }

    /// A body this side did not see go: it keeps planning around it.
    pub fn keep_standing(&mut self, prop: Prop) {
        self.stale.insert(prop.id);
        if let Some(was) = self.standing.remove(&prop.id) {
            self.standing_index.remove(&was);
        }
        self.standing_index.insert(&prop);
        self.standing.insert(prop.id, prop);
    }

    /// Read-only remembered snapshots; mutations keep the sight index in sync.
    pub(crate) fn standing(&self) -> &BTreeMap<PropId, Prop> {
        &self.standing
    }

    /// Ascending unique remembered candidates across all visibility views.
    pub(crate) fn standing_ids_near_many(&self, views: &[(V2, f64)], out: &mut Vec<PropId>) {
        self.standing_index.near_many(views, out);
    }

    /// The side sees that a body it kept standing is gone.
    pub fn saw_fallen(&mut self, prop: PropId) {
        if let Some(gone) = self.standing.remove(&prop) {
            self.standing_index.remove(&gone);
            let was = self.seen.remove(&prop).map(|s| s.center);
            self.stale.insert(prop);
            self.changed(prop, span(&gone, was));
        }
    }

    /// A prop this side plans with is gone: plan again without it.
    pub fn forget(&mut self, prop: &Prop) {
        let was = self.seen.remove(&prop.id).map(|s| s.center);
        self.stale.insert(prop.id);
        self.changed(prop.id, span(prop, was));
    }

    /// Whether this side plans with `prop`: authored with the map, remains
    /// every side plans with ([`Prop::known_to_all`]), or learned.
    pub fn knows(&self, prop: &Prop, authored: PropId) -> bool {
        prop.id < authored || prop.known_to_all || self.seen.contains_key(&prop.id)
    }

    /// `prop` as this side believes it stands, if it knows it at all.
    pub fn belief(&self, prop: &Prop, authored: PropId) -> Option<Prop> {
        if !self.knows(prop, authored) {
            return None;
        }
        let mut p = prop.clone();
        if let Some(seen) = self.seen.get(&prop.id) {
            p.center = seen.center;
            p.yaw = seen.yaw;
            p.base_z = seen.base_z;
        }
        Some(p)
    }

    /// Where this side plans otherwise than `world` stands: each body added
    /// since setup it does not know (none), each it places elsewhere, and
    /// each gone that it keeps standing. Every other body of `world` it plans
    /// with as it stands, so this costs what the side has learned, not the
    /// map's size.
    pub(crate) fn beliefs<'a>(
        &'a self,
        world: &'a WorldGeometry,
        authored: PropId,
    ) -> impl Iterator<Item = (PropId, Option<Prop>)> + 'a {
        let unknown = world
            .props_from(authored)
            .filter(move |p| !self.knows(p, authored))
            .map(|p| (p.id, None));
        let placed = self
            .seen
            .keys()
            .filter_map(move |&id| Some((id, self.belief(world.prop(id)?, authored))));
        let standing = self.standing.values().map(|p| (p.id, Some(p.clone())));
        unknown.chain(placed).chain(standing)
    }

    /// The world as this side plans in it ([`Self::beliefs`]).
    pub(crate) fn planning_world(&self, world: &WorldGeometry, authored: PropId) -> WorldGeometry {
        world.planning_snapshot(self.beliefs(world, authored))
    }

    /// A body where this side knows it, including an unseen collapse.
    pub fn prop(&self, world: &WorldGeometry, authored: PropId, id: PropId) -> Option<Prop> {
        self.belief(world.prop(id).or_else(|| self.standing.get(&id))?, authored)
    }

    /// The side's planning grid. It refreshes changed body beliefs and
    /// newly cleared ground, which advance independently.
    pub fn grid(&mut self, world: &WorldGeometry, authored: PropId) -> &NavGrid {
        // Body beliefs and cleared ground advance independently.
        if self.grid_revision != self.revision
            || self.cleared_taken != world.cleared_cells() as usize
        {
            self.stale.extend(world.touched());
            let beliefs: Vec<(PropId, Option<Prop>)> = std::mem::take(&mut self.stale)
                .into_iter()
                .map(|id| (id, self.prop(world, authored, id)))
                .collect();
            let cleared = world.cleared_since(self.cleared_taken);
            self.cleared_taken = world.cleared_cells() as usize;
            self.grid.update(world, beliefs.into_iter(), cleared);
            self.grid_revision = self.revision;
        }
        &self.grid
    }

    /// The side's grid as it stands once it has taken in everything the
    /// side believes now, beside a grid built whole from those beliefs.
    #[cfg(test)]
    pub(crate) fn grid_beside_whole_build(
        &mut self,
        world: &WorldGeometry,
        authored: PropId,
        soldier_radius: f64,
    ) -> (&NavGrid, NavGrid) {
        let believed: Vec<Prop> = world
            .props()
            .chain(self.standing.values())
            .filter_map(|p| self.belief(p, authored))
            .collect();
        let whole = NavBase::build(world, believed.iter(), soldier_radius);
        (self.grid(world, authored), NavGrid::new(Arc::new(whole)))
    }

    /// Fold the side's planning knowledge into a digest.
    pub fn digest(&self, d: &mut crate::digest::Digest) {
        d.u64(self.revision).u64(self.seen.len() as u64);
        for (id, s) in &self.seen {
            d.u64(*id as u64)
                .f64(s.center.x)
                .f64(s.center.y)
                .f64(s.yaw)
                .f64(s.base_z);
        }
        d.u64(self.standing.len() as u64);
        for id in self.standing.keys() {
            d.u64(*id as u64);
        }
        d.u64(self.changes.len() as u64);
        for c in &self.changes {
            c.digest(d);
        }
    }
}

#[cfg(test)]
mod remembered_sight_tests {
    use super::*;
    use crate::math::v2;

    #[test]
    fn remembered_body_queries_follow_replacements_and_observed_removals() {
        let rules = serde_json::from_value(crate::fixtures::test_rules()).unwrap();
        let map = serde_json::from_value(serde_json::json!({
            "size":[256,256],"fog_cell_m":8,"height_grid_m":4,
            "slope_cutoff_deg":35,"props":[{
                "kind":"crate","center":[20,20],"yaw":0,
                "half_extents":[0.8,0.8,0.6]
            }]
        }))
        .unwrap();
        let world = WorldGeometry::new(&map, &rules);
        let base = NavBase::build(&world, world.props(), 0.3);
        let mut side = SideGeometry::new(Arc::new(base));
        let mut prop = world.prop(0).unwrap().clone();
        let at = v2(20.0, 20.0);
        side.keep_standing(prop.clone());
        let mut nearby = Vec::new();
        side.standing_ids_near_many(&[(at, 2.0)], &mut nearby);
        assert_eq!(nearby, vec![0]);
        prop.center = v2(200.0, 200.0);
        side.keep_standing(prop);
        nearby.clear();
        side.standing_ids_near_many(&[(at, 2.0)], &mut nearby);
        assert!(nearby.is_empty(), "replaced memory has no old footprint");
        side.standing_ids_near_many(&[(v2(200.0, 200.0), 2.0)], &mut nearby);
        assert_eq!(nearby, vec![0]);
        side.saw_fallen(0);
        nearby.clear();
        side.standing_ids_near_many(&[(v2(200.0, 200.0), 2.0)], &mut nearby);
        assert!(nearby.is_empty(), "removals leave no remembered body");
    }
}
