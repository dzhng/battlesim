//! Infantry's coarse parent path reconstructed through its connected sub-cell
//! gaps. Each step runs at most two cell-local searches, never a whole-world
//! fine search. Endpoint fallback keeps the existing bounded reach.
use super::{cell_center, sub_center, BlockReason, Mover, NavGrid, NAV_CELL_M, SUB, SUB_M};
use crate::math::V2;
use contract::command::RoutePolicy;

pub(super) struct FootTrace {
    cells: Vec<usize>,
    at: usize,
    points: Vec<V2>,
    goal: V2,
}

impl FootTrace {
    pub fn new(cells: Vec<usize>, from: V2, goal: V2) -> Self {
        Self {
            cells,
            at: 0,
            points: vec![from],
            goal,
        }
    }

    pub fn progress(&self) -> [usize; 2] {
        [self.at, self.points.len()]
    }

    pub fn step(&mut self, grid: &NavGrid, who: Mover) -> Option<Result<Vec<V2>, BlockReason>> {
        let a = self.cells[self.at];
        if self.at + 1 == self.cells.len() {
            if !join(
                grid,
                a,
                *self.points.last().unwrap(),
                self.goal,
                who,
                &mut self.points,
            ) {
                return Some(Err(BlockReason::NoRoute));
            }
            return Some(Ok(std::mem::take(&mut self.points)));
        }
        let b = self.cells[self.at + 1];
        if let Some(exit) = grid.crossing(a, b) {
            if !join(
                grid,
                a,
                *self.points.last().unwrap(),
                exit,
                who,
                &mut self.points,
            ) {
                return Some(Err(BlockReason::NoRoute));
            }
        } else {
            let next = grid.waypoint(b, who.m);
            if grid
                .segment_cost(
                    *self.points.last().unwrap(),
                    next,
                    who,
                    RoutePolicy::Shortest,
                )
                .is_some()
            {
                self.points.push(next);
                self.at += 1;
                return None;
            }
            // A diagonal was admitted only when both orthogonal alternatives
            // fit. Reconstruct one of them through actual shared-edge gaps.
            let via = (a / grid.nx) * grid.nx + b % grid.nx;
            // Knowledge may close an edge after the parent path was found.
            // A failed candidate is replanned by the planner's revision check.
            let (Some(first), Some(second)) = (grid.crossing(a, via), grid.crossing(via, b)) else {
                return Some(Err(BlockReason::NoRoute));
            };
            if !join(
                grid,
                a,
                *self.points.last().unwrap(),
                first,
                who,
                &mut self.points,
            ) || !join(
                grid,
                via,
                *self.points.last().unwrap(),
                second,
                who,
                &mut self.points,
            ) {
                return Some(Err(BlockReason::NoRoute));
            }
        }
        self.at += 1;
        None
    }
}

/// A connected 4×4 free mask needs at most sixteen queue entries. The shared
/// sampled segment reader remains the authority: these points cannot grant a
/// blocked straight segment a finite time.
fn join(grid: &NavGrid, cell: usize, from: V2, to: V2, who: Mover, out: &mut Vec<V2>) -> bool {
    grid.spend(1);
    if grid
        .segment_cost(from, to, who, RoutePolicy::Shortest)
        .is_some()
    {
        if out.last() != Some(&to) {
            out.push(to);
        }
        return true;
    }
    let (i, j) = (cell % grid.nx, cell / grid.nx);
    let origin = cell_center(i, j) - crate::math::v2(NAV_CELL_M / 2.0, NAV_CELL_M / 2.0);
    let bit = |p: V2| {
        let d = p - origin;
        if d.x < 0.0 || d.y < 0.0 || d.x > NAV_CELL_M || d.y > NAV_CELL_M {
            return None;
        }
        let x = ((d.x / SUB_M) as usize).min(SUB - 1);
        let y = ((d.y / SUB_M) as usize).min(SUB - 1);
        Some(y * SUB + x)
    };
    let (Some(start), Some(target)) = (bit(from), bit(to)) else {
        return false;
    };
    let free = grid.cells[cell].free;
    if free & (1 << start) == 0 || free & (1 << target) == 0 {
        return false;
    }
    let mut queue = [0; SUB * SUB];
    let mut parents = [usize::MAX; SUB * SUB];
    parents[start] = start;
    queue[0] = start;
    let (mut head, mut tail) = (0, 1);
    while head < tail && parents[target] == usize::MAX {
        let at = queue[head];
        head += 1;
        grid.spend(1);
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let x = (at % SUB) as isize + dx;
            let y = (at / SUB) as isize + dy;
            if x < 0 || y < 0 || x >= SUB as isize || y >= SUB as isize {
                continue;
            }
            let next = y as usize * SUB + x as usize;
            if free & (1 << next) != 0 && parents[next] == usize::MAX {
                parents[next] = at;
                queue[tail] = next;
                tail += 1;
            }
        }
    }
    if parents[target] == usize::MAX {
        return false;
    }
    let mut path = [0; SUB * SUB];
    let mut count = 0;
    let mut next = target;
    loop {
        path[count] = next;
        count += 1;
        if next == start {
            break;
        }
        next = parents[next];
    }
    let mut a = from;
    for p in path[..count]
        .iter()
        .rev()
        .map(|&bit| sub_center(i, j, bit))
        .chain([to])
    {
        if grid
            .segment_cost(a, p, who, RoutePolicy::Shortest)
            .is_none()
        {
            return false;
        }
        out.push(p);
        a = p;
    }
    true
}
