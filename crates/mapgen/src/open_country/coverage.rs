//! Physical ground-sight witnesses, constructed once from the complete
//! authored map. Coverage cells ask this point index, never a scene ray query.
use super::*;
use crate::layout::geometry::{segment_bounds, Grid};
use contract::forest::{body_contains, TrunkQueries, FOLIAGE_SAMPLE_M};
use contract::generation_physics::GenerationPhysics;
use contract::map::MapDefinition;
use std::{cell::Cell, cmp::Ordering, rc::Rc};

/// Road primitives use the contract's exact stroke cuts and polygon membership.
enum Road<'a> {
    Segment {
        a: Point,
        b: Point,
        cuts: u8,
        half: f64,
    },
    Polygon(&'a GroundShape),
}

// Proof work only, independent of authored compile limits. Charged geometry
// predicates and raster reads stop at this envelope; unsupported edited
// spacing is refused before the shared candidate sampler runs.
const PROOF_OPERATIONS: u64 = 512_000_000;
/// Bound a complete line before either raw samples or grove exclusions are
/// allocated. Thinning checks every earlier sample in the worst case, so
/// its quadratic work belongs here as well as both width-spaced walks.
pub(super) fn tree_line_work(length: f64, rule: TreeLines) -> Result<u64, String> {
    let stretches = libm::ceil(length / rule.stretch_m[0]) + 1.;
    let points = libm::ceil(2. * length / rule.width_m) + 2. * stretches;
    // Centimetre rounding can lengthen every retained segment by at most
    // sqrt(2) centimetres. Each segment's inclusive walk adds two nodes.
    let groves = libm::ceil((length + points * core::f64::consts::SQRT_2 * 0.01) / rule.width_m)
        + 2. * points;
    let work = points * points / 2. + 4. * points + groves;
    if !work.is_finite() || work >= PROOF_OPERATIONS as f64 {
        return Err("tree-line samples exceed the ground-sight operation envelope".into());
    }
    Ok(libm::ceil(work) as u64)
}

pub(super) fn preflight(size: Point, fog: f64, cell: f64, range: f64) -> Result<(), String> {
    let raster = libm::ceil(size[0] / fog) * libm::ceil(size[1] / fog);
    let locations = libm::ceil(size[0] / cell) * libm::ceil(size[1] / cell);
    // Reach is always below the physical circle. Forecast the complete
    // near-first square and its sort before allocating the placement list.
    let placement_axis = 2. * libm::floor(range / (cell / 2.).min(fog)) + 1.;
    let offsets = placement_axis * placement_axis;
    let sort = offsets * libm::ceil(libm::log2(offsets.max(1.)));
    if ![raster, locations, offsets, sort]
        .iter()
        .all(|n| n.is_finite() && *n >= 1.)
        || 16. * raster + 16. * locations + sort > PROOF_OPERATIONS as f64
    {
        return Err("ground-sight resolution exceeds its operation envelope".into());
    }
    Ok(())
}

pub(super) struct Work(Cell<u64>);
impl Work {
    pub(super) fn spend(&self, n: u64) -> bool {
        let left = self.0.get();
        self.0.set(left.saturating_sub(n));
        n < left
    }
    fn check(&self) -> Result<(), String> {
        if self.0.get() == 0 {
            Err("ground-sight proof exceeds its operation envelope".into())
        } else {
            Ok(())
        }
    }
}

struct Queries<'a> {
    work: Rc<Work>,
    size: Point,
    water: Water<'a>,
    roads: Vec<Road<'a>>,
    road_index: Grid,
    bodies: Vec<PropDefinition>,
    body_index: Grid,
}

