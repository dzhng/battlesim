//! Route planning over one side's known geometry. A 2 m grid classifies each
//! cell by the walkable surface under its centre (the same triangle rule the
//! world uses) and, per mover class, by the known props that block that
//! class; a clearance field per class lets a footprint of any width ask
//! whether it fits. Plans are A* over octile moves
//! without corner cutting, then string-pulled only where that keeps the cost.
//!
//! Infantry reads the grid at two resolutions (Q27). Each 2 m cell holds a
//! 4×4 mask of 0.5 m sub-cells, set where a soldier's disc stands clear of
//! every known body. A cell is open to a squad when its free sub-cells form
//! one connected gap, and a move between two cells when free sub-cells meet
//! across their shared edge: a line of teeth or a gap between wrecks stays
//! open, a wall stays closed. Soldiers then find their own way through the
//! gap on the exact bodies (`movement::final_leg`).
use std::cmp::Ordering;
use std::collections::BinaryHeap;

use contract::command::RoutePolicy;
use contract::map::MoverClass;

use crate::math::{v2, Obb2, V2};
use crate::world::{Prop, SurfaceKind, WorldGeometry};

pub const NAV_CELL_M: f64 = 2.0;
/// Sub-cells per cell side for infantry: 0.5 m.
const SUB: usize = 4;
const SUB_M: f64 = NAV_CELL_M / SUB as f64;
/// Every sub-cell of a cell free.
const ALL_FREE: u16 = u16::MAX;
/// Clearance is a distance transform capped here; wider footprints do not exist.
const MAX_CLEARANCE_M: f64 = 16.0;

/// How a unit class moves: its speeds on each surface and how wide it is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mobility {
    pub off_road_mps: f64,
    pub road_mps: f64,
    pub forest_multiplier: f64,
    /// Distance the footprint needs from any obstacle centre line.
    pub half_width_m: f64,
    /// Which props stop it.
    pub class: MoverClass,
}

impl Mobility {
    /// Travel speed on a surface: roads take precedence over forest; slopes
    /// slow continuously up to the shared cutoff.
    pub fn speed(&self, road: bool, forest: bool, slope_deg: f64) -> f64 {
        let base = if road {
            self.road_mps
        } else if forest {
            self.off_road_mps * self.forest_multiplier
        } else {
            self.off_road_mps
        };
        base * slope_multiplier(slope_deg)
    }

    pub fn max_speed(&self) -> f64 {
        self.road_mps.max(self.off_road_mps)
    }
}

/// `max(0.35, 1 - slope/50)` below the cutoff (contracts.md).
pub fn slope_multiplier(slope_deg: f64) -> f64 {
    (1.0 - slope_deg / 50.0).max(0.35)
}

