//! Fire at unlocated contacts and the priority of an identified target.
use super::*;

/// Blue's rifle squad; a red squad hidden behind a low building 250 m east
/// fires every second, leaving firing areas (never identified). From tick 30
/// a red tank drives in from beyond sight, to stand in the open 290 m off.
fn an_area_then_a_tank() -> Battle {
    let fire: Vec<Value> = (0..40)
        .map(|k| json!({ "tick": 5 + k * 30, "fire": { "unit": 1 } }))
        .collect();
    let mut setup = scenario_with(
        &map(json!([crate::common::building(
            0,
            "china-shed-15x24",
            [330.0, 300.0],
            0.0
        )])),
        json!([
            { "side": "blue", "kind": "test_rifle", "position": [100, 300] },
            { "side": "red", "kind": "test_rifle", "position": [350, 300], "engagement": "return_fire_only" },
            { "side": "red", "kind": "test_tank", "position": [760, 150], "yaw": std::f64::consts::PI,
              "engagement": "return_fire_only" },
        ]),
        json!(fire),
        json!([{ "tick": 30, "side": "red", "order":
            { "kind": "move", "units": [2], "gesture": 1, "goal": [350, 150], "route": "shortest" } }]),
    );
    // This selection experiment needs a lob steep enough to clear the shed's
    // 6 m roof and drop behind it, independent of tuning.
    let grenade = setup.rules.weapons.get_mut("grenade").unwrap();
    (
        grenade.ballistics.speed_mps,
        grenade.ballistics.gravity_scale,
    ) = (16.0, 0.09);
    // Its damage cuts off short of the tank's armour: the tank is what it
    // cannot hurt.
    grenade.partial_from = 0.6;
    Battle::new(&setup, 5)
}

#[test]
fn an_area_draws_fire_only_while_no_identified_enemy_is_in_range() {
    let mut b = an_area_then_a_tank();
    let grenade_range = b.rules().weapons["grenade"].ballistics.range_m;
    // Where the squad started: its soldiers shift a few metres into cover.
    let blue = sim::math::v2(100.0, 300.0);
    let mut grenade_at_area = false;
    let mut grenade_after_tank = Vec::new();
    let mut rifles_on_tank = false;
    for _ in 0..ticks(60.0) {
        let shots = run(&mut b, 1);
        let seen = b.observe(Side::Blue).identified.clone();
        let tank_in_reach = seen.iter().any(|e| {
            (sim::math::v2(e.position[0], e.position[1]) - blue).length() <= grenade_range - 10.0
        });
        let (rifles, grenade) = (mount(&b, Side::Blue, 0, 0), mount(&b, Side::Blue, 0, 1));
        if seen.is_empty() {
            grenade_at_area |= matches!(grenade.target, Some(TargetRef::Contact { .. }))
                && !shots_by(&shots, 0, "grenade").is_empty();
        }
        if tank_in_reach {
            grenade_after_tank.extend(shots_by(&shots, 0, "grenade"));
            for m in [&rifles, &grenade] {
                assert!(
                    !matches!(m.target, Some(TargetRef::Contact { .. })),
                    "tick {}: an area held with a tank in reach: {m:?}",
                    b.tick()
                );
            }
            rifles_on_tank |= rifles.target == Some(TargetRef::Identified { id: seen[0].id });
        }
    }
    assert!(
        grenade_at_area,
        "with no enemy in sight the grenade answers the area"
    );
    assert!(
        rifles_on_tank,
        "the rifles take the tank they cannot hurt over the area"
    );
    assert!(
        grenade_after_tank.is_empty(),
        "no grenade spent while a tank it cannot hurt is in reach: {grenade_after_tank:?}"
    );
}

#[test]
fn areas_draw_only_general_purpose_fire_and_never_ap() {
    // A red squad 700 m away, beyond every blue unit's optics, fires at
    // nothing; blue's tank and AT squad hold only its firing area.
    let fire: Vec<Value> = (0..20)
        .map(|k| json!({ "tick": 5 + k * 30, "fire": { "unit": 2 } }))
        .collect();
    let mut b = battle(
        json!([]),
        json!([
            { "side": "blue", "kind": "test_tank", "position": [120, 300] },
            { "side": "blue", "kind": "test_at", "position": [120, 250] },
            { "side": "red", "kind": "test_rifle", "position": [820, 300], "engagement": "return_fire_only" },
        ]),
        json!(fire),
        json!([]),
    );
    let shots = run(&mut b, 600);
    assert!(b.observe(Side::Blue).identified.is_empty(), "never seen");
    assert!(
        !shots_by(&shots, 0, "tank_he").is_empty(),
        "the cannon answers the area with HE"
    );
    assert!(
        shots_by(&shots, 0, "tank_ap").is_empty(),
        "never AP at an area"
    );
    assert!(
        shots_by(&shots, 1, "atgm").is_empty(),
        "the ATGM is kept for identified armour"
    );
    // The AP round loaded at the start was swapped out, not lost.
    let cannon = mount(&b, Side::Blue, 0, 0);
    assert_eq!(
        cannon.ammo[0],
        Some(weapon("tank_ap")["ammo"].as_u64().unwrap() as u32)
    );
}
