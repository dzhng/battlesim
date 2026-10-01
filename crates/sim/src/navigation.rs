//! Route planning over one side's known geometry. A 2 m grid classifies each
//! cell by the walkable surface under its centre (the same triangle rule the
//! world uses) and by the known bodies that block each mover class. What
//! both sides know alike when the battle starts (the terrain, and the bodies
//! the map put on it) is one [`NavBase`], built once; a side's [`NavGrid`]
//! shares it and holds only what the side has since come to know
//! differently, taken in a body at a time ([`NavGrid::update`]). The
//! navigation classes are infantry plus one per vehicle push class (Q13): a
//! vehicle cell remembers the heaviest body over it, so a class that can
//! shove that body passes it at the cost of its shoving speed, and a class
//! that cannot is blocked. A clearance field per push class lets a
//! footprint of any width ask whether it fits.
//!
//! The grid is what a route is checked on, not what works one out. A leg's
//! route is a [`Journey`]: by road where the map's road graph ([`RoadNet`])
//! offers a way worth taking, with a grid search (`search`) to and
//! from the road and round whatever stands on it; otherwise one grid search
//! for the whole leg. All of it is counted work a battle spends a share of
//! a tick on ([`crate::route_planner`]).
//!
//! Infantry reads the grid at two resolutions (Q27). Each 2 m cell holds a
//! 4×4 mask of 0.5 m sub-cells, set where a soldier's disc stands clear of
//! every known body. A cell is open to a squad when its free sub-cells form
//! one connected gap, and a move between two cells when free sub-cells meet
//! across their shared edge: a line of teeth or a gap between wrecks stays
//! open, a wall stays closed. Soldiers then find their own way through the
//! gap on the exact bodies (`movement::final_leg`).
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::Arc;

use contract::command::RoutePolicy;
use contract::map::MoverClass;
use contract::scenario::PushClass;

use crate::math::{v2, Obb2, V2};
use crate::world::PropId;

mod base;
mod cells;
mod journey;
mod regions;
mod roads;
mod search;
mod update;

use base::Body;
pub use base::NavBase;
use cells::{cell_center, cell_of, sub_center, sub_of, Cell, Cells, ALL_FREE, NO_BODY, SUB, SUB_M};
pub use journey::Journey;
use regions::Regions;
pub use roads::RoadNet;
pub use search::{Leg, Scratch, SearchWork};

/// The whole route for `leg` at once, however much work it takes, and what
/// its searches cost: what a battle's planner reaches in steps
/// ([`crate::route_planner`]). For tools and tests.
pub fn plan(
    grid: &NavGrid,
    roads: &RoadNet,
    leg: Leg,
    rules: &contract::scenario::NavigationRules,
) -> (Plan, SearchWork) {
    let mut journey = Journey::new(grid, roads, None, leg, rules);
    journey.advance(grid, roads, u64::MAX);
    let work = journey.work();
    let plan = journey.finish().0;
    (
        plan.expect("a journey with work to spare has finished"),
        work,
    )
}

pub const NAV_CELL_M: f64 = 2.0;
/// Clearance is a distance transform capped here; wider footprints do not exist.
const MAX_CLEARANCE_M: f64 = 16.0;
const TILE_SIDE: usize = 32;
const TILE_SAMPLES: usize = TILE_SIDE * TILE_SIDE;
const CLEARANCE_HALO: usize = (MAX_CLEARANCE_M / NAV_CELL_M) as usize;
const CLEARANCE_SIDE: usize = TILE_SIDE + 2 * CLEARANCE_HALO;
/// A route starts from the nearest cell its mover fits within this far of it.
const START_REACH_M: f64 = NAV_CELL_M * 2.0;
/// Planning work is counted in searched cells: a search's first read of a
/// clearance tile since its side last learned anything costs about this
/// many, which is what working the tile out costs.
const TILE_WORK: u64 = 64;
/// How many samples along a route segment one unit of work reads.
const SAMPLES_PER_WORK: usize = 8;
/// How many cells of a traced path one unit of work reads.
const CELLS_PER_WORK: usize = 32;
/// The most one indivisible step of a search costs, and so the most a tick's
/// planning overruns its allowance by: starting a search whose two ends
/// each lie where four clearance tiles meet that nobody has asked about
/// yet. (Expanding a cell or reading a stretch of a segment crosses fewer.)
pub const LARGEST_STEP: u64 = 9 * TILE_WORK;

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
    /// What it can shove aside (vehicles; infantry shoves nothing).
    pub push: PushClass,
    /// How a vehicle steers and reverses (Q29, Q30); `None` for infantry.
    pub drive: Option<Drive>,
}

