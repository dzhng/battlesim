//! A river: one rounded centreline whose water width and depth vary along it.
//!
//! Its cross-section is a V. The bed falls from the waterline to `depth_m` at
//! the centreline, and the bank climbs away from the waterline at that same
//! grade until it has made up the height the land stands above the water at
//! the edge. So one number, how far a point lies inside the water's edge,
//! gives the membership every rule reads, the height the terrain is carved
//! to and the edge the drawing cuts the water at.
//! The river's own arithmetic is `+ − × ÷` and `libm`, so native and Wasm
//! agree on it bit for bit; the land's height comes from `Relief`.
use crate::curve::Centerline;
use crate::ground::{MAX_STROKE_CONTROLS, MAX_STROKE_SAMPLES};
use crate::map::MapDefinition;
use serde::{ser::SerializeStruct, Deserialize, Deserializer, Serialize, Serializer};

/// A river is at least this many height samples wide, or the grid cannot
/// carve its bed: a narrower channel falls between samples.
pub const MIN_WIDTH_CELLS: f64 = 3.0;
/// A surface whose grade is `g` everywhere is drawn by grid triangles no
/// steeper than `g √2` (a triangle's rise along each axis is one sample's).
/// Banks keep this share of the grade that would reach the slope cutoff.
const GRID_GRADE_MARGIN: f64 = 0.9;

/// One authored point: where the centreline runs, how wide the water is
/// there and how deep at its middle.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RiverPoint {
    #[serde(deserialize_with = "crate::numbers::array")]
    pub xy: [f64; 2],
    #[serde(deserialize_with = "crate::numbers::scalar")]
    pub width_m: f64,
    #[serde(deserialize_with = "crate::numbers::scalar")]
    pub depth_m: f64,
}

/// One point of the rounded line every consumer reads.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RiverSample {
    pub xy: [f64; 2],
    pub half_width_m: f64,
    pub depth_m: f64,
}

#[derive(Clone, Debug)]
pub struct River {
    points: Vec<RiverPoint>,
    samples: Vec<RiverSample>,
    surface_z: f64,
}

/// Where a point sits across one stretch of a river.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Section {
    /// How far inside the water's edge, in metres; negative on the bank.
    pub inside_m: f64,
    /// The fall of the bed and the bank across the stream: depth over half width.
    pub grade: f64,
    /// The point of the water's edge nearest it.
    pub edge: [f64; 2],
}

impl Section {
    /// The cross-section's height here under a water surface at `surface_z`,
    /// its bed and bank no gentler than `least_grade`. Both sides of the
    /// waterline keep one grade, so the V is a plane through it.
    pub fn height(&self, surface_z: f64, least_grade: f64) -> f64 {
        surface_z - self.grade.max(least_grade) * self.inside_m
    }

    /// The height of land that stood `natural` high, once this section is
    /// cut into it: the cross-section in the water; on the bank the land,
    /// less however far the cross-section still lies below the land at the
    /// water's edge (`land_at` the section's `edge`).
    pub fn carved(
        &self,
        natural: f64,
        surface_z: f64,
        least_grade: f64,
        land_at: impl FnOnce([f64; 2]) -> f64,
    ) -> f64 {
        let height = self.height(surface_z, least_grade);
        if self.inside_m >= 0.0 {
            natural.min(height)
        } else {
            natural - (land_at(self.edge) - height).max(0.0)
        }
    }
}

/// The water's two edges across the stretch `a`–`b`, the share `t` of the
/// way along it.
pub fn edges(a: &RiverSample, b: &RiverSample, t: f64) -> [[f64; 2]; 2] {
    let run = [b.xy[0] - a.xy[0], b.xy[1] - a.xy[1]];
    let length = libm::sqrt(run[0] * run[0] + run[1] * run[1]);
    let across = [-run[1] / length, run[0] / length];
    let at = [a.xy[0] + run[0] * t, a.xy[1] + run[1] * t];
    let half = a.half_width_m + (b.half_width_m - a.half_width_m) * t;
    [-1.0, 1.0].map(|side| {
        [
            at[0] + across[0] * half * side,
            at[1] + across[1] * half * side,
        ]
    })
}

