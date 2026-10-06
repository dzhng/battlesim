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
/// How far into a standing soldier's disc a step that leaves him may graze
/// when no way stays a full disc clear.
const SLIDE_M: f64 = 0.05;

/// A fine route and whether constructing it required an A* query.
pub struct FinalLeg {
    pub path: Option<Vec<V2>>,
    pub searched: bool,
}

/// A soldier's own route from `from` to `to`: a proven clear connector, or
/// A* on a `FINE_CELL_M` grid over
/// a square window of side `window` centred between them, where a cell is
/// open if his disc of `radius` at its centre meets none of `solids`, nor
/// the disc of a soldier standing at any of `soldiers` (his size), and
/// `walkable` allows its ground. A solid or a soldier he already overlaps is
/// ignored, as walking ignores it. The route is string-pulled on the exact
/// shapes and ends at `to`; the points follow `from`. The path is `None` when `to` lies
/// outside the window or no way inside it reaches `to`.
///
/// A soldier wedged between a wall and a man he touches has no step on the
/// grid that leads exactly away from the man. When no way keeps a full disc
/// clear of the standing soldiers, he looks again for one whose steps off a
/// man he touches may graze his disc by [`SLIDE_M`], as walking slides off him.
pub fn final_leg(
    from: V2,
    to: V2,
    solids: &[Obb2],
    soldiers: &[V2],
    radius: f64,
    window: f64,
    walkable: impl Fn(V2) -> bool,
) -> FinalLeg {
    let n = (window / FINE_CELL_M).ceil().max(2.0) as usize;
    let origin = (from + to) * 0.5 - v2(n as f64, n as f64) * (FINE_CELL_M / 2.0);
    let grid = (n, origin);
    if let Some(path) = direct_leg(from, to, solids, soldiers, radius, grid, &walkable) {
        return FinalLeg {
            path: Some(path),
            searched: false,
        };
    }
    let search = |slide| search(from, to, solids, soldiers, radius, grid, &walkable, slide);
    FinalLeg {
        path: search(0.0).or_else(|| (!soldiers.is_empty()).then(|| search(SLIDE_M)).flatten()),
        searched: true,
    }
}

/// A straight connector proven on the same exact bodies and fine ground
/// cells as the search. An uncertain ground cell keeps the A* fallback.
fn direct_leg(
    from: V2,
    to: V2,
    solids: &[Obb2],
    soldiers: &[V2],
    radius: f64,
    grid: (usize, V2),
    walkable: impl Fn(V2) -> bool,
) -> Option<Vec<V2>> {
    let (n, origin) = grid;
    let cell = |p: V2| {
        let d = (p - origin) * (1.0 / FINE_CELL_M);
        (d.x.floor(), d.y.floor())
    };
    let (fx, fy) = cell(from);
    let (tx, ty) = cell(to);
    // Raw A* returns no waypoints within one cell; preserve that result.
    if fx == tx && fy == ty {
        return None;
    }
    if fx.min(tx) < 0.0 || fy.min(ty) < 0.0 || fx.max(tx) >= n as f64 || fy.max(ty) >= n as f64 {
        return None;
    }
    let low = origin + v2(fx.min(tx) + 0.5, fy.min(ty) + 0.5) * FINE_CELL_M;
    let high = origin + v2(fx.max(tx) + 0.5, fy.max(ty) + 0.5) * FINE_CELL_M;
    let low = v2(low.x.min(from.x).min(to.x), low.y.min(from.y).min(to.y));
    let high = v2(high.x.max(from.x).max(to.x), high.y.max(from.y).max(to.y));
    let rectangle = Obb2 {
        center: (low + high) * 0.5,
        half: (high - low) * 0.5,
        yaw: 0.0,
    };
    if solids
        .iter()
        .filter(|b| !b.contains(from, radius))
        .any(|b| {
            let grown = Obb2 {
                half: b.half + v2(radius, radius),
                ..*b
            };
            grown.overlaps(&rectangle)
        })
    {
        return None;
    }
    let apart = 2.0 * radius;
    if soldiers
        .iter()
        .filter(|q| (**q - from).at_least_radius(apart) && (**q - to).at_least_radius(apart))
        .any(|q| rectangle.distance(*q) < apart)
    {
        return None;
    }
    // A conservative rectangle includes every grid step and both orthogonal
    // cells at each diagonal corner; a blocked cell falls back to searching.
    for y in fy.min(ty) as usize..=fy.max(ty) as usize {
        for x in fx.min(tx) as usize..=fx.max(tx) as usize {
            let center = origin + v2(x as f64 + 0.5, y as f64 + 0.5) * FINE_CELL_M;
            if !walkable(center) {
                return None;
            }
        }
    }
    Some(vec![to])
}

