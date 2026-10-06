//! Solid props: oriented boxes standing on the ground. One store for static
//! scenery and later dynamic remains; a uniform bucket grid accelerates queries.
use crate::math::{v2, v3, Obb2, Rotation, V2, V3};
use contract::catalog::{PropBody, PropKind};
use contract::map::MoverClass;
use std::collections::BTreeMap;
use std::sync::Arc;

pub type PropId = u32;

#[derive(Clone, Debug, PartialEq)]
pub struct Prop {
    pub id: PropId,
    pub kind: PropKind,
    pub center: V2,
    pub yaw: f64,
    /// Half extents along heading, across heading, vertical.
    pub half: V3,
    pub base_z: f64,
    /// Its prop type's body row: what it blocks, stops, hides and weighs.
    pub body: PropBody,
    /// A tree a forest generated: its crown hides by the one `forests.rule`.
    pub forest_tree: bool,
    /// Every side plans with it, seen or not: remains that close an
    /// authored body's footprint as the body did (a building's ruin).
    pub known_to_all: bool,
}

impl Prop {
    /// Whether it stops a ground mover of `class` (navigation and collision).
    pub fn blocks(&self, class: MoverClass) -> bool {
        self.body.blocks.class(class)
    }

    pub fn top_z(&self) -> f64 {
        self.base_z + 2.0 * self.half.z
    }

    /// The ground footprint.
    pub fn footprint(&self) -> Obb2 {
        Obb2 {
            center: self.center,
            yaw: self.yaw,
            half: self.half.xy(),
        }
    }

    /// Slab test. Returns (t, world normal) of the entry point; an origin
    /// already inside the box hits at t = 0.
    pub fn raycast(&self, origin: V3, dir: V3, max_t: f64) -> Option<(f64, V3)> {
        // Into the box's frame (origin at its centre): one rotation for the
        // point and the direction.
        let into = Rotation::new(-self.yaw);
        let (o, d) = (into.apply(origin.xy() - self.center), into.apply(dir.xy()));
        let (t, n) = ray_box(
            v3(o.x, o.y, origin.z - (self.base_z + self.half.z)),
            v3(d.x, d.y, dir.z),
            self.half,
            max_t,
        )?;
        let nw = n.xy().rotated(self.yaw);
        Some((t, v3(nw.x, nw.y, n.z)))
    }

    /// Radius of the footprint's bounding circle.
    pub fn footprint_radius(&self) -> f64 {
        libm::hypot(self.half.x, self.half.y)
    }

    /// The footprint point nearest `p`, pushed `standoff` further out along
    /// the facade's outward normal: where a squad stands to reach the facade.
    pub fn exterior_point(&self, p: V2, standoff: f64) -> V2 {
        let d = self.footprint().to_local(p);
        let q = v2(
            d.x.clamp(-self.half.x, self.half.x),
            d.y.clamp(-self.half.y, self.half.y),
        );
        // A point inside leaves by the nearest facade.
        let (gap_x, gap_y) = (self.half.x - d.x.abs(), self.half.y - d.y.abs());
        let out = if gap_x <= 0.0 || gap_y <= 0.0 {
            let n = d - q;
            if n.length() > 0.0 {
                n.normalized()
            } else {
                v2(d.x.signum(), 0.0)
            }
        } else if gap_x < gap_y {
            v2(d.x.signum(), 0.0)
        } else {
            v2(0.0, d.y.signum())
        };
        let q = if gap_x > 0.0 && gap_y > 0.0 {
            if out.x != 0.0 {
                v2(self.half.x * out.x, d.y)
            } else {
                v2(d.x, self.half.y * out.y)
            }
        } else {
            q
        };
        self.center + (q + out * standoff).rotated(self.yaw)
    }
}

/// A perimeter firing position just outside a facade (contracts: garrisons).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Slot {
    pub position: V2,
    /// The facade's outward normal.
    pub normal: V2,
    /// Which facade: 0 = +x, 1 = +y, 2 = -x, 3 = -y in the prop's frame.
    pub facade: u8,
}

