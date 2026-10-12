//! The `math` package's 2D shape tests (`math/shapes`) the ground was
//! resolved with, in their own operation order. Polygons are flat
//! `[x0, y0, x1, y1, …]`.

/// `vec2.distance`.
pub fn distance(a: [f64; 2], b: [f64; 2]) -> f64 {
    let (x, y) = (b[0] - a[0], b[1] - a[1]);
    (x * x + y * y).sqrt()
}

/// `segment2.intersects`: whether `a`–`b` crosses `c`–`d` (never when parallel).
pub fn segments_cross(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> bool {
    let (rx, ry) = (b[0] - a[0], b[1] - a[1]);
    let (ex, ey) = (d[0] - c[0], d[1] - c[1]);
    let denom = rx * ey - ry * ex;
    if denom == 0.0 {
        return false;
    }
    let (wx, wy) = (c[0] - a[0], c[1] - a[1]);
    let u = (wx * ey - wy * ex) / denom;
    let v = (wx * ry - wy * rx) / denom;
    (0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v)
}

/// `triangle2.containsPoint`, its edges included.
pub fn triangle_contains(a: [f64; 2], b: [f64; 2], c: [f64; 2], p: [f64; 2]) -> bool {
    let d1 = (p[0] - b[0]) * (a[1] - b[1]) - (a[0] - b[0]) * (p[1] - b[1]);
    let d2 = (p[0] - c[0]) * (b[1] - c[1]) - (b[0] - c[0]) * (p[1] - c[1]);
    let d3 = (p[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (p[1] - a[1]);
    let negative = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let positive = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(negative && positive)
}

/// `polygon2.signedArea`: positive counter-clockwise.
pub fn signed_area(poly: &[f64]) -> f64 {
    let n = poly.len() / 2;
    let mut area = 0.0;
    for i in 0..n {
        let j = (i + n - 1) % n;
        area += poly[j * 2] * poly[i * 2 + 1] - poly[i * 2] * poly[j * 2 + 1];
    }
    area / 2.0
}

/// `polygon2.centroid`; the vertex mean for a degenerate polygon.
pub fn centroid(poly: &[f64]) -> [f64; 2] {
    let n = poly.len() / 2;
    let (mut cx, mut cy, mut twice) = (0.0, 0.0, 0.0);
    for i in 0..n {
        let j = (i + n - 1) % n;
        let (xi, yi, xj, yj) = (poly[i * 2], poly[i * 2 + 1], poly[j * 2], poly[j * 2 + 1]);
        let cross = xj * yi - xi * yj;
        twice += cross;
        cx += (xj + xi) * cross;
        cy += (yj + yi) * cross;
    }
    if twice == 0.0 {
        let (mut sx, mut sy) = (0.0, 0.0);
        for i in 0..n {
            sx += poly[i * 2];
            sy += poly[i * 2 + 1];
        }
        return [sx / n as f64, sy / n as f64];
    }
    let inverse = 1.0 / (3.0 * twice);
    [cx * inverse, cy * inverse]
}

/// `[min, max]` of `poly` projected on `(nx, ny)`.
fn project(nx: f64, ny: f64, poly: &[f64]) -> (f64, f64) {
    let mut lo = nx * poly[0] + ny * poly[1];
    let mut hi = lo;
    for p in poly.as_chunks::<2>().0.iter().skip(1) {
        let d = nx * p[0] + ny * p[1];
        if d < lo {
            lo = d;
        } else if d > hi {
            hi = d;
        }
    }
    (lo, hi)
}

fn separated_by_edges_of(a: &[f64], b: &[f64]) -> bool {
    let n = a.len() / 2;
    (0..n).any(|i| {
        let j = (i + n - 1) % n;
        let nx = a[i * 2 + 1] - a[j * 2 + 1];
        let ny = -(a[i * 2] - a[j * 2]);
        let (a0, a1) = project(nx, ny, a);
        let (b0, b1) = project(nx, ny, b);
        a0 > b1 || b0 > a1
    })
}

/// `polygon2.overlapConvex`: touching counts.
pub fn convex_overlap(a: &[f64], b: &[f64]) -> bool {
    !separated_by_edges_of(a, b) && !separated_by_edges_of(b, a)
}
