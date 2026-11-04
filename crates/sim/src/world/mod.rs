//! Authoritative world geometry: bounded ground triangles, water, roads,
//! bridges, forest volumes and solid props. Collision, sight, routing and the
//! renderer all read these same surfaces.
mod buildings;
mod carve;
pub mod export;
mod forest;
mod props;
mod rays;
mod surfaces;
mod terrain;

pub use forest::Foliage;
use props::PropStore;
pub(crate) use props::{ray_box, PropIndex};
pub use props::{Prop, PropId, Slot};
pub use rays::{Collider, Hit};
use terrain::HeightField;

use crate::math::{v2, v3, Obb2, V2, V3};
use contract::catalog::{PropCatalog, PropKind, PropPlacement, PropType};
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
    /// (`Surface::paved`).
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
    /// The paving under it, whose kind each locomotion's speed factors
    /// scale its road speed by; `None` off any paving and on water.
    pub paved: Option<contract::map::SurfaceKind>,
    /// Forest ground, not cleared (Q16): forest speed applies.
    pub forest: bool,
    /// A swamp's water: inside a swamp's outline and on no road.
    pub swamp: bool,
    /// Ground units may stand here: not water, and below the shared slope cutoff.
    /// Solid props are separate obstacles (see `props_near`).
    pub traversable: bool,
}

impl Surface {
    /// What a mover's speed reads of it.
    pub fn footing(&self) -> crate::navigation::Footing {
        crate::navigation::Footing {
            paved: [self.paved, None],
            forest: self.forest,
            swamp: self.swamp,
            slope_deg: self.slope_deg,
            crater_fill: 0.0,
        }
    }

    /// Standing water, a river's or a swamp's: it takes no marks, and rounds
    /// and sight stop at its surface.
    pub fn standing_water(&self) -> bool {
        self.kind == SurfaceKind::Water || self.swamp
    }
}

#[derive(Clone)]
pub struct WorldGeometry {
    field: Arc<HeightField>,
    /// The land's authored relief, before any river is carved into it.
    relief: Vec<contract::map::Relief>,
    slope_cutoff_deg: f64,
    /// The paving and the rivers: what the ground is at a point.
    surfaces: Arc<surfaces::SurfaceIndex>,
    /// How each kind of ground scales each locomotion's speed.
    speed_factors: contract::movement::SpeedFactorTable,
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
            speed_factors: self.speed_factors,
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
        // An uploaded map was refused before this with these same errors
        // (`contract::maps::admit_world`); anything else reaching here is a bug.
        let refused = contract::maps::admit_world(map, rules);
        assert!(
            refused.is_empty(),
            "map not admitted: {}",
            refused
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; ")
        );
        let authored = map.authored_props().expect("admitted map");
        let forests = &rules.forests;
        let surfaces = Arc::new(surfaces::SurfaceIndex::new(map));
        let field = Arc::new(HeightField::build(map, &surfaces));
        let mut world = WorldGeometry {
            relief: map.relief.clone(),
            slope_cutoff_deg: map.slope_cutoff_deg,
            surfaces,
            speed_factors: rules.movement.speed_factors,
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

    /// How each kind of ground scales each locomotion's speed.
    pub fn speed_factors(&self) -> &contract::movement::SpeedFactorTable {
        &self.speed_factors
    }

    /// Every road authored as a stroke: its centreline's samples, its width
    /// and its kind.
    pub fn road_strokes(
        &self,
    ) -> impl Iterator<Item = (&[[f64; 2]], f64, contract::map::SurfaceKind)> {
        self.surfaces.areas().iter().filter_map(|area| {
            let contract::ground::GroundShape::Stroke {
                centerline,
                width_m,
            } = &area.shape
            else {
                return None;
            };
            area.kind
                .is_road()
                .then(|| (centerline.samples(), *width_m, area.kind))
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
                paved: Some(contract::map::SurfaceKind::Road),
                forest: ground.forest,
                swamp: false,
                traversable: true,
            }),
            None => Some(ground),
        }
    }

    /// Whether a mover may stand at (x, y): [`surface_at`](Self::surface_at)'s
    /// `traversable`, without the road, forest and height the surface also
    /// carries.
    pub fn traversable_at(&self, x: f64, y: f64) -> bool {
        let Some((_, normal)) = self.field.height_normal(x, y) else {
            return false;
        };
        self.bridges.iter().any(|b| bridge_contains(b, v2(x, y)))
            || (libm::acos(normal.z.clamp(-1.0, 1.0)).to_degrees() < self.slope_cutoff_deg
                && !self.surfaces.water_at(v2(x, y)))
    }

    /// The ground triangle at (x, y), ignoring any bridge above it.
    pub fn ground_surface_at(&self, x: f64, y: f64) -> Option<Surface> {
        let (z, normal) = self.field.height_normal(x, y)?;
        let p = v2(x, y);
        let slope_deg = libm::acos(normal.z.clamp(-1.0, 1.0)).to_degrees();
        let paved = self.surfaces.at(p);
        let kind = if self.surfaces.water_at(p) {
            SurfaceKind::Water
        } else {
            paved.map_or(SurfaceKind::Ground, SurfaceKind::of)
        };
        let paved = paved.filter(|_| kind != SurfaceKind::Water);
        Some(Surface {
            z,
            normal,
            slope_deg,
            kind,
            paved,
            // Water is neither road nor forest, whatever is authored over it.
            forest: kind != SurfaceKind::Water && self.forest_ground(x, y),
            swamp: self.surfaces.swamp_water_at(p),
            traversable: kind != SurfaceKind::Water && slope_deg < self.slope_cutoff_deg,
        })
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

    /// The unit type `def`'s wreck was: a wreck prop (`PropType::wreck`)
    /// names a unit type with a hull, whether a death or a map placed it,
    /// and no other prop names one. There is no default wreck to fall back to.
    fn wreck_of(&self, def: &PropDefinition) -> Option<contract::catalog::TypeIndex> {
        let wreck = self.types.by_id(&def.kind).wreck;
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
