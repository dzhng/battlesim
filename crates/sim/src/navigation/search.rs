//! One route search over a side's grid, taken a few steps at a time: A*
//! over octile moves without corner cutting, steered by the cost of the
//! straight way over open ground, then string-pulling where that keeps the
//! cost. It gives up at a limit of cells rather than search a whole map.
//! Roads are not its business: a leg that should go by road is a
//! [`super::Journey`], which uses these searches for the stretches off the
//! road. A search owns its bookkeeping and only reads the grid, so one grid
//! serves many searches at once.
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use contract::command::RoutePolicy;
use contract::map::MoverClass;

use super::{
    cell_center, octile, BlockReason, Mobility, Mover, NavGrid, Plan, Probe, CELLS_PER_WORK,
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
    /// The generation that reached the cell, and the one that expanded it.
    stamp: [u32; TILE_SAMPLES],
    expanded: [u32; TILE_SAMPLES],
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
                tile.expanded.fill(0);
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
    /// Settle `k` for expansion: false if it already was.
    fn settle(&mut self, k: usize) -> bool {
        let (key, at) = self.location(k);
        let tile = self.tiles.get_mut(&key).expect("a queued cell has a tile");
        let first = tile.expanded[at] != self.generation;
        tile.expanded[at] = self.generation;
        first
    }

    /// Reach `k` at `value` if that is its first or a cheaper way there.
    fn relax(&mut self, k: usize, value: Reached) -> bool {
        let (key, at) = self.location(k);
        let tile = self.tiles.entry(key).or_insert_with(|| {
            Box::new(SearchTile {
                g: [0.0; TILE_SAMPLES],
                parent: [0; TILE_SAMPLES],
                stamp: [0; TILE_SAMPLES],
                expanded: [0; TILE_SAMPLES],
            })
        });
        let first = tile.stamp[at] != self.generation;
        // A cell is settled once it is expanded (the estimate is not exact
        // enough for a later, cheaper way to it to be worth the work of
        // expanding it again). Until then only a cost that is less counts.
        let settled = tile.expanded[at] == self.generation;
        let cheaper = value.g.partial_cmp(&tile.g[at]) == Some(Ordering::Less);
        if settled || (!first && !cheaper) {
            return false;
        }
        if first {
            self.visited += 1;
        }
        tile.g[at] = value.g;
        tile.parent[at] = value.parent;
        tile.stamp[at] = self.generation;
        true
    }
}