impl Slot {
    /// Whether a round from this slot toward `p` leaves the facade outward by
    /// more than `min_angle` (radians): wide of a round's truncated spread, so
    /// it never grazes back into its own wall.
    pub fn faces(&self, p: V2, min_angle: f64) -> bool {
        let d = p - self.position;
        let len = d.length();
        len > 0.0 && d.dot(self.normal) > libm::sin(min_angle) * len
    }
}

/// Uniform XY bucket grid over prop footprints.
#[derive(Clone)]
pub struct PropIndex {
    bucket: f64,
    nx: usize,
    ny: usize,
    cells: Vec<Vec<Entry>>,
    changes: Option<BucketChanges>,
}

#[derive(Clone)]
struct BucketChanges {
    revision: u64,
    stamps: Vec<u64>,
}

/// A prop in a bucket, with its footprint's bounding circle, so a segment
/// query can pass it by without reading the prop.
#[derive(Clone, Copy)]
struct Entry {
    id: PropId,
    center: V2,
    radius: f64,
}

impl PropIndex {
    pub fn new(width: f64, depth: f64, bucket: f64) -> Self {
        let nx = (width / bucket).ceil().max(1.0) as usize;
        let ny = (depth / bucket).ceil().max(1.0) as usize;
        PropIndex {
            bucket,
            nx,
            ny,
            cells: vec![Vec::new(); nx * ny],
            changes: None,
        }
    }

    /// An index of nothing over the same buckets, untracked.
    fn empty(&self) -> Self {
        PropIndex {
            bucket: self.bucket,
            nx: self.nx,
            ny: self.ny,
            cells: vec![Vec::new(); self.cells.len()],
            changes: None,
        }
    }

    /// World enables tracking after authored setup. Side-known indexes do not
    /// allocate stamps: their consumers already own their change histories.
    pub(super) fn track_changes(&mut self) {
        self.changes = Some(BucketChanges {
            revision: 0,
            stamps: vec![0; self.cells.len()],
        });
    }

    fn range(&self, center: V2, radius: f64) -> (usize, usize, usize, usize) {
        let clamp = |v: f64, n: usize| ((v / self.bucket).floor().max(0.0) as usize).min(n - 1);
        (
            clamp(center.x - radius, self.nx),
            clamp(center.x + radius, self.nx),
            clamp(center.y - radius, self.ny),
            clamp(center.y + radius, self.ny),
        )
    }

    fn cell_range(&self, p: &Prop) -> (usize, usize, usize, usize) {
        self.range(p.center, p.footprint_radius())
    }

    fn note_change(&mut self, (i0, i1, j0, j1): (usize, usize, usize, usize)) {
        if let Some(changes) = &mut self.changes {
            changes.revision += 1;
            for j in j0..=j1 {
                changes.stamps[j * self.nx + i0..=j * self.nx + i1].fill(changes.revision);
            }
        }
    }

    /// Newest mutation in the same candidate buckets as `near`. Empty buckets
    /// keep their stamp so removal invalidates a previously occupied raster.
    pub(super) fn revision_near(&self, center: V2, radius: f64) -> u64 {
        let changes = self.changes.as_ref().expect("tracked world index");
        let (i0, i1, j0, j1) = self.range(center, radius);
        (j0..=j1)
            .flat_map(|j| &changes.stamps[j * self.nx + i0..=j * self.nx + i1])
            .copied()
            .max()
            .unwrap()
    }

    pub fn insert(&mut self, p: &Prop) {
        self.note_change(self.cell_range(p));
        self.place(p);
    }

    /// Index `p` without counting a change: a body already standing, newly
    /// held by a planning snapshot's own layer.
    fn place(&mut self, p: &Prop) {
        let (i0, i1, j0, j1) = self.cell_range(p);
        let entry = Entry {
            id: p.id,
            center: p.center,
            radius: p.footprint_radius(),
        };
        for j in j0..=j1 {
            for i in i0..=i1 {
                self.cells[j * self.nx + i].push(entry);
            }
        }
    }

