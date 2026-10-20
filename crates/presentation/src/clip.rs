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
