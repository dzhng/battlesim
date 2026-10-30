//! The surface field: every ground rule's primitives (paved, forest, water)
//! once in `records`, eight floats each, and in `index` a ladder of bucket
//! grids over them, so a lookup costs what lies near the point instead of
//! what the map holds. A cell lists the records whose shape, grown by its
//! level's reach, touches it: paved, then forest, then water. Each level
//! serves pixels twice as wide as the one below, with cells that grow with
//! them; the ladder ends at one cell that lists everything, or on a dense map
//! at the last level within the list budget, which then reads no farther than
//! its reach. This module owns the field layout; the shared terrain shader
//! consumes its records and index.
use super::forests::{Forest, Kind};
use super::shapes::{closest, convex_overlap, distance, stroke_inside, triangle_contains};
use super::water::stretch_inside;
use super::{chunks, Source, AREA, FOREST_BOUNDARY, FOREST_STROKE, FOREST_TRIANGLE, RIVER};
use super::{SURFACE_BOUNDARY, SURFACE_STROKE, SURFACE_TRIANGLE};
use std::collections::HashMap;
use std::f64::consts::{PI, SQRT_2};

pub const FLOATS: usize = 8;
// A record's kind, as the index names it, and the rule it serves.
const RECT: u32 = 0;
const STROKE: u32 = 1;
const TRIANGLE: u32 = 2;
const EDGE: u32 = 3;
const PAVED: usize = 0;
const FOREST: usize = 1;
const WATER: usize = 2;
pub const NEW_SHAPE: u32 = 1 << 29;
pub const RECORD_MASK: u32 = NEW_SHAPE - 1;
/// Bits below the primitive kind in a field index entry.
pub const KIND_SHIFT: u32 = 30;
const CELL_WORDS: usize = 3;
/// The finest cells' side and widest pixel, in metres: the play camera looking
/// down sees at most about 2.5 m a pixel, so only ground seen edge-on climbs.
const CELL_M: f64 = 8.0;
pub const FOOTPRINT_M: f64 = 2.0;
/// A level's cells are at least this many of its footprints a side.
const CELL_FOOTPRINTS: f64 = 4.0;
/// The finest level never holds more cells: a larger map doubles its cell.
const MAX_CELLS: f64 = (1 << 18) as f64;
/// The mean records a cell may list, sqrt(2) times more a level up: rows of
/// pixels `F` metres wide thin as 1 / sqrt(F) in a ground view.
const LIST_BUDGET: f64 = 24.0;
/// Cells grow by this share of the largest coordinate, so a point the GPU's
/// f32 arithmetic puts a hair into the next cell still finds its records.
const CELL_PAD: f64 = 1.0 / (1 << 18) as f64;

pub struct Field {
    pub records: Vec<f32>,
    pub index: Vec<u32>,
    /// The grid's low corner `x, y` (exact in f32), the finest cell's side,
    /// columns, rows, the levels, and the widest pixel read exactly.
    pub grid: [f64; 7],
}

/// Whether the closed segment `a`-`b` touches the closed box `low`-`high`,
/// corners included (a diagonal street on a square grid).
fn segment_touches_box(a: [f64; 2], b: [f64; 2], low: [f64; 2], high: [f64; 2]) -> bool {
    let (mut enter, mut leave) = (0.0f64, 1.0f64);
    for axis in 0..2 {
        let run = b[axis] - a[axis];
        if run == 0.0 {
            if a[axis] < low[axis] || a[axis] > high[axis] {
                return false;
            }
            continue;
        }
        let (u, v) = ((low[axis] - a[axis]) / run, (high[axis] - a[axis]) / run);
        enter = enter.max(u.min(v));
        leave = leave.min(u.max(v));
    }
    enter <= leave
}

