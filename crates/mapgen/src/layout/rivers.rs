//! A river's course: a meandering line of water from the north edge to the
//! south, past the main settlement. It is drawn as soon as the main
//! settlement's outline is; that settlement's approaches, every other
//! settlement, the woods and the roads then take their places beside it.
use super::geometry::{
    add, distance, length, round_cm, scale, segment_crossing, segment_distance, sub, Outline,
    Point, TAU,
};
use super::rng::Stream;
use super::roads::{self, Arm};
use super::water::Water;
use super::Context;
use contract::river::{River, RiverPoint};

/// Where a request's rivers come from: whether the seed has one (the type's
/// presets say how often), and the stream its courses are drawn from.
pub struct Source<'a> {
    context: &'a Context<'a>,
    skeleton: &'a [Arm],
    rng: Stream,
    wanted: bool,
}

impl<'a> Source<'a> {
    pub fn new(context: &'a Context<'a>, skeleton: &'a [Arm]) -> Self {
        let mut rng = context.stream("rivers");
        let wanted = rng.chance(context.preset.river_chance);
        Self {
            context,
            skeleton,
            rng,
            wanted,
        }
    }

    /// The map's rivers past a main settlement that stands on `main`: none
    /// when the seed has no river, and `None` when it has one and no course
    /// drawn within the presets' attempts runs clear.
    pub fn past(&mut self, main: &Outline) -> Option<Vec<River>> {
        if !self.wanted {
            return Some(Vec::new());
        }
        (0..self.context.presets.retries.river)
            .find_map(|_| course(self.context, self.skeleton, main, &mut self.rng))
            .map(|river| vec![river])
    }
}

