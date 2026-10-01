use contract::curve::Centerline;

/// Distance from `point` to the nearest authored run.
fn off_runs(controls: &[[f64; 2]], point: [f64; 2]) -> f64 {
    controls
        .windows(2)
        .map(|run| {
            let (dx, dy) = (run[1][0] - run[0][0], run[1][1] - run[0][1]);
            let t = (((point[0] - run[0][0]) * dx + (point[1] - run[0][1]) * dy)
                / (dx * dx + dy * dy))
                .clamp(0.0, 1.0);
            let (x, y) = (
                point[0] - (run[0][0] + t * dx),
                point[1] - (run[0][1] + t * dy),
            );
            (x * x + y * y).sqrt()
        })
        .fold(f64::INFINITY, f64::min)
}

fn gap(pair: &[[f64; 2]]) -> f64 {
    (pair[1][0] - pair[0][0]).hypot(pair[1][1] - pair[0][1])
}

#[test]
fn samples_keep_every_control_and_a_bend_never_skips_two_metres() {
    let controls = vec![[0.0, 0.0], [200.0, 0.0], [200.0, 200.0]];
    let line = Centerline::new(controls.clone(), 6.0, 128).unwrap();
    assert_eq!(line.control_points(), controls);
    let samples = line.samples();
    assert_eq!(samples.first(), controls.first());
    assert_eq!(samples.last(), controls.last());
    for control in &controls {
        assert!(
            samples.contains(control),
            "missing authored control {control:?}"
        );
    }
    // Only a straight stretch of an authored run may span more than two metres.
    for pair in samples.windows(2) {
        let straight = off_runs(&controls, pair[0]) == 0.0 && off_runs(&controls, pair[1]) == 0.0;
        assert!(
            gap(pair) <= 2.0 || straight,
            "skipped ground between {pair:?}"
        );
    }
    assert!(samples.len() > 12, "the bend was not rounded: {samples:?}");
}

#[test]
fn rounding_a_corner_keeps_the_distant_straight_road_one_straight_segment() {
    let line = Centerline::new(vec![[0.0, 0.0], [730.0, 0.0], [730.0, 380.0]], 6.0, 2048).unwrap();
    let long: Vec<_> = line
        .samples()
        .windows(2)
        .filter(|p| gap(p) > 100.0)
        .collect();
    assert_eq!(
        long.len(),
        2,
        "each straight approach is one segment: {long:?}"
    );
    assert!(long
        .iter()
        .all(|p| p[0][1] == 0.0 && p[1][1] == 0.0 || p[0][0] == 730.0 && p[1][0] == 730.0));
    assert!(
        line.samples().iter().any(|p| p[1] < -0.1),
        "the turn stayed angular"
    );
}

#[test]
fn curved_samples_stay_inside_the_authored_run_corridor() {
    let controls = vec![
        [140.0, 780.0],
        [420.0, 420.0],
        [1150.0, 420.0],
        [1150.0, 800.0],
    ];
    let line = Centerline::new(controls.clone(), 6.0, 4096).unwrap();
    for &point in line.samples() {
        let off = off_runs(&controls, point);
        assert!(
            off <= 6.0,
            "centreline left the authored corridor at {point:?}: {off}"
        );
    }
}

#[test]
fn a_line_without_a_bend_is_its_controls() {
    let controls = vec![[0.0, 0.0], [200.0, 0.0]];
    assert_eq!(
        Centerline::new(controls.clone(), 6.0, 2).unwrap().samples(),
        controls
    );
    // Collinear controls are kept, and add nothing between them.
    let collinear = vec![[0.0, 0.0], [100.0, 0.0], [200.0, 0.0]];
    assert_eq!(
        Centerline::new(collinear.clone(), 6.0, 3)
            .unwrap()
            .samples(),
        collinear
    );
}

#[test]
fn an_insufficient_sample_allowance_refuses_the_whole_line() {
    let controls = vec![[0.0, 0.0], [200.0, 0.0], [200.0, 200.0]];
    assert!(Centerline::new(controls.clone(), 6.0, 8).is_err());
    assert!(Centerline::new(controls, 6.0, 256).is_ok());
}

#[test]
fn a_line_cannot_double_back_on_itself_at_a_control() {
    assert!(Centerline::new(vec![[0.0, 0.0], [100.0, 0.0], [0.0, 0.0]], 6.0, 256).is_err());
}