/// The field of the ground's rules with paved `strokes`. `reach[level]` is
/// how far each rule is read at that level's pixel (`FOOTPRINT_M · 2^level`
/// wide), for every level the ladder may build; it must not shrink.
pub fn field(
    s: &Source,
    forests: &[Forest],
    strokes: &[f32],
    reach: &[[f64; 3]],
) -> Result<Field, String> {
    let mut records: Vec<f32> = vec![];
    // Per record: kind, rule, forest shape, whether cells list it.
    let mut meta: Vec<(u32, usize, Option<usize>, bool)> = vec![];
    let [mut min_x, mut min_y, mut max_x, mut max_y] = s.map;
    // A record of `source`, its first `points` pairs positions, `tail` last.
    let mut put =
        |source: &[f32], tail: f32, points: usize, m: (u32, usize, Option<usize>, bool)| {
            let at = records.len();
            records.extend_from_slice(source);
            records.resize(at + FLOATS, 0.0);
            if source.len() < FLOATS {
                records[at + FLOATS - 1] = tail;
            }
            for p in chunks::<2>(&records[at..at + points * 2]) {
                (min_x, max_x) = (min_x.min(p[0] as f64), max_x.max(p[0] as f64));
                (min_y, max_y) = (min_y.min(p[1] as f64), max_y.max(p[1] as f64));
            }
            meta.push(m);
        };
    // A paved stretch's tail is how far along its stroke it starts; a
    // stroke's stretches are exported in order, each from the last's end.
    let (mut along, mut last) = (0.0, None::<&[f32; SURFACE_STROKE]>);
    for r in chunks::<SURFACE_STROKE>(strokes) {
        if !last.is_some_and(|l| r[0] == l[2] && r[1] == l[3]) {
            along = 0.0;
        }
        put(r, along as f32, 2, (STROKE, PAVED, None, true));
        along += libm::hypot(r[2] as f64 - r[0] as f64, r[3] as f64 - r[1] as f64);
        last = Some(r);
    }
    // A paved triangle's tail is its area's slab bearing. Triangles only sign
    // a point's side of the exposed boundary: with none, cells skip them.
    let bounded = !s.surface_boundaries.is_empty();
    let bearings = area_bearings(&s.surface_triangles);
    for (r, b) in chunks::<SURFACE_TRIANGLE>(&s.surface_triangles)
        .iter()
        .zip(bearings)
    {
        put(r, b, 3, (TRIANGLE, PAVED, None, bounded));
    }
    for r in chunks::<SURFACE_BOUNDARY>(&s.surface_boundaries) {
        put(r, 0.0, 2, (EDGE, PAVED, None, true));
    }
    for (shape, f) in forests
        .iter()
        .filter(|f| f.kind != Kind::Rectangle)
        .enumerate()
    {
        let shape = Some(shape);
        for r in chunks::<FOREST_STROKE>(&s.forest_strokes[f.strokes.clone()]) {
            put(r, 0.0, 2, (STROKE, FOREST, shape, true));
        }
        for r in chunks::<FOREST_TRIANGLE>(&s.forest_triangles[f.triangles.clone()]) {
            put(
                r,
                0.0,
                3,
                (TRIANGLE, FOREST, shape, !f.boundaries.is_empty()),
            );
        }
        for r in chunks::<FOREST_BOUNDARY>(&s.forest_boundaries[f.boundaries.clone()]) {
            put(r, 0.0, 2, (EDGE, FOREST, shape, true));
        }
    }
    for r in chunks::<AREA>(&s.forests) {
        let far = |k: usize| (r[k] as f64 + r[k + 2] as f64) as f32;
        put(
            &[r[0], r[1], far(0), far(1)],
            0.0,
            2,
            (RECT, FOREST, None, true),
        );
    }
    // A river stretch is a stroke with a half width and grade at each end;
    // its bank's heights follow in a record cells don't list.
    for r in chunks::<RIVER>(&s.rivers) {
        put(&r[..FLOATS], 0.0, 2, (STROKE, WATER, None, true));
        put(&r[8..10], 0.0, 0, (STROKE, WATER, None, false));
    }
    if meta.len() > RECORD_MASK as usize {
        return Err("surface field: too many ground primitives".into());
    }
    // A storage binding cannot be empty: the table holds a record at least.
    records.resize(records.len().max(FLOATS), 0.0);

    // The grid covers the map and every primitive, so a point past it is
    // nearer the border cell it clamps to than to anything listed elsewhere.
    let origin = [min_x as f32 as f64, min_y as f32 as f64];
    let across = |span: f64, cell: f64| (span / cell).ceil().max(1.0);
    let mut cell_m = CELL_M;
    while across(max_x - origin[0], cell_m) * across(max_y - origin[1], cell_m) > MAX_CELLS {
        cell_m *= 2.0;
    }
    let (cols, rows) = (
        across(max_x - origin[0], cell_m),
        across(max_y - origin[1], cell_m),
    );
    let (cols, rows) = (cols as usize, rows as usize);
    let largest = [
        origin[0].abs(),
        origin[1].abs(),
        max_x.abs(),
        max_y.abs(),
        1.0,
    ];
    let pad = largest.into_iter().fold(f64::NEG_INFINITY, f64::max) * CELL_PAD;

    // How many times `level`'s cells double the finest, and their count.
    let doublings = |level: usize| {
        let mut shift = 0;
        while cell_m * 2f64.powi(shift) < CELL_FOOTPRINTS * FOOTPRINT_M * 2f64.powi(level as i32) {
            shift += 1;
        }
        shift as usize
    };
    let size = |shift: usize| (((cols - 1) >> shift) + 1, ((rows - 1) >> shift) + 1);
    // Every (cell slot, record) of `level`: the records a point there may read.
    let visit = |level: usize, each: &mut dyn FnMut(usize, usize)| {
        let shift = doublings(level);
        let cell = cell_m * 2f64.powi(shift as i32);
        let (level_cols, level_rows) = size(shift);
        let whole = level_cols == 1 && level_rows == 1;
        let reaches = if whole {
            [f64::INFINITY; 3]
        } else {
            reach[level]
        };
        let at = |v: f64, axis: usize, n: usize| {
            (n as f64 - 1.0).min(((v - origin[axis]) / cell).floor().max(0.0)) as usize
        };
        for (record, &(kind, rule, _, listed)) in meta.iter().enumerate() {
            if !listed {
                continue;
            }
            let r: [f64; 6] = std::array::from_fn(|k| records[record * FLOATS + k] as f64);
            // A triangle lists only the cells it touches; a stroke is read out
            // to its half width (a river's wider end) and the reach; an edge
            // and a rect to the reach.
            let half = match (kind, rule) {
                (STROKE, WATER) => r[4].max(r[5]),
                (STROKE, _) => r[4],
                _ => 0.0,
            };
            let grow = (if kind == TRIANGLE {
                0.0
            } else {
                reaches[rule] + half
            }) + pad;
            let (mut low, mut high) = ([r[0], r[1]], [r[2], r[3]]);
            if kind != RECT {
                low = [r[0].min(r[2]), r[1].min(r[3])];
                high = [r[0].max(r[2]), r[1].max(r[3])];
            }
            if kind == TRIANGLE {
                low = [low[0].min(r[4]), low[1].min(r[5])];
                high = [high[0].max(r[4]), high[1].max(r[5])];
            }
            for cy in at(low[1] - grow, 1, level_rows)..=at(high[1] + grow, 1, level_rows) {
                for cx in at(low[0] - grow, 0, level_cols)..=at(high[0] + grow, 0, level_cols) {
                    if kind != RECT && !whole {
                        // The cell grown by the reach, as a square.
                        let x0 = origin[0] + cx as f64 * cell - grow;
                        let y0 = origin[1] + cy as f64 * cell - grow;
                        let x1 = origin[0] + (cx + 1) as f64 * cell + grow;
                        let y1 = origin[1] + (cy + 1) as f64 * cell + grow;
                        let touches = if kind == TRIANGLE {
                            convex_overlap(&r, &[x0, y0, x1, y0, x1, y1, x0, y1])
                        } else {
                            segment_touches_box([r[0], r[1]], [r[2], r[3]], [x0, y0], [x1, y1])
                        };
                        if !touches {
                            continue;
                        }
                    }
                    each((cy * level_cols + cx) * CELL_WORDS + rule, record);
                }
            }
        }
    };

    let (mut counts, mut words, mut exact_to_m) = (Vec::<Vec<u32>>::new(), 0, f64::INFINITY);
    for level in 0.. {
        let (c, r) = size(doublings(level));
        let mut level_counts = vec![0u32; c * r * CELL_WORDS];
        let mut entries = 0;
        visit(level, &mut |slot, _| {
            level_counts[slot] += 1;
            entries += 1;
        });
        let budget = (c * r) as f64 * (LIST_BUDGET * libm::pow(SQRT_2, level as f64));
        if level > 0 && entries as f64 > budget {
            exact_to_m = FOOTPRINT_M * 2f64.powi(level as i32 - 1);
            break;
        }
        words += level_counts.len() + 1 + entries;
        counts.push(level_counts);
        if c * r == 1 {
            break;
        }
    }
    // Per level its header's first word and doublings; then per level its
    // cells' list starts, a closing word, and the lists.
    let levels = counts.len();
    let mut index = vec![0u32; levels * 2 + words];
    let mut at = levels * 2;
    for (level, cursors) in counts.iter_mut().enumerate() {
        let header = at;
        index[level * 2] = header as u32;
        index[level * 2 + 1] = doublings(level) as u32;
        let mut cursor = header + cursors.len() + 1;
        for (slot, size) in cursors.iter_mut().enumerate() {
            index[header + slot] = cursor as u32;
            (cursor, *size) = (cursor + *size as usize, cursor as u32);
        }
        index[header + cursors.len()] = cursor as u32;
        visit(level, &mut |slot, record| {
            index[cursors[slot] as usize] = (meta[record].0 << KIND_SHIFT) | record as u32;
            cursors[slot] += 1;
        });
        // A cell's forest list runs shape by shape: mark each later shape's start.
        for slot in (FOREST..cursors.len()).step_by(CELL_WORDS) {
            let mut shape = None;
            for e in index[header + slot] as usize..index[header + slot + 1] as usize {
                let Some(own) = meta[(index[e] & RECORD_MASK) as usize].2 else {
                    continue;
                };
                if shape.is_some_and(|s| s != own) {
                    index[e] |= NEW_SHAPE;
                }
                shape = Some(own);
            }
        }
        at = cursor;
    }
    let grid = [
        origin[0],
        origin[1],
        cell_m,
        cols as f64,
        rows as f64,
        levels as f64,
        exact_to_m,
    ];
    Ok(Field {
        records,
        index,
        grid,
    })
}

