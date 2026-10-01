use contract::map::SurfaceArea;

#[test]
fn shared_ground_coordinates_keep_the_original_decimal_tokens() {
    use contract::ground::GroundShape;
    let expected = 0x4044_0000_0000_5ccd;
    let polygon: SurfaceArea = serde_json::from_str(
        r#"{"kind":"road","shape":{"kind":"polygon","ring":[[40.000000000168804,0],[50,0],[50,10],[40.000000000168804,10]]}}"#,
    )
    .unwrap();
    let GroundShape::Polygon { ring } = polygon.shape else {
        unreachable!()
    };
    assert_eq!(ring[0][0].to_bits(), expected);
    let stroke: SurfaceArea = serde_json::from_str(
        r#"{"kind":"road","shape":{"kind":"stroke","points":[[40.000000000168804,0],[50,0]],"width_m":40.000000000168804}}"#,
    )
    .unwrap();
    let GroundShape::Stroke {
        centerline,
        width_m,
    } = stroke.shape
    else {
        unreachable!()
    };
    assert_eq!(centerline.control_points()[0][0].to_bits(), expected);
    assert_eq!(width_m.to_bits(), expected);
}

#[test]
fn an_actual_rectangle_crossing_zero_keeps_the_rectangle_fast_path() {
    let shape = contract::ground::GroundShape::polygon(vec![
        [-10_000.0, -2.0],
        [0.1, -2.0],
        [0.1, 3.0],
        [-10_000.0, 3.0],
    ])
    .unwrap();
    assert_eq!(
        shape.exact_rectangle(),
        Some([-10_000.0, -2.0, 10_000.1, 5.0])
    );
    assert!(
        shape.contains([0.1, 1.0], 0.0),
        "the actual east boundary is closed"
    );
}