    pub fn remove(&mut self, p: &Prop) {
        let (i0, i1, j0, j1) = self.cell_range(p);
        self.note_change((i0, i1, j0, j1));
        for j in j0..=j1 {
            for i in i0..=i1 {
                self.cells[j * self.nx + i].retain(|e| e.id != p.id);
            }
        }
    }

    /// Candidate ids whose footprint circle may meet the XY disc.
    pub fn near(&self, center: V2, radius: f64, out: &mut Vec<PropId>) {
        self.append_near(center, radius, out);
        out.sort_unstable();
        out.dedup();
    }

    /// Append one view's bucket entries before `near` canonicalizes its IDs.
    fn append_near(&self, center: V2, radius: f64, out: &mut Vec<PropId>) {
        let (i0, i1, j0, j1) = self.range(center, radius);
        for j in j0..=j1 {
            for i in i0..=i1 {
                out.extend(self.cells[j * self.nx + i].iter().map(|e| e.id));
            }
        }
    }

    /// Ascending unique candidates for a union of views. Merge the views'
    /// bucket intervals before reading props: overlapping eyes read each
    /// bucket once, rather than multiplying its entries before the ID sort.
    pub fn near_many(&self, views: &[(V2, f64)], out: &mut Vec<PropId>) {
        let mut rows = Vec::new();
        for &(center, radius) in views {
            let (i0, i1, j0, j1) = self.range(center, radius);
            rows.extend((j0..=j1).map(|j| (j, i0, i1)));
        }
        rows.sort_unstable();
        let mut run: Option<(usize, usize, usize)> = None;
        let mut read = |(j, i0, i1): (usize, usize, usize)| {
            for entries in &self.cells[j * self.nx + i0..=j * self.nx + i1] {
                out.extend(entries.iter().map(|e| e.id));
            }
        };
        for (j, i0, i1) in rows {
            if let Some((row, _, end)) = &mut run {
                if *row == j && i0 <= *end + 1 {
                    *end = (*end).max(i1);
                    continue;
                }
                read(run.take().unwrap());
            }
            run = Some((j, i0, i1));
        }
        if let Some(run) = run {
            read(run);
        }
        out.sort_unstable();
        out.dedup();
    }

    /// Whether `hit` holds for some prop whose footprint circle comes within
    /// `radius` (and a millimetre) of `center`, stopping at the first. Ids
    /// are not deduplicated: one in several buckets may be offered more than
    /// once.
    pub fn any_near(&self, center: V2, radius: f64, mut hit: impl FnMut(PropId) -> bool) -> bool {
        let (i0, i1, j0, j1) = self.range(center, radius);
        (j0..=j1).any(|j| {
            (i0..=i1).any(|i| {
                self.cells[j * self.nx + i].iter().any(|e| {
                    (e.center - center).within_radius(radius + e.radius + 1e-3) && hit(e.id)
                })
            })
        })
    }

    /// Whether `hit` holds for some prop whose footprint circle the XY
    /// segment `a`→`b` passes within a millimetre of, stopping at the first.
    /// Every prop a segment's hit could lie in is offered (a hit lies on the
    /// segment inside the footprint, so inside its circle). Ids are not
    /// deduplicated: one in several buckets may be offered more than once.
    pub fn any_along(&self, a: V2, b: V2, mut hit: impl FnMut(PropId) -> bool) -> bool {
        let ab = b - a;
        let len2 = ab.dot(ab);
        let meets = |e: &Entry| {
            let t = if len2 > 0.0 {
                ((e.center - a).dot(ab) / len2).clamp(0.0, 1.0)
            } else {
                0.0
            };
            (a + ab * t - e.center).within_radius(e.radius + 1e-3)
        };
        self.buckets_crossed(a, b, |entries| {
            entries.iter().any(|e| meets(e) && hit(e.id))
        })
    }

