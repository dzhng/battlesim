//! Route planning over one side's known geometry. A 2 m grid classifies each
//! cell by the walkable surface under its centre (the same triangle rule the
//! world uses) and by the known bodies that block each mover class. The
//! navigation classes are infantry plus one per vehicle push class (Q13): a
//! vehicle cell remembers the heaviest body over it, so a class that can
//! shove that body passes it at the cost of its shoving speed, and a class
//! that cannot is blocked. A clearance field per push class lets a
//! footprint of any width ask whether it fits. Plans are A* over octile
//! moves without corner cutting, then string-pulled only where that keeps
//! the cost.
//!
//! Infantry reads the grid at two resolutions (Q27). Each 2 m cell holds a
//! 4×4 mask of 0.5 m sub-cells, set where a soldier's disc stands clear of
//! every known body. A cell is open to a squad when its free sub-cells form
//! one connected gap, and a move between two cells when free sub-cells meet
//! across their shared edge: a line of teeth or a gap between wrecks stays
//! open, a wall stays closed. Soldiers then find their own way through the
//! gap on the exact bodies (`movement::final_leg`).
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::ops::{Index, IndexMut};

use contract::command::RoutePolicy;
use contract::map::MoverClass;
use contract::scenario::PushClass;

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
const TILE_SIDE: usize = 32;
const TILE_SAMPLES: usize = TILE_SIDE * TILE_SIDE;
const CLEARANCE_HALO: usize = (MAX_CLEARANCE_M / NAV_CELL_M) as usize;
const CLEARANCE_SIDE: usize = TILE_SIDE + 2 * CLEARANCE_HALO;

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

