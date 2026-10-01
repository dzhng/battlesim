//! Shared physical ground geometry. Admission, closed membership and polygon
//! triangles are owned here so map compilers and the simulation agree.
use crate::curve::Centerline;
use serde::{ser::SerializeStruct, Deserialize, Deserializer, Serialize, Serializer};

pub const MAX_POLYGON_VERTICES: usize = 256;
pub const MAX_STROKE_CONTROLS: usize = 4096;

#[derive(Clone, Debug)]
pub enum GroundShape {
    /// Closed simple ring, either winding, with no repeated endpoint.
    Polygon { ring: Vec<[f64; 2]> },
    /// Shared sampled centreline; physical membership is a union of closed capsules.
    Stroke {
        centerline: Centerline,
        width_m: f64,
    },
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum ShapeKind {
    Polygon,
    Stroke,
}

#[derive(Deserialize)]
struct ShapeKindWire {
    kind: ShapeKind,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PolygonWire {
    #[serde(rename = "kind")]
    _kind: ShapeKind,
    #[serde(deserialize_with = "crate::numbers::points")]
    ring: Vec<[f64; 2]>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StrokeWire {
    #[serde(rename = "kind")]
    _kind: ShapeKind,
    #[serde(deserialize_with = "crate::numbers::points")]
    points: Vec<[f64; 2]>,
    #[serde(deserialize_with = "crate::numbers::scalar")]
    width_m: f64,
}

impl<'de> Deserialize<'de> for GroundShape {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        // Internally tagged Serde enums buffer numbers before physical readers
        // see them. Keep the source tokens through the one shape envelope.
        let raw = Box::<serde_json::value::RawValue>::deserialize(decoder)?;
        let kind: ShapeKindWire =
            serde_json::from_str(raw.get()).map_err(serde::de::Error::custom)?;
        let shape = match kind.kind {
            ShapeKind::Polygon => {
                let wire: PolygonWire =
                    serde_json::from_str(raw.get()).map_err(serde::de::Error::custom)?;
                Self::polygon(wire.ring)
            }
            ShapeKind::Stroke => {
                let wire: StrokeWire =
                    serde_json::from_str(raw.get()).map_err(serde::de::Error::custom)?;
                Self::stroke(wire.points, wire.width_m)
            }
        };
        shape.map_err(serde::de::Error::custom)
    }
}

impl Serialize for GroundShape {
    fn serialize<S: Serializer>(&self, encoder: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Polygon { ring } => {
                let mut record = encoder.serialize_struct("GroundShape", 2)?;
                record.serialize_field("kind", "polygon")?;
                record.serialize_field("ring", ring)?;
                record.end()
            }
            Self::Stroke {
                centerline,
                width_m,
            } => {
                let mut record = encoder.serialize_struct("GroundShape", 3)?;
                record.serialize_field("kind", "stroke")?;
                record.serialize_field("points", centerline.control_points())?;
                record.serialize_field("width_m", width_m)?;
                record.end()
            }
        }
    }
}

impl GroundShape {
    pub fn polygon(ring: Vec<[f64; 2]>) -> Result<Self, String> {
        validate_ring(&ring)?;
        Ok(Self::Polygon { ring })
    }
    pub fn stroke(points: Vec<[f64; 2]>, width_m: f64) -> Result<Self, String> {
        if points.len() < 2 || points.len() > MAX_STROKE_CONTROLS {
            return Err("ground stroke requires 2..=4096 control points".into());
        }
        if !width_m.is_finite() || width_m < 0.0 || !points.iter().flatten().all(|v| v.is_finite())
        {
            return Err("ground stroke requires finite points and nonnegative finite width".into());
        }
        let centerline = Centerline::new(points);
        let half = width_m / 2.0;
        let samples = centerline.samples();
        if !(half as f32).is_finite()
            || !samples.iter().flatten().all(|v| (*v as f32).is_finite())
            || !limits(samples, half).iter().all(|v| v.is_finite())
            || !samples.windows(2).all(|pair| {
                let dx = pair[1][0] - pair[0][0];
                let dy = pair[1][1] - pair[0][1];
                let length_squared = dx * dx + dy * dy;
                length_squared.is_finite() && length_squared > 0.0
            })
        {
            return Err("ground stroke arithmetic or exports cannot be represented".into());
        }
        Ok(Self::Stroke {
            centerline,
            width_m,
        })
    }