/// One drawn course, or `None` when it runs where a river may not.
fn course(context: &Context, skeleton: &[Arm], main: &Outline, rng: &mut Stream) -> Option<River> {
    let rules = &context.presets.rivers;
    let extent = context.extent;
    let widest = rules.width_m[1] / 2.0;
    let across = [
        rules.side_margin_m + widest,
        extent - rules.side_margin_m - widest,
    ];
    // A cubic from the north edge to the south whose two inner controls sit
    // a third and two thirds of the way down: it never turns back north.
    let controls: [Point; 4] =
        [1.0, 2.0 / 3.0, 1.0 / 3.0, 0.0].map(|share| [rng.range(across), extent * share]);
    let meander = &rules.meander;
    let wavelength = rng.range(meander.wavelength_m);
    let amplitude = wavelength * rng.range(meander.amplitude);
    let ratio = rng.range(meander.overtone_ratio);
    let phases = [0; 3].map(|_| rng.range([0.0, TAU]));
    let widths = [rng.range(rules.width_m), rng.range(rules.width_m)];

    // Authored points lie a step apart: near enough that this meander's
    // tightest bend turns the line no more than the presets allow at any one
    // of them.
    let bend = amplitude
        * (TAU / wavelength)
        * (TAU / wavelength)
        * (1.0 + meander.overtone_gain * ratio * ratio);
    let step = rules
        .point_step_m
        .min(rules.point_turn_deg.to_radians() / bend);
    // A course that runs deeper into the main settlement than its meander
    // swings cannot be carried clear of it: most are dropped here, unwalked.
    let swing = amplitude * (1.0 + meander.overtone_gain);
    let sunk = (1..32).any(|k| {
        let (at, _) = cubic(&controls, f64::from(k) / 32.0);
        contract::ground::polygon_contains(&main.ring, at)
            && contract::ground::edges(&main.ring)
                .all(|(a, b)| segment_distance(*a, *b, at) > swing)
    });
    if sunk {
        return None;
    }
    // The course, walked twice as finely, with the distance run to each
    // point.
    let steps = (libm::ceil(
        controls
            .windows(2)
            .map(|run| distance(run[0], run[1]))
            .sum::<f64>()
            / (step / 2.0),
    ) as usize)
        .max(1);
    let base: Vec<(Point, Point)> = (0..=steps)
        .map(|step| cubic(&controls, step as f64 / steps as f64))
        .collect();
    let mut run = vec![0.0];
    for pair in base.windows(2) {
        run.push(run[run.len() - 1] + distance(pair[0].0, pair[1].0));
    }
    let whole = run[steps];
    // The meander swings the line across its course, easing in from each
    // edge so the river meets the edge where the course does.
    let taper = wavelength / 2.0;
    let swung: Vec<Point> = base
        .iter()
        .zip(&run)
        .map(|((at, heading), s)| {
            let ease = (s / taper).min((whole - s) / taper).min(1.0);
            let swing = amplitude
                * ease
                * (libm::sin(TAU * s / wavelength + phases[0])
                    + meander.overtone_gain * libm::sin(TAU * s * ratio / wavelength + phases[1]));
            add(*at, scale([-heading[1], heading[0]], swing))
        })
        .collect();

    let mut along = vec![0.0];
    for pair in swung.windows(2) {
        along.push(along[along.len() - 1] + distance(pair[0], pair[1]));
    }
    let total = along[steps];
    let count = (libm::ceil(total / step) as usize).max(2);
    let mut points = Vec::with_capacity(count + 1);
    let mut walked = 0;
    for index in 0..=count {
        let s = total * index as f64 / count as f64;
        while walked + 1 < steps && along[walked + 1] < s {
            walked += 1;
        }
        let share = (s - along[walked]) / (along[walked + 1] - along[walked]);
        // Both ends are the course's own, exactly on the map's edge.
        let xy = round_cm(match index {
            0 => controls[0],
            _ if index == count => controls[3],
            _ => add(
                swung[walked],
                scale(sub(swung[walked + 1], swung[walked]), share),
            ),
        });
        // The water widens or narrows from one edge to the other, and
        // swells and thins on the way.
        let swell = 1.0 + rules.width_swing * libm::sin(TAU * s / (wavelength * ratio) + phases[2]);
        let width = (widths[0] + (widths[1] - widths[0]) * s / total) * swell;
        let width_m = libm::round(width.clamp(rules.width_m[0], rules.width_m[1]) * 10.0) / 10.0;
        points.push(RiverPoint {
            xy,
            width_m,
            depth_m: libm::round(rules.bank_grade * width_m / 2.0 * 100.0) / 100.0,
        });
    }

    // A main road's line crosses the river once at most: a river that
    // wound back and forth over it would be bridged again and again.
    let winds = skeleton.iter().any(|arm| {
        let crossings = points
            .windows(2)
            .filter(|run| segment_crossing(arm.exit, arm.target, run[0].xy, run[1].xy).is_some());
        crossings.count() > 1
    });
    if winds {
        return None;
    }
    let junctions = || skeleton.iter().flat_map(|arm| [arm.exit, arm.target]);
    let mut halves = [0.0, 0.0];
    for (index, point) in points.iter().enumerate() {
        let (p, half) = (point.xy, point.width_m / 2.0);
        let inside = p[0] >= across[0] && p[0] <= across[1] && p[1] >= 0.0 && p[1] <= extent;
        let clear = (distance(p, main.center) - main.reach - half >= rules.settlement_gap_m
            || main.gap_to(p, half) >= rules.settlement_gap_m)
            && junctions().all(|stop| distance(p, stop) - half >= rules.junction_gap_m);
        if !inside || !clear {
            return None;
        }
        if let Some(next) = points.get(index + 1) {
            let middle = (p[1] + next.xy[1]) / 2.0;
            halves[usize::from(middle < extent / 2.0)] += length(sub(next.xy, p));
        }
    }
    // Well inside the tolerance: a split at its limit reads as uneven.
    let [top, bottom] = halves;
    let allowance = context
        .presets
        .fairness
        .river
        .allowance(top, bottom, extent);
    if (top - bottom).abs() > 0.5 * allowance {
        return None;
    }
    let river = River::new(points, -rules.freeboard_m).ok()?;
    // And the main roads still reach the centre in time by its bridges.
    let in_time = {
        let water = Water::new(core::slice::from_ref(&river), [extent; 2]);
        roads::in_time(context, skeleton, &water)
    };
    in_time.then_some(river)
}

/// A cubic Bézier's point and unit heading at `t`.
fn cubic(controls: &[Point; 4], t: f64) -> (Point, Point) {
    let u = 1.0 - t;
    let weights = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
    let at = controls
        .iter()
        .zip(weights)
        .fold([0.0, 0.0], |sum, (control, weight)| {
            add(sum, scale(*control, weight))
        });
    let slopes = [3.0 * u * u, 6.0 * u * t, 3.0 * t * t];
    let heading = controls
        .windows(2)
        .zip(slopes)
        .fold([0.0, 0.0], |sum, (run, weight)| {
            add(sum, scale(sub(run[1], run[0]), weight))
        });
    (at, scale(heading, 1.0 / length(heading)))
}