/// A vehicle's kinematics, from its body row (Q29, Q30). Planning never
/// reads it: the follower owns the kinematics.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Drive {
    /// Tracked vehicles pivot on the spot; wheeled ones never do.
    pub tracked: bool,
    pub turn_rad_s: f64,
    /// A wheeled vehicle's tightest turn (0 for tracks).
    pub radius_m: f64,
    pub reverse_fraction: f64,
    /// How every vehicle drives (`movement.drive`).
    pub feel: contract::scenario::DriveRules,
}

impl Mobility {
    /// Travel speed on a surface. A road (`road_factor` above 0, the surface
    /// kind's share of road speed) takes precedence over forest and is never
    /// slower than open ground; slopes slow continuously up to the shared
    /// cutoff.
    pub fn speed(&self, road_factor: f64, forest: bool, slope_deg: f64) -> f64 {
        let base = if road_factor > 0.0 {
            (self.road_mps * road_factor).max(self.off_road_mps)
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

/// Push classes, and so at most this many vehicle clearance fields.
const PUSH_CLASSES: usize = PushClass::ALL.len();

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
    /// The search looked as far as the rules let it (`navigation`'s
    /// `search_cells`) without reaching the goal: no route is known, though
    /// one may exist.
    SearchLimit,
}

/// One clearance tile: exact capped distance-transform samples.
struct Clearance {
    values: Box<[f64; TILE_SAMPLES]>,
    /// What the grid's side knew ([`NavGrid::knowledge`]) when a search
    /// last paid to read the tile.
    paid: u64,
}

pub struct NavGrid {
    nx: usize,
    ny: usize,
    /// The map's grid, which this one started as.
    base: Arc<NavBase>,
    cells: Cells,
    /// Every body this side plans with otherwise than the map laid it: where
    /// it believes it stands, or `None` for one of the map's it believes
    /// gone.
    laid: BTreeMap<PropId, Option<Body>>,
    /// Those of `laid` that stand somewhere, by the body buckets they reach.
    laid_in: BTreeMap<usize, Vec<PropId>>,
    /// The clearance tiles read so far, by distinct stopping rank, then tile.
    clearance: [RefCell<Vec<Option<Clearance>>>; PUSH_CLASSES],
    /// How many known bodies stop vehicles, by weight rank.
    stopping: [u32; u8::BITS as usize],
    /// Conservative rectangles containing every nonuniform surface or known body.
    regions: Regions,
    /// How slow the ground is, a tile at a time.
    slow: SlowGround,
    /// How many times the grid has taken in what its side learned, and how
    /// many cells that has had it work out again.
    knowledge: u64,
    relaid: u64,
    /// Planning work done on this grid since it was built.
    work: std::cell::Cell<u64>,
}

/// How slow the ground is, a tile at a time: what a search's estimate of the
/// way still to go reads, so a wood or a hillside ahead is not taken for
/// open ground. Counts, so the order cells are read in never shows.
#[derive(Clone)]
struct SlowGround {
    tiles_x: usize,
    /// Per tile, how many of its cells are forest off any road.
    forest: Vec<u16>,
    /// Per tile, how much longer its cells' slopes make a crossing than flat
    /// ground would, summed over its cells in 1/1024ths.
    steep: Vec<u32>,
}

impl SlowGround {
    const TILE_M: f64 = TILE_SIDE as f64 * NAV_CELL_M;

    fn new(cells: &Cells) -> Self {
        let tiles_x = cells.nx.div_ceil(TILE_SIDE);
        let tiles = tiles_x * cells.ny.div_ceil(TILE_SIDE);
        let mut slow = SlowGround {
            tiles_x,
            forest: vec![0; tiles],
            steep: vec![0; tiles],
        };
        for (at, cell) in cells.stored() {
            let tile = (at / cells.nx / TILE_SIDE) * tiles_x + at % cells.nx / TILE_SIDE;
            slow.forest[tile] += Self::forest(cell);
            slow.steep[tile] += Self::steep(cell);
        }
        slow
    }

    fn forest(cell: &Cell) -> u16 {
        u16::from(cell.forest && cell.road_factor == 0.0)
    }

    fn steep(cell: &Cell) -> u32 {
        let longer = 1.0 / slope_multiplier(cell.slope_deg) - 1.0;
        (longer * 1024.0).round() as u32
    }

    /// The cell at `at`, on a grid `nx` wide, is now `new` instead of `old`.
    fn replace(&mut self, at: usize, nx: usize, old: &Cell, new: &Cell) {
        let tile = (at / nx / TILE_SIDE) * self.tiles_x + at % nx / TILE_SIDE;
        self.forest[tile] = self.forest[tile] - Self::forest(old) + Self::forest(new);
        self.steep[tile] = self.steep[tile] - Self::steep(old) + Self::steep(new);
    }

    /// How many times longer than over open, flat ground the straight way
    /// from `a` to `b` takes `m`, judged by the tiles it crosses.
    fn stretch(&self, a: V2, b: V2, m: &Mobility) -> f64 {
        let samples = (((b - a).length() / Self::TILE_M).ceil() as usize).clamp(1, 32);
        let mut total = 0.0;
        for k in 0..samples {
            let p = a + (b - a) * ((k as f64 + 0.5) / samples as f64);
            let at = |v: f64| (v / Self::TILE_M).floor().max(0.0) as usize;
            let tile = (at(p.y) * self.tiles_x + at(p.x)).min(self.forest.len() - 1);
            let forest = self.forest[tile] as f64 / TILE_SAMPLES as f64;
            let steep = 1.0 + self.steep[tile] as f64 / (1024.0 * TILE_SAMPLES as f64);
            total += steep * (1.0 - forest + forest / m.forest_multiplier);
        }
        total / samples as f64
    }
}

/// Who a plan is for: the mover, and the footprints this one plan treats as
/// solid on top of what its side knows.
#[derive(Clone, Copy)]
struct Mover<'a> {
    m: &'a Mobility,
    avoid: &'a [Obb2],
}

impl<'a> Mover<'a> {
    /// The mover with nothing extra in its way.
    fn free(m: &'a Mobility) -> Self {
        Mover { m, avoid: &[] }
    }
}
/// A straight segment being costed a stretch at a time, so no one step of a
/// search reads a long one whole ([`NavGrid::read`]).
struct Probe {
    a: V2,
    b: V2,
    /// Points sampled along it, and how many are read so far.
    samples: usize,
    next: usize,
    total: f64,
}

impl Probe {
    /// Samples one read takes: 64 m of a vehicle's segment.
    const STRETCH: usize = 128;

