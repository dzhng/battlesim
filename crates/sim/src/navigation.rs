//! Route planning over one side's known geometry. A 2 m grid classifies each
//! cell by the walkable surface under its centre (the same triangle rule the
//! world uses) and by known movement-blocking props; a clearance field lets a
//! footprint of any width ask whether it fits. Plans are A* over octile moves
//! without corner cutting, then string-pulled only where that keeps the cost.
use std::cmp::Ordering;
use std::collections::BinaryHeap;

use contract::command::RoutePolicy;

use crate::math::{v2, Obb2, V2};
use crate::world::{Prop, SurfaceKind, WorldGeometry};

pub const NAV_CELL_M: f64 = 2.0;
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
    passable: bool,
    road: bool,
    forest: bool,
    slope_deg: f64,
    /// Metres from this cell's centre to the nearest impassable cell centre.
    clearance: f64,
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
    /// Plans run since creation, for the "no per-frame search" contract.
    pub searches: u64,
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
    pub fn build<'a>(world: &WorldGeometry, known_props: impl Iterator<Item = &'a Prop>) -> Self {
        let nx = (world.width() / NAV_CELL_M).floor() as usize;
        let ny = (world.depth() / NAV_CELL_M).floor() as usize;
        let mut cells = vec![Cell::default(); nx * ny];
        for j in 0..ny {
            for i in 0..nx {
                let c = cell_center(i, j);
                if let Some(s) = world.surface_at(c.x, c.y) {
                    cells[j * nx + i] = Cell {
                        passable: s.traversable,
                        road: s.kind == SurfaceKind::Road || s.kind == SurfaceKind::Bridge,
                        forest: s.forest,
                        slope_deg: s.slope_deg,
                        clearance: 0.0,
                    };
                }
            }
        }
        for prop in known_props.filter(|p| p.kind.blocks_movement()) {
            let r = prop.footprint_radius();
            let (i0, j0) = cell_of(prop.center - v2(r, r));
            let (i1, j1) = cell_of(prop.center + v2(r, r));
            for j in j0.max(0)..=j1.min(ny as isize - 1) {
                for i in i0.max(0)..=i1.min(nx as isize - 1) {
                    let (i, j) = (i as usize, j as usize);
                    if prop
                        .footprint()
                        .contains(cell_center(i, j), NAV_CELL_M / 2.0)
                    {
                        cells[j * nx + i].passable = false;
                    }
                }
            }
        }
        let mut grid = NavGrid {
            nx,
            ny,
            cells,
            avoid: Vec::new(),
            scratch: Scratch::default(),
            searches: 0,
        };
        grid.compute_clearance();
        grid
    }

    /// Two-pass chamfer distance transform (3-4 weights) from impassable cells
    /// and the map edge, which is closed.
    fn compute_clearance(&mut self) {
        let (nx, ny) = (self.nx, self.ny);
        let big = MAX_CLEARANCE_M;
        for j in 0..ny {
            for i in 0..nx {
                let edge =
                    (i.min(nx - 1 - i).min(j).min(ny - 1 - j)) as f64 * NAV_CELL_M + NAV_CELL_M;
                let c = &mut self.cells[j * nx + i];
                c.clearance = if c.passable { big.min(edge) } else { 0.0 };
            }
        }
        let (a, b) = (NAV_CELL_M, NAV_CELL_M * std::f64::consts::SQRT_2);
        let relax = |cells: &mut [Cell], at: usize, from: usize, w: f64| {
            let d = cells[from].clearance + w;
            if d < cells[at].clearance {
                cells[at].clearance = d;
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
        c.passable
            && c.clearance - NAV_CELL_M / 2.0 >= m.half_width_m
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

    /// Whether `p` lies in a cell the footprint fits.
    pub fn fits_at(&self, p: V2, m: &Mobility) -> bool {
        let (i, j) = cell_of(p);
        self.index(i, j).is_some_and(|k| self.fits(k, m))
    }

    /// Snap a destination to the nearest point the footprint can stand on.
    pub fn snap(&self, p: V2, m: &Mobility, radius: f64) -> Option<V2> {
        if self.fits_at(p, m) {
            return Some(p);
        }
        self.nearest_fit(p, m, radius)
            .map(|k| cell_center(k % self.nx, k / self.nx))
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
        self.searches += 1;
        let Some(start) = self.nearest_fit(from, m, NAV_CELL_M * 2.0) else {
            return Plan::Blocked(BlockReason::StartEnclosed);
        };
        let Some(target) = self.nearest_fit(goal, m, NAV_CELL_M * 3.0) else {
            return Plan::Blocked(BlockReason::NoRoute);
        };
        let goal = if self.fits_at(goal, m) {
            goal
        } else {
            cell_center(target % self.nx, target / self.nx)
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
                    // No corner cutting: both orthogonal neighbours must fit.
                    let a = self.index(ci + di, cj).is_some_and(|k| self.fits(k, m));
                    let b = self.index(ci, cj + dj).is_some_and(|k| self.fits(k, m));
                    if !(a && b) {
                        continue;
                    }
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
        let mut points: Vec<V2> = cells
            .iter()
            .map(|&k| cell_center(k % self.nx, k / self.nx))
            .collect();
        points[0] = from;
        *points.last_mut().unwrap() = goal;
        Plan::Route(self.smooth(&points, m, policy))
    }

    /// Cost of travelling a straight segment, walking the cells it crosses; `None`
    /// when any sampled cell does not fit the footprint.
    fn segment_cost(&self, a: V2, b: V2, m: &Mobility, policy: RoutePolicy) -> Option<f64> {
        let length = (b - a).length();
        let samples = ((length / (NAV_CELL_M / 4.0)).ceil() as usize).max(1);
        let piece = length / samples as f64;
        let mut total = 0.0;
        for k in 0..samples {
            let p = a + (b - a) * ((k as f64 + 0.5) / samples as f64);
            let (i, j) = cell_of(p);
            let cell = self.index(i, j)?;
            if !self.fits(cell, m) {
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

fn cell_of(p: V2) -> (isize, isize) {
    (
        (p.x / NAV_CELL_M).floor() as isize,
        (p.y / NAV_CELL_M).floor() as isize,
    )
}

pub fn route_length(from: V2, route: &[V2]) -> f64 {
    let mut a = from;
    route
        .iter()
        .map(|&b| {
            let l = (b - a).length();
            a = b;
            l
        })
        .sum()
}