    /// Closed physical membership; a positive margin grows its edge.
    pub fn contains(&self, point: [f64; 2], margin: f64) -> bool {
        match self {
            Self::Polygon { ring } => {
                polygon_contains(ring, point)
                    || (margin > 0.0
                        && edges(ring).any(|(a, b)| segment_distance(*a, *b, point) <= margin))
            }
            Self::Stroke {
                centerline,
                width_m,
            } => centerline
                .samples()
                .windows(2)
                .any(|pair| segment_distance(pair[0], pair[1], point) <= width_m / 2.0 + margin),
        }
    }

    /// Actual maxima keep closed-boundary queries conservative despite subtraction rounding.
    pub fn limits(&self) -> [f64; 4] {
        match self {
            Self::Polygon { ring } => limits(ring, 0.0),
            Self::Stroke {
                centerline,
                width_m,
            } => limits(centerline.samples(), width_m / 2.0),
        }
    }

    pub fn bounds(&self) -> [f64; 4] {
        let [x, y, max_x, max_y] = self.limits();
        [x, y, max_x - x, max_y - y]
    }

    /// Only actual four-corner rectangles use the original forest lattice and
    /// renderer rectangle arithmetic. Bounds are never substituted for a shape.
    pub fn exact_rectangle(&self) -> Option<[f64; 4]> {
        let Self::Polygon { ring } = self else {
            return None;
        };
        if ring.len() != 4 {
            return None;
        }
        let [x, y, max_x, max_y] = self.limits();
        let corners = [[x, y], [max_x, y], [max_x, max_y], [x, max_y]];
        if ring.iter().all(|p| corners.contains(p))
            && (0..4).all(|i| {
                (ring[i][0] == ring[(i + 1) % 4][0]) != (ring[i][1] == ring[(i + 1) % 4][1])
            })
        {
            Some([x, y, max_x - x, max_y - y])
        } else {
            None
        }
    }
}

pub fn edges(ring: &[[f64; 2]]) -> impl Iterator<Item = (&[f64; 2], &[f64; 2])> {
    ring.iter()
        .zip(ring.iter().cycle().skip(1))
        .take(ring.len())
}
pub fn cross(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}
pub fn polygon_area(ring: &[[f64; 2]]) -> f64 {
    (1..ring.len() - 1)
        .map(|i| cross(ring[0], ring[i], ring[i + 1]))
        .sum()
}
pub fn limits(points: &[[f64; 2]], margin: f64) -> [f64; 4] {
    let min = [0, 1].map(|i| points.iter().map(|p| p[i]).fold(f64::INFINITY, f64::min) - margin);
    let max = [0, 1].map(|i| {
        points
            .iter()
            .map(|p| p[i])
            .fold(f64::NEG_INFINITY, f64::max)
            + margin
    });
    [min[0], min[1], max[0], max[1]]
}
pub fn polygon_contains(ring: &[[f64; 2]], p: [f64; 2]) -> bool {
    let mut inside = false;
    for (a, b) in edges(ring) {
        let turn = cross(*a, *b, p);
        if turn == 0.0 && on_segment(*a, *b, p) {
            return true;
        }
        if (a[1] > p[1]) != (b[1] > p[1])
            && p[0] < a[0] + (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1])
        {
            inside = !inside;
        }
    }
    inside
}
pub fn segment_distance(a: [f64; 2], b: [f64; 2], p: [f64; 2]) -> f64 {
    let ab = [b[0] - a[0], b[1] - a[1]];
    let len2 = ab[0] * ab[0] + ab[1] * ab[1];
    let t = if len2 > 0.0 {
        ((p[0] - a[0]) * ab[0] + (p[1] - a[1]) * ab[1]) / len2
    } else {
        0.0
    }
    .clamp(0.0, 1.0);
    let delta = [p[0] - (a[0] + ab[0] * t), p[1] - (a[1] + ab[1] * t)];
    delta[0].hypot(delta[1])
}
fn on_segment(a: [f64; 2], b: [f64; 2], p: [f64; 2]) -> bool {
    (0..2).all(|k| p[k] >= a[k].min(b[k]) && p[k] <= a[k].max(b[k]))
}
pub fn validate_ring(ring: &[[f64; 2]]) -> Result<(), String> {
    let n = ring.len();
    if !(3..=MAX_POLYGON_VERTICES).contains(&n) {
        return Err("ground polygon requires 3..=256 vertices".into());
    }
    if !ring.iter().flatten().all(|v| v.is_finite()) {
        return Err("ground polygon vertices must be finite".into());
    }
    for i in 0..n {
        if ring[i] == ring[(i + 1) % n] {
            return Err("ground polygon has a repeated adjacent vertex".into());
        }
        for j in i + 1..n {
            if j == i + 1 || (i == 0 && j == n - 1) {
                continue;
            }
            let (a, b, c, d) = (ring[i], ring[(i + 1) % n], ring[j], ring[(j + 1) % n]);
            let turns = [
                cross(a, b, c),
                cross(a, b, d),
                cross(c, d, a),
                cross(c, d, b),
            ];
            if !turns.iter().all(|v| v.is_finite()) {
                return Err("ground polygon arithmetic must be finite".into());
            }
            let intersects =
                turns[0].signum() != turns[1].signum() && turns[2].signum() != turns[3].signum();
            if intersects
                || (turns[0] == 0.0 && on_segment(a, b, c))
                || (turns[1] == 0.0 && on_segment(a, b, d))
                || (turns[2] == 0.0 && on_segment(c, d, a))
                || (turns[3] == 0.0 && on_segment(c, d, b))
            {
                return Err("ground polygon must be simple".into());
            }
        }
    }
    let area = polygon_area(ring);
    if !area.is_finite() || area == 0.0 {
        return Err("ground polygon must have finite nonzero area".into());
    }
    Ok(())
}