    fn new(a: V2, b: V2, m: &Mobility) -> Self {
        let spacing = match m.class {
            MoverClass::Infantry => SUB_M / 2.0,
            MoverClass::Vehicle => NAV_CELL_M / 4.0,
        };
        Probe {
            a,
            b,
            samples: (((b - a).length() / spacing).ceil() as usize).max(1),
            next: 0,
            total: 0.0,
        }
    }
}

/// The length of the shortest eight-way walk between two cells over open
/// ground.
fn octile(a: usize, b: usize, nx: usize) -> f64 {
    let x = (a % nx).abs_diff(b % nx) as f64;
    let y = (a / nx).abs_diff(b / nx) as f64;
    NAV_CELL_M * (x.max(y) + (std::f64::consts::SQRT_2 - 1.0) * x.min(y))
}
/// What a grid holds.
#[derive(Debug, serde::Serialize)]
pub struct NavigationStorage {
    /// Pages of cells that are not all open flat ground.
    pub cell_pages: usize,
    /// Those of them this grid shares with no other.
    pub own_cell_pages: usize,
    /// Bodies it plans with otherwise than the map laid them.
    pub own_bodies: usize,
    pub clearance_samples: usize,
}

impl NavGrid {
    pub fn storage(&self) -> NavigationStorage {
        let (cell_pages, own_cell_pages) = self.cells.pages();
        NavigationStorage {
            cell_pages,
            own_cell_pages,
            own_bodies: self.laid.len(),
            clearance_samples: self
                .clearance
                .iter()
                .map(|c| c.borrow().iter().flatten().count() * TILE_SAMPLES)
                .sum(),
        }
    }

