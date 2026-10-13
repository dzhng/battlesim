//! The `math` package's 2D shape tests (`math/shapes`) the ground was
//! resolved with, in their own operation order. Polygons are flat
//! `[x0, y0, x1, y1, …]`.
use super::{CUT_A, CUT_B};

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

/// `segment2.closestPoint`: the point of `a`–`b` nearest `p`.
pub fn closest(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    let (qx, qy) = (b[0] - a[0], b[1] - a[1]);
    let d = qx * qx + qy * qy;
    let mut t = qx * (p[0] - a[0]) + qy * (p[1] - a[1]);
    if d > 0.0 {
        t /= d;
    }
    let t = t.clamp(0.0, 1.0);
    [a[0] + t * qx, a[1] + t * qy]
}

/// How far `p` lies inside the stretch `r` (`a, b, half width, kind, cuts`;
/// negative outside): its half width less the distance to it, and past a
/// cut end the distance to that square end instead
/// (`contract::ground::stretch_contains`).
pub fn stroke_inside(r: &[f32], [x, y]: [f64; 2]) -> f64 {
    let [ax, ay, bx, by, half] = [r[0], r[1], r[2], r[3], r[4]].map(f64::from);
    let inside = half - distance([x, y], closest([x, y], [ax, ay], [bx, by]));
    let cuts = r[6] as u32;
    if cuts == 0 {
        return inside;
    }
    let (dx, dy) = (bx - ax, by - ay);
    let length = libm::hypot(dx, dy);
    let along = ((x - ax) * dx + (y - ay) * dy) / length;
    let cut = |bit: u32, v: f64| {
        if cuts & bit != 0 {
            v
        } else {
            f64::NEG_INFINITY
        }
    };
    let past = cut(CUT_A, -along).max(cut(CUT_B, along - length));
    if past <= 0.0 {
        return inside.min(-past);
    }
    let aside = (((x - ax) * dy - (y - ay) * dx).abs() / length - half).max(0.0);
    -libm::hypot(aside, past)
}

/// `polygon2.containsPoint`: edges and vertices count as inside.
pub fn polygon_contains(v: &[f64], [x, y]: [f64; 2]) -> bool {
    let n = v.len() / 2;
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi, xj, yj) = (v[i * 2], v[i * 2 + 1], v[j * 2], v[j * 2 + 1]);
        let side = (yi - yj) * (x - xi) - (xi - xj) * (y - yi);
        let before = v[(if j == 0 { n - 1 } else { j - 1 }) * 2 + 1];
        let crossing = if yj < yi {
            (y >= yj && y < yi).then_some((side > 0.0, y > before))
        } else if yi < yj {
            (y > yi && y <= yj).then_some((side < 0.0, y < before))
        } else {
            if y == yi && ((x >= xj && x <= xi) || (x >= xi && x <= xj)) {
                return true;
            }
            None
        };
        if let Some((toggles, past_vertex)) = crossing {
            if side == 0.0 {
                return true;
            }
            if toggles && (y != yj || past_vertex) {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}
