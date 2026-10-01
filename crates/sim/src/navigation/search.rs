//! One route search over a side's grid, taken a few steps at a time: A*
//! over octile moves without corner cutting, then string-pulling where that
//! keeps the cost. A battle spends a share of each tick's planning allowance
//! on it ([`crate::route_planner`]); [`NavGrid::plan`] runs one to its end.
//! A search owns its bookkeeping and only reads the grid, so one grid serves
//! many searches at once.
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use contract::command::RoutePolicy;
use contract::map::MoverClass;

use super::{
    cell_center, BlockReason, Mobility, Mover, NavGrid, Plan, SearchBound, CELLS_PER_WORK,
    NAV_CELL_M, START_REACH_M, TILE_SAMPLES, TILE_SIDE,
};
use crate::digest::Digest;
use crate::math::{Obb2, V2};

/// A cell the search has reached: its cost from the start and the cell it
/// was reached from.
#[derive(Clone, Copy)]
struct Reached {
    g: f64,
    parent: u32,
}

struct SearchTile {
    g: [f64; TILE_SAMPLES],
    parent: [u32; TILE_SAMPLES],
    stamp: [u32; TILE_SAMPLES],
}

/// The cells one search has reached, in tiles that outlive it: a finished or
/// cancelled search hands its scratch to the next, which starts a new
/// generation instead of clearing or freeing anything.
pub struct Scratch {
    nx: usize,
    tiles: HashMap<usize, Box<SearchTile>>,
    generation: u32,
    visited: usize,
}

impl Scratch {
    pub(super) fn new(nx: usize) -> Self {
        Self {
            nx,
            tiles: HashMap::new(),
            generation: 0,
            visited: 0,
        }
    }
    fn location(&self, k: usize) -> (usize, usize) {
        let (i, j) = (k % self.nx, k / self.nx);
        (
            (j / TILE_SIDE) * self.nx.div_ceil(TILE_SIDE) + i / TILE_SIDE,
            (j % TILE_SIDE) * TILE_SIDE + i % TILE_SIDE,
        )
    }
    fn clear(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        if self.generation == 0 {
            for tile in self.tiles.values_mut() {
                tile.stamp.fill(0);
            }
            self.generation = 1;
        }
        self.visited = 0;
    }
    fn get(&self, k: usize) -> Option<Reached> {
        let (key, at) = self.location(k);
        let tile = self.tiles.get(&key)?;
        (tile.stamp[at] == self.generation).then(|| Reached {
            g: tile.g[at],
            parent: tile.parent[at],
        })
    }
    /// One tile lookup for admission and the accepted cost/parent update.
    fn relax(&mut self, k: usize, value: Reached, admit: impl FnOnce() -> bool) -> bool {
        use std::collections::hash_map::Entry;
        let (key, at) = self.location(k);
        let tile = match self.tiles.entry(key) {
            Entry::Occupied(entry) => {
                let tile = entry.get();
                let improves = tile.stamp[at] != self.generation || value.g < tile.g[at];
                if !improves {
                    return false;
                }
                if !admit() {
                    return false;
                }
                entry.into_mut()
            }
            Entry::Vacant(entry) => {
                if !admit() {
                    return false;
                }
                entry.insert(Box::new(SearchTile {
                    g: [0.0; TILE_SAMPLES],
                    parent: [0; TILE_SAMPLES],
                    stamp: [0; TILE_SAMPLES],
                }))
            }
        };
        if tile.stamp[at] != self.generation {
            self.visited += 1;
        }
        tile.g[at] = value.g;
        tile.parent[at] = value.parent;
        tile.stamp[at] = self.generation;
        true
    }
    pub(super) fn len(&self) -> usize {
        self.visited
    }
    pub(super) fn capacity(&self) -> usize {
        self.tiles.len() * TILE_SAMPLES
    }
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

/// What a search has cost so far.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SearchWork {
    pub queued: usize,
    /// Peak capacity of the open queue, in entries.
    pub heap_peak: usize,
    pub expanded: usize,
    pub stale: usize,
}

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

/// The A* frontier and what it measures cells against.
struct Expanding {
    open: BinaryHeap<Open>,
    bound: Option<SearchBound>,
    /// An open-ground cell's orthogonal and diagonal step costs.
    implicit_costs: [f64; 2],
    heuristic_speed: f64,
    target_center: V2,
}

