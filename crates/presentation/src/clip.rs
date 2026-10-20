//! Conservative selection against the exact f32 matrix submitted by either client.
/// WebGPU clip inequalities: -w <= x,y <= w and 0 <= z <= w.
/// Work directly with the submitted matrix; no camera reconstruction or plane
/// normalization. The infinite projection's degenerate far plane is valid.
pub(crate) struct ClipPlane {
    coefficients: [f64; 4],
    rounding: [f64; 4],
}

pub(crate) fn clip_planes(matrix: &[f32; 16]) -> [ClipPlane; 6] {
    let row = |r: usize| -> [f64; 4] { std::array::from_fn(|c| f64::from(matrix[c * 4 + r])) };
    let w = row(3);
    [
        (0, 1., true),
        (0, -1., true),
        (1, 1., true),
        (1, -1., true),
        (2, 1., false),
        (2, -1., true),
    ]
    .map(|(r, sign, include_w)| {
        let v = row(r);
        ClipPlane {
            coefficients: std::array::from_fn(|i| sign * v[i] + if include_w { w[i] } else { 0. }),
            // GPU matrix products round in f32 before the clip comparison.
            // Bound their multiply/add error before combining rows, including
            // cancellation between a clip coordinate and w at an exact edge.
            rounding: std::array::from_fn(|i| {
                8. * f64::from(f32::EPSILON)
                    * (v[i].abs() + if include_w { w[i].abs() } else { 0. })
            }),
        }
    })
}

pub(crate) fn intersects(bounds: &[f64; 6], planes: &[ClipPlane; 6]) -> bool {
    planes.iter().all(|plane| {
        let p = plane.coefficients;
        let mut distance = p[3];
        let mut rounding = plane.rounding[3];
        for axis in 0..3 {
            distance += p[axis] * bounds[axis + if p[axis] >= 0. { 3 } else { 0 }];
            rounding += plane.rounding[axis] * bounds[axis].abs().max(bounds[axis + 3].abs());
        }
        distance >= -rounding
    })
}

/// Depth extrema of the box/frustum intersection, not of the whole box.
/// Every vertex is on a box face or is one of the frustum's near corners;
/// the camera frustum has no finite far corners. Scratch stays on the stack.
pub(crate) fn depth_range(
    bounds: &[f64; 6],
    planes: &[ClipPlane; 6],
    near_corners: &[[f64; 3]; 4],
    view: &[f32; 16],
) -> Option<(f64, f64)> {
    let mut range = (f64::INFINITY, f64::NEG_INFINITY);
    let mut include = |p: [f64; 3]| {
        let depth = -(0..3)
            .map(|a| f64::from(view[a * 4 + 2]) * p[a])
            .sum::<f64>()
            - f64::from(view[14]);
        range.0 = range.0.min(depth);
        range.1 = range.1.max(depth);
    };
    for &p in near_corners {
        if (0..3).all(|a| p[a] >= bounds[a] && p[a] <= bounds[a + 3]) {
            include(p);
        }
    }
    let corners: [[f64; 3]; 8] = std::array::from_fn(|i| {
        std::array::from_fn(|a| bounds[a + if i & (1 << a) == 0 { 0 } else { 3 }])
    });
    for face in [
        [0, 2, 6, 4],
        [1, 5, 7, 3],
        [0, 4, 5, 1],
        [2, 3, 7, 6],
        [0, 1, 3, 2],
        [4, 6, 7, 5],
    ] {
        let mut polygon = [[0.; 3]; 16];
        for (out, i) in polygon.iter_mut().zip(face) {
            *out = corners[i];
        }
        let mut count = 4;
        for plane in planes {
            if count == 0 {
                break;
            }
            // Use the same GPU roundoff bound as broad-phase selection.
            let rounding = plane.rounding[3]
                + (0..3)
                    .map(|a| plane.rounding[a] * bounds[a].abs().max(bounds[a + 3].abs()))
                    .sum::<f64>();
            let distance = |p: [f64; 3]| {
                plane.coefficients[3]
                    + rounding
                    + (0..3).map(|a| plane.coefficients[a] * p[a]).sum::<f64>()
            };
            let input = polygon;
            let n = count;
            count = 0;
            let mut previous = input[n - 1];
            let mut before = distance(previous);
            for &point in &input[..n] {
                let after = distance(point);
                if (before >= 0.) != (after >= 0.) {
                    let t = before / (before - after);
                    polygon[count] =
                        std::array::from_fn(|a| previous[a] + t * (point[a] - previous[a]));
                    count += 1;
                }
                if after >= 0. {
                    polygon[count] = point;
                    count += 1;
                }
                previous = point;
                before = after;
            }
        }
        for &p in &polygon[..count] {
            include(p);
        }
    }
    range.0.is_finite().then_some(range)
}