    /// Visit, until `visit` returns true, every bucket holding a point
    /// within a centimetre of the XY segment `a`→`b`: column by column,
    /// only the rows the segment spans there (a strip of buckets, not the
    /// segment's bounding box).
    fn buckets_crossed(&self, a: V2, b: V2, mut visit: impl FnMut(&[Entry]) -> bool) -> bool {
        const EPS: f64 = 1e-2;
        let cell = |v: f64, n: usize| ((v / self.bucket).floor().max(0.0) as usize).min(n - 1);
        // The segment's y over x ∈ [lo, hi] (clamped to its ends).
        let dx = b.x - a.x;
        let y_at = |x: f64| {
            if dx.abs() < 1e-12 {
                a.y
            } else {
                a.y + (b.y - a.y) * ((x - a.x) / dx).clamp(0.0, 1.0)
            }
        };
        let (x0, x1) = (a.x.min(b.x), a.x.max(b.x));
        for i in cell(x0 - EPS, self.nx)..=cell(x1 + EPS, self.nx) {
            // The column's span, the edge columns open to the map's outside
            // (the index clamps there too), widened by the margin.
            let lo = if i == 0 {
                f64::NEG_INFINITY
            } else {
                i as f64 * self.bucket
            };
            let hi = if i + 1 == self.nx {
                f64::INFINITY
            } else {
                (i + 1) as f64 * self.bucket
            };
            let (lo, hi) = ((lo - EPS).max(x0), (hi + EPS).min(x1));
            let (ya, yb) = if dx.abs() < 1e-12 {
                (a.y, b.y)
            } else {
                (y_at(lo), y_at(hi))
            };
            for j in cell(ya.min(yb) - EPS, self.ny)..=cell(ya.max(yb) + EPS, self.ny) {
                if visit(&self.cells[j * self.nx + i]) {
                    return true;
                }
            }
        }
        false
    }

    /// Candidate ids in the buckets the XY projection of the segment crosses
    /// (every prop a segment's hit could lie in), ascending.
    pub fn along(&self, a: V2, b: V2, out: &mut Vec<PropId>) {
        self.buckets_crossed(a, b, |entries| {
            out.extend(entries.iter().map(|e| e.id));
            false
        });
        out.sort_unstable();
        out.dedup();
    }
}

/// Every body by id, and the bucket index over them. The world owns its
/// bodies outright; a planning snapshot shares them and keeps only where its
/// side believes otherwise, so taking one costs the side's differences, not
/// the map's props. Queries answer as one index over the merged bodies would.
#[derive(Clone)]
pub(crate) struct PropStore {
    shared: Arc<Bodies>,
    /// A planning snapshot's own bodies; `None` in the world itself.
    own: Option<Box<Overlay>>,
}

#[derive(Clone)]
struct Bodies {
    props: Vec<Option<Prop>>,
    index: PropIndex,
}

/// The bodies a snapshot holds in place of `shared`'s, by id (`None`: none
/// there), and an index of those standing. The snapshot's change stamps live
/// here alone: its shared bodies are where it started.
#[derive(Clone)]
struct Overlay {
    props: BTreeMap<PropId, Option<Prop>>,
    index: PropIndex,
    len: usize,
}

impl PropStore {
    pub fn new(index: PropIndex) -> Self {
        PropStore {
            shared: Arc::new(Bodies {
                props: Vec::new(),
                index,
            }),
            own: None,
        }
    }