impl Expanding {
    fn h(&self, k: usize, nx: usize) -> f64 {
        (cell_center(k % nx, k / nx) - self.target_center).length() / self.heuristic_speed
    }
}

/// Greedy string-pulling over the grid path's turn points: from each kept
/// point, jump to the furthest later turn whose straight segment fits and
/// costs no more than the grid path it replaces. Scanning stops at the
/// first segment that does not fit, so long open runs stay cheap. One step
/// costs one segment.
struct Smoothing {
    points: Vec<V2>,
    turns: Vec<usize>,
    /// Grid-path cost between consecutive turns (straight runs of cells).
    run_costs: Vec<f64>,
    out: Vec<V2>,
    /// The turn kept last, the furthest turn it reaches so far, the next
    /// turn to try (0 before the first try from `at`), and the grid path's
    /// cost from `at` to the turn before that one.
    at: usize,
    next: usize,
    trying: usize,
    path_cost: f64,
}

impl Smoothing {
    fn new(points: Vec<V2>) -> Self {
        let mut turns = vec![0];
        for k in 1..points.len() - 1 {
            let (d0, d1) = (points[k] - points[k - 1], points[k + 1] - points[k]);
            if d0.cross(d1).abs() > 1e-9 || d0.dot(d1) < 0.0 {
                turns.push(k);
            }
        }
        turns.push(points.len() - 1);
        Smoothing {
            points,
            turns,
            run_costs: Vec::new(),
            out: Vec::new(),
            at: 0,
            next: 0,
            trying: 0,
            path_cost: 0.0,
        }
    }

    /// The smoothed route once the last turn is kept.
    fn step(&mut self, grid: &NavGrid, who: Mover, policy: RoutePolicy) -> Option<Vec<V2>> {
        let point = |turn: usize| self.points[self.turns[turn]];
        if self.run_costs.len() + 1 < self.turns.len() {
            let run = self.run_costs.len();
            let cost = grid
                .segment_cost(point(run), point(run + 1), who, policy)
                .unwrap_or(f64::INFINITY);
            self.run_costs.push(cost);
            return None;
        }
        grid.spend(1);
        if self.at + 1 >= self.turns.len() {
            return Some(std::mem::take(&mut self.out));
        }
        if self.trying == 0 {
            self.next = self.at + 1;
            self.path_cost = self.run_costs[self.at];
            self.trying = self.at + 2;
        }
        if self.trying < self.turns.len() {
            self.path_cost += self.run_costs[self.trying - 1];
            match grid.segment_cost(point(self.at), point(self.trying), who, policy) {
                Some(direct) if direct <= self.path_cost + 1e-9 => self.next = self.trying,
                Some(_) => {}
                None => self.trying = self.turns.len() - 1,
            }
            self.trying += 1;
            return None;
        }
        self.out.push(point(self.next));
        self.at = self.next;
        self.trying = 0;
        None
    }
}

enum Stage {
    Expanding(Expanding),
    Smoothing(Smoothing),
    Done(Plan),
}

/// A route search in progress. [`RouteSearch::advance`] spends work on it
/// until it has a [`Plan`].
pub struct RouteSearch {
    m: Mobility,
    policy: RoutePolicy,
    avoid: Vec<Obb2>,
    from: V2,
    goal: V2,
    start: usize,
    target: usize,
    scratch: Scratch,
    stage: Stage,
    work: SearchWork,
}

impl RouteSearch {
    /// Start a search from `from` to `goal` for `m`, as if the `avoid`
    /// footprints were solid. `scratch` is a finished search's bookkeeping
    /// to reuse.
    pub fn new(
        grid: &NavGrid,
        scratch: Option<Scratch>,
        from: V2,
        goal: V2,
        m: &Mobility,
        policy: RoutePolicy,
        avoid: &[Obb2],
    ) -> Self {
        let mut scratch = scratch.unwrap_or_else(|| Scratch::new(grid.nx));
        scratch.clear();
        let mut search = RouteSearch {
            m: *m,
            policy,
            avoid: avoid.to_vec(),
            from,
            goal,
            start: 0,
            target: 0,
            scratch,
            stage: Stage::Done(Plan::Blocked(BlockReason::NoRoute)),
            work: SearchWork::default(),
        };
        search.stage = search.begin(grid);
        search
    }

