//! Connected bare ground, stored as horizontal runs rather than a world-sized
//! label grid. It deliberately ignores bodies and footprints: different labels
//! prove no route; equal labels promise nothing about a mover's passage.
use super::cells::Cells;

#[derive(Clone)]
struct Run {
    start: usize,
    end: usize,
    component: usize,
}

pub(super) struct Terrain {
    rows: Vec<Vec<Run>>,
    bands: Vec<(usize, usize)>,
}

pub(super) struct Probe {
    a: crate::math::V2,
    b: crate::math::V2,
    at: Option<usize>,
}

impl Probe {
    pub fn new(a: crate::math::V2, b: crate::math::V2) -> Self {
        Self { a, b, at: None }
    }
    pub fn digest(&self, d: &mut crate::digest::Digest) {
        d.u64(self.at.map_or(0, |at| at as u64 + 1));
    }
}

impl Terrain {
    pub fn new(cells: &Cells) -> Self {
        let open_default = cells.implicit[0].ground;
        let mut marked = vec![Vec::new(); cells.ny];
        // Only stored pages differ from the implicit terrain. Mark closed
        // cells on an open map, open cells on a closed one.
        for (at, cell) in cells.stored() {
            if (cell.ground || cell.free != 0) != open_default {
                marked[at / cells.nx].push(at % cells.nx);
            }
        }
        let mut parents = Vec::new();
        let mut rows: Vec<Vec<Run>> = Vec::with_capacity(cells.ny);
        for mut marked in marked {
            marked.sort_unstable();
            marked.dedup();
            let mut spans = Vec::new();
            if open_default {
                let mut start = 0;
                for x in marked.into_iter().chain([cells.nx]) {
                    if start < x {
                        spans.push((start, x - 1));
                    }
                    start = x + 1;
                }
            } else {
                for x in marked {
                    match spans.last_mut() {
                        Some((_, end)) if x == *end + 1 => *end = x,
                        _ => spans.push((x, x)),
                    }
                }
            }
            let runs: Vec<_> = spans
                .into_iter()
                .map(|(start, end)| {
                    let component = parents.len();
                    parents.push(component);
                    Run {
                        start,
                        end,
                        component,
                    }
                })
                .collect();
            if let Some(previous) = rows.last() {
                let mut first = 0;
                for run in &runs {
                    while first < previous.len() && previous[first].end + 1 < run.start {
                        first += 1;
                    }
                    for other in &previous[first..] {
                        if other.start > run.end + 1 {
                            break;
                        }
                        let a = root(&mut parents, run.component);
                        let b = root(&mut parents, other.component);
                        parents[a.max(b)] = a.min(b);
                    }
                }
            }
            rows.push(runs);
        }
        for run in rows.iter_mut().flatten() {
            run.component = root(&mut parents, run.component);
        }
        let mut bands = Vec::new();
        let mut start = 0;
        for y in 1..=rows.len() {
            if y == rows.len()
                || rows[y].len() != rows[start].len()
                || rows[y]
                    .iter()
                    .zip(&rows[start])
                    .any(|(a, b)| a.start != b.start || a.end != b.end)
            {
                bands.push((start, y - 1));
                start = y;
            }
        }
        Self { rows, bands }
    }

    /// Intersect the segment with each crossed row, then ask whether its
    /// entire x interval lies in one open run. Identical adjacent row profiles
    /// form one band, so open terrain takes one lookup at every bearing.
    pub fn step(&self, probe: &mut Probe, nx: usize) -> Option<bool> {
        let (a, b) = (probe.a, probe.b);
        let cell = super::NAV_CELL_M;
        if a.x.min(b.x) < 0.0
            || a.y.min(b.y) < 0.0
            || a.x.max(b.x) >= nx as f64 * cell
            || a.y.max(b.y) >= self.rows.len() as f64 * cell
        {
            return Some(false);
        }
        let first = (a.y.min(b.y) / cell) as usize;
        let last = (a.y.max(b.y) / cell) as usize;
        let delta = b - a;
        let at = *probe
            .at
            .get_or_insert_with(|| self.bands.partition_point(|(_, end)| *end < first));
        let mut reads = 0;
        for &(start, end) in self.bands[at..].iter().take(128) {
            if start > last {
                break;
            }
            reads += 1;
            let low_y = start.max(first);
            let high_y = end.min(last);
            let (lo, hi) = if delta.y == 0.0 {
                (a.x.min(b.x), a.x.max(b.x))
            } else {
                let t0 = ((low_y as f64 * cell - a.y) / delta.y).clamp(0.0, 1.0);
                let t1 = (((high_y + 1) as f64 * cell - a.y) / delta.y).clamp(0.0, 1.0);
                let x0 = a.x + delta.x * t0;
                let x1 = a.x + delta.x * t1;
                (x0.min(x1), x0.max(x1))
            };
            let (lo, hi) = ((lo / cell) as usize, (hi / cell) as usize);
            let row = &self.rows[start];
            let at = row.partition_point(|r| r.end < lo);
            if row.get(at).is_none_or(|r| r.start > lo || r.end < hi) {
                return Some(false);
            }
        }
        probe.at = Some(at + reads);
        if self
            .bands
            .get(at + reads)
            .is_none_or(|(start, _)| *start > last)
        {
            Some(true)
        } else {
            None
        }
    }

    pub fn connected(&self, a: usize, b: usize, nx: usize) -> bool {
        let component = |at: usize| {
            let (x, y) = (at % nx, at / nx);
            let runs = &self.rows[y];
            let k = runs.partition_point(|r| r.end < x);
            runs.get(k).filter(|r| r.start <= x).map(|r| r.component)
        };
        match (component(a), component(b)) {
            (Some(a), Some(b)) => a == b,
            _ => false,
        }
    }
}

fn root(parents: &mut [usize], mut at: usize) -> usize {
    while parents[at] != at {
        parents[at] = parents[parents[at]];
        at = parents[at];
    }
    at
}
