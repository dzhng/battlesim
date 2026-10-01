//! Authoritative world geometry: bounded ground triangles, water, roads,
//! bridges, forest volumes and solid props. Collision, sight, routing and the
//! renderer all read these same surfaces.
mod buildings;
pub mod export;
mod forest;
mod props;
mod surfaces;
mod terrain;

pub use forest::Foliage;
pub(crate) use props::ray_box;
use props::PropIndex;
pub use props::{Prop, PropId, Slot};
use terrain::{in_rect, HeightField};

use crate::math::{v2, v3, Obb2, V2, V3};
use contract::catalog::{PropBody, PropCatalog, PropKind, PropType};
use contract::map::{Bridge, Forest, MapDefinition, PropDefinition, Water};
use contract::scenario::Rules;

/// The prop index's bucket. Line tests measured this against 8, 16 and 64 m
/// buckets (27 perf): 32 and 64 tie, finer is dearer.
const PROP_BUCKET_M: f64 = 32.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceKind {
    Ground,
    Road,
    Water,
    Bridge,
    Sidewalk,
}

impl SurfaceKind {
    /// Every carriageway is a road here; its kind only sets its speed
    /// (`Surface::road_factor`).
    pub fn of(kind: contract::map::SurfaceKind) -> Self {
        if kind.is_road() {
            SurfaceKind::Road
        } else {
            SurfaceKind::Sidewalk
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

pub struct WorldGeometry {
    field: HeightField,
    slope_cutoff_deg: f64,
    water: Vec<Water>,
    surfaces: surfaces::SurfaceIndex,
    /// Each surface kind's speed factor, by `contract::map::SurfaceKind` order.
    surface_factors: [f64; contract::map::SurfaceKind::ALL.len()],
    bridges: Vec<Bridge>,
    /// Authoritative authored shapes; generated trunks and cleared ground are runtime state.
    forests: Vec<Forest>,
    /// The forests at runtime: foliage per fog cell and the cleared mask.
    forest: forest::ForestState,
    props: Vec<Option<Prop>>,
    buildings: buildings::Buildings,
    template_catalog_hash: Option<String>,
    authored_props: PropId,
    authored_sources: std::collections::BTreeMap<PropId, PropId>,
    index: PropIndex,
    revision: u64,
    /// The prop types: each new prop takes its type's body row.
    types: PropCatalog,
    /// The last tick each shoved prop moved: one not shoved last tick or this
    /// one has come to rest.
    moved: std::collections::BTreeMap<PropId, u64>,
    /// Props added, moved, removed or made known to all since
    /// [`Self::take_touched`] last emptied it.
    touched: Vec<PropId>,
}

impl WorldGeometry {
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
        for prop in &map.props {
            assert!(
                !rules.catalog.props().by_id(&prop.kind).body.garrison,
                "garrison-capable authored bodies require a placed aggregate"
            );
        }
        for building in &map.buildings {
            let body = rules.catalog.props().by_id(&building.kind).body;
            assert_eq!(
                body.weight_class,
                contract::scenario::WeightClass::Immovable,
                "placed aggregates require immovable bodies until composite motion exists"
            );
            assert!(
                !body.garrison || building.geometry.edges.iter().any(|e| e.exposed),
                "garrison-capable aggregates require an exposed physical span"
            );
        }
        let forests = &rules.forests;
        let field = HeightField::build(map);
        let index = PropIndex::new(field.width(), field.depth(), PROP_BUCKET_M);
        let mut world = WorldGeometry {
            slope_cutoff_deg: map.slope_cutoff_deg,
            water: map.water.clone(),
            surfaces: surfaces::SurfaceIndex::new(&map.surfaces, map.size),
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
            props: Vec::new(),
            buildings: buildings::Buildings::new(&map.buildings, rules.catalog.props()),
            template_catalog_hash: map.template_catalog_hash.clone(),
            authored_props: 0,
            authored_sources: Default::default(),
            index,
            revision: 0,
            types: rules.catalog.props().clone(),
            moved: Default::default(),
            touched: Vec::new(),
            field,
        };
        for (id, def) in &authored {
            assert_eq!(world.add_prop(def), *id);
        }
        for bridge in &map.bridges {
            world.add_prop(&PropDefinition {
                kind: bridge.deck.clone(),
                center: bridge.center,
                yaw: bridge.yaw,
                half_extents: [
                    bridge.half_extents[0],
                    bridge.half_extents[1],
                    bridge.thickness_m / 2.0,
                ],
                base_z: Some(bridge.deck_z - bridge.thickness_m),
            });
        }
        for (index, forest) in map.forests.iter().enumerate() {
            let rule = forests.rule;
            let first = u32::try_from(world.props.len()).expect("world exceeds u32 prop IDs");
            for p in world.trunk_positions(index, forest) {
                let id = world.add_prop(&PropDefinition {
                    kind: forests.tree.clone(),
                    center: [p.x, p.y],
                    yaw: 0.0,
                    half_extents: [
                        rule.trunk_radius_m,
                        rule.trunk_radius_m,
                        rule.trunk_height_m / 2.0,
                    ],
                    base_z: None,
                });
                if let Some(Some(prop)) = world.props.get_mut(id as usize) {
                    prop.forest_tree = true;
                }
            }
            let end = u32::try_from(world.props.len()).expect("world exceeds u32 prop IDs");
            world.note_forest(forest, [first, end]);
        }
        // Authored setup is revision 0; only later changes count.
        world.revision = 0;
        world.touched.clear();
        world.authored_props =
            u32::try_from(world.props.len()).expect("authored world exceeds u32 IDs");
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
        regions.extend(self.water.iter().map(|w| w.rect));
        regions.extend(self.forests.iter().map(|f| f.shape.bounds()));
        regions.extend_from_slice(self.forest.bounds());
        regions.extend(self.surfaces.navigation_regions());
        for bridge in &self.bridges {
            let (s, c) = bridge.yaw.sin_cos();
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
    pub(crate) fn note_replacement(&mut self, new: PropId, old: PropId) {
        if let Some(source) = self.authored_prop(old) {
            self.authored_sources.insert(new, source);
        }
    }
    pub(crate) fn digest_buildings(&self, d: &mut crate::digest::Digest) {
        self.buildings.digest(d)
    }
    pub(crate) fn building_states(&self) -> impl Iterator<Item = (PropId, &[PropId], &[PropId])> {
        self.buildings.states()
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
            let (s, c) = (prop.yaw - yaw).sin_cos();
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
        self.buildings.replace(owner, replacements);
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
                && !self.water.iter().any(|w| in_rect(w.rect, x, y)))
    }

    /// The ground triangle at (x, y), ignoring any bridge above it.
    pub fn ground_surface_at(&self, x: f64, y: f64) -> Option<Surface> {
        let (z, normal) = self.field.height_normal(x, y)?;
        let p = v2(x, y);
        let slope_deg = normal.z.clamp(-1.0, 1.0).acos().to_degrees();
        let paved = self.surfaces.at(p);
        let kind = if self.water.iter().any(|w| in_rect(w.rect, x, y)) {
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
            forest: self.forest_ground(x, y),
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
        self.index
            .along(origin.xy(), (origin + dir * max_t).xy(), &mut ids);
        for id in ids
            .into_iter()
            .filter(|&id| !self.skips_structure(id, past))
        {
            let prop = self.props[id as usize]
                .as_ref()
                .expect("indexed prop is live");
            if prop.body.stops_rounds || prop.body.hp.is_none() {
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
        let mut best: Option<Hit> = self
            .field
            .raycast(origin, dir, max_t)
            .map(|(t, normal)| Hit {
                t,
                point: origin + dir * t,
                normal,
                collider: Collider::Terrain,
            });
        let mut ids = Vec::new();
        self.index
            .along(origin.xy(), (origin + dir * max_t).xy(), &mut ids);
        for id in ids
            .into_iter()
            .filter(|&id| !self.skips_structure(id, skip))
        {
            let prop = self.props[id as usize]
                .as_ref()
                .expect("indexed prop is live");
            if !admits(&prop.body) {
                continue;
            }
            if let Some((t, normal)) = prop.raycast(origin, dir, max_t) {
                if best.is_none_or(|b| t < b.t) {
                    best = Some(Hit {
                        t,
                        point: origin + dir * t,
                        normal,
                        collider: Collider::Prop(id),
                    });
                }
            }
        }
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
        len == 0.0 || !self.blocked_by(a, d * (1.0 / len), len, None, |b| b.occludes)
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
        let by_prop = self.index.any_along(a, b, |id| {
            let prop = self.props[id as usize]
                .as_ref()
                .expect("indexed prop is live");
            !self.skips_structure(id, skip)
                && admits(&prop.body)
                && prop.raycast(origin, dir, max_t).is_some()
        });
        by_prop || self.field.raycast(origin, dir, max_t).is_some()
    }

    pub fn add_prop(&mut self, def: &PropDefinition) -> PropId {
        let id = u32::try_from(self.props.len()).expect("world exceeds u32 prop IDs");
        let center = v2(def.center[0], def.center[1]);
        let base_z = def
            .base_z
            .unwrap_or_else(|| self.height_at(center.x, center.y).unwrap_or(0.0));
        let kind = self.types.kind(&def.kind);
        let prop = Prop {
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
        };
        self.index.insert(&prop);
        self.props.push(Some(prop));
        self.revision += 1;
        self.touched.push(id);
        id
    }

    /// Every side plans with `id` from now on ([`Prop::known_to_all`]).
    pub fn set_known_to_all(&mut self, id: PropId) {
        if let Some(Some(p)) = self.props.get_mut(id as usize) {
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
        let prop = self.props.get_mut(id as usize)?.take()?;
        self.index.remove(&prop);
        self.revision += 1;
        self.touched.push(id);
        Some(prop)
    }

    /// Shove a prop to a new pose at `tick` (Q2): it keeps its id and kind,
    /// stands on the ground there, and the obstacle revision bumps.
    pub fn move_prop(&mut self, id: PropId, center: V2, yaw: f64, tick: u64) {
        let Some(mut prop) = self.props.get_mut(id as usize).and_then(Option::take) else {
            return;
        };
        self.index.remove(&prop);
        let ground = |p: V2| self.height_at(p.x, p.y).unwrap_or(0.0);
        prop.base_z += ground(center) - ground(prop.center);
        prop.center = center;
        prop.yaw = yaw;
        self.index.insert(&prop);
        self.props[id as usize] = Some(prop);
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
        self.props.get(id as usize)?.as_ref()
    }

    pub fn props(&self) -> impl Iterator<Item = &Prop> {
        self.props.iter().flatten()
    }

    /// Props whose footprint may reach within `radius` of `center`.
    pub fn props_near(&self, center: V2, radius: f64) -> Vec<&Prop> {
        let mut ids = Vec::new();
        self.index.near(center, radius, &mut ids);
        ids.into_iter().filter_map(|id| self.prop(id)).collect()
    }

    /// Increments whenever a prop is added, moved or removed after authored setup.
    pub fn obstacle_revision(&self) -> u64 {
        self.revision
    }

    pub fn forests(&self) -> &[Forest] {
        &self.forests
    }

    pub fn water(&self) -> &[Water] {
        &self.water
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