    fn begin(&mut self, grid: &NavGrid) -> Stage {
        let (m, policy) = (&self.m, self.policy);
        let who = Mover {
            m,
            avoid: &self.avoid,
        };
        let Some(start) = grid.nearest_fit(self.from, who, START_REACH_M) else {
            return Stage::Done(Plan::Blocked(BlockReason::StartEnclosed));
        };
        let Some(target) = grid.nearest_fit(self.goal, who, NAV_CELL_M * 3.0) else {
            return Stage::Done(Plan::Blocked(BlockReason::NoRoute));
        };
        if !grid.stands(self.goal, who) {
            self.goal = grid.waypoint(target, m);
        }
        (self.start, self.target) = (start, target);
        let totals = grid.totals(m, policy);
        if grid.separated(start, target, &totals) {
            return Stage::Done(Plan::Blocked(BlockReason::NoRoute));
        }
        if grid.uniform_segment(self.from, self.goal, start, target, who, policy) {
            return Stage::Done(Plan::Route(vec![self.goal]));
        }
        let implicit = &grid.cells.implicit[3];
        let mut expanding = Expanding {
            open: BinaryHeap::new(),
            bound: grid.search_bound(start, target, who, policy, &totals),
            implicit_costs: [
                NavGrid::cell_cost(implicit, m, policy, NAV_CELL_M),
                NavGrid::cell_cost(implicit, m, policy, NAV_CELL_M * std::f64::consts::SQRT_2),
            ],
            heuristic_speed: match policy {
                RoutePolicy::Shortest => 1.0,
                RoutePolicy::Fastest => m.max_speed(),
            },
            target_center: cell_center(target % grid.nx, target / grid.nx),
        };
        self.scratch.relax(
            start,
            Reached {
                g: 0.0,
                parent: start as u32,
            },
            || true,
        );
        expanding.open.push(Open {
            f: expanding.h(start, grid.nx),
            cell: start as u32,
        });
        self.work.queued = 1;
        self.work.heap_peak = expanding.open.capacity();
        Stage::Expanding(expanding)
    }

    /// Spend up to `allowance` work on the search, and say how much was
    /// spent: at most one step more than the allowance.
    pub fn advance(&mut self, grid: &NavGrid, allowance: u64) -> u64 {
        let before = grid.work();
        while !matches!(self.stage, Stage::Done(_)) && grid.work() - before < allowance {
            self.step(grid);
        }
        grid.work() - before
    }

    /// The plan, once the search has one.
    pub fn plan(&self) -> Option<&Plan> {
        match &self.stage {
            Stage::Done(plan) => Some(plan),
            _ => None,
        }
    }

    pub fn work(&self) -> SearchWork {
        self.work
    }

    /// The plan if the search reached one, and its bookkeeping for the next
    /// search to reuse.
    pub fn finish(self) -> (Option<Plan>, Scratch) {
        let plan = match self.stage {
            Stage::Done(plan) => Some(plan),
            _ => None,
        };
        (plan, self.scratch)
    }

    /// How far the search has got. With what was asked and the grid it
    /// reads, that fixes everything it will do: a search is a function of
    /// those and the work spent.
    pub fn digest(&self, d: &mut Digest) {
        let stage = match &self.stage {
            Stage::Expanding(e) => [0, e.open.len(), 0],
            Stage::Smoothing(s) => [1, s.at, s.trying],
            Stage::Done(_) => [2, 0, 0],
        };
        for v in stage
            .into_iter()
            .chain([self.work.queued, self.work.expanded, self.work.stale])
        {
            d.u64(v as u64);
        }
    }

    fn step(&mut self, grid: &NavGrid) {
        let who = Mover {
            m: &self.m,
            avoid: &self.avoid,
        };
        let next = match &mut self.stage {
            Stage::Done(_) => return,
            Stage::Smoothing(smoothing) => smoothing
                .step(grid, who, self.policy)
                .map(|route| Stage::Done(Plan::Route(route))),
            Stage::Expanding(expanding) => {
                grid.spend(1);
                match expanding.open.pop() {
                    None => Some(Stage::Done(Plan::Blocked(BlockReason::NoRoute))),
                    Some(Open { cell, .. }) if cell as usize == self.target => {
                        Some(Stage::Smoothing(Smoothing::new(trace(
                            grid,
                            &self.scratch,
                            who,
                            (self.from, self.start),
                            (self.goal, self.target),
                        ))))
                    }
                    Some(Open { f, cell }) => {
                        expand(
                            grid,
                            &mut self.scratch,
                            &mut self.work,
                            expanding,
                            who,
                            self.policy,
                            (f, cell as usize),
                        );
                        None
                    }
                }
            }
        };
        if let Some(next) = next {
            self.stage = next;
        }
    }
}