    /// The time the straight way from `a` to `b` takes `m` across country,
    /// judged by the ground it crosses (a wood or a hillside is slow) and
    /// by nothing that stands on it.
    fn across(&self, a: V2, b: V2, m: &Mobility) -> f64 {
        (b - a).length() / m.off_road_mps * self.slow.stretch(a, b, m)
    }

    /// Planning work done on this grid since it was built.
    pub fn work(&self) -> u64 {
        self.work.get()
    }

    /// Cells worked out again since the grid was built, as its side learned
    /// of bodies and cleared ground: what following its knowledge has cost.
    pub fn relaid(&self) -> u64 {
        self.relaid
    }

    fn spend(&self, work: u64) {
        self.work.set(self.work.get() + work);
    }
    /// Whether a vehicle of class `push` may enter the cell: walkable ground
    /// with no known body over it, or only bodies it can shove.
    fn vehicle_enters(c: &Cell, push: PushClass) -> bool {
        c.ground && c.heaviest < push.rank().max(1)
    }

    /// The lightest weight rank that stops class `push` among the bodies
    /// this grid knows: one above the heaviest it can shove that is present.
    /// Classes that meet the same bodies share it, and so a clearance field.
    fn stopping(&self, push: PushClass) -> u8 {
        let reach = push.rank().max(1);
        (1..reach)
            .rev()
            .find(|w| self.stopping[usize::from(*w)] != 0)
            .map_or(1, |w| w + 1)
    }

    /// Capping the transform means a source beyond eight cells cannot affect
    /// the tile. Evaluate the same two passes over a tile plus that finite halo.
    fn clearance_at(&self, push: PushClass, at: usize) -> f64 {
        let stop = self.stopping(push);
        let (i, j) = (at % self.nx, at / self.nx);
        let tile = (j / TILE_SIDE) * self.nx.div_ceil(TILE_SIDE) + i / TILE_SIDE;
        let local = (j % TILE_SIDE) * TILE_SIDE + i % TILE_SIDE;
        let center = cell_center(i, j);
        let near = |&[x, y, w, h]: &regions::Rect| {
            center.x + MAX_CLEARANCE_M + NAV_CELL_M >= x
                && center.x - MAX_CLEARANCE_M - NAV_CELL_M <= x + w
                && center.y + MAX_CLEARANCE_M + NAV_CELL_M >= y
                && center.y - MAX_CLEARANCE_M - NAV_CELL_M <= y + h
        };
        if !self.regions.any_near(&self.base.regions, center, near) {
            return MAX_CLEARANCE_M.min(
                i.min(self.nx - 1 - i).min(j).min(self.ny - 1 - j) as f64 * NAV_CELL_M + NAV_CELL_M,
            );
        }
        // A search pays to read a tile once each time its side has learned
        // something, whether or not the tile had to be worked out again.
        if let Some(Some(held)) = self.clearance[usize::from(stop)].borrow_mut().get_mut(tile) {
            if held.paid != self.knowledge {
                held.paid = self.knowledge;
                self.spend(TILE_WORK);
            }
            return held.values[local];
        }
        self.spend(TILE_WORK);
        let (tx, ty) = (i / TILE_SIDE * TILE_SIDE, j / TILE_SIDE * TILE_SIDE);
        let (x0, y0) = (
            tx.saturating_sub(CLEARANCE_HALO),
            ty.saturating_sub(CLEARANCE_HALO),
        );
        let (x1, y1) = (
            (tx + TILE_SIDE - 1 + CLEARANCE_HALO).min(self.nx - 1),
            (ty + TILE_SIDE - 1 + CLEARANCE_HALO).min(self.ny - 1),
        );
        let (nx, ny) = (x1 - x0 + 1, y1 - y0 + 1);
        let mut out = [0.0; CLEARANCE_SIDE * CLEARANCE_SIDE];
        for y in 0..ny {
            for x in 0..nx {
                let (ii, jj) = (x + x0, y + y0);
                let edge = ii.min(self.nx - 1 - ii).min(jj).min(self.ny - 1 - jj) as f64
                    * NAV_CELL_M
                    + NAV_CELL_M;
                let c = &self.cells[jj * self.nx + ii];
                out[y * nx + x] = if c.ground && c.heaviest < stop {
                    MAX_CLEARANCE_M.min(edge)
                } else {
                    0.0
                };
            }
        }
        let (a, b) = (NAV_CELL_M, NAV_CELL_M * std::f64::consts::SQRT_2);
        let relax = |out: &mut [f64], at: usize, from: usize, w: f64| {
            let d = out[from] + w;
            if d < out[at] {
                out[at] = d;
            }
        };
        for y in 0..ny {
            for x in 0..nx {
                let k = y * nx + x;
                if x > 0 {
                    relax(&mut out, k, k - 1, a);
                }
                if y > 0 {
                    relax(&mut out, k, k - nx, a);
                    if x > 0 {
                        relax(&mut out, k, k - nx - 1, b);
                    }
                    if x + 1 < nx {
                        relax(&mut out, k, k - nx + 1, b);
                    }
                }
            }
        }
        for y in (0..ny).rev() {
            for x in (0..nx).rev() {
                let k = y * nx + x;
                if x + 1 < nx {
                    relax(&mut out, k, k + 1, a);
                }
                if y + 1 < ny {
                    relax(&mut out, k, k + nx, a);
                    if x > 0 {
                        relax(&mut out, k, k + nx - 1, b);
                    }
                    if x + 1 < nx {
                        relax(&mut out, k, k + nx + 1, b);
                    }
                }
            }
        }
        let mut values = Box::new([0.0; TILE_SAMPLES]);
        for y in ty..(ty + TILE_SIDE).min(self.ny) {
            for x in tx..(tx + TILE_SIDE).min(self.nx) {
                values[(y - ty) * TILE_SIDE + x - tx] = out[(y - y0) * nx + x - x0];
            }
        }
        let d = values[local];
        let mut tiles = self.clearance[usize::from(stop)].borrow_mut();
        if tiles.is_empty() {
            let count = self.nx.div_ceil(TILE_SIDE) * self.ny.div_ceil(TILE_SIDE);
            tiles.resize_with(count, || None);
        }
        tiles[tile] = Some(Clearance {
            values,
            paid: self.knowledge,
        });
        d
    }