/// An entry of a best-first queue: a cell (or a road node) and the
/// estimated cost of the whole way through it.
#[derive(PartialEq)]
pub(super) struct Open {
    pub(super) f: f64,
    pub(super) cell: u32,
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
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct SearchWork {
    /// Cells the search has reached.
    pub cells: usize,
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

/// Of the many cells that look equally good across open ground, the search
/// takes the one nearer the goal: the estimate is stretched by this share,
/// so it walks one line instead of every equal one. A route may cost this
/// share more than the best.
const NEARER_FIRST: f64 = 1.0 / 1024.0;

/// The A* frontier and what it measures cells against.
struct Expanding {
    open: BinaryHeap<Open>,
    /// An open-ground cell's orthogonal and diagonal step costs.
    implicit_costs: [f64; 2],
    target: usize,
}

impl Expanding {
    /// The estimated cost of the way still to go from cell `k`: the walk to
    /// the target over open ground, and for the fastest policy at the pace
    /// the ground straight ahead allows (a wood or a hillside on the line
    /// is slow going, so a way round it is worth looking at). A road on
    /// the way is taken for what it saves, but none is looked for far off
    /// the line.
    fn h(&self, grid: &NavGrid, m: &Mobility, policy: RoutePolicy, k: usize) -> f64 {
        let walk = octile(k, self.target, grid.nx) * (1.0 + NEARER_FIRST);
        match policy {
            RoutePolicy::Shortest => walk,
            RoutePolicy::Fastest => {
                let center = |k: usize| cell_center(k % grid.nx, k / grid.nx);
                let (here, there) = (center(k), center(self.target));
                walk / (there - here).length().max(NAV_CELL_M) * grid.across(here, there, m)
            }
        }
    }
}

/// Greedy string-pulling over the grid path's turn points: from each kept
/// point, jump to the furthest later turn whose straight segment fits and
/// costs no more than the grid path it replaces. Scanning stops at the
/// first segment that does not fit, so long open runs stay cheap. One step
/// reads one stretch of one segment.
struct Smoothing {
    /// The segment being costed.
    reading: Option<Probe>,
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
            reading: None,
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

    /// Read the next stretch of the segment between two turns: `None` while
    /// there is more of it, then its cost (`None` if it does not fit).
    fn cost(
        &mut self,
        grid: &NavGrid,
        who: Mover,
        policy: RoutePolicy,
        (from, to): (usize, usize),
    ) -> Option<Option<f64>> {
        let point = |turn: usize| self.points[self.turns[turn]];
        let mut probe = self
            .reading
            .take()
            .unwrap_or_else(|| Probe::new(point(from), point(to), who.m));
        let cost = grid.read(&mut probe, who, policy);
        if cost.is_none() {
            self.reading = Some(probe);
        }
        cost
    }

    /// The smoothed route once the last turn is kept.
    fn step(&mut self, grid: &NavGrid, who: Mover, policy: RoutePolicy) -> Option<Vec<V2>> {
        if self.run_costs.len() + 1 < self.turns.len() {
            let run = self.run_costs.len();
            let cost = self.cost(grid, who, policy, (run, run + 1))?;
            self.run_costs.push(cost.unwrap_or(f64::INFINITY));
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
            let direct = self.cost(grid, who, policy, (self.at, self.trying))?;
            self.path_cost += self.run_costs[self.trying - 1];
            match direct {
                Some(direct) if direct <= self.path_cost + 1e-9 => self.next = self.trying,
                Some(_) => {}
                None => self.trying = self.turns.len() - 1,
            }
            self.trying += 1;
            return None;
        }
        self.out.push(self.points[self.turns[self.next]]);
        self.at = self.next;
        self.trying = 0;
        None
    }
}

/// What a route is asked for: a way for `m` from `from` to `goal` by
/// `policy`, as if the `avoid` footprints were solid.
#[derive(Clone, Copy)]
pub struct Leg<'a> {
    pub from: V2,
    pub goal: V2,
    pub m: &'a Mobility,
    pub policy: RoutePolicy,
    pub avoid: &'a [Obb2],
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
    /// The cells it may expand before it gives the route up.
    limit: usize,
}

impl RouteSearch {
    /// Start the search for `leg`, to be given up once `limit` cells are
    /// expanded. `scratch` is a finished search's bookkeeping to reuse.
    pub fn new(grid: &NavGrid, scratch: Option<Scratch>, leg: Leg, limit: usize) -> Self {
        let mut scratch = scratch.unwrap_or_else(|| Scratch::new(grid.nx));
        scratch.clear();
        let mut search = RouteSearch {
            m: *leg.m,
            policy: leg.policy,
            avoid: leg.avoid.to_vec(),
            from: leg.from,
            goal: leg.goal,
            start: 0,
            target: 0,
            scratch,
            stage: Stage::Done(Plan::Blocked(BlockReason::NoRoute)),
            work: SearchWork::default(),
            limit,
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
        let implicit = &grid.cells.implicit[3];
        let mut expanding = Expanding {
            open: BinaryHeap::new(),
            implicit_costs: [
                NavGrid::cell_cost(implicit, m, policy, NAV_CELL_M),
                NavGrid::cell_cost(implicit, m, policy, NAV_CELL_M * std::f64::consts::SQRT_2),
            ],
            target,
        };
        self.scratch.relax(
            start,
            Reached {
                g: 0.0,
                parent: start as u32,
            },
        );
        expanding.open.push(Open {
            f: expanding.h(grid, m, policy, start),
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
        SearchWork {
            cells: self.scratch.visited,
            ..self.work
        }
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
            Stage::Smoothing(s) => [
                1,
                s.run_costs.len() + s.at + s.trying,
                s.reading.as_ref().map_or(0, |probe| probe.next),
            ],
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
                    Some(_) if self.work.expanded >= self.limit => {
                        Some(Stage::Done(Plan::Blocked(BlockReason::SearchLimit)))
                    }
                    Some(Open { cell, .. }) => {
                        expand(
                            grid,
                            &mut self.scratch,
                            &mut self.work,
                            expanding,
                            who,
                            self.policy,
                            cell as usize,
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

/// Relax the neighbours of a cell that has come off the queue.
fn expand(
    grid: &NavGrid,
    scratch: &mut Scratch,
    work: &mut SearchWork,
    expanding: &mut Expanding,
    who: Mover,
    policy: RoutePolicy,
    cell: usize,
) {
    let m = who.m;
    // Each cell is expanded once, the first time it comes off the queue.
    if !scratch.settle(cell) {
        work.stale += 1;
        return;
    }
    let g = scratch.get(cell).expect("a queued cell has a cost").g;
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
        let reached = Reached {
            g: tentative,
            parent: cell as u32,
        };
        if scratch.relax(next, reached) {
            expanding.open.push(Open {
                f: tentative + expanding.h(grid, m, policy, next),
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
