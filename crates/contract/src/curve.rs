//! One immutable authored centreline and the points every consumer samples.
//! Each authored corner is rounded by a pair of cubic Béziers that meet at the
//! control point, so the line still passes through every control and never
//! leaves the corridor of the runs it was authored on. The straight middle of
//! a run stays one segment: only bends are sampled, at most two metres apart.
//! Arithmetic is `+ − × ÷` and `libm`, so native and Wasm sample identically.

const SAMPLE_SPACING_M: f64 = 2.0;
type Point = [f64; 2];
type Cubic = [Point; 4];

#[derive(Clone, Debug)]
pub struct Centerline {
    controls: Vec<Point>,
    samples: Vec<Point>,
}

impl Centerline {
    /// Sample once at load. A bend strays at most `max_deviation_m` from its
    /// authored runs and takes at most a third of either. `max_samples` is
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
            let bisector = add(before, after);
            let length = norm(bisector);
            if length == 0.0 {
                return Err("centreline cannot reverse at an authored control".into());
            }
            // A third of the shorter run at most, so two bends never meet and
            // a remote corner never bends a straight road.
            let reach = max_deviation_m.min(lengths[i - 1].min(lengths[i]) / 9.0);
            let trim = 3.0 * reach;
            if trim == 0.0 || cross(before, after) == 0.0 {
                push(control, &mut samples, max_samples)?;
                continue;
            }
            let handle = scale(bisector, reach / length);
            let (start, end) = (
                sub(control, scale(before, trim)),
                add(control, scale(after, trim)),
            );
            push(start, &mut samples, max_samples)?;
            append_curve(
                [
                    start,
                    add(start, scale(before, trim / 3.0)),
                    sub(control, handle),
                    control,
                ],
                &mut samples,
                max_samples,
            )?;
            append_curve(
                [
                    control,
                    add(control, handle),
                    sub(end, scale(after, trim / 3.0)),
                    end,
                ],
                &mut samples,
                max_samples,
            )?;
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

/// Append the curve after its first point, split until no piece is longer
/// than the sample spacing.
fn append_curve(curve: Cubic, samples: &mut Vec<Point>, limit: usize) -> Result<(), String> {
    let mut pending = vec![curve];
    while let Some(curve) = pending.pop() {
        // A Bézier is no longer than its control polygon, so bounding that
        // bounds the chord and the arc between two samples.
        let arc_bound: f64 = curve.windows(2).map(|p| norm(sub(p[1], p[0]))).sum();
        if !arc_bound.is_finite() {
            return Err("centreline curve arithmetic must be finite".into());
        }
        if arc_bound <= SAMPLE_SPACING_M {
            push(curve[3], samples, limit)?;
            continue;
        }
        if samples.len() + pending.len() + 2 > limit {
            return Err("centreline exceeds its sample allowance".into());
        }
        let a = midpoint(curve[0], curve[1]);
        let b = midpoint(curve[1], curve[2]);
        let c = midpoint(curve[2], curve[3]);
        let d = midpoint(a, b);
        let e = midpoint(b, c);
        let middle = midpoint(d, e);
        if middle == curve[0] || middle == curve[3] {
            return Err("centreline spacing cannot be represented at these coordinates".into());
        }
        pending.push([middle, e, c, curve[3]]);
        pending.push([curve[0], a, d, middle]);
    }
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
fn norm(a: Point) -> f64 {
    libm::sqrt(a[0] * a[0] + a[1] * a[1])
}
fn midpoint(a: Point, b: Point) -> Point {
    add(a, scale(sub(b, a), 0.5))
}
