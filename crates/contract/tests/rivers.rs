//! A river's contract: the rounded line carries width and depth, one
//! distance says what is water, and a map the terrain cannot carry is refused.
use contract::map::MapDefinition;
use contract::river::{self, River, RiverPoint};

fn point(x: f64, y: f64, width_m: f64, depth_m: f64) -> RiverPoint {
    RiverPoint {
        xy: [x, y],
        width_m,
        depth_m,
    }
}

fn map(extra: &str) -> Result<MapDefinition, serde_json::Error> {
    serde_json::from_str(&format!(
        r#"{{"size":[400,300],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35{extra}}}"#
    ))
}

fn straight(width_m: f64, depth_m: f64, surface_z: f64) -> String {
    format!(
        r#","rivers":[{{"points":[{{"xy":[200,0],"width_m":{width_m},"depth_m":{depth_m}}},{{"xy":[200,300],"width_m":{width_m},"depth_m":{depth_m}}}],"surface_z":{surface_z}}}]"#
    )
}

fn refusal(extra: &str) -> String {
    river::validate(&map(extra).expect("the map parses")).expect_err("the map is refused")
}

#[test]
fn the_rounded_line_passes_through_each_authored_point_at_its_width_and_depth() {
    let authored = vec![
        point(0.0, 100.0, 12.0, 1.5),
        point(120.0, 100.0, 20.0, 2.0),
        point(200.0, 180.0, 30.0, 3.0),
        point(330.0, 150.0, 16.0, 1.75),
    ];
    let river = River::new(authored.clone(), -0.5).unwrap();
    let samples = river.samples();
    assert!(samples.len() > 20, "the bends were not rounded");
    let mut from = 0;
    for pair in authored.windows(2) {
        let at = |p: &RiverPoint, from: usize| {
            samples[from..]
                .iter()
                .position(|s| s.xy == p.xy)
                .map(|i| i + from)
                .expect("the line passes through the point")
        };
        let (a, b) = (at(&pair[0], from), at(&pair[1], from));
        for end in [(a, pair[0]), (b, pair[1])] {
            assert_eq!(samples[end.0].half_width_m, end.1.width_m / 2.0);
            assert_eq!(samples[end.0].depth_m, end.1.depth_m);
        }
        // Between two authored points the water widens or narrows one way.
        let rising = pair[1].width_m >= pair[0].width_m;
        for w in samples[a..=b].windows(2) {
            assert!(
                (w[1].half_width_m >= w[0].half_width_m) == rising
                    || w[1].half_width_m == w[0].half_width_m,
                "width does not run one way between authored points"
            );
        }
        from = b;
    }
}

#[test]
fn width_and_depth_run_linearly_along_a_straight_run() {
    let river = River::new(
        vec![point(0.0, 0.0, 12.0, 1.0), point(300.0, 0.0, 30.0, 4.0)],
        -1.0,
    )
    .unwrap();
    // A third of the way: 18 m of water, 2 m deep at the middle.
    assert!((river.inside([100.0, 0.0]) - 9.0).abs() < 1e-12);
    assert!((river.inside([100.0, 9.0])).abs() < 1e-12);
    assert!((river.inside([100.0, -12.0]) + 3.0).abs() < 1e-12);
    // The bed is a V through the waterline; the bank carries its grade on.
    let grade = 2.0 / 9.0;
    for (y, inside) in [(0.0, 9.0), (4.5, 4.5), (9.0, 0.0), (13.0, -4.0)] {
        let height = river.height_at([100.0, y], 0.0);
        assert!(
            (height - (-1.0 - grade * inside)).abs() < 1e-12,
            "height {height} at {y}"
        );
    }
    // A steepened section is still one plane through the waterline.
    assert!((river.height_at([100.0, 13.0], 0.4) - (-1.0 + 0.4 * 4.0)).abs() < 1e-12);
    assert!((river.height_at([100.0, 4.5], 0.4) - (-1.0 - 0.4 * 4.5)).abs() < 1e-12);
}

#[test]
fn a_river_end_is_a_round_cap() {
    let river = River::new(
        vec![point(50.0, 50.0, 12.0, 1.5), point(50.0, 150.0, 12.0, 1.5)],
        -0.5,
    )
    .unwrap();
    assert_eq!(river.inside([50.0, 156.0]), 0.0);
    let diagonal = river.inside([50.0 + 6.0, 150.0 + 6.0]);
    assert!((diagonal - (6.0 - 72f64.sqrt())).abs() < 1e-12);
}

#[test]
fn a_river_keeps_its_authored_points_through_json() {
    let source = r#"{"points":[{"xy":[0.0,100.25],"width_m":12.0,"depth_m":1.5},{"xy":[120.5,100.0],"width_m":20.0,"depth_m":2.0},{"xy":[200.0,180.0],"width_m":30.0,"depth_m":3.0}],"surface_z":-0.5}"#;
    let river: River = serde_json::from_str(source).unwrap();
    assert_eq!(serde_json::to_string(&river).unwrap(), source);
}

