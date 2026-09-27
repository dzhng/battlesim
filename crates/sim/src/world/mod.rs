//! Authoritative world geometry: bounded ground triangles, water, roads,
//! bridges, forest volumes and solid props. Collision, sight, routing and the
//! renderer all read these same surfaces.
pub mod export;
mod props;
mod terrain;

pub(crate) use props::ray_box;
use props::PropIndex;
pub use props::{Prop, PropId, Slot};
use terrain::{in_rect, HeightField};

use crate::math::{v2, v3, Obb2, V2, V3};
use contract::map::{Bridge, Forest, MapDefinition, PropDefinition, PropKind, Water};
use contract::scenario::{PropBody, PropTable};

const PROP_BUCKET_M: f64 = 32.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceKind {
    Ground,
    Road,
    Water,
    Bridge,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Surface {
    pub z: f64,
    pub normal: V3,
    pub slope_deg: f64,
    pub kind: SurfaceKind,
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
    roads: Vec<(Vec<V2>, f64)>,
    bridges: Vec<Bridge>,
    forests: Vec<Forest>,
    props: Vec<Option<Prop>>,
    index: PropIndex,
    revision: u64,
    /// The body table: each new prop takes its kind's row.
    table: PropTable,
    /// The last tick each shoved prop moved: one not shoved last tick or this
    /// one has come to rest.
    moved: std::collections::BTreeMap<PropId, u64>,
}

impl WorldGeometry {
    /// The map's ground and props, each prop with its kind's row of `table`
    /// (the fixture's body table; every kind placed must have one).
    pub fn new(map: &MapDefinition, table: &PropTable) -> Self {
        let field = HeightField::build(map);
        let index = PropIndex::new(field.width(), field.depth(), PROP_BUCKET_M);
        let mut world = WorldGeometry {
            slope_cutoff_deg: map.slope_cutoff_deg,
            water: map.water.clone(),
            roads: map
                .roads
                .iter()
                .map(|r| (r.points.iter().map(|p| v2(p[0], p[1])).collect(), r.width_m))
                .collect(),
            bridges: map.bridges.clone(),
            forests: map.forests.clone(),
            props: Vec::new(),
            index,
            revision: 0,
            table: table.clone(),
            moved: Default::default(),
            field,
        };
        for def in &map.props {
            world.add_prop(def);
        }
        for bridge in &map.bridges {
            world.add_prop(&PropDefinition {
                kind: PropKind::BridgeDeck,
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
        for forest in map.forests.clone() {
            for p in world.trunk_positions(&forest) {
                world.add_prop(&PropDefinition {
                    kind: PropKind::Trunk,
                    center: [p.x, p.y],
                    yaw: 0.0,
                    half_extents: [
                        forest.trunk_radius_m,
                        forest.trunk_radius_m,
                        forest.trunk_height_m / 2.0,
                    ],
                    base_z: None,
                });
            }
        }
        // Authored setup is revision 0; only later changes count.
        world.revision = 0;
        world
    }

    fn trunk_positions(&self, forest: &Forest) -> Vec<V2> {
        let [x0, y0, w, h] = forest.rect;
        let step = forest.trunk_spacing_m;
        let mut out = Vec::new();
        let mut y = y0 + step / 2.0;
        while y <= y0 + h {
            let mut x = x0 + step / 2.0;
            while x <= x0 + w {
                let p = v2(x, y);
                let near_road = self.roads.iter().any(|(pts, width)| {
                    distance_to_polyline(pts, p) <= width / 2.0 + forest.trunk_clearance_m
                });
                let near_prop = self.props().any(|prop| {
                    prop.kind != PropKind::Trunk
                        && prop.footprint().contains(p, forest.trunk_clearance_m)
                });
                if !near_road && !near_prop && self.field.contains(x, y) {
                    out.push(p);
                }
                x += step;
            }
            y += step;
        }
        out
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
                forest: ground.forest,
                traversable: true,
            }),
            None => Some(ground),
        }
    }

    /// The ground triangle at (x, y), ignoring any bridge above it.
    pub fn ground_surface_at(&self, x: f64, y: f64) -> Option<Surface> {
        let (z, normal) = self.field.height_normal(x, y)?;
        let p = v2(x, y);
        let slope_deg = normal.z.clamp(-1.0, 1.0).acos().to_degrees();
        let kind = if self.water.iter().any(|w| in_rect(w.rect, x, y)) {
            SurfaceKind::Water
        } else if self
            .roads
            .iter()
            .any(|(pts, width)| distance_to_polyline(pts, p) <= width / 2.0)
        {
            SurfaceKind::Road
        } else {
            SurfaceKind::Ground
        };
        Some(Surface {
            z,
            normal,
            slope_deg,
            kind,
            forest: self.forests.iter().any(|f| in_rect(f.rect, x, y)),
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
        for id in ids.into_iter().filter(|&id| Some(id) != skip) {
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
        len == 0.0
            || self
                .raycast_by(a, d * (1.0 / len), len, skip, |b| b.stops_rounds)
                .is_none()
    }

    /// Whether an eye at `a` sees `b`: neither the terrain nor a body that
    /// occludes lies between (Q25: sensing's line of sight).
    pub fn sight_clear(&self, a: V3, b: V3) -> bool {
        let d = b - a;
        let len = d.length();
        len == 0.0
            || self
                .raycast_by(a, d * (1.0 / len), len, None, |b| b.occludes)
                .is_none()
    }

    pub fn add_prop(&mut self, def: &PropDefinition) -> PropId {
        let id = self.props.len() as PropId;
        let center = v2(def.center[0], def.center[1]);
        let base_z = def
            .base_z
            .unwrap_or_else(|| self.height_at(center.x, center.y).unwrap_or(0.0));
        let prop = Prop {
            id,
            kind: def.kind,
            center,
            yaw: def.yaw,
            half: v3(
                def.half_extents[0],
                def.half_extents[1],
                def.half_extents[2],
            ),
            base_z,
            body: *self
                .table
                .get(&def.kind)
                .unwrap_or_else(|| panic!("the body table has no row for {:?}", def.kind)),
        };
        self.index.insert(&prop);
        self.props.push(Some(prop));
        self.revision += 1;
        id
    }

    pub fn remove_prop(&mut self, id: PropId) -> Option<Prop> {
        let prop = self.props.get_mut(id as usize)?.take()?;
        self.index.remove(&prop);
        self.revision += 1;
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

    /// The body table's row for `kind`.
    pub fn body(&self, kind: PropKind) -> Option<&PropBody> {
        self.table.get(&kind)
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

    /// Metres of the segment `a`→`b` that pass through foliage: inside a forest
    /// rectangle and below its canopy top over the ground there. Trunks are
    /// solid props; the foliage itself only attenuates.
    pub fn forest_path_length(&self, a: V3, b: V3) -> f64 {
        const STEP_M: f64 = 1.0;
        let d = b - a;
        let mut total = 0.0;
        for f in &self.forests {
            let [x0, y0, w, h] = f.rect;
            let (mut t0, mut t1) = (0.0f64, 1.0f64);
            for (o, dv, lo, hi) in [(a.x, d.x, x0, x0 + w), (a.y, d.y, y0, y0 + h)] {
                if dv.abs() < 1e-12 {
                    if o < lo || o > hi {
                        t1 = -1.0;
                    }
                } else {
                    let (p, q) = ((lo - o) / dv, (hi - o) / dv);
                    t0 = t0.max(p.min(q));
                    t1 = t1.min(p.max(q));
                }
            }
            if t0 >= t1 {
                continue;
            }
            let span = (t1 - t0) * d.length();
            let n = (span / STEP_M).ceil().max(1.0) as usize;
            let piece = span / n as f64;
            for k in 0..n {
                let p = a + d * (t0 + (t1 - t0) * ((k as f64 + 0.5) / n as f64));
                if let Some(ground) = self.height_at(p.x, p.y) {
                    if p.z < ground + f.canopy_height_m {
                        total += piece;
                    }
                }
            }
        }
        total
    }

    /// How deep inside a forest a ground point is (distance to its nearest
    /// edge), or `None` outside every forest. The deepest forest wins.
    pub fn forest_depth(&self, x: f64, y: f64) -> Option<f64> {
        self.forests
            .iter()
            .filter(|f| in_rect(f.rect, x, y))
            .map(|f| {
                let [x0, y0, w, h] = f.rect;
                (x - x0).min(x0 + w - x).min(y - y0).min(y0 + h - y)
            })
            .reduce(f64::max)
    }

    pub fn forests(&self) -> &[Forest] {
        &self.forests
    }

    pub fn water(&self) -> &[Water] {
        &self.water
    }

    /// The exact ground triangles, for rendering and diagnostics.
    pub fn terrain_mesh(&self) -> (Vec<V3>, Vec<u32>) {
        self.field.mesh()
    }
}

pub fn in_forest(f: &Forest, x: f64, y: f64) -> bool {
    in_rect(f.rect, x, y)
}

fn bridge_contains(b: &Bridge, p: V2) -> bool {
    Obb2 {
        center: v2(b.center[0], b.center[1]),
        yaw: b.yaw,
        half: v2(b.half_extents[0], b.half_extents[1]),
    }
    .contains(p, 0.0)
}

pub fn distance_to_polyline(points: &[V2], p: V2) -> f64 {
    points
        .windows(2)
        .map(|w| {
            let (a, b) = (w[0], w[1]);
            let ab = b - a;
            let len2 = ab.dot(ab);
            let t = if len2 > 0.0 {
                ((p - a).dot(ab) / len2).clamp(0.0, 1.0)
            } else {
                0.0
            };
            (p - (a + ab * t)).length()
        })
        .fold(f64::INFINITY, f64::min)
}