/// The cross-section at the point of the stretch `a`–`b` closest to `p`:
/// width and depth run linearly between the two samples.
pub fn section(a: &RiverSample, b: &RiverSample, p: [f64; 2]) -> Section {
    let ab = [b.xy[0] - a.xy[0], b.xy[1] - a.xy[1]];
    let len2 = ab[0] * ab[0] + ab[1] * ab[1];
    let t = if len2 > 0.0 {
        ((p[0] - a.xy[0]) * ab[0] + (p[1] - a.xy[1]) * ab[1]) / len2
    } else {
        0.0
    }
    .clamp(0.0, 1.0);
    let closest = [a.xy[0] + ab[0] * t, a.xy[1] + ab[1] * t];
    let delta = [p[0] - closest[0], p[1] - closest[1]];
    let distance = libm::sqrt(delta[0] * delta[0] + delta[1] * delta[1]);
    let half = a.half_width_m + (b.half_width_m - a.half_width_m) * t;
    let depth = a.depth_m + (b.depth_m - a.depth_m) * t;
    // On the centreline itself either edge is as near: take the left one.
    let outward = if distance > 0.0 {
        [delta[0] / distance, delta[1] / distance]
    } else {
        let length = libm::sqrt(len2);
        [-ab[1] / length, ab[0] / length]
    };
    Section {
        inside_m: half - distance,
        grade: depth / half,
        edge: [
            closest[0] + outward[0] * half,
            closest[1] + outward[1] * half,
        ],
    }
}

/// The steepest grade a carved surface may take and still be drawn by
/// triangles under the slope cutoff.
pub fn steepest_grade(slope_cutoff_deg: f64) -> f64 {
    GRID_GRADE_MARGIN * libm::tan(slope_cutoff_deg.to_radians()) / std::f64::consts::SQRT_2
}

impl River {
    pub fn new(points: Vec<RiverPoint>, surface_z: f64) -> Result<Self, String> {
        if points.len() < 2 || points.len() > MAX_STROKE_CONTROLS {
            return Err("river requires 2..=4096 points".into());
        }
        let sound = |p: &RiverPoint| {
            p.xy.iter().all(|v| v.is_finite())
                && p.width_m.is_finite()
                && p.width_m > 0.0
                && p.depth_m.is_finite()
                && p.depth_m > 0.0
        };
        if !surface_z.is_finite() || !points.iter().all(sound) {
            return Err(
                "river requires finite points, positive widths and depths and a finite surface"
                    .into(),
            );
        }
        // A bend stays inside the narrowest water it is authored through.
        let line = Centerline::new(
            points.iter().map(|p| p.xy).collect(),
            narrowest_width(&points) / 2.0,
            MAX_STROKE_SAMPLES,
        )?;
        let samples = along(&points, line.samples())?;
        if !samples.iter().all(|s| {
            [s.xy[0], s.xy[1], s.half_width_m, s.depth_m]
                .iter()
                .all(|v| (*v as f32).is_finite())
        }) || !(surface_z as f32).is_finite()
        {
            return Err("river exports cannot be represented".into());
        }
        Ok(Self {
            points,
            samples,
            surface_z,
        })
    }

    /// The authored points: what a field is cut along.
    pub fn points(&self) -> &[RiverPoint] {
        &self.points
    }

    /// The rounded line, with the width and depth at each of its points.
    pub fn samples(&self) -> &[RiverSample] {
        &self.samples
    }

    pub fn surface_z(&self) -> f64 {
        self.surface_z
    }

    /// The narrowest water it is authored through.
    pub fn narrowest_width_m(&self) -> f64 {
        narrowest_width(&self.points)
    }

    /// Every stretch's cross-section at `p`.
    fn sections(&self, p: [f64; 2]) -> impl Iterator<Item = Section> + '_ {
        self.samples
            .windows(2)
            .map(move |pair| section(&pair[0], &pair[1], p))
    }

    /// How far `p` lies inside the water's edge: the deepest any stretch
    /// puts it, negative on land. Water is the closed set `inside >= 0`.
    pub fn inside(&self, p: [f64; 2]) -> f64 {
        self.sections(p)
            .map(|s| s.inside_m)
            .fold(f64::NEG_INFINITY, f64::max)
    }

    /// The lowest any stretch's cross-section stands at `p`, no gentler
    /// than `least_grade`: the bed in the water, and on the bank how high the
    /// river's grade has climbed from the waterline.
    pub fn height_at(&self, p: [f64; 2], least_grade: f64) -> f64 {
        self.sections(p)
            .map(|s| s.height(self.surface_z, least_grade))
            .fold(f64::INFINITY, f64::min)
    }
}

fn narrowest_width(points: &[RiverPoint]) -> f64 {
    points
        .iter()
        .map(|p| p.width_m)
        .fold(f64::INFINITY, f64::min)
}

