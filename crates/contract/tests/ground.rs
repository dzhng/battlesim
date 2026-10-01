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