#[derive(Clone, Copy, Debug, Default)]
struct Cell {
    /// Per mover class: the ground is walkable and no known prop that stops
    /// the class covers the cell.
    passable: [bool; 2],
    road: bool,
    forest: bool,
    slope_deg: f64,
    /// Per mover class: metres from this cell's centre to the nearest cell
    /// impassable to that class.
    clearance: [f64; 2],
    /// Infantry's free sub-cells, bit `row * 4 + column` from the cell's
    /// south-west corner.
    free: u16,
    /// Infantry may cross into the east (bit 0) and north (bit 1) neighbour.
    open: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Plan {
    /// Waypoints after the start, ending at the (possibly snapped) goal.
    Route(Vec<V2>),
    Blocked(BlockReason),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockReason {
    /// Nothing reachable lies near the requested point.
    NoRoute,
    /// The unit's own position is not in any cell it fits.
    StartEnclosed,
}

pub struct NavGrid {
    nx: usize,
    ny: usize,
    cells: Vec<Cell>,
    /// Temporary obstacles for the plan in progress.
    avoid: Vec<Obb2>,
    /// Search bookkeeping reused between plans.
    scratch: Scratch,
}

#[derive(Default)]
struct Scratch {
    g: Vec<f64>,
    parent: Vec<u32>,
    stamp: Vec<u32>,
    generation: u32,
}

#[derive(PartialEq)]
struct Open {
    f: f64,
    cell: u32,
}
impl Eq for Open {}
impl Ord for Open {
    fn cmp(&self, o: &Self) -> Ordering {
        // Min-heap on f, then lowest cell index for determinism.
        o.f.total_cmp(&self.f).then_with(|| o.cell.cmp(&self.cell))
    }
}
impl PartialOrd for Open {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

impl NavGrid {
    /// Build from the world's surfaces plus the props this side knows about.
    /// `known_props` are the movement blockers the planner may use; the world's
    /// own prop list is ignored here so hidden changes cannot leak into routes.
    /// `soldier_radius` sizes infantry's sub-cell gaps.
    pub fn build<'a>(
        world: &WorldGeometry,
        known_props: impl Iterator<Item = &'a Prop>,
        soldier_radius: f64,
    ) -> Self {
        let nx = (world.width() / NAV_CELL_M).floor() as usize;
        let ny = (world.depth() / NAV_CELL_M).floor() as usize;
        let mut cells = vec![Cell::default(); nx * ny];
        for j in 0..ny {
            for i in 0..nx {
                let c = cell_center(i, j);
                if let Some(s) = world.surface_at(c.x, c.y) {
                    cells[j * nx + i] = Cell {
                        passable: [s.traversable; 2],
                        road: s.kind == SurfaceKind::Road || s.kind == SurfaceKind::Bridge,
                        forest: s.forest,
                        slope_deg: s.slope_deg,
                        clearance: [0.0; 2],
                        free: if s.traversable { ALL_FREE } else { 0 },
                        open: 0,
                    };
                }
            }
        }
        // Beside ground nobody crosses (water, a slope past the cutoff), each
        // sub-cell reads the ground under its own centre.
        let border: Vec<usize> = (0..ny)
            .flat_map(|j| (0..nx).map(move |i| (i, j)))
            .filter(|&(i, j)| {
                cells[j * nx + i].free != 0
                    && (j.saturating_sub(1)..=(j + 1).min(ny - 1)).any(|jj| {
                        (i.saturating_sub(1)..=(i + 1).min(nx - 1))
                            .any(|ii| cells[jj * nx + ii].free == 0)
                    })
            })
            .map(|(i, j)| j * nx + i)
            .collect();
        for at in border {
            let (i, j) = (at % nx, at / nx);
            for bit in 0..SUB * SUB {
                let c = sub_center(i, j, bit);
                if !world.surface_at(c.x, c.y).is_some_and(|s| s.traversable) {
                    cells[at].free &= !(1 << bit);
                }
            }
        }
        let infantry = MoverClass::Infantry.index();
        for prop in known_props {
            let blocked: Vec<usize> = MoverClass::ALL
                .into_iter()
                .filter(|&c| prop.kind.blocks(c) && c != MoverClass::Infantry)
                .map(MoverClass::index)
                .collect();
            let stops_infantry = prop.kind.blocks(MoverClass::Infantry);
            if blocked.is_empty() && !stops_infantry {
                continue;
            }
            let footprint = prop.footprint();
            let r = prop.footprint_radius();
            // Vehicles over the cells the footprint's circle spans; infantry's
            // sub-cells over the footprint grown by a soldier.
            let reach = r + soldier_radius * std::f64::consts::SQRT_2;
            let (i0, j0) = cell_of(prop.center - v2(reach, reach));
            let (i1, j1) = cell_of(prop.center + v2(reach, reach));
            let (vi0, vj0) = cell_of(prop.center - v2(r, r));
            let (vi1, vj1) = cell_of(prop.center + v2(r, r));
            for j in j0.max(0)..=j1.min(ny as isize - 1) {
                for i in i0.max(0)..=i1.min(nx as isize - 1) {
                    let vehicles = (vi0..=vi1).contains(&i) && (vj0..=vj1).contains(&j);
                    let (i, j) = (i as usize, j as usize);
                    let cell = &mut cells[j * nx + i];
                    if vehicles && footprint.contains(cell_center(i, j), NAV_CELL_M / 2.0) {
                        for &c in &blocked {
                            cell.passable[c] = false;
                        }
                    }
                    if stops_infantry && cell.free != 0 {
                        for bit in 0..SUB * SUB {
                            if footprint.contains(sub_center(i, j, bit), soldier_radius) {
                                cell.free &= !(1 << bit);
                            }
                        }
                    }
                }
            }
        }
        for cell in &mut cells {
            cell.passable[infantry] = cell.free != 0 && one_gap(cell.free);
        }
        for j in 0..ny {
            for i in 0..nx {
                let at = j * nx + i;
                if !cells[at].passable[infantry] {
                    continue;
                }
                let free = cells[at].free;
                let mut open = 0;
                if i + 1 < nx && cells[at + 1].passable[infantry] {
                    // Our east column against their west column, row by row.
                    let east = (free & EAST_COLUMN) >> (SUB - 1);
                    open |= u8::from(east & cells[at + 1].free & WEST_COLUMN != 0);
                }
                if j + 1 < ny && cells[at + nx].passable[infantry] {
                    let north = (free & NORTH_ROW) >> (SUB * (SUB - 1));
                    open |= u8::from(north & cells[at + nx].free & SOUTH_ROW != 0) << 1;
                }
                cells[at].open = open;
            }
        }
        let mut grid = NavGrid {
            nx,
            ny,
            cells,
            avoid: Vec::new(),
            scratch: Scratch::default(),
        };
        grid.compute_clearance(MoverClass::Vehicle.index());
        grid
    }

    /// Two-pass chamfer distance transform (3-4 weights) from the cells
    /// impassable to class `k` and the map edge, which is closed.
    fn compute_clearance(&mut self, k: usize) {
        let (nx, ny) = (self.nx, self.ny);
        let big = MAX_CLEARANCE_M;
        for j in 0..ny {
            for i in 0..nx {
                let edge =
                    (i.min(nx - 1 - i).min(j).min(ny - 1 - j)) as f64 * NAV_CELL_M + NAV_CELL_M;
                let c = &mut self.cells[j * nx + i];
                c.clearance[k] = if c.passable[k] { big.min(edge) } else { 0.0 };
            }
        }
        let (a, b) = (NAV_CELL_M, NAV_CELL_M * std::f64::consts::SQRT_2);
        let relax = |cells: &mut [Cell], at: usize, from: usize, w: f64| {
            let d = cells[from].clearance[k] + w;
            if d < cells[at].clearance[k] {
                cells[at].clearance[k] = d;
            }
        };
        for j in 0..ny {
            for i in 0..nx {
                let at = j * nx + i;
                if i > 0 {
                    relax(&mut self.cells, at, at - 1, a);
                }
                if j > 0 {
                    relax(&mut self.cells, at, at - nx, a);
                    if i > 0 {
                        relax(&mut self.cells, at, at - nx - 1, b);
                    }
                    if i + 1 < nx {
                        relax(&mut self.cells, at, at - nx + 1, b);
                    }
                }
            }
        }
        for j in (0..ny).rev() {
            for i in (0..nx).rev() {
                let at = j * nx + i;
                if i + 1 < nx {
                    relax(&mut self.cells, at, at + 1, a);
                }
                if j + 1 < ny {
                    relax(&mut self.cells, at, at + nx, a);
                    if i + 1 < nx {
                        relax(&mut self.cells, at, at + nx + 1, b);
                    }
                    if i > 0 {
                        relax(&mut self.cells, at, at + nx - 1, b);
                    }
                }
            }
        }
    }

    fn index(&self, i: isize, j: isize) -> Option<usize> {
        (i >= 0 && j >= 0 && (i as usize) < self.nx && (j as usize) < self.ny)
            .then(|| j as usize * self.nx + i as usize)
    }

    /// Whether a footprint fits with its centre in this cell.
    fn fits(&self, cell: usize, m: &Mobility) -> bool {
        let c = &self.cells[cell];
        // The nearest blocked cell's centre is `clearance` away; its near edge
        // half a cell closer.
        let k = m.class.index();
        let room = match m.class {
            // Infantry's room is its sub-cell gap, judged at build.
            MoverClass::Infantry => true,
            MoverClass::Vehicle => c.clearance[k] - NAV_CELL_M / 2.0 >= m.half_width_m,
        };
        c.passable[k]
            && room
            && self.avoid.iter().all(|f| {
                !f.contains(
                    cell_center(cell % self.nx, cell / self.nx),
                    m.half_width_m + NAV_CELL_M / 2.0,
                )
            })
    }

    fn cost(&self, cell: usize, m: &Mobility, policy: RoutePolicy, length: f64) -> f64 {
        match policy {
            RoutePolicy::Shortest => length,
            RoutePolicy::Fastest => {
                let c = &self.cells[cell];
                length / m.speed(c.road, c.forest, c.slope_deg)
            }
        }
    }

    /// Nearest cell within `radius` of `p` that the footprint fits, by distance.
    fn nearest_fit(&self, p: V2, m: &Mobility, radius: f64) -> Option<usize> {
        let (ci, cj) = cell_of(p);
        let reach = (radius / NAV_CELL_M).ceil() as isize;
        let mut best: Option<(f64, usize)> = None;
        for dj in -reach..=reach {
            for di in -reach..=reach {
                if let Some(k) = self.index(ci + di, cj + dj) {
                    let d = (cell_center(k % self.nx, k / self.nx) - p).length();
                    if d <= radius
                        && self.fits(k, m)
                        && best.is_none_or(|(bd, bk)| (d, k) < (bd, bk))
                    {
                        best = Some((d, k));
                    }
                }
            }
        }
        best.map(|(_, k)| k)
    }

    /// Whether `p` lies in a cell the footprint fits (for infantry, in one
    /// of its free sub-cells).
    pub fn fits_at(&self, p: V2, m: &Mobility) -> bool {
        let (i, j) = cell_of(p);
        self.index(i, j).is_some_and(|k| {
            self.fits(k, m)
                && (m.class != MoverClass::Infantry || self.cells[k].free & (1 << sub_of(p)) != 0)
        })
    }

    /// Whether `m` may step from cell `a` to its orthogonal neighbour `b`:
    /// always for vehicles (their clearance decides), and for infantry only
    /// where free sub-cells meet across the shared edge.
    fn crosses(&self, a: usize, b: usize, m: &Mobility) -> bool {
        if m.class != MoverClass::Infantry {
            return true;
        }
        let (lo, hi) = (a.min(b), a.max(b));
        let bit = if hi - lo == 1 { 1 } else { 2 };
        self.cells[lo].open & bit != 0
    }

    /// The middle of the open span of the edge between orthogonal
    /// neighbours `a` and `b`: where infantry crosses from one to the other.
    fn crossing(&self, a: usize, b: usize) -> Option<V2> {
        let (lo, hi) = (a.min(b), a.max(b));
        let (free_lo, free_hi) = (self.cells[lo].free, self.cells[hi].free);
        let (i, j) = (lo % self.nx, lo / self.nx);
        let across: Vec<usize> = if hi - lo == 1 {
            (0..SUB)
                .filter(|r| {
                    free_lo & (1 << (r * SUB + SUB - 1)) != 0 && free_hi & (1 << (r * SUB)) != 0
                })
                .collect()
        } else if hi - lo == self.nx {
            (0..SUB)
                .filter(|c| free_lo & (1 << (SUB * (SUB - 1) + c)) != 0 && free_hi & (1 << c) != 0)
                .collect()
        } else {
            return None;
        };
        // The longest run of open sub-cells (two openings either side of a
        // tooth are two gaps, not one); ties to the one nearest the middle.
        let mut runs: Vec<(usize, usize)> = Vec::new();
        for &k in &across {
            match runs.last_mut() {
                Some((_, end)) if *end + 1 == k => *end = k,
                _ => runs.push((k, k)),
            }
        }
        let centre = (SUB as f64 - 1.0) / 2.0;
        let (first, last) = runs.into_iter().min_by(|a, b| {
            let key = |r: &(usize, usize)| {
                let mid = (r.0 + r.1) as f64 / 2.0;
                (std::cmp::Reverse(r.1 - r.0), (mid - centre).abs())
            };
            let (ka, kb) = (key(a), key(b));
            ka.0.cmp(&kb.0).then(ka.1.total_cmp(&kb.1))
        })?;
        let mid = (first + last) as f64 / 2.0;
        let along = (mid + 0.5) * SUB_M;
        Some(if hi - lo == 1 {
            v2((i + 1) as f64 * NAV_CELL_M, j as f64 * NAV_CELL_M + along)
        } else {
            v2(i as f64 * NAV_CELL_M + along, (j + 1) as f64 * NAV_CELL_M)
        })
    }

    /// Where a route through cell `k` passes: its centre, or for infantry
    /// the free sub-cell nearest the centre.
    fn waypoint(&self, k: usize, m: &Mobility) -> V2 {
        let (i, j) = (k % self.nx, k / self.nx);
        let center = cell_center(i, j);
        let free = self.cells[k].free;
        if m.class != MoverClass::Infantry || free == ALL_FREE {
            return center;
        }
        (0..SUB * SUB)
            .filter(|bit| free & (1 << bit) != 0)
            .map(|bit| sub_center(i, j, bit))
            .min_by(|a, b| (*a - center).length().total_cmp(&(*b - center).length()))
            .unwrap_or(center)
    }

    /// Snap a destination to the nearest point the footprint can stand on.
    pub fn snap(&self, p: V2, m: &Mobility, radius: f64) -> Option<V2> {
        if self.fits_at(p, m) {
            return Some(p);
        }
        self.nearest_fit(p, m, radius).map(|k| self.waypoint(k, m))
    }

    /// Plan as if these footprints were solid, for this search only.
    pub fn plan_avoiding(
        &mut self,
        from: V2,
        goal: V2,
        m: &Mobility,
        policy: RoutePolicy,
        avoid: &[Obb2],
    ) -> Plan {
        self.avoid = avoid.to_vec();
        let plan = self.plan(from, goal, m, policy);
        self.avoid.clear();
        plan
    }

    pub fn plan(&mut self, from: V2, goal: V2, m: &Mobility, policy: RoutePolicy) -> Plan {
        let Some(start) = self.nearest_fit(from, m, NAV_CELL_M * 2.0) else {
            return Plan::Blocked(BlockReason::StartEnclosed);
        };
        let Some(target) = self.nearest_fit(goal, m, NAV_CELL_M * 3.0) else {
            return Plan::Blocked(BlockReason::NoRoute);
        };
        let goal = if self.fits_at(goal, m) {
            goal
        } else {
            self.waypoint(target, m)
        };
        let n = self.cells.len();
        let s = &mut self.scratch;
        if s.g.len() != n {
            s.g = vec![0.0; n];
            s.parent = vec![0; n];
            s.stamp = vec![0; n];
        }
        s.generation = s.generation.wrapping_add(1);
        let generation = s.generation;
        let heuristic_speed = match policy {
            RoutePolicy::Shortest => 1.0,
            RoutePolicy::Fastest => m.max_speed(),
        };
        let target_center = cell_center(target % self.nx, target / self.nx);
        let h = |k: usize, nx: usize| {
            (cell_center(k % nx, k / nx) - target_center).length() / heuristic_speed
        };
        let mut open = BinaryHeap::new();
        s.g[start] = 0.0;
        s.parent[start] = start as u32;
        s.stamp[start] = generation;
        open.push(Open {
            f: h(start, self.nx),
            cell: start as u32,
        });
        const STEPS: [(isize, isize); 8] = [
            (1, 0),
            (-1, 0),
            (0, 1),
            (0, -1),
            (1, 1),
            (1, -1),
            (-1, 1),
            (-1, -1),
        ];
        let mut found = false;
        while let Some(Open { f, cell }) = open.pop() {
            let cell = cell as usize;
            if cell == target {
                found = true;
                break;
            }
            let g = self.scratch.g[cell];
            if f > g + h(cell, self.nx) + 1e-9 {
                continue; // stale entry
            }
            let (ci, cj) = ((cell % self.nx) as isize, (cell / self.nx) as isize);
            for (di, dj) in STEPS {
                let Some(next) = self.index(ci + di, cj + dj) else {
                    continue;
                };
                if !self.fits(next, m) {
                    continue;
                }
                if di != 0 && dj != 0 {
                    // No corner cutting: both orthogonal neighbours must fit,
                    // and be crossed into and out of.
                    let via = |k: usize| {
                        self.fits(k, m) && self.crosses(cell, k, m) && self.crosses(k, next, m)
                    };
                    let a = self.index(ci + di, cj).is_some_and(via);
                    let b = self.index(ci, cj + dj).is_some_and(via);
                    if !(a && b) {
                        continue;
                    }
                } else if !self.crosses(cell, next, m) {
                    continue;
                }
                let length = if di != 0 && dj != 0 {
                    NAV_CELL_M * std::f64::consts::SQRT_2
                } else {
                    NAV_CELL_M
                };
                // Half the step on each cell's surface.
                let step =
                    (self.cost(cell, m, policy, length) + self.cost(next, m, policy, length)) / 2.0;
                let tentative = g + step;
                let s = &mut self.scratch;
                if s.stamp[next] != generation || tentative < s.g[next] {
                    s.stamp[next] = generation;
                    s.g[next] = tentative;
                    s.parent[next] = cell as u32;
                    open.push(Open {
                        f: tentative + h(next, self.nx),
                        cell: next as u32,
                    });
                }
            }
        }
        if !found {
            return Plan::Blocked(BlockReason::NoRoute);
        }
        let mut cells = vec![target];
        while *cells.last().unwrap() != start {
            let k = *cells.last().unwrap();
            cells.push(self.scratch.parent[k] as usize);
        }
        cells.reverse();
        // Infantry passes each orthogonal step through the middle of the
        // gap on the shared edge, so a corridor through a narrow gap runs
        // down its middle.
        let mut points: Vec<V2> = vec![from];
        for w in cells.windows(2) {
            let crossing = (m.class == MoverClass::Infantry)
                .then(|| self.crossing(w[0], w[1]))
                .flatten();
            points.push(crossing.unwrap_or_else(|| self.waypoint(w[1], m)));
        }
        *points.last_mut().unwrap() = goal;
        Plan::Route(self.smooth(&points, m, policy))
    }

    /// Cost of travelling a straight segment, walking the cells it crosses; `None`
    /// when any sampled cell does not fit the footprint.
    /// Infantry samples every half sub-cell, and each sample must lie in a
    /// free sub-cell.
    fn segment_cost(&self, a: V2, b: V2, m: &Mobility, policy: RoutePolicy) -> Option<f64> {
        let length = (b - a).length();
        let infantry = m.class == MoverClass::Infantry;
        let spacing = if infantry {
            SUB_M / 2.0
        } else {
            NAV_CELL_M / 4.0
        };
        let samples = ((length / spacing).ceil() as usize).max(1);
        let piece = length / samples as f64;
        let mut total = 0.0;
        for k in 0..samples {
            let p = a + (b - a) * ((k as f64 + 0.5) / samples as f64);
            let (i, j) = cell_of(p);
            let cell = self.index(i, j)?;
            if !self.fits(cell, m) || (infantry && self.cells[cell].free & (1 << sub_of(p)) == 0) {
                return None;
            }
            total += self.cost(cell, m, policy, piece);
        }
        Some(total)
    }

    /// Greedy string-pulling over the grid path's turn points: from each kept
    /// point, jump to the furthest later turn whose straight segment fits and
    /// costs no more than the grid path it replaces. Scanning stops at the
    /// first segment that does not fit, so long open runs stay cheap.
    fn smooth(&self, points: &[V2], m: &Mobility, policy: RoutePolicy) -> Vec<V2> {
        let mut turns = vec![0];
        for k in 1..points.len() - 1 {
            let (d0, d1) = (points[k] - points[k - 1], points[k + 1] - points[k]);
            if d0.cross(d1).abs() > 1e-9 || d0.dot(d1) < 0.0 {
                turns.push(k);
            }
        }
        turns.push(points.len() - 1);
        // Grid-path cost between consecutive turns (straight runs of cells).
        let run_costs: Vec<f64> = turns
            .windows(2)
            .map(|w| {
                self.segment_cost(points[w[0]], points[w[1]], m, policy)
                    .unwrap_or(f64::INFINITY)
            })
            .collect();
        let mut out = Vec::new();
        let mut at = 0;
        while at + 1 < turns.len() {
            let mut next = at + 1;
            let mut path_cost = run_costs[at];
            for k in at + 2..turns.len() {
                path_cost += run_costs[k - 1];
                match self.segment_cost(points[turns[at]], points[turns[k]], m, policy) {
                    Some(direct) if direct <= path_cost + 1e-9 => next = k,
                    Some(_) => {}
                    None => break,
                }
            }
            out.push(points[turns[next]]);
            at = next;
        }
        out
    }

    /// Whether any known blocker now overlaps the remaining route for this footprint.
    pub fn route_fits(&self, from: V2, route: &[V2], m: &Mobility) -> bool {
        let mut a = from;
        route.iter().all(|&b| {
            let ok = self.segment_cost(a, b, m, RoutePolicy::Shortest).is_some();
            a = b;
            ok
        })
    }

    /// Travel time of a route for this footprint, for diagnostics and tests.
    pub fn route_time(&self, from: V2, route: &[V2], m: &Mobility) -> f64 {
        let mut a = from;
        route
            .iter()
            .map(|&b| {
                let t = self
                    .segment_cost(a, b, m, RoutePolicy::Fastest)
                    .unwrap_or(f64::INFINITY);
                a = b;
                t
            })
            .sum()
    }
}

fn cell_center(i: usize, j: usize) -> V2 {
    v2((i as f64 + 0.5) * NAV_CELL_M, (j as f64 + 0.5) * NAV_CELL_M)
}

/// Sub-cell masks: a 4×4 cell's west and east columns, south and north rows.
const WEST_COLUMN: u16 = 0x1111;
const EAST_COLUMN: u16 = 0x8888;
const SOUTH_ROW: u16 = 0x000F;
const NORTH_ROW: u16 = 0xF000;

/// Centre of sub-cell `bit` of cell (i, j).
fn sub_center(i: usize, j: usize, bit: usize) -> V2 {
    let (c, r) = (bit % SUB, bit / SUB);
    v2(
        i as f64 * NAV_CELL_M + (c as f64 + 0.5) * SUB_M,
        j as f64 * NAV_CELL_M + (r as f64 + 0.5) * SUB_M,
    )
}

/// The sub-cell of its cell that `p` lies in.
fn sub_of(p: V2) -> usize {
    let c = (p.x.rem_euclid(NAV_CELL_M) / SUB_M) as usize;
    let r = (p.y.rem_euclid(NAV_CELL_M) / SUB_M) as usize;
    r.min(SUB - 1) * SUB + c.min(SUB - 1)
}

/// Whether a cell's free sub-cells are one gap, connected edge to edge.
fn one_gap(free: u16) -> bool {
    let mut gap = free & free.wrapping_neg();
    loop {
        let grown = (gap
            | ((gap << 1) & !WEST_COLUMN)
            | ((gap >> 1) & !EAST_COLUMN)
            | (gap << SUB)
            | (gap >> SUB))
            & free;
        if grown == gap {
            return gap == free;
        }
        gap = grown;
    }
}

fn cell_of(p: V2) -> (isize, isize) {
    (
        (p.x / NAV_CELL_M).floor() as isize,
        (p.y / NAV_CELL_M).floor() as isize,
    )
}