impl<'a> Queries<'a> {
    fn new(map: &'a MapDefinition, clearance: f64) -> Result<Self, String> {
        let work = Rc::new(Work(Cell::new(PROOF_OPERATIONS)));
        let water = Water::new_charged(&map.rivers, map.size, || work.spend(1))
            .ok_or("water index exceeds ground-sight operation envelope")?;
        let mut q = Self {
            work,
            // The world's sampled height field has closed, rounded bounds.
            size: map
                .size
                .map(|n| (n / map.height_grid_m).round() * map.height_grid_m),
            water,
            roads: Vec::new(),
            road_index: Grid::new(map.size, 64.),
            bodies: map.authored_props()?.into_iter().map(|(_, p)| p).collect(),
            body_index: Grid::new(map.size, 64.),
        };
        q.bodies.extend(map.bridges.iter().map(|b| PropDefinition {
            kind: b.deck.clone(),
            center: b.center,
            yaw: b.yaw,
            half_extents: [b.half_extents[0], b.half_extents[1], b.thickness_m / 2.],
            base_z: Some(b.deck_z - b.thickness_m),
            wreck_of: None,
        }));
        for area in map.surfaces.iter().filter(|s| s.kind.is_road()) {
            match &area.shape {
                GroundShape::Polygon { .. } => {
                    let [x0, y0, x1, y1] = area.shape.limits();
                    let r = clearance * core::f64::consts::SQRT_2;
                    q.road_index.insert_charged(
                        [x0 - r, y0 - r, x1 + r, y1 + r],
                        q.roads.len() as u32,
                        || q.work.spend(1),
                    );
                    q.roads.push(Road::Polygon(&area.shape));
                }
                GroundShape::Stroke {
                    centerline,
                    width_m,
                } => {
                    for (a, b, cuts) in
                        contract::ground::stretches(centerline.samples(), width_m / 2.)
                    {
                        q.road_index.insert_charged(
                            segment_bounds(
                                a,
                                b,
                                width_m / 2. + clearance * core::f64::consts::SQRT_2,
                            ),
                            q.roads.len() as u32,
                            || q.work.spend(1),
                        );
                        q.roads.push(Road::Segment {
                            a,
                            b,
                            cuts,
                            half: width_m / 2.,
                        });
                    }
                }
            }
        }
        for (i, b) in q.bodies.iter().enumerate() {
            let r = libm::hypot(b.half_extents[0] + clearance, b.half_extents[1] + clearance);
            q.body_index.insert_charged(
                [
                    b.center[0] - r,
                    b.center[1] - r,
                    b.center[0] + r,
                    b.center[1] + r,
                ],
                i as u32,
                || q.work.spend(1),
            );
        }
        q.work.check()?;
        Ok(q)
    }
}

impl TrunkQueries for Queries<'_> {
    fn keep_sampling(&self) -> bool {
        self.work.0.get() > 0
    }
    fn road_near(&self, p: Point, margin: f64) -> bool {
        self.road_index.any_charged(
            [p[0] - margin, p[1] - margin, p[0] + margin, p[1] + margin],
            |i| match self.roads[i as usize] {
                Road::Polygon(shape) => {
                    let edges = match shape {
                        GroundShape::Polygon { ring } => ring.len(),
                        _ => unreachable!(),
                    };
                    !self.work.spend(edges as u64) || shape.contains(p, margin)
                }
                Road::Segment { a, b, cuts, half } => {
                    contract::ground::stretch_contains(a, b, cuts, half, p, margin)
                }
            },
            || self.work.spend(1),
        )
    }
    fn water_near(&self, p: Point, margin: f64) -> bool {
        self.water.near_charged(p, margin, || self.work.spend(1))
    }
    fn body_near(&self, p: Point, margin: f64) -> bool {
        // Stored body bounds already include the rule's trunk clearance.
        self.body_index.any_charged(
            [p[0], p[1], p[0], p[1]],
            |i| {
                let b = &self.bodies[i as usize];
                body_contains(
                    b.center,
                    [b.half_extents[0], b.half_extents[1]],
                    b.yaw,
                    p,
                    margin,
                )
            },
            || self.work.spend(1),
        )
    }
    fn contains_ground(&self, p: Point) -> bool {
        p[0] >= 0. && p[1] >= 0. && p[0] <= self.size[0] && p[1] <= self.size[1]
    }
}

pub(super) struct Coverage<'a> {
    queries: Queries<'a>,
    physics: &'a GenerationPhysics,
    cell: f64,
    nx: usize,
    ny: usize,
    occupied: Vec<bool>,
    physical: Vec<bool>,
    witnessed: Vec<bool>,
    points: Vec<Point>,
    index: Grid,
    far_samples: Vec<(f64, f64)>,
    /// Physical witness reach reserves following fog steps. Every corner
    /// of the complete clipped rectangle must fit inside it.
    pub reach: f64,
}