    /// A snapshot sharing these bodies, holding `differences` (each id's
    /// body instead, or none) in its own layer, over this store's own layer
    /// when it is itself a snapshot. Later entries for an id win. Its change
    /// stamps start at zero, as a freshly built index's do.
    pub fn planning_layer(
        &self,
        differences: impl Iterator<Item = (PropId, Option<Prop>)>,
    ) -> Self {
        let mut own = Overlay {
            props: self
                .own
                .as_ref()
                .map_or_else(BTreeMap::new, |own| own.props.clone()),
            index: self.shared.index.empty(),
            len: self.len(),
        };
        for (id, prop) in differences {
            own.props.insert(id, prop);
        }
        for (&id, prop) in &own.props {
            own.len = own.len.max(id as usize + 1);
            if let Some(prop) = prop {
                own.index.place(prop);
            }
        }
        own.index.track_changes();
        PropStore {
            shared: Arc::clone(&self.shared),
            own: Some(Box::new(own)),
        }
    }

    /// One past the highest id ever held.
    pub fn len(&self) -> usize {
        self.own.as_ref().map_or(self.shared.props.len(), |o| o.len)
    }

    pub fn get(&self, id: PropId) -> Option<&Prop> {
        if let Some(own) = &self.own {
            if let Some(prop) = own.props.get(&id) {
                return prop.as_ref();
            }
        }
        self.shared.props.get(id as usize)?.as_ref()
    }

    /// Every body, by ascending id.
    pub fn iter(&self) -> impl Iterator<Item = &Prop> {
        let own = self.own.as_deref();
        let shared_len = self.shared.props.len() as PropId;
        self.shared
            .props
            .iter()
            .enumerate()
            .filter_map(move |(id, prop)| match own {
                None => prop.as_ref(),
                Some(own) => own
                    .props
                    .get(&(id as PropId))
                    .map_or(prop.as_ref(), Option::as_ref),
            })
            .chain(
                own.into_iter()
                    .flat_map(move |own| own.props.range(shared_len..))
                    .filter_map(|(_, prop)| prop.as_ref()),
            )
    }

    /// Bodies with ids from `first` on, ascending.
    pub fn iter_from(&self, first: PropId) -> impl Iterator<Item = &Prop> {
        (first..self.len() as PropId).filter_map(|id| self.get(id))
    }

    /// Add a body under the next id.
    pub fn push(&mut self, prop: Prop) {
        debug_assert_eq!(prop.id as usize, self.len());
        match &mut self.own {
            None => {
                let bodies = Arc::make_mut(&mut self.shared);
                bodies.index.insert(&prop);
                bodies.props.push(Some(prop));
            }
            Some(own) => {
                own.index.insert(&prop);
                own.len += 1;
                own.props.insert(prop.id, Some(prop));
            }
        }
    }

    pub fn take(&mut self, id: PropId) -> Option<Prop> {
        match &mut self.own {
            None => {
                let bodies = Arc::make_mut(&mut self.shared);
                let prop = bodies.props.get_mut(id as usize)?.take()?;
                bodies.index.remove(&prop);
                Some(prop)
            }
            Some(own) => {
                let prop = match own.props.get_mut(&id) {
                    Some(slot) => slot.take()?,
                    None => {
                        let prop = self.shared.props.get(id as usize)?.clone()?;
                        own.props.insert(id, None);
                        prop
                    }
                };
                // Stamps the change; a shared body has no entry here to drop.
                own.index.remove(&prop);
                Some(prop)
            }
        }
    }

    /// Put back a body taken from its id, for a move.
    pub fn put(&mut self, prop: Prop) {
        let id = prop.id;
        match &mut self.own {
            None => {
                let bodies = Arc::make_mut(&mut self.shared);
                bodies.index.insert(&prop);
                bodies.props[id as usize] = Some(prop);
            }
            Some(own) => {
                own.index.insert(&prop);
                own.props.insert(id, Some(prop));
            }
        }
    }

    /// Change a body's flags in place: never its footprint.
    pub fn get_mut(&mut self, id: PropId) -> Option<&mut Prop> {
        match &mut self.own {
            None => Arc::make_mut(&mut self.shared)
                .props
                .get_mut(id as usize)?
                .as_mut(),
            Some(own) => {
                if !own.props.contains_key(&id) {
                    let prop = self.shared.props.get(id as usize)?.clone()?;
                    own.index.place(&prop);
                    own.props.insert(id, Some(prop));
                }
                own.props.get_mut(&id)?.as_mut()
            }
        }
    }

