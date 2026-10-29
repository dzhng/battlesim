use super::*;

/// A corridor too tight for the truck's turning circle round its corner:
/// it gets as far as it can and gives up, never up a cliff.
#[test]
fn a_truck_gives_up_on_a_corner_too_tight_to_turn() {
    use contract::observation::MoveState;
    let mut setup = common::scenario(
        &json!({"size":[1000,500], "fog_cell_m":8, "height_grid_m":1,
        "slope_cutoff_deg":35, "relief":[
                {"kind":"mesa", "rect":[0,24,490,476], "height_m":100, "side_degrees":89},
                {"kind":"mesa", "rect":[514,0,486,500], "height_m":100, "side_degrees":89}
        ]})
        .to_string(),
        json!([{"side":"blue", "kind":"test_supply", "position":[100,12], "yaw":0}]),
        json!([]),
    );
    let mut rules = common::scenario_rules();
    // Its hull is past what the game admits.
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_supply",
        json!({
            "mobility":{"wheeled":{"offroad_kmh":25, "road_kmh":76,
                "turn_deg_s":40, "turning_radius_m":50, "reverse_speed_factor":0.35}}
        }),
    );
    setup.rules = serde_json::from_value(rules).unwrap();
    setup
        .rules
        .catalog
        .check_limits(
            sim::fixtures::lift_hull_limits(),
            contract::AirSpeedLimits::SHIPPED,
        )
        .unwrap();
    let mut battle = Battle::new(&setup, 1);
    let ack = battle.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 1,
        queued: false,
        order: move_to(&[0], [502.0, 400.0]),
    });
    assert_eq!(
        ack.error, None,
        "the corridor is one piece of ground: {ack:?}"
    );
    let ended = settle(&mut battle, 0, 600);
    let truck = battle.unit(UnitId(0)).unwrap();
    let p = truck.position.xy();
    assert!(
        battle.world().traversable_at(p.x, p.y),
        "it is never up a cliff: {p:?}"
    );
    assert_ne!(ended, MoveState::Moving, "it settles, at {p:?}");
}