impl<'a> Coverage<'a> {
    pub fn new(
        map: &'a MapDefinition,
        physics: &'a GenerationPhysics,
        cell: f64,
    ) -> Result<Self, String> {
        if map
            .size
            .iter()
            .any(|n| (n / map.height_grid_m).round() * map.height_grid_m != *n)
        {
            return Err(
                "ground-sight proof requires the generated and sampled height extents to agree"
                    .into(),
            );
        }
        let fog = map.fog_cell_m;
        if 1.5 * fog <= FOLIAGE_SAMPLE_M {
            return Err("ground-sight witness is narrower than physical foliage sampling".into());
        }
        let bank_margin = map
            .rivers
            .iter()
            .map(|r| {
                let grade = r
                    .samples()
                    .iter()
                    .map(|p| p.depth_m / p.half_width_m)
                    .fold(f64::INFINITY, f64::min);
                -r.surface_z().min(0.) / grade
            })
            .fold(0., f64::max)
            + map.height_grid_m * core::f64::consts::SQRT_2;
        if !bank_margin.is_finite() {
            return Err("ground-sight proof needs a finite dry-bank clearance".into());
        }
        let range = physics.circular_range_m()?;
        let eye = physics
            .circular_ground_observers()
            .map(|u| u.hull().map_or(physics.infantry_eye_m, |h| h.eye_m))
            .fold(physics.fog_target_height_m, f64::max);
        let high = map.bridges.iter().map(|b| b.deck_z).fold(0., f64::max);
        // Generated land has no relief. Each river can only lower it;
        // summing each possible fall is conservative even for overlap.
        if !map.relief.is_empty() {
            return Err("ground-sight coverage does not support relief yet".into());
        }
        let low = map
            .rivers
            .iter()
            .map(|r| {
                r.surface_z().min(0.) - r.samples().iter().map(|p| p.depth_m).fold(0., f64::max)
            })
            .sum::<f64>();
        let tree = physics.catalog.props().by_id(&physics.forests.tree).body;
        let rule = physics.forests.rule;
        if low + rule.canopy_height_m <= high + eye {
            return Err("forest canopy does not stand above ground eyes and targets across the generated height span".into());
        }
        let mut reach = f64::INFINITY;
        let mut far_samples = Vec::new();
        for observer in physics.circular_ground_observers() {
            let range = observer.sensors.ground_m * observer.sensors.sight_shape.front;
            let rays = (libm::ceil(TAU * range / fog) as usize).max(64);
            let delta = TAU / rays as f64;
            let far = libm::floor(range / fog) * fog;
            let diagonal = core::f64::consts::SQRT_2 * fog;
            // The nearest native ray differs from the witness bearing by
            // at most half an angular step. This bounds either coordinate
            // of its far endpoint relative to the exact witness bearing.
            far_samples.push((far, 2. * far * libm::sin(delta / 4.) + diagonal));
            // Three neighbouring rays must stop before any sample could enter
            // the central ray's far cell. Every other ray stays more than that
            // cell's diagonal from its chosen sample, even at another radius.
            if (far * libm::sin(2. * delta)).partial_cmp(&diagonal) != Some(Ordering::Greater) {
                return Err("ground circle rays cannot isolate a far fog cell".into());
            }
            let depth = rule.attenuation_per_m * tree.conceals * fog;
            if (range * libm::exp(-depth)).partial_cmp(&(far - diagonal)) != Some(Ordering::Less) {
                return Err("one occupied foliage step cannot isolate a far fog cell".into());
            }
            // A fully occupied 3x3 block contains a 1.5-cell disk. The nearest
            // ray and its two neighbours are at most 1.5 angular steps away;
            // rounding along their rays adds at most half a radial step.
            let q = 2. * (1. - libm::cos(1.5 * delta));
            if !(q.is_finite() && q > 0.) {
                return Err("ground-sight angular resolution cannot earn a witness margin".into());
            }
            let angular = (-fog / 2. + libm::sqrt(fog * fog / 4. + 8. * fog * fog / q)) / 2.;
            let before_far = range - (fog / 2. + fog + diagonal);
            let admitted = (libm::ceil(angular.min(before_far) / fog) - 1.) * fog;
            reach = reach.min(admitted);
        }
        if !(reach.is_finite() && reach - cell / core::f64::consts::SQRT_2 > 3. * fog) {
            return Err("ground circle leaves no whole-cell furnishing margin".into());
        }
        preflight(map.size, fog, cell, range)?;
        let nx = libm::floor(map.size[0] / fog) as usize;
        let ny = libm::floor(map.size[1] / fog) as usize;
        let count = nx
            .checked_mul(ny)
            .filter(|n| *n >= 9 && *n as u64 <= PROOF_OPERATIONS / 16)
            .ok_or("ground-sight raster exceeds its operation envelope")?;
        if nx < 3 || ny < 3 {
            return Err("ground-sight raster has no complete witness patch".into());
        }
        let mut c = Self {
            queries: Queries::new(map, rule.trunk_clearance_m)?,
            physics,
            cell: fog,
            nx,
            ny,
            occupied: vec![false; count],
            physical: vec![false; count],
            witnessed: vec![false; nx * ny],
            points: Vec::new(),
            index: Grid::new(map.size, cell),
            far_samples,
            reach,
        };
        c.queries.work.spend(count as u64 * 3);
        // Tall physical parts on flat supporting ground also contribute to
        // the exact fog raster. Keep dry-bank interpolation away from them.
        for b in &c.queries.bodies {
            let body = physics
                .catalog
                .props()
                .index(&b.kind)
                .ok_or_else(|| format!("unknown physical body {}", b.kind))?;
            let bottom = b.base_z.unwrap_or(0.);
            let radius = libm::hypot(b.half_extents[0], b.half_extents[1]);
            c.queries.work.spend(1);
            c.queries.work.check()?;
            if !physics.catalog.props().get(body).body.occludes
                || bottom > 0.
                || bottom + 2. * b.half_extents[2] <= high + eye
            {
                continue;
            }
            let near_water = c
                .queries
                .water
                .gap_charged(b.center, radius + bank_margin, || c.queries.work.spend(1));
            c.queries.work.check()?;
            if near_water < radius + bank_margin {
                continue;
            }
            let (x0, y0, x1, y1) = c.bounds(b.center, radius);
            for y in y0..=y1 {
                for x in x0..=x1 {
                    if !c.queries.work.spend(1) {
                        return Err("body raster exceeds ground-sight operation envelope".into());
                    }
                    if body_contains(
                        b.center,
                        [b.half_extents[0], b.half_extents[1]],
                        b.yaw,
                        c.middle(x, y),
                        0.,
                    ) {
                        c.occupied[y * nx + x] = true;
                        // Keep a metre inside the real body so the continuous
                        // physical sight claim does not rest on a raster edge.
                        c.physical[y * nx + x] = body_contains(
                            b.center,
                            [b.half_extents[0], b.half_extents[1]],
                            b.yaw,
                            c.middle(x, y),
                            -FOLIAGE_SAMPLE_M,
                        );
                    }
                }
            }
        }
        for (i, f) in map.forests.iter().enumerate() {
            c.stamp_forest(i, f, None)?;
        }
        c.note_patches(0, 0, nx - 1, ny - 1);
        c.queries.work.check()?;
        Ok(c)
    }