#[test]
fn a_river_needs_points_widths_and_depths() {
    for (points, why) in [
        (r#"[{"xy":[0,0],"width_m":12,"depth_m":1}]"#, "one point"),
        (
            r#"[{"xy":[0,0],"width_m":0,"depth_m":1},{"xy":[9,0],"width_m":12,"depth_m":1}]"#,
            "no width",
        ),
        (
            r#"[{"xy":[0,0],"width_m":12,"depth_m":0},{"xy":[9,0],"width_m":12,"depth_m":1}]"#,
            "no depth",
        ),
        (
            r#"[{"xy":[0,0],"width_m":12,"depth_m":1},{"xy":[0,0],"width_m":12,"depth_m":1}]"#,
            "a repeated point",
        ),
        (
            r#"[{"xy":[0,0],"width_m":12,"depth_m":1,"bed_z":-2},{"xy":[9,0],"width_m":12,"depth_m":1}]"#,
            "an unknown field",
        ),
    ] {
        let river =
            serde_json::from_str::<River>(&format!(r#"{{"points":{points},"surface_z":-0.5}}"#));
        assert!(river.is_err(), "a river with {why} loaded");
    }
}

#[test]
fn a_river_narrower_than_three_height_samples_is_refused() {
    assert!(river::validate(&map(&straight(12.0, 1.5, -0.5)).unwrap()).is_ok());
    let message = refusal(&straight(11.9, 1.5, -0.5));
    assert!(message.contains("11.9 m wide"), "{message}");
    // The limit is the grid's: a coarser grid needs a wider river.
    let coarse: MapDefinition = serde_json::from_str(&format!(
        r#"{{"size":[400,300],"fog_cell_m":8,"height_grid_m":8,"slope_cutoff_deg":35{}}}"#,
        straight(20.0, 1.5, -0.5)
    ))
    .unwrap();
    assert!(river::validate(&coarse).is_err());
}

#[test]
fn a_bank_steeper_than_the_slope_cutoff_allows_is_refused() {
    // 12 m of water: the bank's grade is depth over 6 m. The 4 m grid can
    // turn a grade g into triangles of g √2, which must stay under tan 35°.
    let steepest = river::steepest_grade(35.0);
    assert!(steepest * 2f64.sqrt() < 35f64.to_radians().tan());
    assert!(steepest > 0.4, "a 22° bank is refused: {steepest}");
    let deepest = 6.0 * steepest;
    assert!(river::validate(&map(&straight(12.0, deepest - 0.01, -0.5)).unwrap()).is_ok());
    let message = refusal(&straight(12.0, deepest + 0.01, -0.5));
    assert!(message.contains("slope cutoff"), "{message}");
}

#[test]
fn a_surface_above_its_bank_is_refused() {
    let message = refusal(&straight(12.0, 1.5, 0.25));
    assert!(message.contains("above its bank"), "{message}");
    // Land that rises beside the water carries a higher surface.
    let raised = format!(
        r#","relief":[{{"kind":"mesa","rect":[100,-50,200,400],"height_m":3,"side_degrees":20}}]{}"#,
        straight(12.0, 1.5, 2.5)
    );
    assert!(river::validate(&map(&raised).unwrap()).is_ok());
    // High land at both ends of one straight stretch says nothing of the low
    // land between them.
    let between = format!(
        r#","relief":[{{"kind":"mesa","rect":[100,-50,200,100],"height_m":3,"side_degrees":20}},{{"kind":"mesa","rect":[100,250,200,100],"height_m":3,"side_degrees":20}}]{}"#,
        straight(12.0, 1.5, 2.5)
    );
    let message = refusal(&between);
    assert!(message.contains("above its bank"), "{message}");
}

#[test]
fn a_bridge_whose_end_a_ramp_cannot_reach_is_refused() {
    let bridge = |half_length: f64, deck_z: f64| {
        format!(
            r#"{},"bridges":[{{"deck":"bridge_deck","center":[200,150],"half_extents":[{half_length},5],"yaw":0,"deck_z":{deck_z},"thickness_m":0.8}}]"#,
            straight(24.0, 2.0, -1.5)
        )
    };
    // The deck ends 6 m past the water: a bank at the steepest grade climbs
    // 2.6 m from the waterline by there, past the land 1.5 m above it.
    assert!(river::validate(&map(&bridge(18.0, 0.1)).unwrap()).is_ok());
    // A deck ending over the water has nothing to stand its end on.
    let message = refusal(&bridge(11.0, 0.1));
    assert!(message.contains("too near the water"), "{message}");
    // Three metres out the bank has climbed 1.3 m: short of the land.
    assert!(river::validate(&map(&bridge(15.0, 0.1)).unwrap()).is_err());
    // A deck sunk below the land needs the bank only as high as itself.
    assert!(river::validate(&map(&bridge(15.0, -0.5)).unwrap()).is_ok());
    // A crest along the water's edge deepens the cut: the bank is cut from
    // the crest's height, so the long deck's end is no longer clear of it.
    let crest = |x: f64| {
        format!(r#"{{"kind":"mesa","rect":[{x},-50,4,400],"height_m":2,"side_degrees":60}}"#)
    };
    let crested = format!(
        r#","relief":[{},{}]{}"#,
        crest(186.0),
        crest(210.0),
        bridge(18.0, 0.1)
    );
    let message = refusal(&crested);
    assert!(message.contains("too near the water"), "{message}");
    // A bridge on a map without water needs nothing.
    assert!(river::validate(
        &map(r#","bridges":[{"deck":"bridge_deck","center":[200,150],"half_extents":[18,5],"yaw":0,"deck_z":3,"thickness_m":0.8}]"#).unwrap()
    )
    .is_ok());
}