/// No known body that stops vehicles covers a cell.
const NO_BODY: u8 = 0;
/// Push classes, and so at most this many vehicle clearance fields.
const PUSH_CLASSES: usize = PushClass::ALL.len();

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Cell {
    /// The ground under the centre is walkable.
    ground: bool,
    /// Infantry: the ground is walkable and the free sub-cells are one gap.
    infantry: bool,
    /// The heaviest known body stopping vehicles over the cell: its weight
    /// class's rank, or [`NO_BODY`].
    heaviest: u8,
    road: bool,
    forest: bool,
    slope_deg: f64,
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

/// Open flat ground is implicit; only sampled surface/body exceptions own cells.
struct Cells {
    changed: HashMap<usize, Cell>,
    implicit: [Cell; 4],
    nx: usize,
    ny: usize,
}
impl Cells {
    fn new(traversable: bool, nx: usize, ny: usize) -> Self {
        Self {
            changed: HashMap::new(),
            nx,
            ny,
            implicit: std::array::from_fn(|open| Cell {
                ground: traversable,
                infantry: traversable,
                heaviest: NO_BODY,
                road: false,
                forest: false,
                slope_deg: 0.0,
                free: if traversable { ALL_FREE } else { 0 },
                open: if traversable { open as u8 } else { 0 },
            }),
        }
    }
    fn default_at(&self, at: usize) -> Cell {
        let (i, j) = (at % self.nx, at / self.nx);
        self.implicit[usize::from(i + 1 < self.nx) + 2 * usize::from(j + 1 < self.ny)]
    }
}
impl Index<usize> for Cells {
    type Output = Cell;
    fn index(&self, at: usize) -> &Cell {
        if let Some(cell) = self.changed.get(&at) {
            return cell;
        }
        let (i, j) = (at % self.nx, at / self.nx);
        &self.implicit[usize::from(i + 1 < self.nx) + 2 * usize::from(j + 1 < self.ny)]
    }
}
impl IndexMut<usize> for Cells {
    fn index_mut(&mut self, at: usize) -> &mut Cell {
        let implicit = self.default_at(at);
        self.changed.entry(at).or_insert(implicit)
    }
}

pub struct NavGrid {
    nx: usize,
    ny: usize,
    cells: Cells,
    /// Exact capped distance-transform samples, by distinct stopping rank.
    clearance: [RefCell<HashMap<usize, Box<[f64; TILE_SAMPLES]>>>; PUSH_CLASSES],
    /// Bit w is set when a known stopping body has weight rank w.
    weights: u8,
    /// Temporary obstacles for the plan in progress.
    avoid: Vec<Obb2>,
    /// Visited search bookkeeping retained between plans.
    scratch: Scratch,
    /// Conservative rectangles containing every nonuniform surface or known body.
    regions: Vec<[f64; 4]>,
    queued: usize,
    heap_peak: usize,
    expanded: usize,
    stale: usize,
}
#[derive(Clone, Copy)]
struct Search {
    g: f64,
    parent: u32,
}

/// Every route crossing this row/column must visit one of these cells.
struct SearchCut {
    vertical: bool,
    at: usize,
    low: usize,
    high: usize,
    portals: Vec<usize>,
    openings: Vec<[usize; 2]>,
}
impl SearchCut {
    fn distance(&self, a: usize, b: usize, nx: usize) -> f64 {
        let coordinate = |k| {
            if self.vertical {
                (k % nx, k / nx)
            } else {
                (k / nx, k % nx)
            }
        };
        let ((ax, mut ay), (bx, mut by)) = (coordinate(a), coordinate(b));
        let (mut dx, mut ex) = (ax.abs_diff(self.at), bx.abs_diff(self.at));
        if ay > by {
            std::mem::swap(&mut ay, &mut by);
            std::mem::swap(&mut dx, &mut ex);
        }
        // The octile sum is convex in the free coordinate. Between the two
        // endpoint coordinates its slopes change only at ay+dx and by-ex.
        // Their lesser breakpoint, clamped between endpoints, begins the
        // minimum plateau. Clamp that minimizer into each opening interval.
        let optimum = (ay + dx).min(by.saturating_sub(ex)).clamp(ay, by);
        self.openings
            .iter()
            .map(|&[lo, hi]| {
                let y = optimum.clamp(lo, hi);
                let p = if self.vertical {
                    y * nx + self.at
                } else {
                    self.at * nx + y
                };
                octile(a, p, nx) + octile(p, b, nx)
            })
            .fold(f64::INFINITY, f64::min)
    }
}

struct SearchBound {
    upper: f64,
    unit: f64,
    minimum_unit: f64,
    discount: f64,
    rectangle_discount: f64,
    rounding: f64,
    cut: Option<SearchCut>,
    target: usize,
    nx: usize,
}
fn octile(a: usize, b: usize, nx: usize) -> f64 {
    let x = (a % nx).abs_diff(b % nx) as f64;
    let y = (a / nx).abs_diff(b / nx) as f64;
    NAV_CELL_M * (x.max(y) + (std::f64::consts::SQRT_2 - 1.0) * x.min(y))
}
impl SearchBound {
    fn rejects(&self, cell: usize, g: f64) -> bool {
        let distance = octile(cell, self.target, self.nx);
        let mut base = distance * self.unit;
        let mut lower =
            (base - self.discount.min(self.rectangle_discount)).max(distance * self.minimum_unit);
        if let Some(cut) = &self.cut {
            let coordinate = |k| {
                if cut.vertical {
                    k % self.nx
                } else {
                    k / self.nx
                }
            };
            let (a, b) = (coordinate(cell), coordinate(self.target));
            if (a < cut.at && b > cut.at) || (a > cut.at && b < cut.at) {
                let via = cut.distance(cell, self.target, self.nx);
                base = base.max(via * self.unit);
                // The mandatory visit splits the route into two legs. Each
                // relaxed leg can use the faster rectangle once; discounting
                // only one diameter here would overestimate some detours.
                lower = lower.max(
                    (via * self.unit - self.discount.min(2.0 * self.rectangle_discount))
                        .max(via * self.minimum_unit),
                );
            }
        }
        // Strict pruning retains all equal-cost chains and queue/parent ties.
        // The allowance covers both bound arithmetic and the original A*'s
        // accumulated additions and slightly rounded Euclidean priority.
        g + lower > self.upper + self.rounding * (g + base + self.discount + self.upper)
    }
}

struct SearchTile {
    g: [f64; TILE_SAMPLES],
    parent: [u32; TILE_SAMPLES],
    stamp: [u32; TILE_SAMPLES],
}
struct Scratch {
    nx: usize,
    tiles: HashMap<usize, Box<SearchTile>>,
    generation: u32,
    visited: usize,
}
impl Scratch {
    fn new(nx: usize) -> Self {
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
    fn get(&self, k: &usize) -> Option<Search> {
        let (key, at) = self.location(*k);
        let tile = self.tiles.get(&key)?;
        (tile.stamp[at] == self.generation).then(|| Search {
            g: tile.g[at],
            parent: tile.parent[at],
        })
    }
    fn insert(&mut self, k: usize, value: Search) {
        let (key, at) = self.location(k);
        let tile = self.tiles.entry(key).or_insert_with(|| {
            Box::new(SearchTile {
                g: [0.0; TILE_SAMPLES],
                parent: [0; TILE_SAMPLES],
                stamp: [0; TILE_SAMPLES],
            })
        });
        if tile.stamp[at] != self.generation {
            self.visited += 1;
        }
        tile.g[at] = value.g;
        tile.parent[at] = value.parent;
        tile.stamp[at] = self.generation;
    }
    fn len(&self) -> usize {
        self.visited
    }
    fn capacity(&self) -> usize {
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

/// Retained container capacities and the latest search's peak queue storage.
#[derive(Debug, serde::Serialize)]
pub struct NavigationStorage {
    pub cells: usize,
    pub cell_capacity: usize,
    pub clearance_samples: usize,
    /// Hash-table capacity in tile entries; each tile owns 1024 f64 samples.
    pub clearance_capacity: usize,
    pub search_cells: usize,
    /// Retained samples, including untouched positions in resident tiles.
    pub search_capacity: usize,
    pub queued: usize,
    /// Peak Vec capacity in Open entries for the latest search.
    pub heap_capacity: usize,
    pub expanded: usize,
    pub stale: usize,
}

impl NavGrid {
    pub fn storage(&self) -> NavigationStorage {
        NavigationStorage {
            cells: self.cells.changed.len(),
            cell_capacity: self.cells.changed.capacity(),
            clearance_samples: self
                .clearance
                .iter()
                .map(|c| c.borrow().len() * TILE_SAMPLES)
                .sum(),
            clearance_capacity: self.clearance.iter().map(|c| c.borrow().capacity()).sum(),
            search_cells: self.scratch.len(),
            search_capacity: self.scratch.capacity(),
            queued: self.queued,
            heap_capacity: self.heap_peak,
            expanded: self.expanded,
            stale: self.stale,
        }
    }
    /// Build from the world's surfaces plus the props this side knows about,
    /// where it believes they stand. `known_props` are the movement blockers
    /// the planner may use; the world's own prop list is ignored here so
    /// hidden changes cannot leak into routes. `soldier_radius` sizes
    /// infantry's sub-cell gaps.
    pub fn build(
        world: &WorldGeometry,
        known_props: impl Iterator<Item = Prop>,
        soldier_radius: f64,
    ) -> Self {
        let nx = (world.width() / NAV_CELL_M).floor() as usize;
        let ny = (world.depth() / NAV_CELL_M).floor() as usize;
        let mut cells = Cells::new(world.slope_cutoff_deg() > 0.0, nx, ny);
        let mut regions = world.navigation_regions();
        let mut weights = 0u8;
        for &[x, y, w, h] in &regions {
            let (i0, j0) = cell_of(v2(x - NAV_CELL_M, y - NAV_CELL_M));
            let (i1, j1) = cell_of(v2(x + w + NAV_CELL_M, y + h + NAV_CELL_M));
            for j in j0.max(0)..=j1.min(ny as isize - 1) {
                for i in i0.max(0)..=i1.min(nx as isize - 1) {
                    let (i, j) = (i as usize, j as usize);
                    let c = cell_center(i, j);
                    if let Some(s) = world.surface_at(c.x, c.y) {
                        let cell = Cell {
                            ground: s.traversable,
                            infantry: s.traversable,
                            heaviest: NO_BODY,
                            road: s.kind == SurfaceKind::Road || s.kind == SurfaceKind::Bridge,
                            forest: s.forest,
                            slope_deg: s.slope_deg,
                            free: if s.traversable { ALL_FREE } else { 0 },
                            open: if s.traversable {
                                u8::from(i + 1 < nx) + 2 * u8::from(j + 1 < ny)
                            } else {
                                0
                            },
                        };
                        if cell != cells.default_at(j * nx + i) {
                            cells.changed.insert(j * nx + i, cell);
                        }
                    }
                }
            }
        }
        // Beside ground nobody crosses (water, a slope past the cutoff), each
        // sub-cell reads the ground under its own centre.
        let mut border = Vec::new();
        for (&at, cell) in &cells.changed {
            if cell.free != 0 {
                continue;
            }
            let (i, j) = (at % nx, at / nx);
            for jj in j.saturating_sub(1)..=(j + 1).min(ny - 1) {
                for ii in i.saturating_sub(1)..=(i + 1).min(nx - 1) {
                    if cells[jj * nx + ii].free != 0 {
                        border.push(jj * nx + ii);
                    }
                }
            }
        }
        border.sort_unstable();
        border.dedup();
        for at in border {
            let (i, j) = (at % nx, at / nx);
            for bit in 0..SUB * SUB {
                let c = sub_center(i, j, bit);
                if !world.traversable_at(c.x, c.y) {
                    cells[at].free &= !(1 << bit);
                }
            }
        }
        for prop in known_props {
            let stops_vehicles = prop.blocks(MoverClass::Vehicle);
            let stops_infantry = prop.blocks(MoverClass::Infantry);
            if !stops_vehicles && !stops_infantry {
                continue;
            }
            let weight = prop.body.weight_class.rank();
            if stops_vehicles && usize::from(weight) < u8::BITS as usize {
                weights |= 1 << weight;
            }
            let footprint = prop.footprint();
            let radius = prop.footprint_radius() + soldier_radius * std::f64::consts::SQRT_2;
            regions.push([
                prop.center.x - radius,
                prop.center.y - radius,
                2.0 * radius,
                2.0 * radius,
            ]);
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
                    if stops_vehicles
                        && vehicles
                        && footprint.contains(cell_center(i, j), NAV_CELL_M / 2.0)
                    {
                        cell.heaviest = cell.heaviest.max(weight);
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
        for cell in cells.changed.values_mut() {
            cell.infantry = cell.free != 0 && one_gap(cell.free);
        }
        // A changed cell also changes its west/south neighbour's crossing bits.
        let mut affected: Vec<usize> = cells.changed.keys().copied().collect();
        for at in affected.clone() {
            if at % nx > 0 {
                affected.push(at - 1);
            }
            if at / nx > 0 {
                affected.push(at - nx);
            }
        }
        affected.sort_unstable();
        affected.dedup();
        for at in affected {
            let (i, j) = (at % nx, at / nx);
            let free = cells[at].free;
            let mut open = 0;
            if cells[at].infantry {
                if i + 1 < nx && cells[at + 1].infantry {
                    let east = (free & EAST_COLUMN) >> (SUB - 1);
                    open |= u8::from(east & cells[at + 1].free & WEST_COLUMN != 0);
                }
                if j + 1 < ny && cells[at + nx].infantry {
                    let north = (free & NORTH_ROW) >> (SUB * (SUB - 1));
                    open |= u8::from(north & cells[at + nx].free & SOUTH_ROW != 0) << 1;
                }
            }
            cells[at].open = open;
        }
        let implicit = cells.implicit;
        cells.changed.retain(|&at, cell| {
            let (i, j) = (at % nx, at / nx);
            *cell != implicit[usize::from(i + 1 < nx) + 2 * usize::from(j + 1 < ny)]
        });
        NavGrid {
            nx,
            ny,
            cells,
            clearance: Default::default(),
            weights,
            avoid: Vec::new(),
            scratch: Scratch::new(nx),
            regions,
            queued: 0,
            heap_peak: 0,
            expanded: 0,
            stale: 0,
        }
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
            .find(|w| self.weights & (1 << w) != 0)
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
        if !self.regions.iter().any(|&[x, y, w, h]| {
            center.x + MAX_CLEARANCE_M + NAV_CELL_M >= x
                && center.x - MAX_CLEARANCE_M - NAV_CELL_M <= x + w
                && center.y + MAX_CLEARANCE_M + NAV_CELL_M >= y
                && center.y - MAX_CLEARANCE_M - NAV_CELL_M <= y + h
        }) {
            return MAX_CLEARANCE_M.min(
                i.min(self.nx - 1 - i).min(j).min(self.ny - 1 - j) as f64 * NAV_CELL_M + NAV_CELL_M,
            );
        }
        if let Some(values) = self.clearance[usize::from(stop)].borrow().get(&tile) {
            return values[local];
        }
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
        self.clearance[usize::from(stop)]
            .borrow_mut()
            .insert(tile, values);
        d
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
        let enters = match m.class {
            // Infantry's room is its sub-cell gap, judged at build.
            MoverClass::Infantry => c.infantry,
            MoverClass::Vehicle => {
                Self::vehicle_enters(c, m.push)
                    && self.clearance_at(m.push, cell) - NAV_CELL_M / 2.0 >= m.half_width_m
            }
        };
        enters
            && self.avoid.iter().all(|f| {
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
            RoutePolicy::Fastest => length / m.speed(c.road, c.forest, c.slope_deg),
        };
        match (m.class, c.heaviest) {
            (MoverClass::Vehicle, NO_BODY) | (MoverClass::Infantry, _) => base,
            (MoverClass::Vehicle, weight) => base / m.push.shove_speed(weight),
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
        self.scratch.clear();
        self.queued = 0;
        self.heap_peak = 0;
        self.expanded = 0;
        self.stale = 0;
        if self.separated(start, target, m) {
            return Plan::Blocked(BlockReason::NoRoute);
        }
        if self.uniform_segment(from, goal, start, target, m, policy) {
            return Plan::Route(vec![goal]);
        }
        let heuristic_speed = match policy {
            RoutePolicy::Shortest => 1.0,
            RoutePolicy::Fastest => m.max_speed(),
        };
        let target_center = cell_center(target % self.nx, target / self.nx);
        let h = |k: usize, nx: usize| {
            (cell_center(k % nx, k / nx) - target_center).length() / heuristic_speed
        };
        let bound = self.search_bound(start, target, m, policy);
        let mut open = BinaryHeap::new();
        self.scratch.insert(
            start,
            Search {
                g: 0.0,
                parent: start as u32,
            },
        );
        open.push(Open {
            f: h(start, self.nx),
            cell: start as u32,
        });
        self.queued = 1;
        self.heap_peak = open.capacity();
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
            let g = self.scratch.get(&cell).expect("a queued cell has a cost").g;
            if f > g + h(cell, self.nx) {
                self.stale += 1;
                continue; // stale entry
            }
            self.expanded += 1;
            let (ci, cj) = ((cell % self.nx) as isize, (cell / self.nx) as isize);
            // Geometry is fixed for the search. Classify each neighbour once;
            // diagonal moves reuse the same orthogonal fit/crossing answers.
            let nexts = STEPS.map(|(di, dj)| self.index(ci + di, cj + dj));
            let uniform = self.uniform_stencil(cell, m);
            let fits = nexts.map(|next| next.is_some_and(|k| uniform || self.fits(k, m)));
            let crosses: [bool; 4] = std::array::from_fn(|k| {
                fits[k] && (uniform || self.crosses(cell, nexts[k].unwrap(), m))
            });
            let c = if uniform {
                &self.cells.implicit[3]
            } else {
                &self.cells[cell]
            };
            let costs = [
                Self::cell_cost(c, m, policy, NAV_CELL_M),
                Self::cell_cost(c, m, policy, NAV_CELL_M * std::f64::consts::SQRT_2),
            ];
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
                    let via = |k: usize| {
                        crosses[k] && (uniform || self.crosses(nexts[k].unwrap(), next, m))
                    };
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
                    self.cost(next, m, policy, length)
                };
                let step = (costs[usize::from(diagonal)] + next_cost) / 2.0;
                let tentative = g + step;
                if self.scratch.get(&next).is_none_or(|s| tentative < s.g) {
                    if bound.as_ref().is_some_and(|b| b.rejects(next, tentative)) {
                        continue;
                    }
                    self.scratch.insert(
                        next,
                        Search {
                            g: tentative,
                            parent: cell as u32,
                        },
                    );
                    open.push(Open {
                        f: tentative + h(next, self.nx),
                        cell: next as u32,
                    });
                    self.queued += 1;
                    self.heap_peak = self.heap_peak.max(open.capacity());
                }
            }
        }
        if !found {
            return Plan::Blocked(BlockReason::NoRoute);
        }
        let mut cells = vec![target];
        while *cells.last().unwrap() != start {
            let k = *cells.last().unwrap();
            cells.push(
                self.scratch
                    .get(&k)
                    .expect("a route cell has a parent")
                    .parent as usize,
            );
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

    /// Certify the entire nine-cell stencil from conservative source reach.
    /// This reuses implicit cell answers; no sampled cost arithmetic changes.
    fn uniform_stencil(&self, cell: usize, m: &Mobility) -> bool {
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
        if self.regions.iter().any(|&[x, y, w, h]| {
            [x, y, w, h].iter().any(|v| !v.is_finite())
                || w < 0.0
                || h < 0.0
                || (p.x + halo >= x
                    && p.x - halo <= x + w
                    && p.y + halo >= y
                    && p.y - halo <= y + h)
        }) {
            return false;
        }
        !self.avoid.iter().any(|b| {
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

    /// Find a feasible cost bound without assigning any winning parents. The
    /// monotone walks are checked against this search's exact footprint,
    /// avoidance, shared-edge and no-corner-cut graph. Failure leaves A* intact.
    fn walk_bound(
        &self,
        from: usize,
        to: usize,
        m: &Mobility,
        policy: RoutePolicy,
        mut total: f64,
    ) -> Option<f64> {
        let mut at = from;
        while at != to {
            let (x, y) = (at % self.nx, at / self.nx);
            let (tx, ty) = (to % self.nx, to / self.nx);
            let di = (tx as isize - x as isize).signum();
            let dj = (ty as isize - y as isize).signum();
            let next = self.index(x as isize + di, y as isize + dj)?;
            if !self.fits(next, m) {
                return None;
            }
            let diagonal = di != 0 && dj != 0;
            if diagonal {
                let a = self.index(x as isize + di, y as isize)?;
                let b = self.index(x as isize, y as isize + dj)?;
                if !self.fits(a, m)
                    || !self.fits(b, m)
                    || !self.crosses(at, a, m)
                    || !self.crosses(a, next, m)
                    || !self.crosses(at, b, m)
                    || !self.crosses(b, next, m)
                {
                    return None;
                }
            } else if !self.crosses(at, next, m) {
                return None;
            }
            let length = if diagonal {
                NAV_CELL_M * std::f64::consts::SQRT_2
            } else {
                NAV_CELL_M
            };
            total += (self.cost(at, m, policy, length) + self.cost(next, m, policy, length)) / 2.0;
            if !total.is_finite() {
                return None;
            }
            at = next;
        }
        Some(total)
    }

    fn search_cut(&self, start: usize, target: usize, m: &Mobility) -> Option<SearchCut> {
        let mut counts = [HashMap::<usize, usize>::new(), HashMap::new()];
        for (&k, c) in &self.cells.changed {
            let enters = match m.class {
                MoverClass::Infantry => c.infantry,
                MoverClass::Vehicle => Self::vehicle_enters(c, m.push),
            };
            if !enters {
                *counts[0].entry(k % self.nx).or_default() += 1;
                *counts[1].entry(k / self.nx).or_default() += 1;
            }
        }
        let mut best = None;
        for (axis, columns) in counts.iter().enumerate() {
            let vertical = axis == 0;
            let coordinate = |k| if vertical { k % self.nx } else { k / self.nx };
            let (a, b) = (coordinate(start), coordinate(target));
            let extent = if vertical { self.ny } else { self.nx };
            let candidates: Vec<_> = columns
                .iter()
                .filter(|&(at, count)| *at > a.min(b) && *at < a.max(b) && *count > extent / 2)
                .collect();
            let Some((&at, &count)) = candidates.iter().copied().max_by_key(|&(at, count)| {
                (
                    *count,
                    std::cmp::Reverse((2 * *at).abs_diff(a + b)),
                    std::cmp::Reverse(*at),
                )
            }) else {
                continue;
            };
            // A wide opening offers little useful pruning and expensive bounds.
            // This is an optional proof optimization, never a route admission cap.
            if extent - count > 64 {
                continue;
            }
            let low = candidates.iter().map(|(at, _)| **at).min().unwrap();
            let high = candidates.iter().map(|(at, _)| **at).max().unwrap();
            let portals: Vec<_> = (0..extent)
                .map(|other| {
                    if vertical {
                        other * self.nx + at
                    } else {
                        at * self.nx + other
                    }
                })
                .filter(|&k| self.fits(k, m))
                .collect();
            let mut openings: Vec<[usize; 2]> = Vec::new();
            for &k in &portals {
                let y = if vertical { k / self.nx } else { k % self.nx };
                match openings.last_mut() {
                    Some([_, end]) if *end + 1 == y => *end = y,
                    _ => openings.push([y, y]),
                }
            }
            if !portals.is_empty() && best.as_ref().is_none_or(|(_, n)| count > *n) {
                best = Some((
                    SearchCut {
                        vertical,
                        at,
                        low,
                        high,
                        portals,
                        openings,
                    },
                    count,
                ));
            }
        }
        best.map(|(cut, _)| cut)
    }

    fn search_bound(
        &self,
        start: usize,
        target: usize,
        m: &Mobility,
        policy: RoutePolicy,
    ) -> Option<SearchBound> {
        let unit = match policy {
            RoutePolicy::Shortest => 1.0,
            RoutePolicy::Fastest => 1.0 / m.off_road_mps,
        };
        if !unit.is_normal() || unit <= 0.0 {
            return None;
        }
        let mut minimum_unit = unit;
        let mut discount = 0.0;
        let mut rectangle_discount = f64::INFINITY;
        let mut faster_bounds: Option<[usize; 4]> = None;
        for (&k, c) in &self.cells.changed {
            let enters = match m.class {
                MoverClass::Infantry => c.infantry,
                MoverClass::Vehicle => Self::vehicle_enters(c, m.push),
            };
            if !enters {
                continue;
            }
            let cost = self.cost(k, m, policy, 1.0);
            if !cost.is_normal() || cost <= 0.0 {
                return None;
            }
            minimum_unit = minimum_unit.min(cost);
            // In a simple route a cell contributes at most two half-diagonal
            // steps. Discount every faster cell once, including cells the
            // actual route never reaches: this can only lower the estimate.
            discount += (unit - cost).max(0.0) * NAV_CELL_M * std::f64::consts::SQRT_2;
            if cost < unit {
                let (x, y) = (k % self.nx, k / self.nx);
                faster_bounds = Some(match faster_bounds {
                    None => [x, y, x, y],
                    Some([x0, y0, x1, y1]) => [x0.min(x), y0.min(y), x1.max(x), y1.max(y)],
                });
            }
        }
        if let Some([x0, y0, x1, y1]) = faster_bounds {
            // Relax all faster-cell half-edges to a convex rectangle at the
            // fastest cost. A minimum-cost relaxed path visits that rectangle
            // once: an excursion can be replaced inside it by the octile
            // segment. Its saving is at most the rectangle's octile diameter
            // times the unit-cost difference, including half-edge reach.
            let (x, y) = ((x1 - x0 + 1) as f64, (y1 - y0 + 1) as f64);
            let diameter = NAV_CELL_M * (x.max(y) + (std::f64::consts::SQRT_2 - 1.0) * x.min(y));
            rectangle_discount = diameter * (unit - minimum_unit);
        }
        let heuristic_unit = match policy {
            RoutePolicy::Shortest => 1.0,
            RoutePolicy::Fastest => 1.0 / m.max_speed(),
        };
        if !discount.is_finite()
            || !heuristic_unit.is_finite()
            || heuristic_unit <= 0.0
            || heuristic_unit > minimum_unit * (1.0 + 8.0 * f64::EPSILON)
        {
            return None;
        }
        // Positive finite costs give simple improving parent chains of at most
        // N cells. 128*N*EPS exceeds the gamma error of cost divisions, means,
        // path sums, octile/cut arithmetic, discount sum and old goal-pop bound.
        // Outside the small-error regime use the unchanged exhaustive planner.
        let samples = self.nx.checked_mul(self.ny)?.checked_add(1)?;
        let rounding = 128.0 * samples as f64 * f64::EPSILON;
        if rounding >= 0.001 {
            return None;
        }
        let cut = self.search_cut(start, target, m);
        let mut upper = self
            .walk_bound(start, target, m, policy, 0.0)
            .unwrap_or(f64::INFINITY);
        if let Some(cut) = &cut {
            let mut portals = cut.portals.clone();
            portals.sort_by(|&a, &b| {
                (octile(start, a, self.nx) + octile(a, target, self.nx))
                    .total_cmp(&(octile(start, b, self.nx) + octile(b, target, self.nx)))
                    .then(a.cmp(&b))
            });
            for portal in portals.into_iter().take(8) {
                let other = if cut.vertical {
                    portal / self.nx
                } else {
                    portal % self.nx
                };
                let point = |axis| {
                    if cut.vertical {
                        other * self.nx + axis
                    } else {
                        axis * self.nx + other
                    }
                };
                let limit = if cut.vertical { self.nx } else { self.ny };
                let (lo, hi) = (
                    cut.low.saturating_sub(CLEARANCE_HALO + 1),
                    (cut.high + CLEARANCE_HALO + 1).min(limit - 1),
                );
                let (a, b) = if (if cut.vertical {
                    start % self.nx
                } else {
                    start / self.nx
                }) < cut.at
                {
                    (point(lo), point(hi))
                } else {
                    (point(hi), point(lo))
                };
                if !self.fits(a, m) || !self.fits(b, m) {
                    continue;
                }
                if let Some(cost) = self
                    .walk_bound(start, a, m, policy, 0.0)
                    .and_then(|cost| self.walk_bound(a, b, m, policy, cost))
                    .and_then(|cost| self.walk_bound(b, target, m, policy, cost))
                {
                    upper = upper.min(cost);
                }
            }
        }
        upper.is_finite().then_some(SearchBound {
            upper,
            unit,
            minimum_unit,
            discount,
            rectangle_discount,
            rounding,
            cut,
            target,
            nx: self.nx,
        })
    }

    /// A closed row or column across the grid is a cut in the existing
    /// eight-neighbour graph. This certifies disconnection without a flood fill.
    fn separated(&self, start: usize, target: usize, m: &Mobility) -> bool {
        let (sx, sy) = (start % self.nx, start / self.nx);
        let (tx, ty) = (target % self.nx, target / self.nx);
        let mut columns = HashMap::<usize, usize>::new();
        let mut rows = HashMap::<usize, usize>::new();
        for (&at, c) in &self.cells.changed {
            let enters = match m.class {
                MoverClass::Infantry => c.infantry,
                MoverClass::Vehicle => Self::vehicle_enters(c, m.push),
            };
            if enters {
                continue;
            }
            let (x, y) = (at % self.nx, at / self.nx);
            if x > sx.min(tx) && x < sx.max(tx) {
                let count = columns.entry(x).or_default();
                *count += 1;
                if *count == self.ny {
                    return true;
                }
            }
            if y > sy.min(ty) && y < sy.max(ty) {
                let count = rows.entry(y).or_default();
                *count += 1;
                if *count == self.nx {
                    return true;
                }
            }
        }
        false
    }

    /// In a uniform, open endpoint rectangle every optimal grid path is
    /// monotonic, and string-pulling returns the goal alone. Faster terrain
    /// anywhere else invalidates this certificate for the fastest policy.
    fn uniform_segment(
        &self,
        from: V2,
        goal: V2,
        start: usize,
        target: usize,
        m: &Mobility,
        policy: RoutePolicy,
    ) -> bool {
        // The existing crossing graph requires distinct east/north strides.
        if self.nx < 2 || !self.fits_at(from, m) {
            return false;
        }
        if policy == RoutePolicy::Fastest {
            let diagonal = Self::cell_cost(
                &self.cells.implicit[3],
                m,
                policy,
                NAV_CELL_M * std::f64::consts::SQRT_2,
            );
            let ceiling = ((diagonal + diagonal) / 2.0) * (self.nx * self.ny) as f64;
            if !m.off_road_mps.is_finite()
                || m.off_road_mps <= 0.0
                || !m.max_speed().is_finite()
                || m.max_speed() <= 0.0
                || !(1.0 / m.off_road_mps).is_normal()
                || !ceiling.is_finite()
                || self.cells.changed.iter().any(|(&k, c)| {
                    let enters = match m.class {
                        MoverClass::Infantry => c.infantry,
                        MoverClass::Vehicle => Self::vehicle_enters(c, m.push),
                    };
                    enters && {
                        let cost = self.cost(k, m, policy, 1.0);
                        !cost.is_finite() || cost <= 0.0
                    }
                })
            {
                return false;
            }
        }
        if policy == RoutePolicy::Fastest
            && self
                .cells
                .changed
                .values()
                .any(|c| m.speed(c.road, c.forest, c.slope_deg) > m.off_road_mps)
        {
            return false;
        }
        let a = cell_center(start % self.nx, start / self.nx);
        let b = cell_center(target % self.nx, target / self.nx);
        let x0 = from.x.min(goal.x).min(a.x).min(b.x);
        let y0 = from.y.min(goal.y).min(a.y).min(b.y);
        let x1 = from.x.max(goal.x).max(a.x).max(b.x);
        let y1 = from.y.max(goal.y).max(a.y).max(b.y);
        // This halo also certifies the clearance of every monotonic grid path.
        let halo = MAX_CLEARANCE_M + NAV_CELL_M;
        if self.cells.changed.iter().any(|(&k, c)| {
            let p = cell_center(k % self.nx, k / self.nx);
            let exceptional = !c.ground
                || c.heaviest != NO_BODY
                || (m.class == MoverClass::Infantry && c.free != ALL_FREE)
                || (policy == RoutePolicy::Fastest
                    && m.speed(c.road, c.forest, c.slope_deg) != m.off_road_mps);
            exceptional
                && p.x <= x1 + halo
                && p.x >= x0 - halo
                && p.y <= y1 + halo
                && p.y >= y0 - halo
        }) {
            return false;
        }
        if self.avoid.iter().any(|b| {
            let radius = b.half.length() + m.half_width_m + NAV_CELL_M / 2.0;
            b.center.x + radius >= x0
                && b.center.x - radius <= x1
                && b.center.y + radius >= y0
                && b.center.y - radius <= y1
        }) {
            return false;
        }
        self.segment_cost(from, goal, m, policy).is_some()
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