    fn middle(&self, x: usize, y: usize) -> Point {
        [(x as f64 + 0.5) * self.cell, (y as f64 + 0.5) * self.cell]
    }
    fn bounds(&self, p: Point, r: f64) -> (usize, usize, usize, usize) {
        let at = |v: f64, n: usize| (libm::floor(v / self.cell).max(0.) as usize).min(n - 1);
        (
            at(p[0] - r, self.nx),
            at(p[1] - r, self.ny),
            at(p[0] + r, self.nx),
            at(p[1] + r, self.ny),
        )
    }

    fn stamp_forest(
        &mut self,
        i: usize,
        f: &Forest,
        mut changed: Option<&mut Vec<(usize, bool)>>,
    ) -> Result<usize, String> {
        let rule = self.physics.forests.rule;
        let bounds = f.shape.limits();
        let step = rule.trunk_spacing_m;
        if bounds
            .iter()
            .any(|v| !(*v + step).is_finite() || *v + step <= *v)
        {
            return Err("forest lattice cannot advance at the generated bounds".into());
        }
        let width = libm::ceil((bounds[2] - bounds[0]) / step) + 1.;
        let height = libm::ceil((bounds[3] - bounds[1]) / step) + 1.;
        let edges = match &f.shape {
            GroundShape::Polygon { ring } => ring.len(),
            GroundShape::Stroke { centerline, .. } => centerline.samples().len() * 2,
        };
        let forecast = width * height * (edges + 4) as f64;
        if !forecast.is_finite()
            || forecast > self.queries.work.0.get() as f64
            || !self.queries.work.spend(forecast as u64)
        {
            return Err("forest candidates exceed the ground-sight operation envelope".into());
        }
        let mut trunks = 0;
        for p in contract::forest::trunk_positions(i, f, &rule, &self.queries) {
            trunks += 1;
            let r = rule.canopy_radius_m;
            let (x0, y0, x1, y1) = self.bounds(p, r);
            for y in y0..=y1 {
                for x in x0..=x1 {
                    if !self.queries.work.spend(1) {
                        return Err("foliage raster exceeds ground-sight operation envelope".into());
                    }
                    let mid = self.middle(x, y);
                    if distance(mid, p) <= r {
                        let k = y * self.nx + x;
                        if !self.occupied[k] {
                            if let Some(changes) = changed.as_mut() {
                                changes.push((k, false));
                            }
                            self.occupied[k] = true;
                        }
                        // foliage_depth samples every metre and clips the
                        // forest's canopy-expanded bounds. Certify a whole
                        // final step inside both owners of that foliage.
                        if distance(mid, p) <= r - FOLIAGE_SAMPLE_M
                            && mid[0] > bounds[0] - r + FOLIAGE_SAMPLE_M
                            && mid[0] < bounds[2] + r - FOLIAGE_SAMPLE_M
                            && mid[1] > bounds[1] - r + FOLIAGE_SAMPLE_M
                            && mid[1] < bounds[3] + r - FOLIAGE_SAMPLE_M
                            && !self.physical[k]
                        {
                            if let Some(changes) = changed.as_mut() {
                                changes.push((k, true));
                            }
                            self.physical[k] = true;
                        }
                    }
                }
            }
        }
        self.queries.work.check()?;
        Ok(trunks)
    }