    /// The cell at `at` now carries a body of rank `new` where it carried
    /// one of rank `old`: forget every clearance tile that changes.
    fn forget_clearance(&mut self, at: usize, old: u8, new: u8) {
        let (i, j) = (at % self.nx, at / self.nx);
        let tiles_x = self.nx.div_ceil(TILE_SIDE);
        let span = |v: usize, n: usize| {
            v.saturating_sub(CLEARANCE_HALO) / TILE_SIDE
                ..=(v + CLEARANCE_HALO).min(n - 1) / TILE_SIDE
        };
        for (stop, tiles) in self.clearance.iter_mut().enumerate() {
            let tiles = tiles.get_mut();
            if tiles.is_empty() || (usize::from(old) < stop) == (usize::from(new) < stop) {
                continue;
            }
            for ty in span(j, self.ny) {
                for tx in span(i, self.nx) {
                    tiles[ty * tiles_x + tx] = None;
                }
            }
        }
    }

    fn index(&self, i: isize, j: isize) -> Option<usize> {
        (i >= 0 && j >= 0 && (i as usize) < self.nx && (j as usize) < self.ny)
            .then(|| j as usize * self.nx + i as usize)
    }

    /// Whether a footprint fits with its centre in this cell.
    fn fits(&self, cell: usize, who: Mover) -> bool {
        self.fits_off_centre(cell, who, 0.0)
    }