#[test]
fn a_forest_is_only_its_shape() {
    // A second rectangle must not silently disappear beside the shape, and a
    // forest cannot name its own density or tree size (Q-G8b): its art could
    // then imply a sight or movement rule the other forests don't share.
    let shape = r#""shape":{"kind":"polygon","ring":[[0,0],[10,0],[10,10],[0,10]]}"#;
    assert!(serde_json::from_str::<contract::map::Forest>(&format!("{{{shape}}}")).is_ok());
    for (field, value) in [
        ("rect", "[80,80,10,10]"),
        ("density", r#""light""#),
        ("canopy_height_m", "20"),
    ] {
        let error = serde_json::from_str::<contract::map::Forest>(&format!(
            r#"{{{shape},"{field}":{value}}}"#
        ))
        .expect_err(field)
        .to_string();
        assert!(
            error.contains(&format!("unknown field `{field}`")),
            "{error}"
        );
    }
}

#[test]
fn finite_stroke_inputs_must_still_have_representable_physical_geometry() {
    for shape in [
        r#"{"kind":"stroke","points":[[0,0],[1e200,0]],"width_m":2}"#,
        r#"{"kind":"stroke","points":[[1e308,0],[1.1e308,0]],"width_m":1.7e308}"#,
        r#"{"kind":"stroke","points":[[0,0],[0,0]],"width_m":2}"#,
        r#"{"kind":"stroke","points":[[0,0],[1e-200,0]],"width_m":2}"#,
    ] {
        let input = format!(r#"{{"kind":"road","shape":{shape}}}"#);
        // Which owner refuses (the centreline or the stroke) is not the contract.
        serde_json::from_str::<SurfaceArea>(&input)
            .expect_err("finite source numbers must not admit NaN distance arithmetic");
    }
}

#[test]
fn malformed_ground_ring_is_rejected_at_decode() {
    let invalid =
        r#"{"kind":"road","shape":{"kind":"polygon","ring":[[0,0],[10,10],[0,10],[10,0]]}}"#;
    let decoded = serde_json::from_str::<SurfaceArea>(invalid);
    assert!(
        decoded.is_err(),
        "a crossing ring must not enter the physical map"
    );
}

#[test]
fn ground_area_refuses_unknown_physical_fields() {
    let input = r#"{"kind":"road","speed":900,"shape":{"kind":"stroke","points":[[0,0],[10,0]],"width_m":2}}"#;
    assert!(
        serde_json::from_str::<SurfaceArea>(input).is_err(),
        "physical area fields must not vanish silently"
    );
}

#[test]
fn physical_shape_fields_and_source_work_have_bounded_admission() {
    let unknown = r#"{"kind":"road","shape":{"kind":"stroke","points":[[0,0],[10,0]],"control_points":[[0,0],[100,0]],"width_m":2}}"#;
    assert!(serde_json::from_str::<SurfaceArea>(unknown).is_err());
    let polygon = serde_json::json!({"kind":"road","shape":{"kind":"polygon","ring":
        (0..257).map(|i| {let a=std::f64::consts::TAU*i as f64/257.0;[a.cos(),a.sin()]}).collect::<Vec<_>>()}});
    let error = serde_json::from_value::<SurfaceArea>(polygon)
        .unwrap_err()
        .to_string();
    assert!(error.contains("3..=256 vertices"), "{error}");
    let stroke = serde_json::json!({"kind":"road","shape":{"kind":"stroke","points":
        (0..4097).map(|i|[i,0]).collect::<Vec<_>>(),"width_m":2}});
    let error = serde_json::from_value::<SurfaceArea>(stroke)
        .unwrap_err()
        .to_string();
    assert!(error.contains("2..=4096 control points"), "{error}");
}

fn stroke(points: &[[f64; 2]], width_m: f64) -> contract::ground::GroundShape {
    contract::ground::GroundShape::stroke(points.to_vec(), width_m).unwrap()
}

#[test]
fn a_stroke_ends_square_across_its_first_and_last_point() {
    // A road 6 m wide that runs east from (10, 20) to (50, 20).
    let road = stroke(&[[10.0, 20.0], [50.0, 20.0]], 6.0);
    for (end, beyond) in [(10.0, -1.0), (50.0, 1.0)] {
        for across in [-3.0, 0.0, 3.0] {
            assert!(
                road.contains([end, 20.0 + across], 0.0),
                "the end face itself is road, {across} m across it at x = {end}"
            );
            assert!(
                !road.contains([end + beyond * 0.01, 20.0 + across], 0.0),
                "a centimetre past the end at x = {end} is open ground, {across} m across"
            );
        }
        assert!(
            !road.contains([end + beyond * 2.0, 20.0], 0.0),
            "no half-disc caps the end at x = {end}"
        );
    }
    assert!(road.contains([30.0, 23.0], 0.0) && !road.contains([30.0, 23.01], 0.0));
}

#[test]
fn a_stroke_that_curls_back_past_its_own_start_still_covers_that_ground() {
    // East along y = 0, north, then back west along y = 4: the last run
    // passes 4 m from the first, 2 m behind where the stroke starts.
    let lane = stroke(&[[0.0, 0.0], [40.0, 0.0], [40.0, 4.0], [-10.0, 4.0]], 6.0);
    assert!(
        lane.contains([-2.0, 1.5], 0.0),
        "behind the start, but inside the returning run"
    );
    assert!(
        !lane.contains([-2.0, -2.0], 0.0),
        "behind the start and beside nothing else"
    );
}

#[test]
fn a_bend_close_behind_an_end_does_not_round_past_it() {
    // A lane 8 m wide whose first run is 2 m long: every sample of the line
    // for its first 4 m is nearer the end than half the width, and a round
    // joint at any of them would bulge out past the end's face.
    for lane in [
        stroke(&[[0.0, 0.0], [2.0, 0.0], [40.0, 6.0]], 8.0),
        // The same end in pieces along one straight line.
        stroke(
            &[[0.0, 0.0], [1.0, 0.0], [2.0, 0.0], [3.0, 0.0], [40.0, 0.0]],
            8.0,
        ),
    ] {
        for across in [-3.5, -1.0, 0.0, 1.0, 3.5] {
            assert!(
                !lane.contains([-0.05, across], 0.0),
                "5 cm behind the end, {across} m across it"
            );
            assert!(lane.contains([0.0, across], 0.0), "on the end's face");
        }
        assert!(lane.contains([1.0, 3.9], 0.0) && lane.contains([30.0, 2.0], 0.0));
    }
}

#[test]
fn a_stroke_stays_round_at_a_bend() {
    // A right-angle turn. On the outside of the bend the paving runs round
    // the corner: the rounded line passes through the control point, so the
    // ground half a width out along the bisector is paved.
    let road = stroke(&[[0.0, 0.0], [50.0, 0.0], [50.0, 50.0]], 6.0);
    let out = 3.0 * std::f64::consts::FRAC_1_SQRT_2;
    assert!(road.contains([50.0 + out - 0.01, -out + 0.01], 0.0));
    assert!(!road.contains([50.0 + out + 0.01, -out - 0.01], 0.0));
}

#[test]
fn a_margin_grows_a_square_end_like_any_other_edge() {
    let road = stroke(&[[10.0, 20.0], [50.0, 20.0]], 6.0);
    assert!(road.contains([8.0, 20.0], 2.0), "2 m past the end face");
    assert!(!road.contains([7.99, 20.0], 2.0));
    assert!(road.contains([8.0, 23.0], 2.0), "2 m past the end's corner");
    // Diagonally off the corner (10, 23): 2 m away is in, a little more is out.
    let d = std::f64::consts::FRAC_1_SQRT_2;
    assert!(road.contains([10.0 - 1.99 * d, 23.0 + 1.99 * d], 2.0));
    assert!(!road.contains([10.0 - 2.01 * d, 23.0 + 2.01 * d], 2.0));
}