    pub fn track_changes(&mut self) {
        match &mut self.own {
            None => Arc::make_mut(&mut self.shared).index.track_changes(),
            Some(own) => own.index.track_changes(),
        }
    }

    /// Run an id query over the shared index and, for a snapshot, its own:
    /// ids the snapshot holds itself answer from its own index alone.
    fn ids(&self, out: &mut Vec<PropId>, query: impl Fn(&PropIndex, &mut Vec<PropId>)) {
        let Some(own) = &self.own else {
            return query(&self.shared.index, out);
        };
        let mut ids = Vec::new();
        query(&self.shared.index, &mut ids);
        ids.retain(|id| !own.props.contains_key(id));
        query(&own.index, &mut ids);
        out.extend(ids);
        out.sort_unstable();
        out.dedup();
    }

    pub fn near(&self, center: V2, radius: f64, out: &mut Vec<PropId>) {
        self.ids(out, |index, out| index.near(center, radius, out));
    }

    pub fn near_many(&self, views: &[(V2, f64)], out: &mut Vec<PropId>) {
        self.ids(out, |index, out| index.near_many(views, out));
    }

    pub fn along(&self, a: V2, b: V2, out: &mut Vec<PropId>) {
        self.ids(out, |index, out| index.along(a, b, out));
    }

    pub fn any_along(&self, a: V2, b: V2, mut hit: impl FnMut(PropId) -> bool) -> bool {
        match &self.own {
            None => self.shared.index.any_along(a, b, hit),
            Some(own) => {
                self.shared
                    .index
                    .any_along(a, b, |id| !own.props.contains_key(&id) && hit(id))
                    || own.index.any_along(a, b, hit)
            }
        }
    }

    pub fn any_near(&self, center: V2, radius: f64, mut hit: impl FnMut(PropId) -> bool) -> bool {
        match &self.own {
            None => self.shared.index.any_near(center, radius, hit),
            Some(own) => {
                self.shared
                    .index
                    .any_near(center, radius, |id| !own.props.contains_key(&id) && hit(id))
                    || own.index.any_near(center, radius, hit)
            }
        }
    }

    pub fn revision_near(&self, center: V2, radius: f64) -> u64 {
        match &self.own {
            None => self.shared.index.revision_near(center, radius),
            Some(own) => own.index.revision_near(center, radius),
        }
    }
}

/// Slab test of `o + d * t`, t ∈ [0, max_t], against the box of half extents
/// `half` centred at the local origin. Returns (t, local outward normal) of the
/// entry point; an origin already inside hits at t = 0.
pub(crate) fn ray_box(o: V3, d: V3, half: V3, max_t: f64) -> Option<(f64, V3)> {
    let (mut t0, mut t1) = (f64::NEG_INFINITY, f64::INFINITY);
    let mut axis0 = 0usize;
    let mut sign0 = 0.0;
    let h = [half.x, half.y, half.z];
    let (oa, da) = ([o.x, o.y, o.z], [d.x, d.y, d.z]);
    for a in 0..3 {
        if da[a].abs() < 1e-12 {
            if oa[a].abs() > h[a] {
                return None;
            }
            continue;
        }
        let mut near = (-h[a] - oa[a]) / da[a];
        let mut far = (h[a] - oa[a]) / da[a];
        let mut sign = -1.0;
        if near > far {
            std::mem::swap(&mut near, &mut far);
            sign = 1.0;
        }
        if near > t0 {
            t0 = near;
            axis0 = a;
            sign0 = sign;
        }
        t1 = t1.min(far);
        if t0 > t1 {
            return None;
        }
    }
    let t = if t0 >= 0.0 { t0 } else { 0.0 };
    if t > t1 || t > max_t || t1 < 0.0 {
        return None;
    }
    let mut n = [0.0; 3];
    n[axis0] = sign0;
    Some((t, v3(n[0], n[1], n[2])))
}