    /// Whether a footprint fits passing `off` metres from this cell's
    /// centre. A cell's room is measured from its centre, so a vehicle
    /// passing to one side of it has that much less.
    fn fits_off_centre(&self, cell: usize, who: Mover, off: f64) -> bool {
        let m = who.m;
        let c = &self.cells[cell];
        // The nearest blocked cell's centre is `clearance` away; its near edge
        // half a cell closer.
        let enters = match m.class {
            // Infantry's room is its sub-cell gap, judged at build.
            MoverClass::Infantry => c.infantry,
            MoverClass::Vehicle => {
                Self::vehicle_enters(c, m.push)
                    && self.clearance_at(m.push, cell) - NAV_CELL_M / 2.0 - off >= m.half_width_m
            }
        };
        enters
            && who.avoid.iter().all(|f| {
                !f.contains(
                    cell_center(cell % self.nx, cell / self.nx),
                    m.half_width_m + NAV_CELL_M / 2.0,
                )
            })
    }

    /// A step's cost on this cell: its length, or its time for the fastest
    /// route; either way stretched by the shoving speed where the class
    /// passes a body by pushing it (Q2, Q13), so A* weighs a shove against
    /// a detour.
    fn cost(&self, cell: usize, m: &Mobility, policy: RoutePolicy, length: f64) -> f64 {
        Self::cell_cost(&self.cells[cell], m, policy, length)
    }

    fn cell_cost(c: &Cell, m: &Mobility, policy: RoutePolicy, length: f64) -> f64 {
        let base = match policy {
            RoutePolicy::Shortest => length,
            RoutePolicy::Fastest => length / m.speed(c.road_factor, c.forest, c.slope_deg),
        };
        match (m.class, c.heaviest) {
            (MoverClass::Vehicle, NO_BODY) | (MoverClass::Infantry, _) => base,
            (MoverClass::Vehicle, weight) => base / m.push.shove_speed(weight),
        }
    }

    /// Nearest cell within `radius` of `p` that the footprint fits, by distance.
    fn nearest_fit(&self, p: V2, who: Mover, radius: f64) -> Option<usize> {
        let (ci, cj) = cell_of(p);
        let reach = (radius / NAV_CELL_M).ceil() as isize;
        self.spend(1);
        let mut best: Option<(f64, usize)> = None;
        for dj in -reach..=reach {
            for di in -reach..=reach {
                if let Some(k) = self.index(ci + di, cj + dj) {
                    let d = (cell_center(k % self.nx, k / self.nx) - p).length();
                    if d <= radius
                        && self.fits(k, who)
                        && best.is_none_or(|(bd, bk)| (d, k) < (bd, bk))
                    {
                        best = Some((d, k));
                    }
                }
            }
        }
        best.map(|(_, k)| k)
    }

    /// Whether any part of the remaining route crosses a known body this
    /// footprint would shove aside: a pusher weighs every such body against
    /// a detour when it learns of it (Q13).
    pub fn route_pushes(&self, from: V2, route: &[V2], m: &Mobility) -> bool {
        if m.class == MoverClass::Infantry {
            return false;
        }
        let mut a = from;
        route.iter().any(|&b| {
            let length = (b - a).length();
            let samples = ((length / (NAV_CELL_M / 4.0)).ceil() as usize).max(1);
            let pushes = (0..samples).any(|k| {
                let p = a + (b - a) * ((k as f64 + 0.5) / samples as f64);
                let (i, j) = cell_of(p);
                self.index(i, j)
                    .is_some_and(|c| self.cells[c].heaviest != NO_BODY)
            });
            a = b;
            pushes
        })
    }

    /// Whether `p` lies in a cell the footprint fits (for infantry, in one
    /// of its free sub-cells).
    pub fn fits_at(&self, p: V2, m: &Mobility) -> bool {
        self.stands(p, Mover::free(m))
    }

