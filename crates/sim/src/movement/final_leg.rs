//! A soldier's own route over the exact bodies: the fine half of infantry's
//! two resolutions (Q27). The squad's corridor comes from the 2 m grid; a
//! soldier who needs to find his own way (the final stretch to his spot, or
//! back to the corridor) plans on a 0.5 m grid over a small window around
//! him, with his disc against each body's box, and string-pulls the result on
//! the boxes themselves. That is how he walks round a crate stack and
//! squeezes through a gap one man wide.
use std::cmp::Ordering;
use std::collections::BinaryHeap;

use crate::math::{v2, Obb2, V2};

/// The fine grid's cell side.
pub const FINE_CELL_M: f64 = 0.5;

/// A soldier's own route from `from` to `to`: A* on a `FINE_CELL_M` grid over
/// a square window of side `window` centred between them, where a cell is
/// open if his disc of `radius` at its centre meets none of `solids`, nor
/// the disc of a soldier standing at any of `soldiers` (his size), and
/// `walkable` allows its ground. A solid or a soldier he already overlaps is
/// ignored, as walking ignores it. The route is string-pulled on the exact
/// shapes and ends at `to`; the points follow `from`. `None` when `to` lies
/// outside the window or no way inside it reaches `to`.
pub fn final_leg(
    from: V2,
    to: V2,
    solids: &[Obb2],
    soldiers: &[V2],
    radius: f64,
    window: f64,
    walkable: impl Fn(V2) -> bool,
) -> Option<Vec<V2>> {
    let n = (window / FINE_CELL_M).ceil().max(2.0) as usize;
    let origin = (from + to) * 0.5 - v2(n as f64, n as f64) * (FINE_CELL_M / 2.0);
    let cell_of = |p: V2| -> Option<usize> {
        let d = (p - origin) * (1.0 / FINE_CELL_M);
        let (i, j) = (d.x.floor(), d.y.floor());
        (i >= 0.0 && j >= 0.0 && (i as usize) < n && (j as usize) < n)
            .then(|| j as usize * n + i as usize)
    };
    let center = |k: usize| origin + v2((k % n) as f64 + 0.5, (k / n) as f64 + 0.5) * FINE_CELL_M;
    let start = cell_of(from)?;
    let goal = cell_of(to)?;
    let solids: Vec<Obb2> = solids
        .iter()
        .filter(|b| !b.contains(from, radius))
        .copied()
        .collect();
    let apart = 2.0 * radius;
    let soldiers: Vec<V2> = soldiers
        .iter()
        .filter(|q| (**q - from).length() >= apart && (**q - to).length() >= apart)
        .copied()
        .collect();
    // 0 unknown, 1 open, 2 closed; the ends are open whatever their centre.
    let mut state = vec![0u8; n * n];
    state[start] = 1;
    state[goal] = 1;
    let mut open_cell = |k: usize| -> bool {
        if state[k] == 0 {
            let c = center(k);
            let free = walkable(c)
                && !solids.iter().any(|b| b.contains(c, radius))
                && soldiers.iter().all(|q| (*q - c).length() >= apart);
            state[k] = if free { 1 } else { 2 };
        }
        state[k] == 1
    };
    let mut g = vec![f64::INFINITY; n * n];
    let mut parent = vec![u32::MAX; n * n];
    let goal_c = center(goal);
    let h = |k: usize| {
        let d = center(k) - goal_c;
        let (a, b) = (d.x.abs(), d.y.abs());
        a.max(b) + (std::f64::consts::SQRT_2 - 1.0) * a.min(b)
    };
    let mut heap = BinaryHeap::new();
    g[start] = 0.0;
    parent[start] = start as u32;
    heap.push(Open {
        f: h(start),
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
    let index = |i: isize, j: isize| {
        (i >= 0 && j >= 0 && (i as usize) < n && (j as usize) < n)
            .then(|| j as usize * n + i as usize)
    };
    let mut found = false;
    while let Some(Open { f, cell }) = heap.pop() {
        let cell = cell as usize;
        if cell == goal {
            found = true;
            break;
        }
        if f > g[cell] + h(cell) + 1e-9 {
            continue;
        }
        let (ci, cj) = ((cell % n) as isize, (cell / n) as isize);
        for (di, dj) in STEPS {
            let Some(next) = index(ci + di, cj + dj) else {
                continue;
            };
            if !open_cell(next) {
                continue;
            }
            let diagonal = di != 0 && dj != 0;
            if diagonal {
                // No corner cutting.
                let a = index(ci + di, cj).is_some_and(&mut open_cell);
                let b = index(ci, cj + dj).is_some_and(&mut open_cell);
                if !(a && b) {
                    continue;
                }
            }
            let step = if diagonal {
                FINE_CELL_M * std::f64::consts::SQRT_2
            } else {
                FINE_CELL_M
            };
            let tentative = g[cell] + step;
            if tentative < g[next] {
                g[next] = tentative;
                parent[next] = cell as u32;
                heap.push(Open {
                    f: tentative + h(next),
                    cell: next as u32,
                });
            }
        }
    }
    if !found {
        return None;
    }
    let mut cells = vec![goal];
    while *cells.last().unwrap() != start {
        cells.push(parent[*cells.last().unwrap()] as usize);
    }
    cells.reverse();
    let mut points: Vec<V2> = cells.into_iter().map(center).collect();
    points[0] = from;
    *points.last_mut().unwrap() = to;
    Some(pull(&points, &solids, &soldiers, radius))
}

/// Greedy string-pulling on the exact shapes: from each kept point, on to
/// the furthest later point in plain sight, stopping at the first that is not.
fn pull(points: &[V2], solids: &[Obb2], soldiers: &[V2], radius: f64) -> Vec<V2> {
    let apart = 2.0 * radius;
    let clear = |a: V2, b: V2| {
        solids
            .iter()
            .all(|s| !s.meets_segment(a, b, radius) || s.contains(a, radius))
            && soldiers.iter().all(|&q| {
                let ab = b - a;
                let t = ((q - a).dot(ab) / ab.dot(ab).max(1e-12)).clamp(0.0, 1.0);
                (a + ab * t - q).length() >= apart || (q - a).length() < apart
            })
    };
    let mut out = Vec::new();
    let mut at = 0;
    while at + 1 < points.len() {
        let mut next = at + 1;
        for k in at + 2..points.len() {
            if clear(points[at], points[k]) {
                next = k;
            } else {
                break;
            }
        }
        out.push(points[next]);
        at = next;
    }
    out
}

#[derive(PartialEq)]
struct Open {
    f: f64,
    cell: u32,
}
impl Eq for Open {}
impl Ord for Open {
    fn cmp(&self, o: &Self) -> Ordering {
        o.f.total_cmp(&self.f).then_with(|| o.cell.cmp(&self.cell))
    }
}
impl PartialOrd for Open {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wall(x: f64, y0: f64, y1: f64) -> Obb2 {
        Obb2 {
            center: v2(x, (y0 + y1) / 2.0),
            yaw: 0.0,
            half: v2(0.5, (y1 - y0) / 2.0),
        }
    }

    /// Whether the polyline from `from` through `route` keeps a disc of
    /// `radius` clear of every box.
    fn clear(from: V2, route: &[V2], solids: &[Obb2], radius: f64) -> bool {
        let mut a = from;
        route.iter().all(|&b| {
            let ok = solids.iter().all(|s| !s.meets_segment(a, b, radius - 1e-6));
            a = b;
            ok
        })
    }

    #[test]
    fn a_soldier_threads_a_one_man_gap_and_never_crosses_a_box() {
        // A wall across the window with a 1.2 m gap at y 10: room for a
        // 0.3 m disc on the 0.5 m grid wherever it falls, off his straight line.
        let solids = [wall(0.0, -20.0, 9.4), wall(0.0, 10.6, 20.0)];
        let (from, to) = (v2(-8.0, 0.0), v2(8.0, 0.0));
        let route = final_leg(from, to, &solids, &[], 0.3, 40.0, |_| true).expect("a way through");
        assert_eq!(route.last(), Some(&to));
        assert!(clear(from, &route, &solids, 0.3));
        assert!(
            route.iter().any(|p| (p.y - 10.0).abs() < 0.5),
            "through the gap"
        );
    }

    #[test]
    fn a_soldier_goes_round_a_man_standing_in_a_gap_by_a_wall() {
        // A wall's end at the origin, running east; a man stands 0.5 m off
        // its south face at the end, leaving a 0.2 m slot. The way east
        // along the face goes round him, never through the slot.
        let face = Obb2 {
            center: v2(10.0, 1.0),
            yaw: 0.0,
            half: v2(10.0, 1.0),
        };
        let man = v2(0.0, -0.5);
        let (from, to) = (v2(-0.5, 0.3), v2(2.0, -0.5));
        let route = final_leg(from, to, &[face], &[man], 0.3, 12.0, |_| true).expect("a way");
        let mut a = from;
        for &b in &route {
            let ab = b - a;
            let t = ((man - a).dot(ab) / ab.dot(ab)).clamp(0.0, 1.0);
            assert!((a + ab * t - man).length() >= 0.6 - 1e-9, "{route:?}");
            a = b;
        }
        assert!(clear(from, &route, &[face], 0.3), "{route:?}");
    }

    #[test]
    fn no_route_where_the_gap_is_narrower_than_a_man() {
        let solids = [wall(0.0, -20.0, 9.75), wall(0.0, 10.25, 20.0)];
        assert_eq!(
            final_leg(v2(-8.0, 0.0), v2(8.0, 0.0), &solids, &[], 0.3, 40.0, |_| {
                true
            }),
            None
        );
    }
}