#[cfg(test)]
mod query_tests {
    use super::*;
    use crate::world::WorldGeometry;
    use contract::map::PropDefinition;

    #[test]
    fn overlapping_queries_keep_exact_candidates_through_index_edits() {
        let rules = serde_json::from_value(crate::fixtures::game()).unwrap();
        let map = serde_json::from_value(serde_json::json!({
            "size":[128,128],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,
            "props":[
                {"kind":"crate","center":[16,16],"yaw":0,"half_extents":[1,1,1]},
                {"kind":"crate","center":[40,16],"yaw":0,"half_extents":[1,1,1]},
                {"kind":"crate","center":[96,96],"yaw":0,"half_extents":[1,1,1]}
            ]
        }))
        .unwrap();
        let world = WorldGeometry::new(&map, &rules);
        let mut index = PropIndex::new(128.0, 128.0, 16.0);
        for prop in world.props() {
            index.insert(prop);
        }
        let views = [
            (v2(16.0, 16.0), 2.0),
            (v2(16.0, 16.0), 2.0),
            (v2(40.0, 16.0), 2.0),
            (v2(-3.0, -3.0), 1.0),
        ];
        let mut out = Vec::new();
        index.near_many(&views, &mut out);
        assert_eq!(out, vec![0, 1]);
        out.clear();
        index.near_many(&[], &mut out);
        assert!(out.is_empty());
        let mut prop = world.prop(0).unwrap().clone();
        index.remove(&prop);
        prop.center = v2(96.0, 16.0);
        index.insert(&prop);
        index.near_many(&views, &mut out);
        assert_eq!(out, vec![1]);
        out.clear();
        index.near_many(&[(v2(96.0, 16.0), 2.0), (v2(96.0, 96.0), 2.0)], &mut out);
        assert_eq!(out, vec![0, 2]);
        index.remove(&prop);
        out.clear();
        index.near_many(&[(v2(96.0, 16.0), 2.0)], &mut out);
        assert!(out.is_empty());
    }

    /// Each body's id and pose, by ascending id.
    fn bodies(world: &WorldGeometry) -> Vec<(PropId, [f64; 3])> {
        world
            .props()
            .map(|p| (p.id, [p.center.x, p.center.y, p.yaw]))
            .collect()
    }

    /// Every id query over `store` answers as one index built afresh over
    /// its bodies does, and its change stamps are `reference`'s.
    fn answers_as_whole(store: &PropStore, reference: &PropIndex) {
        let mut whole = reference.empty();
        for prop in store.iter() {
            whole.place(prop);
        }
        let (mut got, mut want) = (Vec::new(), Vec::new());
        for x in (0..128).step_by(8) {
            for y in (0..128).step_by(8) {
                let p = v2(x as f64, y as f64);
                for r in [1.0, 6.0, 40.0] {
                    got.clear();
                    want.clear();
                    store.near(p, r, &mut got);
                    whole.near(p, r, &mut want);
                    assert_eq!(got, want, "near {p:?} {r}");
                    assert_eq!(
                        store.revision_near(p, r),
                        reference.revision_near(p, r),
                        "stamps near {p:?} {r}"
                    );
                }
                let q = v2(127.0 - y as f64, x as f64 * 0.5);
                got.clear();
                want.clear();
                store.along(p, q, &mut got);
                whole.along(p, q, &mut want);
                assert_eq!(got, want, "along {p:?} {q:?}");
                for id in 0..8 {
                    assert_eq!(
                        store.any_along(p, q, |i| i == id),
                        whole.any_along(p, q, |i| i == id),
                        "any along {p:?} {q:?} meeting {id}"
                    );
                }
                let views = [(p, 4.0), (q, 10.0)];
                got.clear();
                want.clear();
                store.near_many(&views, &mut got);
                whole.near_many(&views, &mut want);
                assert_eq!(got, want, "near many {views:?}");
            }
        }
    }