/// Relax the neighbours of the cell popped at priority `f`.
fn expand(
    grid: &NavGrid,
    scratch: &mut Scratch,
    work: &mut SearchWork,
    expanding: &mut Expanding,
    who: Mover,
    policy: RoutePolicy,
    (f, cell): (f64, usize),
) {
    let m = who.m;
    let g = scratch.get(cell).expect("a queued cell has a cost").g;
    if f > g + expanding.h(cell, grid.nx) {
        work.stale += 1;
        return; // stale entry
    }
    work.expanded += 1;
    let (ci, cj) = ((cell % grid.nx) as isize, (cell / grid.nx) as isize);
    // Geometry is fixed for the search. Classify each neighbour once;
    // diagonal moves reuse the same orthogonal fit/crossing answers.
    let nexts = STEPS.map(|(di, dj)| grid.index(ci + di, cj + dj));
    let uniform = grid.uniform_stencil(cell, who);
    let fits = nexts.map(|next| next.is_some_and(|k| uniform || grid.fits(k, who)));
    let crosses: [bool; 4] =
        std::array::from_fn(|k| fits[k] && (uniform || grid.crosses(cell, nexts[k].unwrap(), m)));
    let costs = if uniform {
        expanding.implicit_costs
    } else {
        let c = &grid.cells[cell];
        [
            NavGrid::cell_cost(c, m, policy, NAV_CELL_M),
            NavGrid::cell_cost(c, m, policy, NAV_CELL_M * std::f64::consts::SQRT_2),
        ]
    };
    for (direction, &(di, dj)) in STEPS.iter().enumerate() {
        if !fits[direction] {
            continue;
        }
        let next = nexts[direction].unwrap();
        let diagonal = di != 0 && dj != 0;
        if diagonal {
            // No corner cutting: both orthogonal neighbours must fit,
            // and be crossed into and out of.
            let horizontal = if di > 0 { 0 } else { 1 };
            let vertical = if dj > 0 { 2 } else { 3 };
            let via =
                |k: usize| crosses[k] && (uniform || grid.crosses(nexts[k].unwrap(), next, m));
            if !(via(horizontal) && via(vertical)) {
                continue;
            }
        } else if !crosses[direction] {
            continue;
        }
        let length = if diagonal {
            NAV_CELL_M * std::f64::consts::SQRT_2
        } else {
            NAV_CELL_M
        };
        // Half the step on each cell's surface. Preserve the separate
        // orthogonal/diagonal cost arithmetic and original step order.
        let next_cost = if uniform {
            costs[usize::from(diagonal)]
        } else {
            grid.cost(next, m, policy, length)
        };
        let step = (costs[usize::from(diagonal)] + next_cost) / 2.0;
        let tentative = g + step;
        let bound = &expanding.bound;
        if scratch.relax(
            next,
            Reached {
                g: tentative,
                parent: cell as u32,
            },
            || !bound.as_ref().is_some_and(|b| b.rejects(next, tentative)),
        ) {
            expanding.open.push(Open {
                f: tentative + expanding.h(next, grid.nx),
                cell: next as u32,
            });
            work.queued += 1;
            work.heap_peak = work.heap_peak.max(expanding.open.capacity());
        }
    }
}

/// The reached target's path back to the start, as the points a mover
/// passes: from where it stands, through each cell, to its goal.
fn trace(
    grid: &NavGrid,
    scratch: &Scratch,
    who: Mover,
    (from, start): (V2, usize),
    (goal, target): (V2, usize),
) -> Vec<V2> {
    let mut cells = vec![target];
    while *cells.last().unwrap() != start {
        let k = *cells.last().unwrap();
        cells.push(scratch.get(k).expect("a route cell has a parent").parent as usize);
    }
    cells.reverse();
    grid.spend((cells.len() / CELLS_PER_WORK) as u64);
    // Infantry passes each orthogonal step through the middle of the
    // gap on the shared edge, so a corridor through a narrow gap runs
    // down its middle.
    let mut points: Vec<V2> = vec![from];
    for w in cells.windows(2) {
        let crossing = (who.m.class == MoverClass::Infantry)
            .then(|| grid.crossing(w[0], w[1]))
            .flatten();
        points.push(crossing.unwrap_or_else(|| grid.waypoint(w[1], who.m)));
    }
    *points.last_mut().unwrap() = goal;
    points
}
