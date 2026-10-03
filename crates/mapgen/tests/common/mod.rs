/// How deep two convex rings overlap: the least depth over their edge
/// normals, negative when an edge separates them.
pub fn overlap_depth(a: &[[f64; 2]], b: &[[f64; 2]]) -> f64 {
    let mut depth = f64::INFINITY;
    for (ring, other) in [(a, b), (b, a)] {
        for (p, q) in contract::ground::edges(ring) {
            let length = contract::ground::segment_distance(*p, *p, *q);
            let normal = [(q[1] - p[1]) / length, (p[0] - q[0]) / length];
            let span = |points: &[[f64; 2]]| {
                let along = points.iter().map(|v| v[0] * normal[0] + v[1] * normal[1]);
                along.fold((f64::INFINITY, f64::NEG_INFINITY), |(low, high), v| {
                    (low.min(v), high.max(v))
                })
            };
            let ((a0, a1), (b0, b1)) = (span(ring), span(other));
            depth = depth.min(a1.min(b1) - a0.max(b0));
        }
    }
    depth
}