impl Field {
    /// The cell under `(x, y)` for a pixel `footprint` metres wide: where its
    /// paved, forest and water lists start, and where the last ends. The
    /// GPU's `groundCell`, in its f32 arithmetic.
    fn cell(&self, x: f64, y: f64, footprint: f64) -> [usize; 4] {
        let [ox, oy, cell, cols, rows, levels, _] = self.grid;
        let (mut level, mut cap) = (0, FOOTPRINT_M);
        while footprint > cap && ((level + 1) as f64) < levels {
            cap *= 2.0;
            level += 1;
        }
        let shift = self.index[level * 2 + 1];
        let inverse = 1.0 / cell as f32;
        let at = |v: f64, o: f64, n: f64| {
            let c = ((v as f32 - o as f32) * inverse).floor() as f64;
            ((n - 1.0).min(c.max(0.0)) as i32 >> shift) as usize
        };
        let (cx, cy) = (at(x, ox, cols), at(y, oy, rows));
        let across = ((cols as i32 - 1) >> shift) as usize + 1;
        let header = self.index[level * 2] as usize + (cy * across + cx) * CELL_WORDS;
        std::array::from_fn(|k| self.index[header + k] as usize)
    }

    /// The entries of list `rule` at `(x, y)`: record kind, new shape, record.
    fn list(
        &self,
        rule: usize,
        [x, y, footprint]: [f64; 3],
    ) -> impl Iterator<Item = (u32, bool, &[f32])> {
        let c = self.cell(x, y, footprint);
        self.index[c[rule]..c[rule + 1]].iter().map(|&e| {
            let at = (e & RECORD_MASK) as usize * FLOATS;
            (
                e >> KIND_SHIFT,
                e & NEW_SHAPE != 0,
                &self.records[at..at + FLOATS],
            )
        })
    }

