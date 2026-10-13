//! The map's forests as shapes: each forest's primitives, as the simulation
//! exports them grouped by forest id, gathered under its metadata row.
use super::shapes::{closest, distance, stroke_inside, triangle_contains};
use super::{chunks, Source, AREA, FOREST_BOUNDARY, FOREST_STROKE, FOREST_TRIANGLE};
use std::collections::HashMap;
use std::ops::Range;

/// In the layout's `forestShapeKinds` order.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Kind {
    Rectangle = 0,
    Stroke = 1,
    Polygon = 2,
}

pub struct Forest {
    pub canopy: f32,
    /// Its trunks' source prop ids, `[first, one past)`.
    pub trunks: [u32; 2],
    pub kind: Kind,
    /// A rectangle's `x, y, w, h`; only rectangles have one.
    pub rect: Option<[f32; 4]>,
    /// Its records in the export's forest streams, as float ranges.
    pub strokes: Range<usize>,
    pub triangles: Range<usize>,
    pub boundaries: Range<usize>,
}

impl Forest {
    /// How far `(x, y)` lies inside the forest's own primitives in `s`
    /// (negative outside): a rectangle exactly, a strip ending square, a
    /// polygon by its membership triangles and its ring's edges.
    pub fn inside(&self, s: &Source, x: f64, y: f64) -> f64 {
        if let Some([sx, sy, w, h]) = self.rect.map(|r| r.map(f64::from)) {
            return (x - sx).min(sx + w - x).min(y - sy).min(sy + h - y);
        }
        if self.kind == Kind::Stroke {
            let strokes = chunks::<FOREST_STROKE>(&s.forest_strokes[self.strokes.clone()]);
            return strokes
                .iter()
                .fold(-1e9, |d, r| d.max(stroke_inside(r, [x, y])));
        }
        let pt = |r: &[f32], k: usize| [r[k * 2] as f64, r[k * 2 + 1] as f64];
        let triangles = chunks::<FOREST_TRIANGLE>(&s.forest_triangles[self.triangles.clone()]);
        let inside = triangles
            .iter()
            .any(|t| triangle_contains(pt(t, 0), pt(t, 1), pt(t, 2), [x, y]));
        let edges = chunks::<FOREST_BOUNDARY>(&s.forest_boundaries[self.boundaries.clone()]);
        let nearest = edges.iter().fold(1e9f64, |n, e| {
            n.min(distance([x, y], closest([x, y], pt(e, 0), pt(e, 1))))
        });
        if inside {
            nearest
        } else {
            -nearest
        }
    }
}

type Groups = HashMap<u32, Range<usize>>;

/// Each run of records in `records` sharing the id at `id_at`, by id.
fn groups(records: &[f32], stride: usize, id_at: usize) -> Result<Groups, String> {
    let mut out = HashMap::new();
    let mut first = 0;
    while first < records.len() {
        let id = records[first + id_at];
        let mut end = first + stride;
        while end < records.len() && records[end + id_at] == id {
            end += stride;
        }
        if out.insert(id as u32, first..end).is_some() {
            return Err("forest primitive groups must be contiguous".into());
        }
        first = end;
    }
    Ok(out)
}

pub fn forests(s: &Source) -> Result<Vec<Forest>, String> {
    let rects: HashMap<u32, [f32; 4]> = (s.forest_rect_ids.iter().enumerate())
        .map(|(r, &id)| (id, s.forests[r * AREA..r * AREA + 4].try_into().unwrap()))
        .collect();
    let strokes = groups(&s.forest_strokes, FOREST_STROKE, 5)?;
    let triangles = groups(&s.forest_triangles, FOREST_TRIANGLE, 6)?;
    let boundaries = groups(&s.forest_boundaries, FOREST_BOUNDARY, 4)?;
    let forest = |m: &[f32; 3]| {
        let id = m[0] as u32;
        let kind = [Kind::Rectangle, Kind::Stroke, Kind::Polygon][m[2] as usize];
        let rect = rects.get(&id).copied();
        if kind == Kind::Rectangle && rect.is_none() {
            return Err("rectangular forest has no native rectangle".to_string());
        }
        let range = id as usize * 2;
        if range + 1 >= s.forest_trunk_ranges.len() {
            return Err("forest has no native source trunk range".into());
        }
        let of = |g: &Groups| g.get(&id).cloned().unwrap_or(0..0);
        let (canopy, trunks) = (m[1], [range, range + 1].map(|k| s.forest_trunk_ranges[k]));
        let (strokes, triangles, boundaries) = (of(&strokes), of(&triangles), of(&boundaries));
        Ok(Forest {
            canopy,
            trunks,
            kind,
            rect,
            strokes,
            triangles,
            boundaries,
        })
    };
    chunks::<3>(&s.forest_metadata).iter().map(forest).collect()
}

/// The long stretches of every strip of forest (`ax, ay, bx, by` each), which
/// fields are cut along so a tree line stands between two; a bend's chords,
/// far shorter than the strip is wide, are left out.
pub fn strip_runs(s: &Source, forests: &[Forest]) -> Vec<f32> {
    let mut runs = vec![];
    for f in forests.iter().filter(|f| f.kind == Kind::Stroke) {
        for r in chunks::<FOREST_STROKE>(&s.forest_strokes[f.strokes.clone()]) {
            let [ax, ay, bx, by, half] = [r[0], r[1], r[2], r[3], r[4]].map(f64::from);
            if libm::hypot(bx - ax, by - ay) > 2.0 * half {
                runs.extend_from_slice(&r[..4]);
            }
        }
    }
    runs
}
