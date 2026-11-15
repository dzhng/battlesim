//! The line a mover keeps along a way by road: each run's right-hand side,
//! as far over as leaves a gap between the mover and the middle, but with
//! the mover's own middle still on the road (so on a narrow one a wide
//! mover straddles the middle). Where two runs meet, their lanes do.
//!
//! The lane follows the road's line straightened where it wobbles: a short
//! bend (within `STRAIGHTEN_SPAN_M`) that strays less than `STRAIGHTEN_M`
//! from the chord across it, such as a corner's outward swing or a stub
//! where two roads meet, is no bend of the lane, since a driver takes the
//! line the road's width allows rather than every curl of its middle. A
//! long bend keeps its shape, or its lane would leave the road.
//!
//! Offset to the inside of a bend tighter than the lane is wide, a run's
//! lane would run backwards: the lane folds back on itself. Such a run is
//! no run of the lane: its far end is dropped and its neighbours meet at
//! one corner, which the route's rounding then makes drivable.
use super::roads::RoadNet;
use crate::math::{v2, V2};

/// A short bend of the road's line straying less than this from the chord
/// across it is straightened out of the lane.
const STRAIGHTEN_M: f64 = 2.0;
/// The longest chord a bend is straightened across.
const STRAIGHTEN_SPAN_M: f64 = 24.0;
/// Over a longer chord, only points this close to it are dropped: the
/// road graph's own chord tolerance.
const KEEP_SHAPE_M: f64 = 0.25;

/// The lane point of each of `middle`'s points (a road point and the arc of
/// the run ending there), dropping from `middle` the points a folded lane
/// would turn back at. Its first and last points always stay.
pub(super) fn lane_line(
    middle: &mut Vec<(V2, u32)>,
    roads: &RoadNet,
    half_width: f64,
    keep: f64,
) -> Vec<V2> {
    straighten(middle);
    loop {
        let line = offsets(middle, roads, half_width, keep);
        let folded = (1..middle.len())
            .find(|&k| (line[k] - line[k - 1]).dot(middle[k].0 - middle[k - 1].0) <= 0.0);
        let drop = match folded {
            Some(k) if k + 1 < middle.len() => k,
            Some(k) if k >= 2 => k - 1,
            _ => return line,
        };
        middle.remove(drop);
    }
}

/// Drop the points of `line` that stray from the chord kept across them
/// less than its span allows (Douglas–Peucker), keeping its ends. A point
/// takes the arc of the run ending there, so a kept point keeps its own.
fn straighten(line: &mut Vec<(V2, u32)>) {
    let mut keep = vec![false; line.len()];
    keep[0] = true;
    *keep.last_mut().expect("a line has ends") = true;
    let mut spans = vec![(0, line.len() - 1)];
    while let Some((a, b)) = spans.pop() {
        let (p, q) = (line[a].0, line[b].0);
        let chord = q - p;
        let length = chord.length();
        let stray = |k: usize| {
            let r = line[k].0 - p;
            if length > 1e-9 {
                (chord.cross(r) / length).abs()
            } else {
                r.length()
            }
        };
        let far = (a + 1..b).max_by(|&i, &j| stray(i).total_cmp(&stray(j)));
        let tolerance = if length <= STRAIGHTEN_SPAN_M {
            STRAIGHTEN_M
        } else {
            KEEP_SHAPE_M
        };
        if let Some(k) = far.filter(|&k| stray(k) >= tolerance) {
            keep[k] = true;
            spans.push((a, k));
            spans.push((k, b));
        }
    }
    let mut k = 0;
    line.retain(|_| {
        k += 1;
        keep[k - 1]
    });
}

fn offsets(middle: &[(V2, u32)], roads: &RoadNet, half_width: f64, keep: f64) -> Vec<V2> {
    let lanes: Vec<V2> = middle
        .windows(2)
        .map(|run| {
            let along = run[1].0 - run[0].0;
            let right = v2(along.y, -along.x) * (1.0 / along.length());
            let edge = (roads.arc(run[1].1).half_width - keep).max(0.0);
            right * (half_width + keep).min(edge)
        })
        .collect();
    let lane = |k: usize| match (k.checked_sub(1).map(|k| lanes[k]), lanes.get(k)) {
        (Some(a), Some(&b)) => {
            // The corner both lanes share, unless the road doubles back
            // (or has no width to keep to one side of).
            let widths = a.length() * b.length();
            let turn = 1.0 + a.dot(b) / widths;
            if widths > 0.0 && turn > 0.5 {
                (a + b) * (1.0 / turn)
            } else {
                b
            }
        }
        (Some(lane), None) | (None, Some(&lane)) => lane,
        (None, None) => V2::default(),
    };
    middle
        .iter()
        .enumerate()
        .map(|(k, &(point, _))| point + lane(k))
        .collect()
}