    /// How far `(x, y)` lies inside the paving (negative outside): the
    /// deepest stroke, or the polygons' union by their exposed boundary.
    pub fn paved(&self, [x, y, footprint]: [f64; 3]) -> f64 {
        let (mut paved, mut inside, mut nearest) = (-1e9f64, false, 1e9f64);
        for (kind, _, r) in self.list(PAVED, [x, y, footprint]) {
            match kind {
                STROKE => paved = paved.max(stroke_inside(r, [x, y])),
                TRIANGLE => inside = inside || in_triangle(r, [x, y]),
                _ => nearest = nearest.min(edge_distance(r, [x, y])),
            }
        }
        paved.max(if inside { nearest } else { -nearest })
    }

    /// How far `(x, y)` lies inside the deepest forest shape (negative outside).
    pub fn forest(&self, [x, y, footprint]: [f64; 3]) -> f64 {
        let (mut forest, mut distance, mut inside, mut nearest) = (-1e9f64, -1e9f64, false, 1e9f64);
        for (kind, new_shape, r) in self.list(FOREST, [x, y, footprint]) {
            if new_shape {
                forest = forest
                    .max(distance)
                    .max(if inside { nearest } else { -nearest });
                (distance, inside, nearest) = (-1e9, false, 1e9);
            }
            match kind {
                RECT => {
                    let [x0, y0, x1, y1] = [r[0], r[1], r[2], r[3]].map(f64::from);
                    forest = forest.max((x - x0).min(x1 - x).min((y - y0).min(y1 - y)));
                }
                STROKE => distance = distance.max(stroke_inside(r, [x, y])),
                TRIANGLE => inside = inside || in_triangle(r, [x, y]),
                _ => nearest = nearest.min(edge_distance(r, [x, y])),
            }
        }
        forest
            .max(distance)
            .max(if inside { nearest } else { -nearest })
    }

