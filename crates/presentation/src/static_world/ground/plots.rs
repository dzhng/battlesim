//! The patchwork's plots: a seeded binary split of the land into convex
//! polygons, cut first along the roads (so fields meet them edge-on) and then
//! by the biome's field rules. The land is cut on one heading down to tracts;
//! each tract takes a grain of its own (its longest road's, or a turn off the
//! land's), and its plots keep it. The terrain material walks the split tree
//! per pixel: the nearest cut on the way down is the distance to the plot's
//! edge, where the verge grows, as sharp up close as from afar.
use super::biome::{Biome, FieldRules, PLOT_HUE_JITTER};
use super::shapes::{centroid, segments_cross, signed_area};
use super::water::StandingWater;
use super::{chunks, is_road, Source, RUN, SURFACE_BOUNDARY, SURFACE_STROKE, SURFACE_TRIANGLE};
use crate::seed::Mulberry32;
use sim::world::export::SWAMP_STRIDE as SWAMP;
use std::collections::HashMap;
use std::f64::consts::PI;

/// A leaf of the split: one field, meadow or ploughed plot.
pub struct Plot {
    /// Index into the biome's plot kinds.
    pub kind: usize,
    /// sRGB, jittered from the kind's palette.
    pub colour: [f64; 3],
    /// Unit vector across the plot's rows (the rows run along its length).
    pub across: [f64; 2],
    /// Convex outline, counter-clockwise, flat `[x0, y0, x1, y1, …]`.
    pub outline: Vec<f64>,
}

/// Floats per node: the cut's unit normal and offset, then the children.
pub const NODE: usize = 5;
/// A split tree deeper than this is a runaway (bad rules), not a patchwork.
pub const MAX_DEPTH: usize = 64;

pub struct Plots {
    /// The map box widened by `field_rules.extent_m`, or the map's own
    /// rendered surroundings.
    pub region: [f64; 4],
    /// Per node: `nx, ny, c, front, back`; `p` is in front where
    /// `nx·px + ny·py >= c`. A child `>= 0` is a node; `-1 - k` is plot k.
    pub nodes: Vec<f64>,
    pub plots: Vec<Plot>,
    /// The longest path from the root to a plot, in nodes.
    pub depth: usize,
    /// The kind of a building's own ground.
    pub settlement: usize,
}

const DEG: f64 = PI / 180.0;
/// A cut leaves both sides at least this much area (m²): no slivers.
const MIN_PIECE_AREA_M2: f64 = 100.0;
/// A road cuts a plot only where it runs along this share of the cut's chord
/// through it (else the plot is cut by size first); land not yet a tract asks
/// for more, as a cut there runs on for hundreds of metres past the road.
const TRACT_ROAD_SHARE: f64 = 0.7;
const LAND_ROAD_SHARE: f64 = 0.9;
/// A road stays a plot's candidate while its box comes this near the plot's.
const ROAD_BOX_PAD_M: f64 = 1e-3;

/// Keeps the part of convex `poly` where `n·p >= c` (Sutherland–Hodgman).
fn clip(poly: &[f64], [nx, ny, c]: [f64; 3]) -> Vec<f64> {
    let mut out = vec![];
    let count = poly.len() / 2;
    for i in 0..count {
        let j = (i + 1) % count;
        let (ax, ay, bx, by) = (poly[i * 2], poly[i * 2 + 1], poly[j * 2], poly[j * 2 + 1]);
        let (da, db) = (nx * ax + ny * ay - c, nx * bx + ny * by - c);
        if da >= 0.0 {
            out.extend([ax, ay]);
        }
        if (da >= 0.0) != (db >= 0.0) {
            let t = da / (da - db);
            out.extend([ax + (bx - ax) * t, ay + (by - ay) * t]);
        }
    }
    out
}

/// Extent of `poly` along unit `(ax, ay)`: `(min, max)`.
fn span(poly: &[f64], ax: f64, ay: f64) -> (f64, f64) {
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for p in poly.as_chunks::<2>().0 {
        let d = p[0] * ax + p[1] * ay;
        (lo, hi) = (lo.min(d), hi.max(d));
    }
    (lo, hi)
}

