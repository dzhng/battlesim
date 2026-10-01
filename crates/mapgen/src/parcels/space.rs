//! Oriented rectangles and lines to cut parcels along: what the parcel pass
//! tests a parcel against. Arithmetic is `+ − × ÷` and `libm`, as in
//! `layout::geometry`.
use crate::layout::geometry::{add, cross, distance, scale, segment_distance, sub, Point};
use contract::ground::polygon_contains;

/// A rectangle with `axis` (unit) along its first half extent.
#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub center: Point,
    pub axis: Point,
    pub half: [f64; 2],
}

impl Rect {
    /// Counter-clockwise from the corner at `−axis, −normal`.
    pub fn corners(&self) -> [Point; 4] {
        [[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]].map(|[x, y]| self.at([x, y]))
    }

    /// The point at shares of the half extents: `[−1, −1]` to `[1, 1]`.
    fn at(&self, [x, y]: Point) -> Point {
        let along = scale(self.axis, x * self.half[0]);
        let across = scale([-self.axis[1], self.axis[0]], y * self.half[1]);
        add(self.center, add(along, across))
    }

    fn local(&self, p: Point) -> Point {
        let d = sub(p, self.center);
        [
            d[0] * self.axis[0] + d[1] * self.axis[1],
            cross(self.axis, d),
        ]
    }

    /// `[min_x, min_y, max_x, max_y]`.
    pub fn bounds(&self) -> [f64; 4] {
        let reach = [0, 1]
            .map(|i| self.axis[i].abs() * self.half[0] + self.axis[1 - i].abs() * self.half[1]);
        [
            self.center[0] - reach[0],
            self.center[1] - reach[1],
            self.center[0] + reach[0],
            self.center[1] + reach[1],
        ]
    }

    /// Whether the two share more than `slack` of ground in every direction
    /// (separating axes): rectangles that only touch do not overlap.
    pub fn overlaps(&self, other: &Rect, slack: f64) -> bool {
        let separated = |a: &Rect, b: &Rect| {
            let center = a.local(b.center);
            let along = b.axis[0] * a.axis[0] + b.axis[1] * a.axis[1];
            let across = cross(a.axis, b.axis);
            let reach = [
                along.abs() * b.half[0] + across.abs() * b.half[1],
                across.abs() * b.half[0] + along.abs() * b.half[1],
            ];
            (0..2).any(|i| center[i].abs() + slack >= a.half[i] + reach[i])
        };
        !separated(self, other) && !separated(other, self)
    }

    /// Distance from the rectangle to the segment; zero when they meet.
    pub fn segment_gap(&self, a: Point, b: Point) -> f64 {
        let (a, b) = (self.local(a), self.local(b));
        // Clip the segment to the rectangle (Liang–Barsky): any part inside
        // is a meeting.
        let delta = sub(b, a);
        let (mut enter, mut leave): (f64, f64) = (0.0, 1.0);
        let mut misses = false;
        for i in 0..2 {
            if delta[i] == 0.0 {
                misses |= a[i].abs() > self.half[i];
            } else {
                let (t0, t1) = (
                    (-self.half[i] - a[i]) / delta[i],
                    (self.half[i] - a[i]) / delta[i],
                );
                enter = enter.max(t0.min(t1));
                leave = leave.min(t0.max(t1));
            }
        }
        if !misses && enter <= leave {
            return 0.0;
        }
        let to_box = |p: Point| {
            let out = [0, 1].map(|i| (p[i].abs() - self.half[i]).max(0.0));
            libm::sqrt(out[0] * out[0] + out[1] * out[1])
        };
        let corners = [[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]]
            .map(|[x, y]| [x * self.half[0], y * self.half[1]]);
        corners
            .into_iter()
            .map(|corner| segment_distance(a, b, corner))
            .fold(to_box(a).min(to_box(b)), f64::min)
    }

    /// Wholly inside the ring: every corner is, and no edge of the ring
    /// enters the rectangle.
    pub fn inside(&self, ring: &[Point]) -> bool {
        self.corners().iter().all(|p| polygon_contains(ring, *p))
            && contract::ground::edges(ring).all(|(a, b)| self.segment_gap(*a, *b) > 0.0)
    }

    /// Whether any of the rectangle lies on the filled ring.
    pub fn touches(&self, ring: &[Point]) -> bool {
        polygon_contains(ring, self.center)
            || contract::ground::edges(ring).any(|(a, b)| self.segment_gap(*a, *b) == 0.0)
    }
}

/// A line of points with the distance along it to each.
pub struct Run {
    pub points: Vec<Point>,
    along: Vec<f64>,
}

impl Run {
    pub fn new(points: Vec<Point>) -> Self {
        let mut along = vec![0.0];
        for pair in points.windows(2) {
            along.push(along[along.len() - 1] + distance(pair[0], pair[1]));
        }
        Self { points, along }
    }

    pub fn length(&self) -> f64 {
        self.along[self.along.len() - 1]
    }

    /// The point `s` metres along, clamped to the ends.
    pub fn at(&self, s: f64) -> Point {
        let next = self
            .along
            .partition_point(|at| *at <= s)
            .min(self.points.len() - 1);
        let (from, span) = (
            self.along[next - 1],
            self.along[next] - self.along[next - 1],
        );
        let share = if span > 0.0 {
            ((s - from) / span).clamp(0.0, 1.0)
        } else {
            0.0
        };
        add(
            self.points[next - 1],
            scale(sub(self.points[next], self.points[next - 1]), share),
        )
    }
}

/// The parts of segment `ab` inside the ring or within `reach` of its edge,
/// as shares along it. A district's edge is a road or a street as often as
/// not, and a road's rounded line strays from the edge it was cut along by
/// less than its own half width: `reach` is that.
pub fn clip(a: Point, b: Point, ring: &[Point], reach: f64) -> Vec<[f64; 2]> {
    let on_edge =
        |p: Point| contract::ground::edges(ring).any(|(c, d)| segment_distance(*c, *d, p) <= reach);
    let step = sub(b, a);
    let span = step[0] * step[0] + step[1] * step[1];
    let mut cuts = vec![0.0, 1.0];
    for (c, d) in contract::ground::edges(ring) {
        if let Some((t, _)) = crate::layout::geometry::segment_crossing(a, b, *c, *d) {
            cuts.push(t);
        }
        // A line along the edge crosses nothing: it comes to the ring where
        // it passes a corner.
        if segment_distance(a, b, *c) <= reach {
            let offset = sub(*c, a);
            cuts.push(((offset[0] * step[0] + offset[1] * step[1]) / span).clamp(0.0, 1.0));
        }
    }
    cuts.sort_by(f64::total_cmp);
    let mut inside: Vec<[f64; 2]> = Vec::new();
    for pair in cuts.windows(2) {
        let middle = add(a, scale(sub(b, a), (pair[0] + pair[1]) / 2.0));
        if pair[1] > pair[0] && (polygon_contains(ring, middle) || on_edge(middle)) {
            match inside.last_mut() {
                Some(last) if last[1] == pair[0] => last[1] = pair[1],
                _ => inside.push([pair[0], pair[1]]),
            }
        }
    }
    inside
}