    /// How far `(x, y)` lies inside the water's edge (negative outside).
    pub fn water(&self, [x, y, footprint]: [f64; 3]) -> f64 {
        self.list(WATER, [x, y, footprint])
            .fold(-1e9, |bed, (_, _, r)| bed.max(stretch_inside(r, x, y)))
    }

    /// Paved, forest and water distances at `(x, y)` for a pixel `footprint` wide.
    pub fn distances(&self, at: [f64; 3]) -> [f64; 3] {
        [self.paved(at), self.forest(at), self.water(at)]
    }
}

fn point(r: &[f32], k: usize) -> [f64; 2] {
    [r[k * 2] as f64, r[k * 2 + 1] as f64]
}

fn in_triangle(r: &[f32], p: [f64; 2]) -> bool {
    triangle_contains(point(r, 0), point(r, 1), point(r, 2), p)
}

fn edge_distance(r: &[f32], p: [f64; 2]) -> f64 {
    distance(p, closest(p, point(r, 0), point(r, 1)))
}

/// Each paved triangle's area's slab bearing, within a quarter turn: the
/// triangles joined by shared corners (one polygon) or lying over one another
/// (an apron on a court) are one area, laid square to as much of its outline
/// as it can be: the bearing whose outer sides within 1.5° of it are longest
/// in all, the longest such side breaking a tie.
fn area_bearings(triangles: &[f32]) -> Vec<f32> {
    let tri = chunks::<SURFACE_TRIANGLE>(triangles);
    let mut parent: Vec<usize> = (0..tri.len()).collect();
    let root = |p: &[usize], mut i: usize| {
        while p[i] != i {
            i = p[i];
        }
        i
    };
    let join = |p: &mut Vec<usize>, a: usize, b: usize| {
        let (a, b) = (root(p, a), root(p, b));
        p[a] = b;
    };
    // −0 and +0 are one corner.
    let corner = |t: usize, k: usize| {
        (
            (tri[t][2 * k] + 0.0).to_bits(),
            (tri[t][2 * k + 1] + 0.0).to_bits(),
        )
    };
    let at = |t: usize, k: usize| [tri[t][2 * k] as f64, tri[t][2 * k + 1] as f64];
    let mut owner = HashMap::new();
    let bucket = |v: f64| (v / 32.0).floor() as i64;
    let mut cells: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
    for t in 0..tri.len() {
        for k in 0..3 {
            match owner.get(&corner(t, k)) {
                Some(&seen) => join(&mut parent, t, seen),
                None => _ = owner.insert(corner(t, k), t),
            }
        }
        let [xs, ys] = [0, 1].map(|axis| [0, 1, 2].map(|k| at(t, k)[axis]));
        let low = |v: [f64; 3]| bucket(v[0].min(v[1]).min(v[2]));
        let high = |v: [f64; 3]| bucket(v[0].max(v[1]).max(v[2]));
        for i in low(xs)..=high(xs) {
            for j in low(ys)..=high(ys) {
                cells.entry((i, j)).or_default().push(t);
            }
        }
    }
    for t in 0..tri.len() {
        let [a, b, c] = [0, 1, 2].map(|k| at(t, k));
        let middle = [(a[0] + b[0] + c[0]) / 3.0, (a[1] + b[1] + c[1]) / 3.0];
        for &u in cells
            .get(&(bucket(middle[0]), bucket(middle[1])))
            .into_iter()
            .flatten()
        {
            if root(&parent, u) != root(&parent, t)
                && triangle_contains(at(u, 0), at(u, 1), at(u, 2), middle)
            {
                join(&mut parent, t, u);
            }
        }
    }
    // An edge two triangles share is inside their area.
    let edge = |t: usize, k: usize| {
        let (a, b) = (corner(t, k), corner(t, (k + 1) % 3));
        (a.min(b), a.max(b))
    };
    let mut edges: HashMap<_, u32> = HashMap::new();
    for t in 0..tri.len() {
        for k in 0..3 {
            *edges.entry(edge(t, k)).or_default() += 1;
        }
    }
    // Each area's outer sides: bearing within a quarter turn, length.
    let quarter = PI / 2.0;
    let mut sides: HashMap<usize, Vec<(f64, f64)>> = HashMap::new();
    for t in 0..tri.len() {
        for k in (0..3).filter(|&k| edges[&edge(t, k)] == 1) {
            let (a, b) = (at(t, k), at(t, (k + 1) % 3));
            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
            let turn = libm::atan2(dy, dx);
            let side = (
                turn - (turn / quarter).floor() * quarter,
                libm::hypot(dx, dy),
            );
            sides.entry(root(&parent, t)).or_default().push(side);
        }
    }
    let mut bearing = vec![0f32; tri.len()];
    for (r, list) in &sides {
        let mut best = (-1.0, -1.0, 0.0);
        for &(turn, length) in list {
            let square = |&&(other, _): &&(f64, f64)| {
                (((other - turn + quarter * 1.5) % quarter) - quarter / 2.0).abs()
                    <= 1.5 * PI / 180.0
            };
            let along = list.iter().filter(square).fold(0.0, |sum, &(_, l)| sum + l);
            if along > best.0 || (along == best.0 && length > best.1) {
                best = (along, length, turn);
            }
        }
        bearing[*r] = best.2 as f32;
    }
    (0..tri.len()).map(|t| bearing[root(&parent, t)]).collect()
}

