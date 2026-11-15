//! One immutable authored centreline and the points every consumer samples.
//! Each authored corner is swept at one radius through its control point, so
//! the line still passes through every control and never leaves the corridor
//! of the runs it was authored on. The straight middle of a run stays one
//! segment: only bends are sampled, at most two metres apart.
//! Arithmetic is `+ − × ÷` and `libm`, so native and Wasm sample identically.

const SAMPLE_SPACING_M: f64 = 2.0;
type Point = [f64; 2];

#[derive(Clone, Debug)]
pub struct Centerline {
    controls: Vec<Point>,
    samples: Vec<Point>,
}

impl Centerline {
    /// Sample once at load. A bend strays at most `max_deviation_m` from its
    /// authored runs and leaves the middle third of each straight. `max_samples` is
    /// the caller's allocation bound: exhausting it refuses the line rather
    /// than dropping geometry.
    pub fn new(
        controls: Vec<Point>,
        max_deviation_m: f64,
        max_samples: usize,
    ) -> Result<Self, String> {
        if controls.len() < 2
            || controls.iter().flatten().any(|v| !v.is_finite())
            || !max_deviation_m.is_finite()
            || max_deviation_m < 0.0
            || controls.len() > max_samples
        {
            return Err(
                "centreline requires finite controls, nonnegative deviation and a sample allowance"
                    .into(),
            );
        }
        let mut directions = Vec::with_capacity(controls.len() - 1);
        let mut lengths = Vec::with_capacity(controls.len() - 1);
        for run in controls.windows(2) {
            let delta = sub(run[1], run[0]);
            let length = norm(delta);
            if !length.is_finite() || length == 0.0 {
                return Err("centreline runs must have finite positive length".into());
            }
            directions.push(scale(delta, 1.0 / length));
            lengths.push(length);
        }
        let mut samples = vec![controls[0]];
        for i in 1..controls.len() - 1 {
            let control = controls[i];
            let (before, after) = (directions[i - 1], directions[i]);
            let (sin, cos) = (cross(before, after), dot(before, after));
            if sin == 0.0 && cos < 0.0 {
                return Err("centreline cannot reverse at an authored control".into());
            }
            // Every run keeps its middle third straight: a bend takes a third
            // of a run it shares with another bend, and two thirds of one that
            // ends the line, whose end stays straight and square. Three
            // deviations at most, so a corner bends only the road near it.
            let share = |run: usize| {
                let end = run == 0 || run == lengths.len() - 1;
                lengths[run] * if end { 2.0 } else { 1.0 } / 3.0
            };
            let Some(bend) = Bend::new(
                libm::atan2(sin.abs(), cos),
                max_deviation_m,
                share(i - 1).min(share(i)).min(3.0 * max_deviation_m),
            ) else {
                push(control, &mut samples, max_samples)?;
                continue;
            };
            let side = if sin > 0.0 { 1.0 } else { -1.0 };
            let start = sub(control, scale(before, bend.trim));
            let end = add(control, scale(after, bend.trim));
            push(start, &mut samples, max_samples)?;
            bend.approach(start, before, side, &mut samples, max_samples)?;
            push(control, &mut samples, max_samples)?;
            // The way out is the way in, driven backwards from its far end.
            let mut out = Vec::new();
            bend.approach(end, scale(after, -1.0), -side, &mut out, max_samples)?;
            for &point in out.iter().rev() {
                push(point, &mut samples, max_samples)?;
            }
            push(end, &mut samples, max_samples)?;
        }
        push(*controls.last().unwrap(), &mut samples, max_samples)?;
        Ok(Self { controls, samples })
    }

    /// The authored points: what a plot or parcel is cut along.
    pub fn control_points(&self) -> &[Point] {
        &self.controls
    }

    /// The rounded line every physical and drawn consumer reads.
    pub fn samples(&self) -> &[Point] {
        &self.samples
    }
}