    /// Candidate foliage is staged. A failed proposal cannot contribute
    /// phantom cells or prevent a later proposal from finding physical room.
    pub fn try_forests(
        &mut self,
        start: usize,
        forests: &[Forest],
        target: Option<(Point, Point)>,
    ) -> Result<bool, String> {
        let mut changed = Vec::new();
        let mut bounds = [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ];
        let mut trunks = 0;
        for (j, f) in forests.iter().enumerate() {
            trunks += self.stamp_forest(start + j, f, Some(&mut changed))?;
            let b = f.shape.limits();
            for i in 0..2 {
                bounds[i] = bounds[i].min(b[i]);
                bounds[i + 2] = bounds[i + 2].max(b[i + 2]);
            }
        }
        let r = self.physics.forests.rule.canopy_radius_m + self.cell;
        let (x0, y0, _, _) = self.bounds([bounds[0], bounds[1]], r);
        let (_, _, x1, y1) = self.bounds([bounds[2], bounds[3]], r);
        let points = self.patches(x0, y0, x1, y1);
        let earned = target.map_or(trunks > 0, |(low, high)| {
            points.iter().any(|(_, p)| self.admits(*p, low, high))
        });
        if earned {
            self.commit_patches(points);
        } else {
            for (k, physical) in changed.into_iter().rev() {
                if physical {
                    self.physical[k] = false;
                } else {
                    self.occupied[k] = false;
                }
            }
        }
        self.queries.work.check()?;
        Ok(earned)
    }

    fn patches(&self, x0: usize, y0: usize, x1: usize, y1: usize) -> Vec<(usize, Point)> {
        let mut points = Vec::new();
        for y in y0.max(1)..=y1.min(self.ny - 2) {
            for x in x0.max(1)..=x1.min(self.nx - 2) {
                let k = y * self.nx + x;
                if !self.queries.work.spend(10) {
                    return points;
                }
                if !self.physical[k]
                    || self.witnessed[k]
                    || !(y - 1..=y + 1)
                        .all(|j| (x - 1..=x + 1).all(|i| self.occupied[j * self.nx + i]))
                {
                    continue;
                }
                points.push((k, self.middle(x, y)));
            }
        }
        points
    }
    fn commit_patches(&mut self, points: Vec<(usize, Point)>) {
        for (k, p) in points {
            if !self.index.insert_charged(
                [p[0], p[1], p[0], p[1]],
                self.points.len() as u32,
                || self.queries.work.spend(1),
            ) {
                return;
            }
            self.points.push(p);
            self.witnessed[k] = true;
        }
    }
    fn note_patches(&mut self, x0: usize, y0: usize, x1: usize, y1: usize) {
        self.commit_patches(self.patches(x0, y0, x1, y1));
    }