/// The parameter range of the line `a + t (b − a)` inside convex `poly` grown
/// by `pad`, from `t`; `None` where it runs parallel to a side, outside it.
/// The side normals are of unit length where `unit`.
fn chord(
    poly: &[f64],
    a: [f64; 2],
    b: [f64; 2],
    pad: f64,
    unit: bool,
    t: (f64, f64),
) -> Option<(f64, f64)> {
    let (mut t0, mut t1) = t;
    let count = poly.len() / 2;
    let turn = if signed_area(poly) >= 0.0 { 1.0 } else { -1.0 };
    let mut i = 0;
    while i < count && t0 < t1 {
        let (px, py) = (poly[i * 2], poly[i * 2 + 1]);
        let j = (i + 1) % count;
        let (ex, ey) = (poly[j * 2] - px, poly[j * 2 + 1] - py);
        i += 1;
        // The side's inward normal.
        let edge = libm::hypot(ex, ey);
        let (nx, ny) = match () {
            _ if !unit && turn > 0.0 => (-ey, ex),
            _ if !unit => (ey, -ex),
            _ if edge == 0.0 => continue,
            _ => ((-ey / edge) * turn, (ex / edge) * turn),
        };
        let fa = nx * (a[0] - px) + ny * (a[1] - py) + pad;
        let slope = nx * (b[0] - a[0]) + ny * (b[1] - a[1]);
        if slope == 0.0 {
            if fa < 0.0 {
                return None;
            }
        } else if slope > 0.0 {
            t0 = t0.max(-fa / slope);
        } else {
            t1 = t1.min(-fa / slope);
        }
    }
    Some((t0, t1))
}

/// The cut line along road `a`→`b`: its unit normal and offset.
fn line(a: [f64; 2], b: [f64; 2]) -> ([f64; 3], f64) {
    let len = libm::hypot(b[0] - a[0], b[1] - a[1]);
    let (nx, ny) = (-(b[1] - a[1]) / len, (b[0] - a[0]) / len);
    ([nx, ny, nx * a[0] + ny * a[1]], len)
}

/// Whether a road cuts `poly` into two real pieces. Its line must run along
/// the road for `share` of its chord through the plot, or the plot be no
/// longer than `short_chord` that way: no cut runs far past a road's end.
fn road_cuts(poly: &[f64], a: [f64; 2], b: [f64; 2], short_chord: f64, share: f64) -> bool {
    let ([nx, ny, c], len) = line(a, b);
    if len == 0.0 {
        return false;
    }
    let all = (f64::NEG_INFINITY, f64::INFINITY);
    let Some((t0, t1)) = chord(poly, a, b, 0.0, false, all) else {
        return false;
    };
    // Also when either is NaN.
    if t1.partial_cmp(&t0) != Some(std::cmp::Ordering::Greater) {
        return false;
    }
    let along = t1.min(1.0) - t0.max(0.0);
    if along * len < 1.0 || (along < share * (t1 - t0) && (t1 - t0) * len > short_chord) {
        return false;
    }
    let pieces = [clip(poly, [nx, ny, c]), clip(poly, [-nx, -ny, -c])];
    pieces
        .iter()
        .all(|p| p.len() >= 6 && signed_area(p).abs() >= MIN_PIECE_AREA_M2)
}

/// The grain a piece is cut on: its heading, depth and whether it's a tract.
type Grain = (f64, usize, bool);

struct Split<'a> {
    rules: &'a FieldRules,
    biome: &'a Biome,
    source: &'a Source,
    /// Where the ground stands flooded, so no crop is drilled there.
    water: StandingWater<'a>,
    rng: Mulberry32,
    /// The guides fields are cut along, `ax, ay, bx, by` each, and which
    /// of them are the rivers'.
    roads: Vec<f32>,
    rivers: std::ops::Range<usize>,
    /// Building centres in cells a settlement's reach wide.
    reach: f64,
    buildings: HashMap<(i64, i64), Vec<[f64; 2]>>,
    surround: usize,
    out: Plots,
}

