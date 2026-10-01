//! Plane geometry for layout. Arithmetic is `+ − × ÷` and `libm`, as in
//! `contract::curve`, so native and Wasm place every point identically.
use super::presets::OutlineShape;
use super::rng::Stream;

pub type Point = [f64; 2];
pub const PI: f64 = core::f64::consts::PI;
pub const TAU: f64 = core::f64::consts::TAU;

pub fn add(a: Point, b: Point) -> Point {
    [a[0] + b[0], a[1] + b[1]]
}
pub fn sub(a: Point, b: Point) -> Point {
    [a[0] - b[0], a[1] - b[1]]
}
pub fn scale(a: Point, factor: f64) -> Point {
    [a[0] * factor, a[1] * factor]
}
pub fn cross(a: Point, b: Point) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}
pub fn length(a: Point) -> f64 {
    libm::sqrt(a[0] * a[0] + a[1] * a[1])
}
pub fn distance(a: Point, b: Point) -> f64 {
    length(sub(a, b))
}
pub fn direction(angle: f64) -> Point {
    [libm::cos(angle), libm::sin(angle)]
}
pub fn bearing(from: Point, to: Point) -> f64 {
    libm::atan2(to[1] - from[1], to[0] - from[0])
}

/// Plan coordinates are whole centimetres: short in JSON and far below
/// anything a parcel or a tree can tell apart.
pub fn round_cm(point: Point) -> Point {
    point.map(|v| libm::round(v * 100.0) / 100.0)
}

pub fn area(ring: &[Point]) -> f64 {
    let twice: f64 = (0..ring.len())
        .map(|i| cross(ring[i], ring[(i + 1) % ring.len()]))
        .sum();
    (twice / 2.0).abs()
}

/// Area of the part of `ring` with `y ≥ line`.
pub fn area_above(ring: &[Point], line: f64) -> f64 {
    let mut kept = Vec::with_capacity(ring.len() + 2);
    for i in 0..ring.len() {
        let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
        if a[1] >= line {
            kept.push(a);
        }
        if (a[1] >= line) != (b[1] >= line) {
            let t = (line - a[1]) / (b[1] - a[1]);
            kept.push([a[0] + (b[0] - a[0]) * t, line]);
        }
    }
    if kept.len() < 3 {
        0.0
    } else {
        area(&kept)
    }
}

pub fn segment_distance(a: Point, b: Point, p: Point) -> f64 {
    let ab = sub(b, a);
    let len2 = ab[0] * ab[0] + ab[1] * ab[1];
    let t = if len2 > 0.0 {
        (((p[0] - a[0]) * ab[0] + (p[1] - a[1]) * ab[1]) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    distance(p, add(a, scale(ab, t)))
}

/// Distance from `p` to a filled ring; zero inside it.
pub fn ring_distance(ring: &[Point], p: Point) -> f64 {
    if contract::ground::polygon_contains(ring, p) {
        return 0.0;
    }
    contract::ground::edges(ring)
        .map(|(a, b)| segment_distance(*a, *b, p))
        .fold(f64::INFINITY, f64::min)
}

/// Distances along the ray `origin + t × direction` (unit `direction`) at
/// which it crosses the ring's edges.
pub fn ray_crossings<'a>(
    origin: Point,
    direction: Point,
    ring: &'a [Point],
) -> impl Iterator<Item = f64> + 'a {
    contract::ground::edges(ring).filter_map(move |(a, b)| {
        let edge = sub(*b, *a);
        let den = cross(direction, edge);
        if den == 0.0 {
            return None;
        }
        let to_a = sub(*a, origin);
        let t = cross(to_a, edge) / den;
        let u = cross(to_a, direction) / den;
        (t >= 0.0 && (0.0..=1.0).contains(&u)).then_some(t)
    })
}

/// Where segments `ab` and `cd` cross, as shares along each.
pub fn segment_crossing(a: Point, b: Point, c: Point, d: Point) -> Option<(f64, f64)> {
    let (r, s) = (sub(b, a), sub(d, c));
    let den = cross(r, s);
    if den == 0.0 {
        return None;
    }
    let t = cross(sub(c, a), s) / den;
    let u = cross(sub(c, a), r) / den;
    ((0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u)).then_some((t, u))
}

/// The smallest turn between two bearings, in `[0, π]`.
pub fn turn(a: f64, b: f64) -> f64 {
    let mut delta = (a - b) % TAU;
    if delta < 0.0 {
        delta += TAU;
    }
    if delta > PI {
        TAU - delta
    } else {
        delta
    }
}

/// An irregular closed outline, star-shaped about its centre: a superellipse
/// whose radius a few low harmonics push in and out. Settlements and woods
/// are both this shape.
#[derive(Clone, Debug)]
pub struct Outline {
    pub center: Point,
    pub ring: Vec<Point>,
    /// Farthest vertex from the centre.
    pub reach: f64,
}