#[allow(clippy::too_many_arguments)]
fn search(
    from: V2,
    to: V2,
    solids: &[Obb2],
    soldiers: &[V2],
    radius: f64,
    grid: (usize, V2),
    walkable: &impl Fn(V2) -> bool,
    slide: f64,
) -> Option<Vec<V2>> {
    let (n, origin) = grid;
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
        .filter(|q| (**q - from).at_least_radius(apart) && (**q - to).at_least_radius(apart))
        .copied()
        .collect();
    let reach = Reach::new(&solids, radius, grid);
    // 0 unknown, 1 open, 2 closed; the ends are open whatever their centre.
    let mut state = vec![0u8; n * n];
    state[start] = 1;
    state[goal] = 1;
    let mut open_cell = |k: usize| -> bool {
        if state[k] == 0 {
            let c = center(k);
            let free = walkable(c)
                && !reach.near(c, c, &solids).any(|b| b.contains(c, radius))
                && soldiers.iter().all(|q| (*q - c).at_least_radius(apart));
            state[k] = if free { 1 } else { 2 };
        }
        state[k] == 1
    };
    let mut g = vec![f64::INFINITY; n * n];
    let mut parent = vec![u32::MAX; n * n];
    let point = |k| {
        if k == start {
            from
        } else if k == goal {
            to
        } else {
            center(k)
        }
    };
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
            let (a, b) = (point(cell), point(next));
            if !clear_segment(a, b, reach.near(a, b, &solids), &soldiers, radius, slide) {
                continue;
            }
            if diagonal {
                // Ground remains a cell field; do not cut a water/slope corner.
                let a = index(ci + di, cj).is_some_and(|k| walkable(center(k)));
                let b = index(ci, cj + dj).is_some_and(|k| walkable(center(k)));
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
    Some(pull(&points, &solids, &soldiers, radius, slide))
}

/// Which solids may reach each cell of a search's window: those whose box,
/// grown by the soldier's radius, overlaps the cell. A segment, or a disc at
/// a point, meets no solid that reaches none of the cells its bounds cover,
/// so a step is tested against those alone, with the same answer. A mask
/// holds 64 solids; with more, every step is tested against all.
struct Reach {
    masks: Option<Vec<u64>>,
    grid: (usize, V2),
}

impl Reach {
    fn new(solids: &[Obb2], radius: f64, grid: (usize, V2)) -> Self {
        let n = grid.0;
        if solids.len() > 64 {
            return Self { masks: None, grid };
        }
        let mut masks = vec![0u64; n * n];
        for (k, solid) in solids.iter().enumerate() {
            // The box grown by the radius in its own frame, square corners
            // and all (as `Obb2::contains` and `meets_segment` grow it),
            // then turned onto the grid's axes.
            let (sin, cos) = libm::sincos(solid.yaw);
            let half = (solid.half + v2(radius, radius)) * (1.0 + 1e-9);
            let extent = v2(
                cos.abs() * half.x + sin.abs() * half.y,
                sin.abs() * half.x + cos.abs() * half.y,
            ) + v2(1e-6, 1e-6);
            let [(i0, i1), (j0, j1)] =
                Self::span(grid, solid.center - extent, solid.center + extent);
            for j in j0..=j1 {
                for i in i0..=i1 {
                    masks[j * n + i] |= 1 << k;
                }
            }
        }
        Self {
            masks: Some(masks),
            grid,
        }
    }

    /// The cells, clamped to the window, that the box from `low` to `high`
    /// covers.
    fn span(grid: (usize, V2), low: V2, high: V2) -> [(usize, usize); 2] {
        let (n, origin) = grid;
        let cell =
            |v: f64, o: f64| ((v - o) / FINE_CELL_M).floor().clamp(0.0, (n - 1) as f64) as usize;
        [
            (cell(low.x, origin.x), cell(high.x, origin.x)),
            (cell(low.y, origin.y), cell(high.y, origin.y)),
        ]
    }

    /// The solids that may meet the segment `a`→`b` (or the point `a`).
    fn near<'s>(&self, a: V2, b: V2, solids: &'s [Obb2]) -> impl Iterator<Item = &'s Obb2> {
        let mask = self.masks.as_ref().map_or(u64::MAX, |masks| {
            let n = self.grid.0;
            let low = v2(a.x.min(b.x), a.y.min(b.y));
            let high = v2(a.x.max(b.x), a.y.max(b.y));
            let [(i0, i1), (j0, j1)] = Self::span(self.grid, low, high);
            let mut mask = 0;
            for j in j0..=j1 {
                for i in i0..=i1 {
                    mask |= masks[j * n + i];
                }
            }
            mask
        });
        solids
            .iter()
            .enumerate()
            .filter(move |(k, _)| *k >= 64 || mask & (1 << k) != 0)
            .map(|(_, s)| s)
    }
}

