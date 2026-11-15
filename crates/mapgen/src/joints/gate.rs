//! Where a way meets the map's edge.
use super::Way;
use crate::layout::geometry::{cross, dot, round_cm, scale, sub};

/// A way that ends on the map's edge at a slant runs in square to the edge
/// for `edge_run_m` before it turns: its flat end lies along the edge and
/// none of it shows, and an entry there has a straight road ahead of it.
pub(super) fn gate(policy: &crate::layout::JointPolicy, ways: &mut [Way], size: [f64; 2]) {
    for way in ways.iter_mut().filter(|way| !way.pinned) {
        for last in [false, true] {
            let (at, out_of) = way.end(last);
            // The edge it stands on and that edge's outward normal.
            let normal = [
                (at[0] <= 0.0, [-1.0, 0.0]),
                (at[0] >= size[0], [1.0, 0.0]),
                (at[1] <= 0.0, [0.0, -1.0]),
                (at[1] >= size[1], [0.0, 1.0]),
            ]
            .into_iter()
            .find_map(|(on, normal)| (on && dot(out_of, normal) > 0.0).then_some(normal));
            let Some(normal) = normal else { continue };
            // Already square to the edge, as an entry needs it exactly.
            if cross(out_of, normal) == 0.0 {
                continue;
            }
            // How far in from the edge the point before the end stands.
            let room = dot(sub(at, way.before(last)), normal);
            // The straight run in, and the most the bend at its end takes of
            // it (three half widths: a rounded centreline's reach). Where the
            // run on would keep no room for both its bends, half the room.
            let reach = 3.0 * way.half();
            let full = policy.edge_run_m + reach;
            let depth = if full + 3.0 * reach <= room {
                full
            } else {
                full.min(room / 2.0)
            };
            if depth < policy.shortest_run_m {
                continue;
            }
            let turn = round_cm(sub(at, scale(normal, depth)));
            let mut gated = way.points.clone();
            gated.insert(if last { gated.len() - 1 } else { 1 }, turn);
            way.relay(gated);
        }
    }
}