impl Outline {
    /// Draw an outline of exactly `area_m2` about the origin. `aspect` is the
    /// short axis over the long one; `rotation` turns the long axis.
    pub fn draw(
        shape: OutlineShape,
        area_m2: f64,
        aspect: f64,
        rotation: f64,
        rng: &mut Stream,
    ) -> Self {
        let harmonics = [2.0, 3.0, 4.0, 5.0].map(|k| {
            let amplitude = shape.noise / k * rng.range([0.5, 1.0]);
            (amplitude, k, rng.range([0.0, TAU]))
        });
        let count = shape.points as usize;
        let unit: Vec<Point> = (0..count)
            .map(|i| {
                let angle = TAU * i as f64 / count as f64;
                let (c, s) = (libm::cos(angle), libm::sin(angle));
                let base = libm::pow(
                    libm::pow(c.abs(), shape.exponent)
                        + libm::pow(s.abs() / aspect, shape.exponent),
                    -1.0 / shape.exponent,
                );
                let wobble: f64 = harmonics
                    .iter()
                    .map(|(amplitude, k, phase)| amplitude * libm::cos(k * angle + phase))
                    .sum();
                scale(direction(angle + rotation), base * (1.0 + wobble))
            })
            .collect();
        let grow = libm::sqrt(area_m2 / area(&unit));
        let ring: Vec<Point> = unit.into_iter().map(|p| scale(p, grow)).collect();
        let reach = ring.iter().map(|p| length(*p)).fold(0.0, f64::max);
        Self {
            center: [0.0, 0.0],
            ring,
            reach,
        }
    }

    /// The outline moved to `center`, on the centimetre grid.
    pub fn at(&self, center: Point) -> Self {
        let center = round_cm(center);
        Self {
            center,
            ring: self
                .ring
                .iter()
                .map(|p| round_cm(add(*p, center)))
                .collect(),
            reach: self.reach,
        }
    }

    /// Pull every vertex in along its own ray until it lies inside the
    /// playable square, which keeps the outline star-shaped and so simple.
    pub fn clipped(&self, extent: f64) -> Self {
        let ring = self
            .ring
            .iter()
            .map(|vertex| {
                let ray = sub(*vertex, self.center);
                let mut keep: f64 = 1.0;
                for (along, from) in ray.into_iter().zip(self.center) {
                    if along > 0.0 {
                        keep = keep.min((extent - from) / along);
                    } else if along < 0.0 {
                        keep = keep.min(-from / along);
                    }
                }
                round_cm(add(self.center, scale(ray, keep))).map(|v| v.clamp(0.0, extent))
            })
            .collect();
        Self {
            center: self.center,
            ring,
            reach: self.reach,
        }
    }

    /// Distance from the centre to the outline's edge along `angle`.
    pub fn edge(&self, angle: f64) -> f64 {
        ray_crossings(self.center, direction(angle), &self.ring).fold(0.0, f64::max)
    }

    /// Open ground between this outline and a circle of `radius` about `p`.
    pub fn gap_to(&self, p: Point, radius: f64) -> f64 {
        ring_distance(&self.ring, p) - radius
    }
}

/// Buckets of item indices over the map, for "what is near this box".
pub struct Grid {
    cell: f64,
    columns: usize,
    rows: usize,
    buckets: Vec<Vec<u32>>,
}

impl Grid {
    pub fn new(size: [f64; 2], cell: f64) -> Self {
        let count = |extent: f64| (libm::ceil(extent / cell) as usize).max(1);
        let (columns, rows) = (count(size[0]), count(size[1]));
        Self {
            cell,
            columns,
            rows,
            buckets: vec![Vec::new(); columns * rows],
        }
    }

    /// The buckets a box `[min_x, min_y, max_x, max_y]` reaches; ground
    /// outside the map falls in the edge buckets.
    fn span(&self, bounds: [f64; 4]) -> impl Iterator<Item = usize> {
        let index =
            |v: f64, count: usize| (libm::floor(v / self.cell).max(0.0) as usize).min(count - 1);
        let (x0, x1) = (
            index(bounds[0], self.columns),
            index(bounds[2], self.columns),
        );
        let (y0, y1) = (index(bounds[1], self.rows), index(bounds[3], self.rows));
        let columns = self.columns;
        (y0..=y1).flat_map(move |y| (x0..=x1).map(move |x| y * columns + x))
    }

    pub fn insert(&mut self, bounds: [f64; 4], item: u32) {
        for bucket in self.span(bounds) {
            self.buckets[bucket].push(item);
        }
    }

    /// Whether `test` holds for any item whose box may reach `bounds`. An
    /// item in several buckets is tested once per bucket.
    pub fn any(&self, bounds: [f64; 4], mut test: impl FnMut(u32) -> bool) -> bool {
        self.span(bounds)
            .any(|bucket| self.buckets[bucket].iter().any(|item| test(*item)))
    }

    /// Whether `test` holds for any item whose box may come within `margin`
    /// of the segment `ab`. The segment is walked a bucket's width at a
    /// time, so a long diagonal asks only of the strip it runs through.
    pub fn any_along(
        &self,
        a: Point,
        b: Point,
        margin: f64,
        mut test: impl FnMut(u32) -> bool,
    ) -> bool {
        let pieces = libm::ceil(distance(a, b) / self.cell).max(1.0);
        (0..pieces as usize).any(|piece| {
            let at = |share: f64| add(a, scale(sub(b, a), share / pieces));
            let bounds = segment_bounds(at(piece as f64), at(piece as f64 + 1.0), margin);
            self.any(bounds, &mut test)
        })
    }
}

/// The box of two points, grown by `margin`.
pub fn segment_bounds(a: Point, b: Point, margin: f64) -> [f64; 4] {
    [
        a[0].min(b[0]) - margin,
        a[1].min(b[1]) - margin,
        a[0].max(b[0]) + margin,
        a[1].max(b[1]) + margin,
    ]
}
