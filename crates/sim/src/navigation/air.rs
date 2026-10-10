//! Where aircraft may fly (D21): a plain grid over the map whose only walls
//! are bodies standing taller than the rules' obstacle height. Ground,
//! water, slope, roads and pushing mean nothing up here, so one grid serves
//! every aircraft of a side, built from the bodies that side believes in.

use crate::math::{v2, V2};
use crate::world::{Prop, WorldGeometry};
use contract::scenario::Rules;
use std::collections::BinaryHeap;

/// Grid pitch, metres: fine enough to thread between towers a street apart.
const CELL_M: f64 = 4.0;

#[derive(Clone, Debug)]
pub struct AirGrid {
    cols: usize,
    rows: usize,
    blocked: Vec<bool>,
}

/// Whether `prop` is a wall to aircraft: its top stands more than the
/// obstacle height above the ground beneath it.
pub fn walls_aircraft(world: &WorldGeometry, rules: &Rules, prop: &Prop) -> bool {
    let ground = world
        .height_at(prop.center.x, prop.center.y)
        .unwrap_or(prop.base_z);
    prop.top_z() - ground > rules.air.obstacle_m
}

impl AirGrid {
    /// The grid over `world` with `walls` closed, each widened by the largest
    /// airframe any battle may field (`hull_limits.air`).
    pub fn build<'a>(
        world: &WorldGeometry,
        rules: &Rules,
        walls: impl Iterator<Item = &'a Prop>,
    ) -> Self {
        let cols = (world.width() / CELL_M).ceil().max(1.0) as usize;
        let rows = (world.depth() / CELL_M).ceil().max(1.0) as usize;
        let mut grid = AirGrid {
            cols,
            rows,
            blocked: vec![false; cols * rows],
        };
        let margin = rules.hull_limits.air.half_length_m;
        for prop in walls {
            let footprint = prop.footprint();
            let reach = prop.footprint_radius() + margin;
            let (lo, hi) = (
                grid.cell(footprint.center - v2(reach, reach)),
                grid.cell(footprint.center + v2(reach, reach)),
            );
            for row in lo.1..=hi.1 {
                for col in lo.0..=hi.0 {
                    if footprint.distance(grid.centre((col, row))) <= margin {
                        grid.blocked[row * cols + col] = true;
                    }
                }
            }
        }
        grid
    }

    fn cell(&self, p: V2) -> (usize, usize) {
        let clamp = |v: f64, n: usize| ((v / CELL_M).floor().max(0.0) as usize).min(n - 1);
        (clamp(p.x, self.cols), clamp(p.y, self.rows))
    }

    fn centre(&self, (col, row): (usize, usize)) -> V2 {
        v2((col as f64 + 0.5) * CELL_M, (row as f64 + 0.5) * CELL_M)
    }

    fn open(&self, (col, row): (usize, usize)) -> bool {
        !self.blocked[row * self.cols + col]
    }

    /// Whether the straight flight from `a` to `b` crosses no wall.
    pub fn clear(&self, a: V2, b: V2) -> bool {
        let steps = ((b - a).length() / (CELL_M * 0.5)).ceil().max(1.0) as usize;
        (0..=steps).all(|k| self.open(self.cell(a + (b - a) * (k as f64 / steps as f64))))
    }

    /// Waypoints from `from` to `to`, ending at `to`, or at the open point
    /// nearest it when `to` stands inside a wall. A flight starting inside a
    /// wall leaves it first. `searched` counts the cells the search expanded.
    pub fn route(&self, from: V2, to: V2, searched: &mut u64) -> Vec<V2> {
        let goal_cell = self.nearest_open(self.cell(to));
        let goal = if self.open(self.cell(to)) {
            to
        } else {
            self.centre(goal_cell)
        };
        if self.clear(from, goal) {
            return vec![goal];
        }
        let start = self.nearest_open(self.cell(from));
        let cells = self.search(start, goal_cell, searched);
        // Pull the string: keep only the corners the straight flight needs.
        let mut points: Vec<V2> = cells.into_iter().map(|c| self.centre(c)).collect();
        if let Some(last) = points.last_mut() {
            *last = goal;
        }
        let mut route = Vec::new();
        let mut here = from;
        let mut k = 0;
        while k < points.len() {
            let mut far = k;
            while far + 1 < points.len() && self.clear(here, points[far + 1]) {
                far += 1;
            }
            route.push(points[far]);
            here = points[far];
            k = far + 1;
        }
        route
    }

    /// The open cell nearest `cell`, by rings.
    fn nearest_open(&self, cell: (usize, usize)) -> (usize, usize) {
        if self.open(cell) {
            return cell;
        }
        for ring in 1..self.cols.max(self.rows) as i64 {
            let mut best: Option<((usize, usize), i64)> = None;
            for dy in -ring..=ring {
                for dx in -ring..=ring {
                    if dx.abs().max(dy.abs()) != ring {
                        continue;
                    }
                    let (c, r) = (cell.0 as i64 + dx, cell.1 as i64 + dy);
                    if c < 0 || r < 0 || c >= self.cols as i64 || r >= self.rows as i64 {
                        continue;
                    }
                    let at = (c as usize, r as usize);
                    let d = dx * dx + dy * dy;
                    if self.open(at) && best.is_none_or(|(_, b)| d < b) {
                        best = Some((at, d));
                    }
                }
            }
            if let Some((at, _)) = best {
                return at;
            }
        }
        cell
    }

    /// A* over the eight neighbours, octile distances.
    fn search(
        &self,
        start: (usize, usize),
        goal: (usize, usize),
        searched: &mut u64,
    ) -> Vec<(usize, usize)> {
        let index = |(c, r): (usize, usize)| r * self.cols + c;
        let octile = |a: (usize, usize), b: (usize, usize)| {
            let (dx, dy) = (a.0.abs_diff(b.0) as f64, a.1.abs_diff(b.1) as f64);
            dx.max(dy) + (std::f64::consts::SQRT_2 - 1.0) * dx.min(dy)
        };
        let mut cost = vec![f64::INFINITY; self.blocked.len()];
        let mut came = vec![usize::MAX; self.blocked.len()];
        let mut open = BinaryHeap::new();
        cost[index(start)] = 0.0;
        open.push(Node {
            estimate: octile(start, goal),
            cell: start,
        });
        while let Some(Node { cell, estimate }) = open.pop() {
            if cell == goal {
                break;
            }
            let here = cost[index(cell)];
            if estimate > here + octile(cell, goal) + 1e-9 {
                continue;
            }
            *searched += 1;
            for (dx, dy) in [
                (1, 0),
                (-1, 0),
                (0, 1),
                (0, -1),
                (1, 1),
                (1, -1),
                (-1, 1),
                (-1, -1),
            ] {
                let (c, r) = (cell.0 as i64 + dx, cell.1 as i64 + dy);
                if c < 0 || r < 0 || c >= self.cols as i64 || r >= self.rows as i64 {
                    continue;
                }
                let next = (c as usize, r as usize);
                // No corner cutting past a wall.
                if !self.open(next) || !self.open((next.0, cell.1)) || !self.open((cell.0, next.1))
                {
                    continue;
                }
                let step = if dx != 0 && dy != 0 {
                    std::f64::consts::SQRT_2
                } else {
                    1.0
                };
                let through = here + step;
                if through < cost[index(next)] {
                    cost[index(next)] = through;
                    came[index(next)] = index(cell);
                    open.push(Node {
                        estimate: through + octile(next, goal),
                        cell: next,
                    });
                }
            }
        }
        if !cost[index(goal)].is_finite() {
            return vec![start];
        }
        let mut path = vec![goal];
        let mut at = index(goal);
        while came[at] != usize::MAX {
            at = came[at];
            path.push((at % self.cols, at / self.cols));
        }
        path.reverse();
        path
    }
}

#[derive(PartialEq)]
struct Node {
    estimate: f64,
    cell: (usize, usize),
}

impl Eq for Node {}

impl Ord for Node {
    /// The lowest estimate first; ties by cell, so the search is deterministic.
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other
            .estimate
            .total_cmp(&self.estimate)
            .then_with(|| other.cell.cmp(&self.cell))
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