/// How one corner is swept. A line that must pass through the corner point
/// and stay tangent to both runs first eases outward by `swing`, then turns
/// in through the corner by `swing` plus half the turn, and mirrors that on
/// the way out, all at `radius`: the widest radius that stays within the
/// corridor and within its share of each run.
struct Bend {
    radius: f64,
    swing: f64,
    half_turn: f64,
    /// How far before and after the corner point the sweep begins and ends.
    trim: f64,
}

impl Bend {
    fn new(turn: f64, max_deviation_m: f64, room: f64) -> Option<Self> {
        let half_turn = turn / 2.0;
        // The outward swing and the turn in come back to the run's line at
        // the corner point: 1 − cos(swing) = (1 − cos(half_turn)) / 2.
        let quarter = libm::sin(turn / 4.0);
        let swing = 2.0 * libm::asin(quarter / core::f64::consts::SQRT_2);
        // The widest the line strays from the run it leaves, per metre of radius.
        let stray = 2.0 * quarter * quarter;
        let reach = 2.0 * libm::sin(swing) + libm::sin(half_turn);
        let radius = (max_deviation_m / stray).min(room / reach);
        let trim = radius * reach;
        (turn > 0.0 && radius.is_finite() && radius > 0.0 && trim > 0.0).then_some(Self {
            radius,
            swing,
            half_turn,
            trim,
        })
    }

    /// The samples after `start` up to, not including, the corner point, for
    /// a line leaving `start` along `heading` that turns toward `side` (+1
    /// counter-clockwise).
    fn approach(
        &self,
        start: Point,
        heading: Point,
        side: f64,
        samples: &mut Vec<Point>,
        limit: usize,
    ) -> Result<(), String> {
        let (at, heading) = self.arc(start, heading, -side, self.swing, samples, limit)?;
        self.arc(
            at,
            heading,
            side,
            self.swing + self.half_turn,
            samples,
            limit,
        )?;
        samples.pop();
        Ok(())
    }

    /// Append an arc of `angle` from `from` along `heading`, turning toward
    /// `side`, split so no piece is longer than the sample spacing. Returns
    /// where it ends and its heading there.
    fn arc(
        &self,
        from: Point,
        heading: Point,
        side: f64,
        angle: f64,
        samples: &mut Vec<Point>,
        limit: usize,
    ) -> Result<(Point, Point), String> {
        let normal = scale([-heading[1], heading[0]], side);
        let pieces = libm::ceil(self.radius * angle / SAMPLE_SPACING_M).max(1.0);
        if !pieces.is_finite() || samples.len() as f64 + pieces > limit as f64 {
            return Err("centreline exceeds its sample allowance".into());
        }
        let mut end = (from, heading);
        for piece in 1..=pieces as usize {
            let turned = angle * piece as f64 / pieces;
            let (sin, cos) = (libm::sin(turned), libm::cos(turned));
            let point = add(
                from,
                add(
                    scale(heading, self.radius * sin),
                    scale(normal, self.radius * (1.0 - cos)),
                ),
            );
            if samples.last() == Some(&point) {
                return Err("centreline spacing cannot be represented at these coordinates".into());
            }
            samples.push(point);
            end = (point, add(scale(heading, cos), scale(normal, sin)));
        }
        Ok(end)
    }
}

fn push(point: Point, samples: &mut Vec<Point>, limit: usize) -> Result<(), String> {
    if samples.last() == Some(&point) {
        return Ok(());
    }
    if samples.len() == limit {
        return Err("centreline exceeds its sample allowance".into());
    }
    samples.push(point);
    Ok(())
}

fn add(a: Point, b: Point) -> Point {
    [a[0] + b[0], a[1] + b[1]]
}
fn sub(a: Point, b: Point) -> Point {
    [a[0] - b[0], a[1] - b[1]]
}
fn scale(a: Point, factor: f64) -> Point {
    [a[0] * factor, a[1] * factor]
}
fn cross(a: Point, b: Point) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}
fn dot(a: Point, b: Point) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}
fn norm(a: Point) -> f64 {
    libm::sqrt(a[0] * a[0] + a[1] * a[1])
}