#[cfg(test)]
mod tests {
    use super::super::Source;
    use super::{area_bearings, field, MAX_CELLS};
    use std::f64::consts::PI;

    const DEG: f64 = PI / 180.0;
    const REACH: [[f64; 3]; 32] = [[3.0, 6.0, 3.0]; 32];

    /// `[w, h]` turned `turn` about its corner at `centre`, as a ring.
    fn turned(turn: f64, [w, h]: [f64; 2], [cx, cy]: [f64; 2]) -> [[f64; 2]; 4] {
        let (c, s) = (libm::cos(turn), libm::sin(turn));
        [[0.0, 0.0], [w, 0.0], [w, h], [0.0, h]]
            .map(|[x, y]| [cx + x * c - y * s, cy + x * s + y * c])
    }

    fn triangles(list: &[[[f64; 2]; 3]]) -> Vec<f32> {
        let row = |t: &[[f64; 2]; 3]| [t[0][0], t[0][1], t[1][0], t[1][1], t[2][0], t[2][1], 3.0];
        list.iter().flat_map(|t| row(t).map(|v| v as f32)).collect()
    }

    fn fan(r: [[f64; 2]; 4]) -> [[[f64; 2]; 3]; 2] {
        [[r[0], r[1], r[2]], [r[0], r[2], r[3]]]
    }