    /// The whole rectangle shares a physical patch and an isolated *in-map*
    /// far fog cell. At a boundary, an outward-only patch cannot earn this.
    pub(super) fn admits(&self, witness: Point, low: Point, high: Point) -> bool {
        if !self.queries.work.spend(1) {
            return false;
        }
        let nearest = [
            witness[0].clamp(low[0], high[0]),
            witness[1].clamp(low[1], high[1]),
        ];
        let near = distance(witness, nearest);
        let far = [low, [low[0], high[1]], [high[0], low[1]], high]
            .into_iter()
            .map(|corner| distance(witness, corner))
            .fold(0., f64::max);
        if !near.is_finite() || !far.is_finite() || far > self.reach {
            return false;
        }
        let direction = [0, 1].map(|i| {
            // The bearing is undefined at the witness itself. Admit all
            // possible directions only when the entire circle fits here.
            if near == 0. {
                return [-1., 1.];
            }
            let a = witness[i] - high[i];
            let b = witness[i] - low[i];
            let j = 1 - i;
            let other = [witness[j] - high[j], witness[j] - low[j]];
            let closest = if other[0] <= 0. && other[1] >= 0. {
                0.
            } else {
                other[0].abs().min(other[1].abs())
            };
            let furthest = other[0].abs().max(other[1].abs());
            // a/hypot(a,b) increases with a, decreases with |b| when
            // a is positive, and increases with |b| when a is negative.
            // Axis intervals are independent in the clipped rectangle.
            let lower = a / libm::hypot(a, if a < 0. { closest } else { furthest });
            let upper = b / libm::hypot(b, if b > 0. { closest } else { furthest });
            [lower, upper]
        });
        self.far_samples.iter().all(|(distance, margin)| {
            [0, 1].into_iter().all(|i| {
                self.queries.work.spend(1)
                    && low[i] + distance * direction[i][0] >= *margin
                    && high[i] + distance * direction[i][1] <= self.queries.size[i] - margin
            })
        })
    }

    pub fn covered(&self, low: Point, high: Point) -> bool {
        let p = scale(add(low, high), 0.5);
        let reach = self.reach;
        self.index.any_charged(
            [p[0] - reach, p[1] - reach, p[0] + reach, p[1] + reach],
            |i| self.admits(self.points[i as usize], low, high),
            || self.queries.work.spend(1),
        )
    }

    pub fn work(&self) -> Rc<Work> {
        Rc::clone(&self.queries.work)
    }
    pub fn check(&self) -> Result<(), String> {
        self.queries.work.check()
    }