impl Split<'_> {
    fn road(&self, r: usize) -> ([f64; 2], [f64; 2]) {
        let e = &self.roads[r * 4..r * 4 + 4];
        ([e[0] as f64, e[1] as f64], [e[2] as f64, e[3] as f64])
    }

    fn cell(&self, x: f64, y: f64) -> (i64, i64) {
        (
            (x / self.reach).floor() as i64,
            (y / self.reach).floor() as i64,
        )
    }

    /// Whether no carriageway (stroke centreline or road triangle edge)
    /// crosses from a yard's centre to its building.
    fn same_road_side(&self, centre: [f64; 2], building: [f64; 2]) -> bool {
        let (x0, x1) = (centre[0].min(building[0]), centre[0].max(building[0]));
        let (y0, y1) = (centre[1].min(building[1]), centre[1].max(building[1]));
        let s = self.source;
        let strokes = chunks::<SURFACE_STROKE>(&s.surface_strokes)
            .iter()
            .map(|r| (&r[..], 5, 1));
        let triangles = chunks::<SURFACE_TRIANGLE>(&s.surface_triangles).iter();
        for (r, kind, edges) in strokes.chain(triangles.map(|r| (&r[..], 6, 3))) {
            for edge in (0..edges).filter(|_| is_road(r[kind])) {
                let (a, b) = (edge * 2, if edges == 1 { 2 } else { (edge + 1) % 3 * 2 });
                let [ax, ay, bx, by] = [r[a], r[a + 1], r[b], r[b + 1]].map(f64::from);
                let apart = ax.max(bx) < x0 || ax.min(bx) > x1;
                if apart || ay.max(by) < y0 || ay.min(by) > y1 {
                    continue;
                }
                if segments_cross(centre, building, [ax, ay], [bx, by]) {
                    return false;
                }
            }
        }
        true
    }

    fn near_building(&self, centre: [f64; 2], within_sq: f64, keep_road_side: bool) -> bool {
        let reach = self.reach;
        let near = |&b: &[f64; 2]| {
            let (x, y) = (centre[0] - b[0], centre[1] - b[1]);
            x * x + y * y <= within_sq && (!keep_road_side || self.same_road_side(centre, b))
        };
        let mut dy = -reach;
        while dy <= reach {
            let mut dx = -reach;
            while dx <= reach {
                let cell = self
                    .buildings
                    .get(&self.cell(centre[0] + dx, centre[1] + dy));
                if cell.is_some_and(|c| c.iter().any(near)) {
                    return true;
                }
                dx += reach;
            }
            dy += reach;
        }
        false
    }

    /// The ground round a building is the settlement's own, and the land
    /// round that its surround, where no crop is drilled; a swamp floods the
    /// ground its look names.
    fn pick_kind(&mut self, centre: [f64; 2]) -> usize {
        if let Some(ground) = self.water.flooded(centre[0], centre[1]) {
            if let Some(p) = self.biome.plots.iter().position(|p| p.name == ground) {
                return p;
            }
        }
        if self.near_building(centre, libm::pow(self.rules.yard_m, 2.0), true) {
            return self.out.settlement;
        }
        if self.near_building(centre, libm::pow(self.rules.settlement_m, 2.0), false) {
            return self.surround;
        }
        let total = self.biome.plots.iter().fold(0.0, |a, p| a + p.weight);
        let mut roll = self.rng.sample() * total;
        for (k, p) in self.biome.plots.iter().enumerate() {
            roll -= p.weight;
            if roll < 0.0 {
                return k;
            }
        }
        self.biome.plots.len() - 1
    }

    fn leaf(&mut self, poly: Vec<f64>, heading: f64) -> f64 {
        let kind = self.pick_kind(centroid(&poly));
        let palette = &self.biome.palettes[&self.biome.plots[kind].palette];
        let base = &palette[(self.rng.sample() * palette.len() as f64).floor() as usize];
        let jitter = self.rules.colour_jitter;
        let value = 1.0 + (self.rng.sample() * 2.0 - 1.0) * jitter;
        let colour = [0, 1, 2].map(|c| {
            let hue = 1.0 + (self.rng.sample() * 2.0 - 1.0) * jitter * PLOT_HUE_JITTER;
            (base[c] * value * hue).clamp(0.0, 1.0)
        });
        // Rows run along the plot's longer side.
        let (ux, uy) = (libm::cos(heading), libm::sin(heading));
        let ((u0, u1), (v0, v1)) = (span(&poly, ux, uy), span(&poly, -uy, ux));
        let across = if u1 - u0 >= v1 - v0 {
            [-uy, ux]
        } else {
            [ux, uy]
        };
        self.out.plots.push(Plot {
            kind,
            colour,
            across,
            outline: poly,
        });
        -(self.out.plots.len() as f64)
    }

    /// The roads of `within`, in order, whose box meets `poly`'s: the only
    /// ones that can cut it or any plot split from it.
    fn roads_near(&self, poly: &[f64], within: &[usize]) -> Vec<usize> {
        let ((x0, x1), (y0, y1)) = (span(poly, 1.0, 0.0), span(poly, 0.0, 1.0));
        let pad = ROAD_BOX_PAD_M;
        let near = |&r: &usize| {
            let (a, b) = self.road(r);
            a[0].max(b[0]) >= x0 - pad
                && a[0].min(b[0]) <= x1 + pad
                && a[1].max(b[1]) >= y0 - pad
                && a[1].min(b[1]) <= y1 + pad
        };
        within.iter().copied().filter(near).collect()
    }

    /// Adds a node cutting `poly` by `line` and returns its index.
    fn cut(
        &mut self,
        poly: &[f64],
        [nx, ny, c]: [f64; 3],
        grain: Grain,
        roads: &[usize],
    ) -> Result<f64, String> {
        let at = self.out.nodes.len();
        self.out.nodes.extend([nx, ny, c, 0.0, 0.0]);
        let (heading, level, tract) = grain;
        for (k, line) in [[nx, ny, c], [-nx, -ny, -c]].into_iter().enumerate() {
            let piece = clip(poly, line);
            self.out.nodes[at + 3 + k] = self.split(piece, heading, level + 1, roads, tract)?;
        }
        Ok((at / NODE) as f64)
    }

    /// The heading of the longest road in `poly` or beside it (no plot fits
    /// between them); `None` where none is worth a plot's width.
    fn road_heading(&self, poly: &[f64], roads: &[usize]) -> Option<f64> {
        let (mut longest, mut heading) = (self.rules.min_width_m, None);
        for &r in roads {
            let (a, b) = self.road(r);
            let pad = self.rules.min_width_m;
            let within = match chord(poly, a, b, pad, true, (0.0, 1.0)) {
                Some((t0, t1)) => (t1 - t0).max(0.0) * libm::hypot(b[0] - a[0], b[1] - a[1]),
                None => 0.0,
            };
            if within > longest {
                (longest, heading) = (within, Some(libm::atan2(b[1] - a[1], b[0] - a[0])));
            }
        }
        heading
    }

    fn split(
        &mut self,
        poly: Vec<f64>,
        land: f64,
        level: usize,
        parents: &[usize],
        tract: bool,
    ) -> Result<f64, String> {
        self.out.depth = self.out.depth.max(level);
        if level >= MAX_DEPTH {
            return Err("field_rules: the plot split runs away".into());
        }
        let rules = self.rules;
        let roads = self.roads_near(&poly, parents);
        // Land no longer than a tract either way becomes one, turned to a
        // grain of its own. In a tract, land with a road in or beside it lies
        // along its longest; the rest keeps the grain it was cut from.
        let (mut heading, mut tract) = (land, tract);
        if !tract {
            let (a0, a1) = span(&poly, libm::cos(land), libm::sin(land));
            let (b0, b1) = span(&poly, -libm::sin(land), libm::cos(land));
            if (a1 - a0).max(b1 - b0) <= rules.tract_m {
                heading = land + self.rng.float(-1.0, 1.0) * rules.orientation_jitter_deg * DEG;
                tract = true;
            }
        }
        if tract {
            heading = self.road_heading(&poly, &roads).unwrap_or(heading);
        }
        // Roads first: the first road crossing this plot cuts it, and both
        // sides are tracts from there. A river's run cuts whatever tract it
        // crosses: no plot lies on both banks.
        for &r in &roads {
            let (a, b) = self.road(r);
            let share = match () {
                _ if !tract => LAND_ROAD_SHARE,
                _ if self.rivers.contains(&r) => 0.0,
                _ => TRACT_ROAD_SHARE,
            };
            if road_cuts(&poly, a, b, rules.size_m[1], share) {
                return self.cut(&poly, line(a, b).0, (heading, level, true), &roads);
            }
        }
        let (ux, uy) = (libm::cos(heading), libm::sin(heading));
        let ((u0, u1), (v0, v1)) = (span(&poly, ux, uy), span(&poly, -uy, ux));
        let (lu, lv) = (u1 - u0, v1 - v0);
        let (long, short) = if lu >= lv { (lu, lv) } else { (lv, lu) };
        let area = signed_area(&poly).abs();
        let side = self.rng.float(rules.size_m[0], rules.size_m[1]);
        let wide = 2.0 * rules.min_width_m;
        if (area <= side * side && long <= rules.max_aspect * short)
            || (short < wide && long < wide)
        {
            return Ok(self.leaf(poly, heading));
        }
        // Across the length by default; along it into strips, by chance,
        // while the strips stay wide enough and not too long.
        let strips = self.rng.sample() < rules.strip_chance
            && short >= wide
            && long <= rules.max_aspect * (short / 2.0);
        let across_u = (lu >= lv) != strips;
        if (!across_u && lv < wide) || (across_u && lu < wide) {
            return Ok(self.leaf(poly, heading));
        }
        let lean = self.rng.float(-1.0, 1.0) * rules.cut_jitter_deg * DEG;
        let theta = (heading + if across_u { 0.0 } else { PI / 2.0 }) + lean;
        let (nx, ny) = (libm::cos(theta), libm::sin(theta));
        let (s0, s1) = span(&poly, nx, ny);
        let extent = s1 - s0;
        // Keep both pieces at least the minimum width.
        let lo = rules.cut_range[0].max(rules.min_width_m / extent);
        let hi = rules.cut_range[1].min(1.0 - rules.min_width_m / extent);
        if lo > hi {
            return Ok(self.leaf(poly, heading));
        }
        let at = s0 + self.rng.float(lo, hi) * extent;
        self.cut(&poly, [nx, ny, at], (heading, level, tract), &roads)
    }
}

