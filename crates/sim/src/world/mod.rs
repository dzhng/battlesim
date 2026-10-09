//! Authoritative world geometry: bounded ground triangles, water, roads,
//! bridges, forest volumes and solid props. Collision, sight, routing and the
//! renderer all read these same surfaces.
mod buildings;
mod carve;
pub mod export;
mod forest;
mod props;
mod surfaces;
mod terrain;

pub use forest::Foliage;
use props::PropStore;
pub(crate) use props::{ray_box, PropIndex};
pub use props::{Prop, PropId, Slot};
use terrain::HeightField;

use crate::math::{v2, v3, Obb2, V2, V3};
use contract::catalog::{PropBody, PropCatalog, PropKind, PropPlacement, PropType};
use contract::map::{Bridge, Forest, MapDefinition, PropDefinition};
use contract::river::River;
use contract::scenario::Rules;
use std::sync::Arc;

/// The prop index's bucket. Line tests measured this against 8, 16 and 64 m
/// buckets (27 perf): 32 and 64 tie, finer is dearer.
pub(crate) const PROP_BUCKET_M: f64 = 32.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceKind {
    Ground,
    Road,
    Water,
    Bridge,
    Paving,
}

impl SurfaceKind {
    /// Every carriageway is a road here; its kind only sets its speed
    /// (`Surface::road_factor`).
    pub fn of(kind: contract::map::SurfaceKind) -> Self {
        if kind.is_road() {
            SurfaceKind::Road
        } else {
            SurfaceKind::Paving
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Surface {
    pub z: f64,
    pub normal: V3,
    pub slope_deg: f64,
    pub kind: SurfaceKind,
    /// The surface kind's `speed_factor` on a road's share of road speed
    /// (the rules' `surfaces` table): 0 where the ground is no road.
    pub road_factor: f64,
    /// Forest ground, not cleared (Q16): forest speed applies.
    pub forest: bool,
    /// Ground units may stand here: not water, and below the shared slope cutoff.
    /// Solid props are separate obstacles (see `props_near`).
    pub traversable: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Collider {
    Terrain,
    Prop(PropId),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    pub t: f64,
    pub point: V3,
    pub normal: V3,
    pub collider: Collider,
}

#[derive(Clone)]
pub struct WorldGeometry {
    field: Arc<HeightField>,
    /// The land's authored relief, before any river is carved into it.
    relief: Vec<contract::map::Relief>,
    slope_cutoff_deg: f64,
    /// The paving and the rivers: what the ground is at a point.
    surfaces: Arc<surfaces::SurfaceIndex>,
    /// Each surface kind's speed factor, by `contract::map::SurfaceKind` order.
    surface_factors: [f64; contract::map::SurfaceKind::ALL.len()],
    bridges: Vec<Bridge>,
    /// Authoritative authored shapes; generated trunks and cleared ground are runtime state.
    forests: Vec<Forest>,
    /// The forests at runtime: foliage per fog cell and the cleared mask.
    forest: forest::ForestState,
    /// Every body and the bucket index over them; a planning snapshot
    /// shares the world's and holds only what its side believes otherwise.
    props: PropStore,
    buildings: Arc<buildings::Buildings>,
    template_catalog_hash: Option<String>,
    /// The map's region (`MapDefinition.regional_family`), for presentation.
    regional_family: Option<String>,
    authored_props: PropId,
    /// Each authored prop's pose as built, bit for bit, and their digest: a
    /// prop still in its pose is held by that one number.
    authored_poses: std::sync::Arc<[[u64; 4]]>,
    authored_digest: u64,
    authored_sources: std::collections::BTreeMap<PropId, PropId>,
    revision: u64,
    /// The prop types: each new prop takes its type's body row.
    types: PropCatalog,
    /// The unit types a wreck may name ([`Prop::wreck_of`]): those with a hull.
    wreck_units: Arc<std::collections::BTreeMap<String, contract::catalog::TypeIndex>>,
    /// The last tick each shoved prop moved: one not shoved last tick or this
    /// one has come to rest.
    moved: std::collections::BTreeMap<PropId, u64>,
    /// Props added, moved, removed or made known to all since
    /// [`Self::take_touched`] last emptied it.
    touched: Vec<PropId>,
    /// The least room a line of sight needs between the bodies it squeezes
    /// past, on its two sides together (`sensors.min_sight_gap_m`).
    min_sight_gap_m: f64,
}

impl WorldGeometry {
    /// Physical movement against one side's remembered bodies: this world's
    /// bodies, except each of `beliefs` (the body the side plans with in
    /// that id's place, or none; later entries win). Every body not listed
    /// must be one the side plans with where it stands. Bodies, terrain and
    /// paving are shared; mutations affect only this scratch world. Forest
    /// ground begins uncleared so unseen clearing cannot certify a move.
    pub(crate) fn planning_snapshot(
        &self,
        beliefs: impl Iterator<Item = (PropId, Option<Prop>)>,
    ) -> Self {
        let mut snapshot = Self {
            field: Arc::clone(&self.field),
            relief: self.relief.clone(),
            slope_cutoff_deg: self.slope_cutoff_deg,
            surfaces: Arc::clone(&self.surfaces),
            surface_factors: self.surface_factors,
            bridges: self.bridges.clone(),
            forests: self.forests.clone(),
            forest: self.forest.clone(),
            props: self.props.planning_layer(beliefs),
            buildings: Arc::clone(&self.buildings),
            template_catalog_hash: self.template_catalog_hash.clone(),
            regional_family: self.regional_family.clone(),
            authored_props: self.authored_props,
            authored_poses: self.authored_poses.clone(),
            authored_digest: self.authored_digest,
            authored_sources: self.authored_sources.clone(),
            revision: 0,
            types: self.types.clone(),
            wreck_units: Arc::clone(&self.wreck_units),
            moved: Default::default(),
            touched: Vec::new(),
            min_sight_gap_m: self.min_sight_gap_m,
        };
        snapshot.forest.reset_cleared();
        snapshot
    }

    /// The map's ground and props, each prop with its type's body row from
    /// the rules' catalog (every type placed must be one), a bridge's deck
    /// as its `deck` type, and each forest's trees (`forests.tree`) where the
    /// one `forests.rule` places them. The foliage grid is the
    /// fog's, the cleared mask the ground layer's.
    pub fn new(map: &MapDefinition, rules: &Rules) -> Self {
        assert!(
            map.forests.len() <= (1 << 24),
            "forest IDs must fit exact public f32 indices"
        );
        // The foliage and cleared grids are cut into cells of these.
        assert!(
            map.fog_cell_m.is_finite() && map.fog_cell_m > 0.0,
            "map.fog_cell_m must be finite and positive"
        );
        assert!(rules.ground.cell_m > 0.0, "ground.cell_m must be positive");
        let authored = map
            .authored_props()
            .expect("invalid authored map IDs or buildings");
        let catalog = rules.catalog.props();
        let ordinary = |id: &str| {
            catalog
                .check_placement(catalog.kind(id), PropPlacement::Ordinary)
                .expect("ordinary world body placement");
        };
        for prop in &map.props {
            ordinary(&prop.kind);
        }
        for bridge in &map.bridges {
            ordinary(&bridge.deck);
        }
        if !map.forests.is_empty() {
            ordinary(&rules.forests.tree);
            for (kind, density) in [
                (&rules.forests.log, rules.forests.rule.logs_per_ha),
                (&rules.forests.boulder, rules.forests.rule.boulders_per_ha),
            ] {
                if density > 0.0 {
                    ordinary(kind.as_deref().expect("validated floor kind"));
                }
            }
        }
        for building in &map.buildings {
            catalog
                .check_placement(catalog.kind(&building.kind), PropPlacement::Aggregate)
                .expect("placed aggregate states");
            let body = catalog.by_id(&building.kind).body;
            assert!(
                !body.garrison || building.geometry.edges.iter().any(|e| e.exposed),
                "garrison-capable aggregates require an exposed physical span"
            );
        }
        contract::river::validate(map).expect("invalid rivers or bridges");
        let forests = &rules.forests;
        let surfaces = Arc::new(surfaces::SurfaceIndex::new(
            &map.surfaces,
            &map.rivers,
            map.size,
        ));
        let field = Arc::new(HeightField::build(map, &surfaces));
        let mut world = WorldGeometry {
            relief: map.relief.clone(),
            slope_cutoff_deg: map.slope_cutoff_deg,
            surfaces,
            surface_factors: contract::map::SurfaceKind::ALL
                .map(|kind| rules.surfaces[&kind].speed_factor),
            bridges: map.bridges.clone(),
            forests: map.forests.clone(),
            forest: forest::ForestState::new(
                field.width(),
                field.depth(),
                map.fog_cell_m,
                rules.ground.cell_m,
                &map.forests,
                rules.forests.rule,
            ),
            props: PropStore::new(PropIndex::new(field.width(), field.depth(), PROP_BUCKET_M)),
            buildings: Arc::new(buildings::Buildings::new(&map.buildings)),
            template_catalog_hash: map.template_catalog_hash.clone(),
            regional_family: map.regional_family.clone(),
            authored_props: 0,
            authored_poses: std::sync::Arc::new([]),
            authored_digest: 0,
            authored_sources: Default::default(),
            revision: 0,
            types: rules.catalog.props().clone(),
            wreck_units: Arc::new(
                rules
                    .catalog
                    .indices()
                    .filter(|&t| rules.catalog.get(t).hull().is_some())
                    .map(|t| (rules.catalog.id(t).to_string(), t))
                    .collect(),
            ),
            moved: Default::default(),
            touched: Vec::new(),
            min_sight_gap_m: rules.sensors.min_sight_gap_m,
            field,
        };
        for (id, def) in &authored {
            assert_eq!(world.insert_prop(def), *id);
        }
        for bridge in &map.bridges {
            world.insert_prop(&PropDefinition {
                kind: bridge.deck.clone(),
                center: bridge.center,
                yaw: bridge.yaw,
                half_extents: [
                    bridge.half_extents[0],
                    bridge.half_extents[1],
                    bridge.thickness_m / 2.0,
                ],
                base_z: Some(bridge.deck_z - bridge.thickness_m),
                wreck_of: None,
            });
        }
        for (index, forest) in map.forests.iter().enumerate() {
            let rule = forests.rule;
            let first = u32::try_from(world.props.len()).expect("world exceeds u32 prop IDs");
            for p in world.trunk_positions(index, forest) {
                let id = world.insert_prop(&PropDefinition {
                    kind: forests.tree.clone(),
                    center: [p.x, p.y],
                    yaw: 0.0,
                    half_extents: [
                        rule.trunk_radius_m,
                        rule.trunk_radius_m,
                        rule.trunk_height_m / 2.0,
                    ],
                    base_z: None,
                    wreck_of: None,
                });
                if let Some(prop) = world.props.get_mut(id) {
                    prop.forest_tree = true;
                }
            }
            let end = u32::try_from(world.props.len()).expect("world exceeds u32 prop IDs");
            world.note_forest(forest, [first, end]);
        }
        // Floor cover follows every forest's trunks, so it cannot displace a
        // later forest's trees or change their immutable source ID ranges.
        let mut floor_nav = (!map.forests.is_empty()
            && (forests.rule.logs_per_ha > 0.0 || forests.rule.boulders_per_ha > 0.0))
            .then(|| {
                crate::navigation::NavGrid::new(std::sync::Arc::new(
                    crate::navigation::NavBase::build(
                        &world,
                        world.props(),
                        rules.physics.soldier_radius_m,
                    ),
                ))
            });
        let floor_movers = floor_nav.as_ref().map(|_| {
            let mut movers: Vec<crate::navigation::Mobility> = Vec::new();
            for kind in rules.catalog.indices() {
                let mover = crate::units::mobility(rules.catalog.get(kind), rules);
                if !movers.iter().any(|m| {
                    m.class == mover.class
                        && m.push == mover.push
                        && m.half_width_m == mover.half_width_m
                }) {
                    movers.push(mover);
                }
            }
            movers
        });
        for (index, forest) in map.forests.iter().enumerate() {
            for (kind, density, half, salt) in [
                (
                    &forests.log,
                    forests.rule.logs_per_ha,
                    forests.rule.log_half_extents_m,
                    3,
                ),
                (
                    &forests.boulder,
                    forests.rule.boulders_per_ha,
                    forests.rule.boulder_half_extents_m,
                    4,
                ),
            ] {
                if density > 0.0 {
                    world.place_forest_bodies(
                        index,
                        forest,
                        forest::FloorBody {
                            kind: kind.as_deref().expect("validated floor kind"),
                            density,
                            half,
                            salt,
                        },
                        floor_nav.as_mut().expect("enabled floor grid"),
                        floor_movers.as_ref().expect("enabled floor movers"),
                    );
                }
            }
        }
        // Authored setup is revision 0; only later changes count.
        world.revision = 0;
        world.props.track_changes();
        world.touched.clear();
        world.authored_props =
            u32::try_from(world.props.len()).expect("authored world exceeds u32 IDs");
        // Every authored prop stands at setup, so its id is its index here.
        world.authored_poses = world.props().map(pose_bits).collect();
        let mut authored = crate::digest::Digest::default();
        world.props().for_each(|p| digest_pose(&mut authored, p));
        world.authored_digest = authored.finish();
        world
    }

    /// Every road authored as a stroke: its centreline's samples, its width
    /// and its kind's share of road speed.
    pub fn road_strokes(&self) -> impl Iterator<Item = (&[[f64; 2]], f64, f64)> {
        self.surfaces.areas().iter().filter_map(|area| {
            let contract::ground::GroundShape::Stroke {
                centerline,
                width_m,
            } = &area.shape
            else {
                return None;
            };
            area.kind.is_road().then(|| {
                (
                    centerline.samples(),
                    *width_m,
                    self.surface_factors[area.kind as usize],
                )
            })
        })
    }

    /// Conservative areas where the surface can differ from open, flat ground.
    pub fn navigation_regions(&self) -> Vec<[f64; 4]> {
        let mut regions = self.field.variation_regions().to_vec();
        regions.extend(self.forests.iter().map(|f| f.shape.bounds()));
        regions.extend_from_slice(self.forest.bounds());
        regions.extend(self.surfaces.navigation_regions());
        for bridge in &self.bridges {
            let (s, c) = libm::sincos(bridge.yaw);
            let [hx, hy] = bridge.half_extents;
            let x = c.abs() * hx + s.abs() * hy;
            let y = s.abs() * hx + c.abs() * hy;
            regions.push([bridge.center[0] - x, bridge.center[1] - y, 2.0 * x, 2.0 * y]);
        }
        regions
    }

    /// Every physical part of one building resolves to its immutable owner.
    pub fn building_of(&self, part: PropId) -> Option<PropId> {
        self.buildings.identity(part)
    }

    pub fn building(&self, owner: PropId) -> Option<&contract::map::BuildingDefinition> {
        self.buildings.definition(owner)
    }

    /// Immutable union area, computed once for each authored aggregate.
    pub(crate) fn building_footprint_area(&self, part: PropId) -> Option<f64> {
        self.buildings.footprint_area(part)
    }

    /// The current live state's one integrity/garrison prop owner.
    pub fn structure_owner(&self, part: PropId) -> Option<PropId> {
        self.prop(part)
            .map(|_| self.remembered_structure_owner(part))
    }
    /// A side may still hold an earlier physical state out of sight. This key
    /// is not public live integrity and is never used for HP without a body.
    pub(crate) fn remembered_structure_owner(&self, part: PropId) -> PropId {
        self.buildings.owner(part)
    }

    /// Immutable public-map source, retained across remains chains. A body
    /// genuinely added during battle has no authored source.
    pub fn authored_prop(&self, part: PropId) -> Option<PropId> {
        if part < self.authored_props {
            Some(part)
        } else {
            self.authored_sources.get(&part).copied()
        }
    }
    fn note_replacement(&mut self, new: PropId, old: PropId) {
        if let Some(source) = self.authored_prop(old) {
            self.authored_sources.insert(new, source);
        }
    }
    pub(crate) fn digest_buildings(&self, d: &mut crate::digest::Digest) {
        self.buildings.digest(d)
    }
    /// Live parts of an authored building, keyed by its immutable identity.
    pub(crate) fn current_building_parts(&self, identity: PropId) -> &[PropId] {
        self.buildings.current_parts(identity)
    }
    /// Every physical part ever belonging to the immutable building identity.
    pub(crate) fn historical_building_parts(&self, identity: PropId) -> &[PropId] {
        self.buildings.historical_parts(identity)
    }
    fn skips_structure(&self, id: PropId, skip: Option<PropId>) -> bool {
        skip.is_some_and(|s| self.structure_owner(id) == self.structure_owner(s))
    }
    pub(crate) fn structure_parts(&self, part: PropId) -> Vec<PropId> {
        self.buildings.parts(part).unwrap_or_else(|| vec![part])
    }

    /// Tight current-part envelope in the building frame, measured around a
    /// live pivot so a distant authored origin cannot inflate arithmetic error.
    pub fn structure_footprint(&self, part: PropId) -> Option<Obb2> {
        let owner = self.structure_owner(part)?;
        let pivot = self.prop(owner)?;
        let yaw = self
            .building(owner)
            .map_or(pivot.yaw, |b| b.geometry.frame.yaw);
        let mut lo = v2(f64::INFINITY, f64::INFINITY);
        let mut hi = v2(f64::NEG_INFINITY, f64::NEG_INFINITY);
        for id in self.structure_parts(owner) {
            let prop = self.prop(id)?;
            let center = (prop.center - pivot.center).rotated(-yaw);
            let (s, c) = libm::sincos(prop.yaw - yaw);
            let half = v2(
                c.abs() * prop.half.x + s.abs() * prop.half.y,
                s.abs() * prop.half.x + c.abs() * prop.half.y,
            );
            lo = v2(lo.x.min(center.x - half.x), lo.y.min(center.y - half.y));
            hi = v2(hi.x.max(center.x + half.x), hi.y.max(center.y + half.y));
        }
        Some(Obb2 {
            center: pivot.center + ((lo + hi) * 0.5).rotated(yaw),
            yaw,
            half: (hi - lo) * 0.5,
        })
    }

    pub(crate) fn replace_building_parts(
        &mut self,
        owner: PropId,
        replacements: &[(PropId, PropId)],
    ) {
        Arc::make_mut(&mut self.buildings).replace(owner, replacements);
    }

    pub fn width(&self) -> f64 {
        self.field.width()
    }

    pub fn depth(&self) -> f64 {
        self.field.depth()
    }

    pub fn slope_cutoff_deg(&self) -> f64 {
        self.slope_cutoff_deg
    }

    /// The highest ground anywhere on the map.
    pub fn max_height(&self) -> f64 {
        self.field.top()
    }

    /// Lowest possible ground height, used to bound the fall time of stray rounds.
    pub fn lowest_ground_height(&self) -> f64 {
        self.field.bottom()
    }

    /// Ground triangle height; `None` outside the closed bounds.
    pub fn height_at(&self, x: f64, y: f64) -> Option<f64> {
        self.field.height(x, y)
    }

    /// The walkable surface at (x, y): a bridge deck where one spans, otherwise the ground.
    pub fn surface_at(&self, x: f64, y: f64) -> Option<Surface> {
        let ground = self.ground_surface_at(x, y)?;
        match self.bridges.iter().find(|b| bridge_contains(b, v2(x, y))) {
            Some(b) => Some(Surface {
                z: b.deck_z,
                normal: v3(0.0, 0.0, 1.0),
                slope_deg: 0.0,
                kind: SurfaceKind::Bridge,
                road_factor: 1.0,
                forest: ground.forest,
                traversable: true,
            }),
            None => Some(ground),
        }
    }

    /// Whether a mover may stand at (x, y): [`surface_at`](Self::surface_at)'s
    /// `traversable`, without working out the road, forest and height the
    /// surface also carries.
    pub fn traversable_at(&self, x: f64, y: f64) -> bool {
        let Some((_, normal)) = self.field.height_normal(x, y) else {
            return false;
        };
        self.bridges.iter().any(|b| bridge_contains(b, v2(x, y)))
            || (normal.z.clamp(-1.0, 1.0).acos().to_degrees() < self.slope_cutoff_deg
                && !self.surfaces.water_at(v2(x, y)))
    }

    /// The ground triangle at (x, y), ignoring any bridge above it.
    pub fn ground_surface_at(&self, x: f64, y: f64) -> Option<Surface> {
        let (z, normal) = self.field.height_normal(x, y)?;
        let p = v2(x, y);
        let slope_deg = normal.z.clamp(-1.0, 1.0).acos().to_degrees();
        let paved = self.surfaces.at(p);
        let kind = if self.surfaces.water_at(p) {
            SurfaceKind::Water
        } else {
            paved.map_or(SurfaceKind::Ground, SurfaceKind::of)
        };
        let road_factor = match (kind, paved) {
            (SurfaceKind::Water, _) | (_, None) => 0.0,
            (_, Some(paved)) => self.surface_factors[paved as usize],
        };
        Some(Surface {
            z,
            normal,
            slope_deg,
            kind,
            road_factor,
            // Water is neither road nor forest, whatever is authored over it.
            forest: kind != SurfaceKind::Water && self.forest_ground(x, y),
            traversable: kind != SurfaceKind::Water && slope_deg < self.slope_cutoff_deg,
        })
    }

    /// Earliest hit of a round along `origin + dir * t`, t ∈ [0, max_t]: the
    /// terrain and every body that stops rounds (flight's colliders, Q28).
    /// `dir` should be unit length for `t` to be metres. Ties resolve to the
    /// terrain, then the lowest prop id.
    pub fn raycast(&self, origin: V3, dir: V3, max_t: f64) -> Option<Hit> {
        self.raycast_by(origin, dir, max_t, None, |b| b.stops_rounds)
    }

    /// [`raycast`](Self::raycast) passing through `past`: the body a
    /// soldier fires from behind.
    pub fn raycast_past(
        &self,
        origin: V3,
        dir: V3,
        max_t: f64,
        past: Option<PropId>,
    ) -> Option<Hit> {
        self.raycast_by(origin, dir, max_t, past, |b| b.stops_rounds)
    }

    /// The destroyable bodies a round flies into along `origin + dir * t`,
    /// t ∈ (0, max_t], without being stopped: rows with integrity that do not
    /// stop rounds. Appends (t, id) in order of t, then id. A segment
    /// starting inside a body does not enter it again, so a round flying
    /// chord by chord meets each body once. `past` is never met.
    pub fn passes(
        &self,
        origin: V3,
        dir: V3,
        max_t: f64,
        past: Option<PropId>,
        out: &mut Vec<(f64, PropId)>,
    ) {
        let first = out.len();
        let mut ids = Vec::new();
        self.props
            .along(origin.xy(), (origin + dir * max_t).xy(), &mut ids);
        for id in ids
            .into_iter()
            .filter(|&id| !self.skips_structure(id, past))
        {
            let prop = self.props.get(id).expect("indexed prop is live");
            if prop.body.stops_rounds
                || prop.body.hp.is_none()
                || !track_near(origin, dir, max_t, prop)
            {
                continue;
            }
            if let Some((t, _)) = prop.raycast(origin, dir, max_t) {
                if t > 0.0 {
                    out.push((t, id));
                }
            }
        }
        out[first..].sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    }

    /// [`raycast`](Self::raycast) against the terrain and the props whose
    /// row `admits`, passing through `skip`.
    fn raycast_by(
        &self,
        origin: V3,
        dir: V3,
        max_t: f64,
        skip: Option<PropId>,
        admits: impl Fn(&PropBody) -> bool,
    ) -> Option<Hit> {
        let ground = self.field.raycast(origin, dir, max_t);
        let water = self
            .water_hit(origin, dir, max_t)
            .filter(|&t| ground.is_none_or(|(ground, _)| t < ground))
            .map(|t| (t, v3(0.0, 0.0, 1.0)));
        let mut best: Option<Hit> = water.or(ground).map(|(t, normal)| Hit {
            t,
            point: origin + dir * t,
            normal,
            collider: Collider::Terrain,
        });
        // Bodies are offered as the index holds them (each whose footprint
        // circle the track passes, perhaps more than once): the nearest hit
        // wins, a body only before the ground strictly nearer, and among
        // bodies hit at the same distance the lowest id.
        let (a, b) = (origin.xy(), (origin + dir * max_t).xy());
        self.props.any_along(a, b, 0.0, |id| {
            if self.skips_structure(id, skip) {
                return false;
            }
            let prop = self.props.get(id).expect("indexed prop is live");
            if !admits(&prop.body) {
                return false;
            }
            if let Some((t, normal)) = prop.raycast(origin, dir, max_t) {
                let nearer = best.is_none_or(|h| {
                    t < h.t || (t == h.t && matches!(h.collider, Collider::Prop(o) if id < o))
                });
                if nearer {
                    best = Some(Hit {
                        t,
                        point: origin + dir * t,
                        normal,
                        collider: Collider::Prop(id),
                    });
                }
            }
            false
        });
        best
    }

    /// Whether a round could fly the straight segment between two points: it
    /// meets neither the terrain nor a body that stops rounds.
    pub fn segment_clear(&self, a: V3, b: V3) -> bool {
        self.segment_clear_except(a, b, None)
    }

    /// [`segment_clear`](Self::segment_clear) ignoring one prop: an occupied
    /// building shelters its occupants through cover, not as a second wall.
    pub fn segment_clear_except(&self, a: V3, b: V3, skip: Option<PropId>) -> bool {
        let d = b - a;
        let len = d.length();
        len == 0.0 || !self.blocked_by(a, d * (1.0 / len), len, skip, |b| b.stops_rounds)
    }

    /// Whether an eye at `a` sees `b`: neither the terrain nor a body that
    /// occludes lies between (Q25: sensing's line of sight).
    pub fn sight_clear(&self, a: V3, b: V3) -> bool {
        let d = b - a;
        let len = d.length();
        len == 0.0
            || !(self.blocked_by(a, d * (1.0 / len), len, None, |b| b.occludes)
                || self.squeezed(a, b))
    }

    /// Whether the line from `a` to `b` squeezes between bodies that hide
    /// what lies behind them, one on its left and one on its right, with
    /// less than `min_sight_gap_m` of room on its two sides together:
    /// through the crack between two houses, or the sliver between a near
    /// house on one side and a far one on the other. A body that stands
    /// within half that of either end is passed, so one at a wall or in a
    /// crack still sees and is seen. Where the line passes a box it misses
    /// most closely is a corner; the body counts where it stands above the
    /// line.
    fn squeezed(&self, a: V3, b: V3) -> bool {
        let least = self.min_sight_gap_m;
        let (from, to) = (a.xy(), b.xy());
        let len = (to - from).length();
        if least <= 0.0 || len <= least {
            return false;
        }
        let dir = (to - from) * (1.0 / len);
        let (mut left, mut right) = (f64::INFINITY, f64::INFINITY);
        self.props.any_along(from, to, least, |id| {
            let prop = self.props.get(id).expect("indexed prop is live");
            let f = prop.footprint();
            if !prop.body.occludes || f.distance(from) < least / 2.0 || f.distance(to) < least / 2.0
            {
                return false;
            }
            let (x, y) = (v2(1.0, 0.0).rotated(f.yaw), v2(0.0, 1.0).rotated(f.yaw));
            let mut nearest: Option<(f64, f64)> = None;
            let mut side = 0.0;
            for (sx, sy) in [(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
                let v = f.center + x * (sx * f.half.x) + y * (sy * f.half.y) - from;
                let t = v.dot(dir);
                if t <= 0.0 || t >= len {
                    continue;
                }
                let off = dir.x * v.y - dir.y * v.x;
                if side * off < 0.0 {
                    // Corners either side: the line crosses the box, which
                    // the body test judges.
                    return false;
                }
                side = off.signum();
                if nearest.is_none_or(|(o, _)| off.abs() < o) {
                    nearest = Some((off.abs(), t));
                }
            }
            if let Some((off, t)) = nearest {
                let z = a.z + (b.z - a.z) * (t / len);
                if prop.top_z() > z {
                    let room = if side > 0.0 { &mut left } else { &mut right };
                    *room = room.min(off);
                }
            }
            left + right < least
        })
    }

    /// Whether [`raycast_by`](Self::raycast_by) would hit anything, without
    /// finding the nearest hit: the first prop found ends the search, only
    /// props whose footprint circle the segment meets are ray-tested, and
    /// the terrain is only searched when no prop is met.
    fn blocked_by(
        &self,
        origin: V3,
        dir: V3,
        max_t: f64,
        skip: Option<PropId>,
        admits: impl Fn(&PropBody) -> bool,
    ) -> bool {
        // A hit lies on the segment inside the footprint, so within its
        // circle: the index offers every prop that could be hit.
        let (a, b) = (origin.xy(), (origin + dir * max_t).xy());
        let by_prop = self.props.any_along(a, b, 0.0, |id| {
            let prop = self.props.get(id).expect("indexed prop is live");
            !self.skips_structure(id, skip)
                && admits(&prop.body)
                && prop.raycast(origin, dir, max_t).is_some()
        });
        by_prop
            || self.field.raycast(origin, dir, max_t).is_some()
            || self.water_hit(origin, dir, max_t).is_some()
    }

    /// Where a line falling along `origin + dir * t`, t ∈ [0, max_t], meets a
    /// river's surface: water stops a round as the ground does, so a shell
    /// bursts on the river, not on its bed.
    fn water_hit(&self, origin: V3, dir: V3, max_t: f64) -> Option<f64> {
        if dir.z >= 0.0 {
            return None;
        }
        self.surfaces
            .rivers()
            .iter()
            .enumerate()
            .filter_map(|(river, definition)| {
                let t = (definition.surface_z() - origin.z) / dir.z;
                let p = origin + dir * t;
                ((0.0..=max_t).contains(&t)
                    && self.field.contains(p.x, p.y)
                    && self.surfaces.in_river(river, p.xy()))
                .then_some(t)
            })
            .min_by(f64::total_cmp)
    }

    /// Add a body with no placed-building geometry owner.
    pub fn add_prop(&mut self, def: &PropDefinition) -> PropId {
        self.types
            .check_placement(self.types.kind(&def.kind), PropPlacement::Ordinary)
            .expect("ordinary prop placement");
        self.insert_prop(def)
    }

    /// A replacement inherits its old body's retained geometry ownership.
    pub(crate) fn add_replacement(&mut self, def: &PropDefinition, old: PropId) -> PropId {
        let placement = if self.buildings.identity(old).is_some() {
            PropPlacement::Aggregate
        } else {
            PropPlacement::Ordinary
        };
        self.types
            .check_placement(self.types.kind(&def.kind), placement)
            .expect("replacement prop placement");
        let id = self.insert_prop(def);
        self.note_replacement(id, old);
        id
    }

    fn insert_prop(&mut self, def: &PropDefinition) -> PropId {
        let prop = self.placed_prop(def);
        let id = prop.id;
        self.props.push(prop);
        self.revision += 1;
        self.touched.push(id);
        id
    }

    fn placed_prop(&self, def: &PropDefinition) -> Prop {
        let id = u32::try_from(self.props.len()).expect("world exceeds u32 prop IDs");
        let center = v2(def.center[0], def.center[1]);
        let base_z = def.base_z.unwrap_or_else(|| self.standing_z(center));
        let kind = self.types.kind(&def.kind);
        let wreck_of = self.wreck_of(def);
        Prop {
            id,
            kind,
            center,
            yaw: def.yaw,
            half: v3(
                def.half_extents[0],
                def.half_extents[1],
                def.half_extents[2],
            ),
            base_z,
            forest_tree: false,
            known_to_all: false,
            body: self.types.get(kind).body,
            wreck_of,
        }
    }

    /// The unit type `def`'s wreck was: a prop drawn by `wreck` names a unit
    /// type with a hull, whether a death or a map placed it, and no other
    /// prop names one. There is no default wreck to fall back to.
    fn wreck_of(&self, def: &PropDefinition) -> Option<contract::catalog::TypeIndex> {
        let wreck = self.types.by_id(&def.kind).appearance.drawn_by == "wreck";
        match &def.wreck_of {
            Some(unit) if wreck => Some(*self.wreck_units.get(unit).unwrap_or_else(|| {
                panic!("a wreck names the unit it was: {unit:?} is no unit type with a hull")
            })),
            None if wreck => panic!(
                "a wreck names the unit it was: a {:?} at {:?} names none",
                def.kind, def.center
            ),
            Some(unit) => panic!("only a wreck names a unit: a {:?} names {unit:?}", def.kind),
            None => None,
        }
    }

    /// Every side plans with `id` from now on ([`Prop::known_to_all`]).
    pub fn set_known_to_all(&mut self, id: PropId) {
        if let Some(p) = self.props.get_mut(id) {
            p.known_to_all = true;
            self.touched.push(id);
        }
    }

    /// The props changed since the last call (added, moved, removed or made
    /// known to all), for what holds a picture of them to bring up to date.
    pub fn touched(&self) -> &[PropId] {
        &self.touched
    }

    /// Empty [`Self::touched`], handing its props over.
    pub fn take_touched(&mut self) -> Vec<PropId> {
        std::mem::take(&mut self.touched)
    }

    pub fn remove_prop(&mut self, id: PropId) -> Option<Prop> {
        let prop = self.props.take(id)?;
        self.revision += 1;
        self.touched.push(id);
        Some(prop)
    }

    /// The height a body at `p` stands at: the walkable surface there, a
    /// bridge's deck where one spans and the ground (a river's bed) elsewhere.
    fn standing_z(&self, p: V2) -> f64 {
        self.surface_at(p.x, p.y).map_or(0.0, |s| s.z)
    }

    /// Shove a prop to a new pose at `tick` (Q2): it keeps its id and kind,
    /// stands on the surface there (pushed off a deck, it drops to the bed),
    /// and the obstacle revision bumps.
    pub fn move_prop(&mut self, id: PropId, center: V2, yaw: f64, tick: u64) {
        let Some(mut prop) = self.props.take(id) else {
            return;
        };
        prop.base_z += self.standing_z(center) - self.standing_z(prop.center);
        prop.center = center;
        prop.yaw = yaw;
        self.props.put(prop);
        self.moved.insert(id, tick);
        self.revision += 1;
        self.touched.push(id);
    }

    /// Whether `id` has come to rest by `tick`: shoved neither this tick nor
    /// the last.
    pub fn resting(&self, id: PropId, tick: u64) -> bool {
        self.moved.get(&id).is_none_or(|&t| tick > t + 1)
    }

    /// Each shoved prop and the last tick it moved, by id.
    pub fn moved(&self) -> impl Iterator<Item = (PropId, u64)> + '_ {
        self.moved.iter().map(|(&id, &t)| (id, t))
    }

    /// The prop types every prop takes its body from.
    pub fn types(&self) -> &PropCatalog {
        &self.types
    }

    /// The prop type of `kind`.
    pub fn prop_type(&self, kind: PropKind) -> &PropType {
        self.types.get(kind)
    }

    pub fn prop(&self, id: PropId) -> Option<&Prop> {
        self.props.get(id)
    }

    pub fn props(&self) -> impl Iterator<Item = &Prop> {
        self.props.iter()
    }

    /// Every body with an id from `first` on: those added after `first`
    /// was, by ascending id.
    pub fn props_from(&self, first: PropId) -> impl Iterator<Item = &Prop> {
        self.props.iter_from(first)
    }

    /// The props' share of the battle's digest: how many stand, and the pose
    /// of each that is not standing as it was authored (shoved, added, put
    /// back by a collapse). A prop still in its authored pose is held by the
    /// authored props' digest, taken once, so it costs a comparison, not a
    /// hash.
    pub fn digest_props(&self, d: &mut crate::digest::Digest) {
        d.u64(self.authored_digest);
        let mut standing = 0u64;
        for p in self.props() {
            standing += 1;
            let pose = pose_bits(p);
            if self.authored_poses.get(p.id as usize) != Some(&pose) {
                digest_pose(d, p);
            }
        }
        d.u64(standing);
    }

    /// Whether `hit` holds for some prop whose footprint comes within
    /// `radius` of `center` (every such prop is offered, and some farther;
    /// one may be offered more than once), stopping at the first. Unlike
    /// [`Self::props_near`] it neither allocates nor sorts.
    pub fn any_prop_near(
        &self,
        center: V2,
        radius: f64,
        mut hit: impl FnMut(&Prop) -> bool,
    ) -> bool {
        self.props.any_near(center, radius, |id| {
            hit(self.prop(id).expect("indexed prop is live"))
        })
    }

    /// Props whose footprint may reach within `radius` of `center`.
    pub fn props_near(&self, center: V2, radius: f64) -> Vec<&Prop> {
        let mut ids = Vec::new();
        self.props.near(center, radius, &mut ids);
        ids.into_iter().filter_map(|id| self.prop(id)).collect()
    }

    /// Ascending unique prop candidates across all visibility views.
    pub(crate) fn prop_ids_near_many(&self, views: &[(V2, f64)], out: &mut Vec<PropId>) {
        self.props.near_many(views, out);
    }

    /// Increments whenever a prop is added, moved or removed after authored setup.
    pub fn obstacle_revision(&self) -> u64 {
        self.revision
    }

    /// Newest prop mutation in the footprint buckets queried by `props_near`.
    pub(crate) fn obstacle_revision_near(&self, center: V2, radius: f64) -> u64 {
        self.props.revision_near(center, radius)
    }

    pub fn forests(&self) -> &[Forest] {
        &self.forests
    }

    pub fn rivers(&self) -> &[River] {
        self.surfaces.rivers()
    }

    /// The exact ground triangles, for rendering and diagnostics.
    pub fn terrain_mesh(&self) -> (&[V3], &[u32]) {
        let (vertices, indices) = self.field.mesh();
        (vertices, indices)
    }
}

fn bridge_contains(b: &Bridge, p: V2) -> bool {
    Obb2 {
        center: v2(b.center[0], b.center[1]),
        yaw: b.yaw,
        half: v2(b.half_extents[0], b.half_extents[1]),
    }
    .contains(p, 0.0)
}

/// A prop's pose, bit for bit.
fn pose_bits(p: &Prop) -> [u64; 4] {
    [
        p.center.x.to_bits(),
        p.center.y.to_bits(),
        p.yaw.to_bits(),
        p.base_z.to_bits(),
    ]
}

/// A prop and its pose, into `d`.
fn digest_pose(d: &mut crate::digest::Digest, p: &Prop) {
    d.u64(p.id as u64)
        .f64(p.center.x)
        .f64(p.center.y)
        .f64(p.yaw)
        .f64(p.base_z);
}

/// Whether the ground track of `origin + dir * t`, t ∈ [0, max_t], comes
/// within `prop`'s footprint circle (its half-sides bound the half-diagonal
/// without a square root): a ray whose track stays outside it cannot meet
/// the box, so the slab test is skipped for most bodies a ray's buckets hold.
fn track_near(origin: V3, dir: V3, max_t: f64, prop: &Prop) -> bool {
    let (a, ab) = (origin.xy(), dir.xy() * max_t);
    let ac = prop.center - a;
    let t = (ac.dot(ab) / ab.dot(ab).max(1e-300)).clamp(0.0, 1.0);
    let off = ab * t - ac;
    let reach = prop.half.x.abs() + prop.half.y.abs();
    off.dot(off) <= reach * reach * (1.0 + 1e-9) + 1e-9
}