    pub fn body_clear(&self, p: Point, r: f64) -> bool {
        // Placement is conservatively clear of every actual authored body.
        !self.queries.body_index.any_charged(
            [p[0] - r, p[1] - r, p[0] + r, p[1] + r],
            |i| {
                let b = &self.queries.bodies[i as usize];
                distance(p, b.center) < r + libm::hypot(b.half_extents[0], b.half_extents[1])
            },
            || self.queries.work.spend(1),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn physics() -> GenerationPhysics {
        GenerationPhysics::from_rules_json(&sim::fixtures::test_game().to_string()).unwrap()
    }
    fn map(size: f64, bounds: [f64; 4]) -> MapDefinition {
        serde_json::from_value(serde_json::json!({
            "size":[size,size], "fog_cell_m":8., "height_grid_m":4., "slope_cutoff_deg":35.,
            "forests":[{"shape":{"kind":"polygon","ring":[
                [bounds[0],bounds[1]],[bounds[2],bounds[1]],
                [bounds[2],bounds[3]],[bounds[0],bounds[3]]
            ]}}], "props":[],"surfaces":[],"relief":[],"buildings":[],"bridges":[]
        }))
        .unwrap()
    }

    #[test]
    fn the_current_twenty_kilometre_forest_profile_fits_the_proof_envelope() {
        let physics = physics();
        let map = map(20_000., [0., 0., 20_000., 20_000.]);
        let c = Coverage::new(&map, &physics, 100.).unwrap();
        for p in [[50., 50.], [10_050., 10_050.], [19_950., 19_950.]] {
            assert!(c.covered(sub(p, [50., 50.]), add(p, [50., 50.])));
        }
        c.check().unwrap();
        eprintln!(
            "20km full-forest charged structural work={} witnesses={}",
            PROOF_OPERATIONS - c.queries.work.0.get(),
            c.points.len()
        );
    }

    #[test]
    fn whole_rectangle_reach_uses_its_furthest_corner() {
        let mut physics = physics();
        physics.forests.rule.trunk_spacing_m = 9.;
        physics.forests.rule.trunk_jitter = 0.;
        physics.forests.rule.canopy_radius_m = 12.;
        let map = map(2_000., [999.5, 999.5, 1008.5, 1008.5]);
        let c = Coverage::new(&map, &physics, 100.).unwrap();
        assert!(c.covered([954., 604.], [1054., 704.]));
        // Its centre is within reach, but its far corner is not.
        assert!(!c.covered([954., 584.], [1054., 704.]));
    }

    #[test]
    fn an_outward_only_patch_cannot_certify_an_in_bounds_fog_cut() {
        let mut physics = physics();
        physics.forests.rule.trunk_spacing_m = 9.;
        physics.forests.rule.trunk_jitter = 0.;
        physics.forests.rule.canopy_radius_m = 12.;
        let map = map(2_000., [7.5, 999.5, 16.5, 1008.5]);
        let c = Coverage::new(&map, &physics, 100.).unwrap();
        assert_eq!(c.points, vec![[12., 1004.]]);
        // The only foliage is west of this eye. Its attenuated rays leave
        // the map before stopping, so it supplies no in-bounds dark bit.
        assert!(!c.covered([112., 1004.], [112., 1004.]));
    }

    #[test]
    fn edited_spacing_is_refused_before_the_shared_sampler_can_expand_it() {
        let mut physics = physics();
        physics.forests.rule.trunk_spacing_m = 1e-9;
        let map = map(2_000., [900., 900., 1_100., 1_100.]);
        let error = Coverage::new(&map, &physics, 100.).err().unwrap();
        assert!(error.contains("operation envelope"), "{error}");
    }

    #[test]
    fn a_three_cell_witness_keeps_its_far_bit_dark_across_native_ray_phases() {
        let mut physics = physics();
        physics.forests.rule.trunk_spacing_m = 9.;
        physics.forests.rule.trunk_jitter = 0.;
        physics.forests.rule.canopy_radius_m = 12.;
        let map = map(2_000., [999.5, 999.5, 1008.5, 1008.5]);
        let c = Coverage::new(&map, &physics, 100.).unwrap();
        assert_eq!(c.points, vec![[1004., 1004.]]);
        let rules: contract::scenario::Rules =
            serde_json::from_value(sim::fixtures::test_game()).unwrap();
        let mut rules = rules;
        rules.forests = physics.forests.clone();
        let world = sim::world::WorldGeometry::new(&map, &rules);
        let jeep = rules.catalog.by_id("test_jeep");
        let sight = sim::sight::Sight {
            forward: 0.,
            shape: jeep.sensors.sight_shape,
            range: jeep.sensors.ground_m,
        };
        let rays = (libm::ceil(TAU * sight.max_range() / map.fog_cell_m) as usize).max(64);
        let far = libm::floor(sight.range / map.fog_cell_m) * map.fog_cell_m;
        let delta = TAU / rays as f64;
        let mut grid = sim::visibility::OcclusionGrid::new(&world, map.fog_cell_m);
        for radial in 0..32 {
            let distance = c.reach - radial as f64 * map.fog_cell_m / 32.;
            for r in 0..rays {
                for phase in [0., 0.5] {
                    let angle = (r as f64 + phase) * delta;
                    let at = sub(c.points[0], scale(direction(angle), distance));
                    let eye = sim::math::v3(at[0], at[1], jeep.hull().unwrap().eye_m);
                    let mut field = grid.field();
                    sim::visibility::sweep(
                        &world,
                        &mut grid,
                        &rules.sensors,
                        eye,
                        &sight,
                        &mut field,
                    );
                    let nearest = libm::round(angle / delta) * delta;
                    let end = add(at, scale(direction(nearest), far));
                    assert!(
                        !field.visible(end[0], end[1]),
                        "far cell refilled at ray {r} phase {phase}, reach {}",
                        c.reach
                    );
                }
            }
        }
    }
}