/// Body clearance belongs to the actual segment, including a route's exact
/// endpoints: their containing grid cell's centre may lie inside a body.
/// A standing soldier is passed a full disc clear, or from a point touching
/// him no more than `slide` nearer than that point.
fn clear_segment<'s>(
    a: V2,
    b: V2,
    solids: impl IntoIterator<Item = &'s Obb2>,
    soldiers: &[V2],
    radius: f64,
    slide: f64,
) -> bool {
    let apart = 2.0 * radius;
    solids
        .into_iter()
        .all(|s| !s.meets_segment(a, b, radius) || s.contains(a, radius))
        && soldiers.iter().all(|&q| {
            let ab = b - a;
            let t = ((q - a).dot(ab) / ab.dot(ab).max(1e-12)).clamp(0.0, 1.0);
            let near = a + ab * t - q;
            near.at_least_radius(apart)
                || (q - a).inside_radius(apart)
                || near.outside_radius((q - a).length() - slide)
        })
}

/// Greedy string-pulling on the exact shapes: from each kept point, on to
/// the furthest later point in plain sight, stopping at the first that is not.
fn pull(points: &[V2], solids: &[Obb2], soldiers: &[V2], radius: f64, slide: f64) -> Vec<V2> {
    let mut out = Vec::new();
    let mut at = 0;
    while at + 1 < points.len() {
        let mut next = at + 1;
        for k in at + 2..points.len() {
            if clear_segment(points[at], points[k], solids, soldiers, radius, slide) {
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
        let route = final_leg(from, to, &solids, &[], 0.3, 40.0, |_| true)
            .path
            .expect("a way through");
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
        let route = final_leg(from, to, &[face], &[man], 0.3, 12.0, |_| true)
            .path
            .expect("a way");
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
    fn a_soldier_beside_a_wall_routes_round_a_man_without_entering_the_wall() {
        let wall = Obb2 {
            center: v2(947.0, 743.0),
            yaw: 0.0,
            half: v2(0.4, 20.0),
        };
        // His disc clears the face, but his containing grid cell's centre
        // lies inside it; he can escape diagonally away from the standing man.
        let from = v2(947.7076674060205, 744.5683391358108);
        let to = v2(947.9, 740.0);
        let man = v2(947.9, 744.0);
        let route = final_leg(from, to, &[wall], &[man], 0.3, 40.0, |_| true)
            .path
            .expect("a way round the standing man");
        assert_eq!(route.last(), Some(&to));
        assert!(clear(from, &route, &[wall], 0.3), "{route:?}");
        let mut a = from;
        for &b in &route {
            let ab = b - a;
            let t = ((man - a).dot(ab) / ab.dot(ab)).clamp(0.0, 1.0);
            assert!((a + ab * t - man).length() >= 0.6 - 1e-9, "{route:?}");
            a = b;
        }
    }

    #[test]
    fn a_soldier_wedged_between_a_wall_and_a_man_he_touches_slides_off_him() {
        // A squad lines up along a wall's south face. One soldier walking
        // west along it has stopped against a squadmate already in place:
        // he touches the face and the man, his spot lies beyond the man, and
        // no grid step from where he stands leads exactly away from him.
        let wall = Obb2 {
            center: v2(60.0, 40.0),
            yaw: 0.0,
            half: v2(12.0, 9.0),
        };
        let man = v2(62.06, 30.19);
        let from = man + v2(0.36, 0.48) * ((0.6 + 1e-6) / 0.6);
        let to = v2(58.51, 30.64);
        let route = final_leg(from, to, &[wall], &[man], 0.3, 40.0, |_| true)
            .path
            .expect("a way round the man in place");
        assert_eq!(route.last(), Some(&to));
        assert!(clear(from, &route, &[wall], 0.3), "{route:?}");
        // Off the man he touches by no more than the slide, then a disc clear.
        let mut a = from;
        for (k, &b) in route.iter().enumerate() {
            let ab = b - a;
            let t = ((man - a).dot(ab) / ab.dot(ab)).clamp(0.0, 1.0);
            let room = if k == 0 { 0.6 - SLIDE_M } else { 0.6 - 1e-9 };
            assert!((a + ab * t - man).length() >= room, "{route:?}");
            a = b;
        }
    }

    #[test]
    fn no_route_where_the_gap_is_narrower_than_a_man() {
        let solids = [wall(0.0, -20.0, 9.75), wall(0.0, 10.25, 20.0)];
        assert_eq!(
            final_leg(v2(-8.0, 0.0), v2(8.0, 0.0), &solids, &[], 0.3, 40.0, |_| {
                true
            })
            .path,
            None
        );
    }
}

#[cfg(test)]
mod reach_tests {
    use super::*;

    /// Every body a point's disc or a short step can meet stays among the
    /// bodies its cells offer, whichever way the body is turned.
    #[test]
    fn the_cells_offer_every_body_a_disc_or_step_can_meet() {
        let radius = 0.4;
        let grid = (40, v2(0.0, 0.0));
        for (turn, shift) in (0..24).flat_map(|t| (0..10).map(move |k| (t, k))) {
            let solid = Obb2 {
                center: v2(10.0, 10.0) + v2(0.05, 0.05) * shift as f64,
                half: v2(2.1, 0.9),
                yaw: turn as f64 * std::f64::consts::PI / 12.0,
            };
            let solids = [solid];
            let reach = Reach::new(&solids, radius, grid);
            for j in 0..160 {
                for i in 0..160 {
                    let p = v2(i as f64 * 0.125, j as f64 * 0.125);
                    if solid.contains(p, radius) {
                        assert_eq!(reach.near(p, p, &solids).count(), 1, "turn {turn} at {p:?}");
                    }
                    let q = p + v2(0.3, 0.2);
                    if solid.meets_segment(p, q, radius) {
                        assert_eq!(reach.near(p, q, &solids).count(), 1, "turn {turn} {p:?}");
                    }
                }
            }
        }
    }
}