    #[test]
    fn a_planning_snapshot_answers_as_a_world_of_its_sides_bodies() {
        let rules = serde_json::from_value(crate::fixtures::game()).unwrap();
        let map = serde_json::from_value(serde_json::json!({
            "size":[128,128],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,
            "props":[
                {"kind":"crate","center":[16,16],"yaw":0,"half_extents":[1,1,1]},
                {"kind":"crate","center":[40,16],"yaw":0,"half_extents":[1,1,1]},
                {"kind":"crate","center":[64,62],"yaw":0,"half_extents":[3,1,1]},
                {"kind":"crate","center":[96,96],"yaw":0,"half_extents":[1,1,1]}
            ]
        }))
        .unwrap();
        let mut world = WorldGeometry::new(&map, &rules);
        let crate_at = |x: f64, y: f64| PropDefinition {
            kind: "crate".into(),
            center: [x, y],
            yaw: 0.0,
            half_extents: [1.0, 1.0, 1.0],
            base_z: None,
        };
        // The truth moves on: 1 is shoved, 2 destroyed, 4 added.
        let unshoved = world.prop(1).unwrap().clone();
        world.move_prop(1, v2(40.0, 70.0), 0.5, 1);
        let gone = world.remove_prop(2).unwrap();
        assert_eq!(world.add_prop(&crate_at(30.0, 100.0)), 4);
        let truth = bodies(&world);

        // A side that did not see any of it.
        let mut snapshot = world.planning_snapshot(
            [
                (4, None),
                (1, Some(unshoved.clone())),
                (2, Some(gone.clone())),
            ]
            .into_iter(),
        );
        assert_eq!(
            bodies(&snapshot),
            [
                (0, [16.0, 16.0, 0.0]),
                (1, [40.0, 16.0, 0.0]),
                (2, [64.0, 62.0, 0.0]),
                (3, [96.0, 96.0, 0.0])
            ]
        );
        let mut reference = snapshot.props.shared.index.empty();
        reference.track_changes();
        answers_as_whole(&snapshot.props, &reference);

        // Its rehearsal shoves, destroys and adds without touching the truth.
        let moved = |p: &Prop, c: V2| Prop {
            center: c,
            ..p.clone()
        };
        let before = snapshot.prop(0).unwrap().clone();
        snapshot.move_prop(0, v2(20.0, 50.0), 0.0, 1);
        reference.remove(&before);
        reference.insert(&moved(&before, v2(20.0, 50.0)));
        let removed = snapshot.remove_prop(3).unwrap();
        reference.remove(&removed);
        let removed = snapshot.remove_prop(1).unwrap();
        assert_eq!(removed.center, unshoved.center);
        reference.remove(&removed);
        assert_eq!(snapshot.add_prop(&crate_at(100.0, 20.0)), 5);
        reference.insert(snapshot.prop(5).unwrap());
        snapshot.set_known_to_all(2);
        assert!(snapshot.prop(2).unwrap().known_to_all);
        assert_eq!(
            bodies(&snapshot),
            [
                (0, [20.0, 50.0, 0.0]),
                (2, [64.0, 62.0, 0.0]),
                (5, [100.0, 20.0, 0.0])
            ]
        );
        answers_as_whole(&snapshot.props, &reference);
        assert_eq!(bodies(&world), truth);
        assert!(!world.prop(4).unwrap().known_to_all);

        // A snapshot of the snapshot plans over the snapshot's bodies.
        let nested = snapshot.planning_snapshot([(0, None)].into_iter());
        assert_eq!(
            bodies(&nested),
            [(2, [64.0, 62.0, 0.0]), (5, [100.0, 20.0, 0.0])]
        );
        let mut reference = reference.empty();
        reference.track_changes();
        answers_as_whole(&nested.props, &reference);
        assert_eq!(bodies(&world), truth);
    }
}