/// Deterministic ear clipping, preserving authored vertex order and winding.
pub fn triangulate(ring: &[[f64; 2]]) -> Result<Vec<[[f64; 2]; 3]>, String> {
    validate_ring(ring)?;
    let winding = polygon_area(ring).signum();
    let mut remaining: Vec<usize> = (0..ring.len()).collect();
    loop {
        let removable = (0..remaining.len()).find(|&i| {
            let n = remaining.len();
            cross(
                ring[remaining[(i + n - 1) % n]],
                ring[remaining[i]],
                ring[remaining[(i + 1) % n]],
            ) == 0.0
        });
        let Some(i) = removable else { break };
        if remaining.len() <= 3 {
            return Err("ground polygon is degenerate".into());
        }
        remaining.remove(i);
    }
    let mut out = Vec::with_capacity(remaining.len() - 2);
    while remaining.len() >= 3 {
        let n = remaining.len();
        let ear = (0..n)
            .find(|&i| {
                let (a, b, c) = (
                    remaining[(i + n - 1) % n],
                    remaining[i],
                    remaining[(i + 1) % n],
                );
                cross(ring[a], ring[b], ring[c]) * winding > 0.0
                    && remaining.iter().all(|&p| {
                        p == a
                            || p == b
                            || p == c
                            || [
                                cross(ring[a], ring[b], ring[p]),
                                cross(ring[b], ring[c], ring[p]),
                                cross(ring[c], ring[a], ring[p]),
                            ]
                            .iter()
                            .any(|v| *v * winding < 0.0)
                    })
            })
            .ok_or("ground polygon cannot be triangulated")?;
        out.push([
            ring[remaining[(ear + n - 1) % n]],
            ring[remaining[ear]],
            ring[remaining[(ear + 1) % n]],
        ]);
        remaining.remove(ear);
        if n == 3 {
            break;
        }
    }
    Ok(out)
}