    fn near(a: f32, b: f64) -> bool {
        (a as f64 - b).abs() < 1e-4
    }

    #[test]
    fn every_triangle_of_a_paved_area_lays_its_slabs_square_to_its_longest_side() {
        // A 40 by 10 m yard turned 30 degrees, as two triangles, and a
        // square apart from it turned 70 degrees.
        let yard = fan(turned(30.0 * DEG, [40.0, 10.0], [100.0, 100.0]));
        let square = fan(turned(70.0 * DEG, [8.0, 8.0], [300.0, 300.0]));
        let b = area_bearings(&triangles(&[yard, square].concat()));
        assert!(near(b[0], 30.0 * DEG) && near(b[1], 30.0 * DEG), "{b:?}");
        // The square's, 70 degrees within a quarter turn: 70 - 90 = -20,
        // which is 70 again mod 90.
        assert!(near(b[2], 70.0 * DEG) && b[2] == b[3], "{b:?}");
    }

    #[test]
    fn an_areas_triangles_and_paving_laid_over_it_share_one_grid() {
        // A fan of a pentagon court whose longest side, 50 m along 10
        // degrees, is an outer edge of its first triangle only; the second
        // triangle's own longest outer edge runs at 100 degrees mod 90.
        let at = |deg: f64, m: f64, [x, y]: [f64; 2]| {
            [x + m * libm::cos(deg * DEG), y + m * libm::sin(deg * DEG)]
        };
        let p0 = [0.0, 0.0];
        let p1 = at(10.0, 50.0, p0);
        let p2 = at(100.0, 12.0, p1);
        let p3 = at(160.0, 30.0, p2);
        let p4 = at(215.0, 12.0, p3);
        // An apron inside it, turned 25 degrees, sharing no corner with it.
        let c = [20.0, 10.0];
        let apron = [c, at(25.0, 8.0, c), at(115.0, 6.0, at(25.0, 8.0, c))];
        let b = area_bearings(&triangles(&[
            [p0, p1, p2],
            [p0, p2, p3],
            [p0, p3, p4],
            apron,
        ]));
        assert!(b.iter().all(|&b| near(b, 10.0 * DEG)), "{b:?}");
    }

    fn empty(size: f64) -> Source {
        Source {
            map: [0.0, 0.0, size, size],
            ..Source::default()
        }
    }

    #[test]
    fn the_fields_memory_is_bounded_by_the_cell_budget_not_the_maps_area() {
        let of = |size: f64| field(&empty(size), &[], &[], &REACH).unwrap();
        let (town, large, huge) = (of(1600.0), of(10000.0), of(40000.0));
        // Six times the side is 39 times the cells at one cell size: the cell grows instead.
        assert!(large.grid[2] > town.grid[2]);
        for f in [&town, &large, &huge] {
            assert!(f.grid[3] * f.grid[4] <= MAX_CELLS);
        }
        // Sixteen times the area again costs under twice the index (more
        // levels share the finest cell).
        assert!(huge.index.len() < large.index.len() * 2);
    }

    #[test]
    fn the_field_is_the_same_bytes_for_the_same_ground() {
        let ground = Source {
            surface_triangles: triangles(&fan(turned(30.0 * DEG, [40.0, 10.0], [100.0, 100.0]))),
            surface_boundaries: vec![100.0, 100.0, 140.0, 100.0, 3.0],
            ..empty(400.0)
        };
        let [a, b] = [0, 1].map(|_| field(&ground, &[], &[], &REACH).unwrap());
        assert_eq!((a.records, a.index), (b.records, b.index));
    }
}