/// Split the map and its surroundings into the biome's plots, deterministic
/// for a biome seed and a source.
pub fn plots(s: &Source, strip_runs: &[f32], biome: &Biome) -> Result<Plots, String> {
    let rules = &biome.field_rules;
    let [mx0, my0, mx1, my1] = s.map;
    let e = rules.extent_m;
    let region = s.rendered.unwrap_or([mx0 - e, my0 - e, mx1 + e, my1 + e]);
    // The guides: the roads' runs, the standing water's (the rivers' runs,
    // the swamps' outlines), the roads' boundary, the strips.
    let swamp_edges = s
        .swamps
        .as_chunks::<SWAMP>()
        .0
        .iter()
        .flat_map(|e| e[..RUN].to_vec());
    let mut roads: Vec<f32> = [&s.surface_runs[..], &s.river_runs[..]].concat();
    roads.extend(swamp_edges);
    let waters = (roads.len() - s.surface_runs.len()) / RUN;
    for b in chunks::<SURFACE_BOUNDARY>(&s.surface_boundaries)
        .iter()
        .filter(|b| is_road(b[4]))
    {
        roads.extend_from_slice(&b[..4]);
    }
    roads.extend_from_slice(strip_runs);
    let kind = |name: &str| {
        biome
            .plots
            .iter()
            .position(|p| p.name == name)
            .ok_or(format!("no plot kind {name}"))
    };
    let rivers = s.surface_runs.len() / RUN..s.surface_runs.len() / RUN + waters;
    let out = Plots {
        region,
        nodes: vec![],
        plots: vec![],
        depth: 0,
        settlement: kind(&rules.settlement_kind)?,
    };
    let reach = rules.settlement_m.max(1.0);
    let rng = Mulberry32::new(biome.seed);
    let surround = kind(&rules.surround_kind)?;
    let buildings = HashMap::new();
    let mut split = Split {
        rules,
        biome,
        source: s,
        water: StandingWater::new(&s.rivers, &s.swamps, &[], &s.grid, biome),
        rng,
        roads,
        rivers,
        reach,
        buildings,
        surround,
        out,
    };
    for b in s.buildings() {
        let b = [b[0] as f64, b[1] as f64];
        split
            .buildings
            .entry(split.cell(b[0], b[1]))
            .or_default()
            .push(b);
    }
    let [x0, y0, x1, y1] = region;
    let all: Vec<usize> = (0..split.roads.len() / 4).collect();
    let land = rules.orientation_deg * DEG;
    let top = split.split(vec![x0, y0, x1, y0, x1, y1, x0, y1], land, 1, &all, false)?;
    if top < 0.0 {
        // A single plot: one node whose both sides are it.
        split
            .out
            .nodes
            .extend([1.0, 0.0, f64::NEG_INFINITY, top, top]);
    }
    Ok(split.out)
}