/// Width and depth at each rounded sample: linear in the distance run along
/// the rounded line between the two authored points either side.
fn along(points: &[RiverPoint], line: &[[f64; 2]]) -> Result<Vec<RiverSample>, String> {
    let mut samples = Vec::with_capacity(line.len());
    let mut start = 0;
    for pair in points.windows(2) {
        // The rounded line passes through every authored point, exactly.
        let end = (start + 1..line.len())
            .find(|&i| line[i] == pair[1].xy)
            .ok_or("river centreline lost an authored point")?;
        let mut run = Vec::with_capacity(end - start + 1);
        let mut total = 0.0;
        for i in start..end {
            run.push(total);
            let d = [line[i + 1][0] - line[i][0], line[i + 1][1] - line[i][1]];
            total += libm::sqrt(d[0] * d[0] + d[1] * d[1]);
        }
        for (i, at) in (start..end).zip(run) {
            let u = at / total;
            samples.push(RiverSample {
                xy: line[i],
                half_width_m: (pair[0].width_m + (pair[1].width_m - pair[0].width_m) * u) / 2.0,
                depth_m: pair[0].depth_m + (pair[1].depth_m - pair[0].depth_m) * u,
            });
        }
        start = end;
    }
    let last = points[points.len() - 1];
    samples.push(RiverSample {
        xy: last.xy,
        half_width_m: last.width_m / 2.0,
        depth_m: last.depth_m,
    });
    Ok(samples)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RiverWire {
    points: Vec<RiverPoint>,
    #[serde(deserialize_with = "crate::numbers::scalar")]
    surface_z: f64,
}

impl<'de> Deserialize<'de> for River {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        let wire = RiverWire::deserialize(decoder)?;
        Self::new(wire.points, wire.surface_z).map_err(serde::de::Error::custom)
    }
}

impl Serialize for River {
    fn serialize<S: Serializer>(&self, encoder: S) -> Result<S::Ok, S::Error> {
        let mut record = encoder.serialize_struct("River", 2)?;
        record.serialize_field("points", &self.points)?;
        record.serialize_field("surface_z", &self.surface_z)?;
        record.end()
    }
}

/// What a map's water has to be for the terrain to carry it:
/// - every authored point at least [`MIN_WIDTH_CELLS`] height samples wide;
/// - no bank steeper than [`steepest_grade`]: the grade is depth over half
///   width, so a point is refused for being too deep for its width;
/// - the surface no higher than the land at the water's edge, checked every
///   height sample along both edges, or the river would stand above its own
///   bank;
/// - each bridge end on dry ground, far enough from the water that a bank
///   as steep as [`steepest_grade`] leaves the ground there uncut, or cut no
///   lower than the deck.
pub fn validate(map: &MapDefinition) -> Result<(), String> {
    let steepest = steepest_grade(map.slope_cutoff_deg);
    let narrowest = MIN_WIDTH_CELLS * map.height_grid_m;
    for (r, river) in map.rivers.iter().enumerate() {
        for (i, point) in river.points.iter().enumerate() {
            if point.width_m < narrowest {
                return Err(format!(
                    "rivers[{r}].points[{i}] is {} m wide; the {} m height grid carves no river under {narrowest} m",
                    point.width_m, map.height_grid_m
                ));
            }
            let grade = point.depth_m / (point.width_m / 2.0);
            if grade > steepest {
                return Err(format!(
                    "rivers[{r}].points[{i}] is {} m deep in {} m of water: its bank would stand steeper than the map's {}° slope cutoff allows",
                    point.depth_m, point.width_m, map.slope_cutoff_deg
                ));
            }
        }
        for pair in river.samples.windows(2) {
            let run = [pair[1].xy[0] - pair[0].xy[0], pair[1].xy[1] - pair[0].xy[1]];
            let length = libm::sqrt(run[0] * run[0] + run[1] * run[1]);
            let steps = libm::ceil(length / map.height_grid_m).max(1.0);
            for step in 0..=steps as usize {
                for edge in edges(&pair[0], &pair[1], step as f64 / steps) {
                    if map.relief_height(edge[0], edge[1]) < river.surface_z {
                        return Err(format!(
                            "rivers[{r}] has its surface at {} m, above its bank at ({}, {})",
                            river.surface_z, edge[0], edge[1]
                        ));
                    }
                }
            }
        }
    }
    for (b, bridge) in map.bridges.iter().enumerate() {
        for end in bridge.ends() {
            let middle = [(end[0][0] + end[1][0]) / 2.0, (end[0][1] + end[1][1]) / 2.0];
            let wet = map.rivers.iter().any(|r| r.inside(middle) >= 0.0);
            let land = map.relief_height(middle[0], middle[1]);
            let carved = map
                .rivers
                .iter()
                .flat_map(|r| {
                    r.sections(middle).map(move |s| {
                        s.carved(land, r.surface_z, steepest, |edge| {
                            map.relief_height(edge[0], edge[1])
                        })
                    })
                })
                .fold(land, f64::min);
            if wet || carved < land.min(bridge.deck_z) {
                return Err(format!(
                    "bridges[{b}] ends at ({}, {}), too near the water for a ramp to reach its deck",
                    middle[0], middle[1]
                ));
            }
        }
    }
    Ok(())
}