    fn stands(&self, p: V2, who: Mover) -> bool {
        let (i, j) = cell_of(p);
        self.index(i, j).is_some_and(|k| {
            // A vehicle standing off the middle of its cell has that much
            // less of the cell's room.
            let off = (cell_center(k % self.nx, k / self.nx) - p).length();
            self.fits_off_centre(k, who, off)
                && (who.m.class != MoverClass::Infantry
                    || self.cells[k].free & (1 << sub_of(p)) != 0)
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
        self.nearest_fit(p, Mover::free(m), radius)
            .map(|k| self.waypoint(k, m))
    }

    /// Certify the entire nine-cell stencil from conservative source reach.
    /// This reuses implicit cell answers; no sampled cost arithmetic changes.
    fn uniform_stencil(&self, cell: usize, who: Mover) -> bool {
        let m = who.m;
        let (x, y) = (cell % self.nx, cell / self.nx);
        if !self.cells.implicit[3].ground
            || x == 0
            || y == 0
            || x + 1 >= self.nx
            || y + 1 >= self.ny
            || !m.half_width_m.is_finite()
        {
            return false;
        }
        let border = x.min(self.nx - 1 - x).min(y).min(self.ny - 1 - y);
        if m.class == MoverClass::Vehicle
            && MAX_CLEARANCE_M.min(border as f64 * NAV_CELL_M) - NAV_CELL_M / 2.0 < m.half_width_m
        {
            return false;
        }
        let p = cell_center(x, y);
        // The cap plus neighbour reach, sampling border and crossing reach.
        let halo = MAX_CLEARANCE_M + 4.0 * NAV_CELL_M;
        let near = |&[x, y, w, h]: &regions::Rect| {
            [x, y, w, h].iter().any(|v| !v.is_finite())
                || w < 0.0
                || h < 0.0
                || (p.x + halo >= x
                    && p.x - halo <= x + w
                    && p.y + halo >= y
                    && p.y - halo <= y + h)
        };
        if self.regions.any_near(&self.base.regions, p, near) {
            return false;
        }
        !who.avoid.iter().any(|b| {
            let reach = b.half.length()
                + ((m.half_width_m + NAV_CELL_M / 2.0).max(0.0) + NAV_CELL_M)
                    * std::f64::consts::SQRT_2;
            !reach.is_finite()
                || !b.center.x.is_finite()
                || !b.center.y.is_finite()
                || (p.x + reach >= b.center.x
                    && p.x - reach <= b.center.x
                    && p.y + reach >= b.center.y
                    && p.y - reach <= b.center.y)
        })
    }

    /// Cost of travelling a straight segment, walking the cells it crosses; `None`
    /// when any sampled cell does not fit the footprint.
    /// Infantry samples every half sub-cell, and each sample must lie in a
    /// free sub-cell.
    fn segment_cost(&self, a: V2, b: V2, who: Mover, policy: RoutePolicy) -> Option<f64> {
        let mut probe = Probe::new(a, b, who.m);
        loop {
            if let Some(cost) = self.read(&mut probe, who, policy) {
                return cost;
            }
        }
    }

    /// Read the next stretch of `probe`'s segment: `None` while there is
    /// more of it to read, then its cost, or `None` if a sample did not fit.
    fn read(&self, probe: &mut Probe, who: Mover, policy: RoutePolicy) -> Option<Option<f64>> {
        let m = who.m;
        let Probe { a, b, samples, .. } = *probe;
        let infantry = m.class == MoverClass::Infantry;
        let length = (b - a).length();
        let piece = length / samples as f64;
        let along = if length > 0.0 {
            (b - a) * (1.0 / length)
        } else {
            b - a
        };
        let until = (probe.next + Probe::STRETCH).min(samples);
        self.spend(1 + ((until - probe.next) / SAMPLES_PER_WORK) as u64);
        for k in probe.next..until {
            let p = a + (b - a) * ((k as f64 + 0.5) / samples as f64);
            let (i, j) = cell_of(p);
            let Some(cell) = self.index(i, j) else {
                return Some(None);
            };
            // How far the segment passes from the middle of the cell.
            let off = (cell_center(cell % self.nx, cell / self.nx) - a)
                .cross(along)
                .abs();
            if !self.fits_off_centre(cell, who, off)
                || (infantry && self.cells[cell].free & (1 << sub_of(p)) == 0)
            {
                return Some(None);
            }
            probe.total += self.cost(cell, m, policy, piece);
        }
        probe.next = until;
        (until == samples).then_some(Some(probe.total))
    }

    /// Whether any known blocker now overlaps the remaining route for this footprint.
    pub fn route_fits(&self, from: V2, route: &[V2], m: &Mobility) -> bool {
        let mut a = from;
        route.iter().all(|&b| {
            let ok = self
                .segment_cost(a, b, Mover::free(m), RoutePolicy::Shortest)
                .is_some();
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
                    .segment_cost(a, b, Mover::free(m), RoutePolicy::Fastest)
                    .unwrap_or(f64::INFINITY);
                a = b;
                t
            })
            .sum()
    }
}