impl Plots {
    /// The plot under `(x, y)`, walking the split tree as the terrain
    /// material does; `None` outside the region.
    pub fn at(&self, x: f64, y: f64) -> Option<usize> {
        let [x0, y0, x1, y1] = self.region;
        if x < x0 || x > x1 || y < y0 || y > y1 {
            return None;
        }
        let mut node = 0;
        for _ in 0..MAX_DEPTH {
            let n = &self.nodes[node * NODE..node * NODE + NODE];
            let child = if n[0] * x + n[1] * y - n[2] >= 0.0 {
                n[3]
            } else {
                n[4]
            };
            if child < 0.0 {
                return Some((-1.0 - child) as usize);
            }
            node = child as usize;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::super::biome::{tests::summer, Biome};
    use super::super::forests::forests;
    use super::super::{forests::strip_runs, Source, AREA_KINDS};
    use super::{plots, Plots, NODE};

    /// The summer biome with fixed field rules, so seeded agricultural cuts
    /// don't obscure the guides.
    fn biome() -> Biome {
        summer(|b| {
            let rules = &mut b["field_rules"];
            rules["extent_m"] = 0.into();
            rules["size_m"] = serde_json::json!([1000, 1000]);
            rules["tract_m"] = 1000.into();
            // A strip between two road edges lies along them, however long.
            rules["max_aspect"] = 50.into();
            // The backdrop reaches no farther than the patchwork.
            b["trees"]["backdrop"]["reach_m"] = 0.into();
        })
        .unwrap()
    }

    fn empty() -> Source {
        Source {
            map: [0.0, 0.0, 100.0, 100.0],
            ..Source::default()
        }
    }

    fn split(s: &Source) -> Plots {
        plots(s, &strip_runs(s, &forests(s).unwrap()), &biome()).unwrap()
    }

    /// The plot under `(x, y)` and the distance to the nearest cut on the
    /// way down to it, or to the region's edge.
    fn at(p: &Plots, x: f64, y: f64) -> (usize, f64) {
        let [x0, y0, x1, y1] = p.region;
        let mut edge = (x - x0).min(x1 - x).min(y - y0).min(y1 - y);
        let mut node = 0;
        loop {
            let n = &p.nodes[node * NODE..node * NODE + NODE];
            let s = n[0] * x + n[1] * y - n[2];
            edge = edge.min(s.abs());
            let child = if s >= 0.0 { n[3] } else { n[4] };
            if child < 0.0 {
                return (p.at(x, y).unwrap(), edge);
            }
            node = child as usize;
        }
    }

    /// A strip of paving of one kind across the map, as a stroke and as a polygon.
    fn paving(named: &str) -> Source {
        let k = AREA_KINDS.iter().position(|n| *n == named).unwrap() as f32;
        Source {
            surface_strokes: vec![0.0, 50.0, 100.0, 50.0, 2.0, k, 3.0],
            surface_runs: if named == "paving" {
                vec![]
            } else {
                vec![0.0, 50.0, 100.0, 50.0]
            },
            surface_triangles: vec![
                0.0, 40.0, 100.0, 40.0, 100.0, 60.0, k, 0.0, 40.0, 100.0, 60.0, 0.0, 60.0, k,
            ],
            surface_boundaries: [
                [0.0, 40.0, 100.0, 40.0],
                [100.0, 40.0, 100.0, 60.0],
                [100.0, 60.0, 0.0, 60.0],
                [0.0, 60.0, 0.0, 40.0],
            ]
            .iter()
            .flat_map(|e| [e[0], e[1], e[2], e[3], k])
            .collect(),
            ..empty()
        }
    }

    /// A strip of forest `width` metres wide along `stretches`.
    fn strip(width: f32, stretches: &[[f32; 4]]) -> Source {
        Source {
            forest_metadata: vec![0.0, 12.0, 1.0],
            forest_trunk_ranges: vec![0, 0],
            forest_strokes: stretches
                .iter()
                .flat_map(|s| [s[0], s[1], s[2], s[3], width / 2.0, 0.0, 0.0])
                .collect(),
            ..empty()
        }
    }

    fn same(a: &Plots, b: &Plots) -> bool {
        a.nodes == b.nodes && a.plots.len() == b.plots.len()
    }

    #[test]
    fn paving_leaves_fields_be_while_every_road_kind_guides_them() {
        assert!(same(&split(&paving("paving")), &split(&empty())));
        for kind in ["road", "country_road", "dirt_track"] {
            let road = split(&paving(kind));
            assert_ne!(at(&road, 50.0, 30.0).0, at(&road, 50.0, 70.0).0, "{kind}");
        }
    }

    #[test]
    fn a_rivers_authored_run_guides_field_boundaries_as_a_roads_does() {
        let river = split(&Source {
            river_runs: vec![0.0, 50.0, 100.0, 50.0],
            swamps: vec![],
            ..empty()
        });
        assert_ne!(at(&river, 50.0, 30.0).0, at(&river, 50.0, 70.0).0);
        // The cut runs along the river: both banks' fields end on its centreline.
        assert!((at(&river, 50.0, 44.0).1 - 6.0).abs() < 1e-9);
        assert!((at(&river, 20.0, 57.0).1 - 7.0).abs() < 1e-9);
        let bare = split(&empty());
        assert_eq!(at(&bare, 50.0, 30.0).0, at(&bare, 50.0, 70.0).0);
    }

    #[test]
    fn a_tree_lines_long_stretches_guide_fields_and_its_bends_chords_do_not() {
        let line = split(&strip(10.0, &[[0.0, 50.0, 100.0, 50.0]]));
        assert_ne!(at(&line, 50.0, 30.0).0, at(&line, 50.0, 70.0).0);
        // The fields either side end on the strip's centreline, under its trees.
        assert!((at(&line, 50.0, 44.0).1 - 6.0).abs() < 1e-9);
        // A bend is exported as chords far shorter than the strip is wide: a
        // field is not cut along one.
        let chord = split(&strip(10.0, &[[49.0, 50.0, 51.0, 50.0]]));
        assert_eq!(at(&chord, 50.0, 30.0).0, at(&chord, 50.0, 70.0).0);
        // A wood is not a line: its outline cuts nothing.
        let wood = Source {
            forest_metadata: vec![0.0, 12.0, 0.0],
            forest_trunk_ranges: vec![0, 0],
            forest_rect_ids: vec![0],
            forests: vec![0.0, 40.0, 100.0, 20.0, 0.0],
            ..empty()
        };
        assert!(same(&split(&wood), &split(&empty())));
    }

    #[test]
    fn a_road_polygon_guides_by_its_ring_never_its_triangulation_diagonal() {
        let polygon = Source {
            surface_strokes: vec![],
            surface_runs: vec![],
            ..paving("road")
        };
        let fields = split(&polygon);
        assert_eq!(at(&fields, 50.0, 50.0).1, 10.0);
        assert_eq!(at(&fields, 50.0, 40.0).1, 0.0);
        assert_ne!(at(&fields, 50.0, 30.0).0, at(&fields, 50.0, 50.0).0);
        assert_ne!(at(&fields, 50.0, 70.0).0, at(&fields, 50.0, 50.0).0);
    }
}
