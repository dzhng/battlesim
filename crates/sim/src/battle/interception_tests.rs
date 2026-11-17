//! Guns and missiles that shoot down rounds in flight, and the flares
//! aircraft dispense against them.

use super::*;

/// A blue jeep at (100, 200) mounting `row` (patched into the rules as
/// `test_interceptor`), and a still, interceptable red round hanging at
/// `at`, assigned to `layer`.
fn interceptor_and_round(row: serde_json::Value, at: V3, layer: AltitudeLayer) -> Battle {
    interceptor_and_flying_round(row, at, v3(0.0, 0.0, 0.0), 30.0, layer).0
}

/// As [`interceptor_and_round`], the round flying at `velocity` without
/// falling, for `lifetime_s`; with its id.
fn interceptor_and_flying_round(
    row: serde_json::Value,
    at: V3,
    velocity: V3,
    lifetime_s: f64,
    layer: AltitudeLayer,
) -> (Battle, ProjectileId) {
    carried_interceptor_and_round(
        "test_jeep",
        row,
        at,
        velocity,
        lifetime_s,
        layer,
        serde_json::json!([]),
    )
}

/// As [`interceptor_and_flying_round`], the interceptor carried by blue's
/// `carrier` (a test unit whose mounts the row's mount follows), which
/// sees 1,500 m at every height, so it sees the round.
fn carried_interceptor_and_round(
    carrier: &str,
    row: serde_json::Value,
    at: V3,
    velocity: V3,
    lifetime_s: f64,
    layer: AltitudeLayer,
    props: serde_json::Value,
) -> (Battle, ProjectileId) {
    let mut rules = crate::fixtures::test_rules();
    rules["weapons"]["test_interceptor"] = row;
    rules["weapons"]["test_interceptor"]["round_cost"] = serde_json::json!(1);
    crate::fixtures::patch_catalog(
        &mut rules,
        "units",
        carrier,
        serde_json::json!({
            "sensors": { "ground_m": 1500, "low_air_m": 1500, "high_air_m": 1500 },
            "mounts": [{ "id": "AA", "name": "AA", "weapons": ["test_interceptor"], "turret": true, "pivot_m": [0, 0, 1.68], "muzzle_m": [1.43, 0, 0.32] }]
        }),
    );
    let setup: ScenarioDefinition = serde_json::from_value(serde_json::json!({
        "map": { "size": [2400, 400], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
            "props": props },
        "rules": rules,
        "units": [
            { "side": "blue", "kind": carrier, "position": [100, 200] },
            { "side": "red", "kind": "test_tank", "position": [2300, 200] }
        ],
        "events": [], "scripts": []
    }))
    .expect("interceptor scenario");
    let mut battle = Battle::new(&setup, 17);
    let projectile = launch_red_round(&mut battle, at, velocity, lifetime_s, layer);
    (battle, projectile)
}

/// A red interceptable round at `at`, flying at `velocity` without
/// falling for `lifetime_s`, from red's unit 1 and assigned to `layer`.
fn launch_red_round(
    battle: &mut Battle,
    at: V3,
    velocity: V3,
    lifetime_s: f64,
    layer: AltitudeLayer,
) -> ProjectileId {
    let incoming = battle
        .arsenal
        .weapons
        .iter()
        .position(|weapon| weapon.def.interceptable)
        .expect("an interceptable row");
    let projectile = battle.projectiles.launch(crate::flight::Launch {
        origin: at,
        carrier: v3(0.0, 0.0, 0.0),
        velocity,
        gravity_scale: 0.0,
        lifetime_s,
        suppression_radius_m: 0.0,
        shooter: None,
        guidance: None,
        motor: None,
        fall: None,
    });
    battle.rounds.insert(
        projectile,
        Round {
            weapon: incoming,
            unit: UnitId(1),
            side: Side::Red,
            seeker_target: None,
            aimed_at: None,
            layer,
        },
    );
    projectile
}

/// A jeep (unit 0) at (1000, 1000) with a gun (its mount 1) firing at
/// 600 m/s out to 800 m, at helicopters and at rounds assigned to low
/// air, locked on a red helicopter hovering 300 m east (unit 1), too
/// tough to bring down here.
fn interceptor_on_a_helicopter() -> Battle {
    let mut rules = crate::fixtures::test_rules();
    let mut row = test_interceptor(serde_json::json!({
        "low_air": { "classes": ["air_rotor", "missile"] }
    }));
    row["targets"]["low_air"]["range_m"] = serde_json::json!(800);
    rules["weapons"]["test_interceptor"] = row;
    rules["weapons"]["test_interceptor"]["round_cost"] = serde_json::json!(1);
    crate::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jeep",
        serde_json::json!({
            "mounts": [{ "id": "AA", "name": "AA", "weapons": ["test_interceptor"], "turret": true, "pivot_m": [0, 0, 1.68], "muzzle_m": [1.43, 0, 0.32] }]
        }),
    );
    crate::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_heli",
        serde_json::json!({ "body": { "hull": { "hp": 1.0e9 } } }),
    );
    let setup: ScenarioDefinition = serde_json::from_value(serde_json::json!({
        "map": { "size": [2400, 2400], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": rules,
        "units": [
            { "side": "blue", "kind": "test_jeep", "position": [1000, 1000] },
            { "side": "red", "kind": "test_heli", "position": [1300, 1000], "engagement": "return_fire_only" }
        ],
        "events": [], "scripts": []
    }))
    .expect("interceptor scenario");
    let mut battle = Battle::new(&setup, 17);
    for _ in 0..30 {
        battle.step();
    }
    assert_eq!(
        gun_lock(&battle),
        Some(Target::Unit(UnitId(1))),
        "on the helicopter"
    );
    battle
}

/// A wreck or a withdrawn airframe dispenses nothing: a seeker still
/// flying at a helicopter that has left the battle draws no flare from
/// it, though its dispenser is ready.
#[test]
fn a_unit_out_of_the_battle_draws_no_flare() {
    let mut rules = crate::fixtures::test_rules();
    rules["weapons"]["atgm"]["aim_s"] = serde_json::json!(0.5);
    rules["weapons"]["atgm"]["seeker"] = serde_json::json!("infrared");
    rules["weapons"]["atgm"]["seeker_fov_deg"] = serde_json::json!(180);
    rules["weapons"]["atgm"]["decoyable"] = serde_json::json!(true);
    rules["weapons"]["atgm"]["targets"] = serde_json::json!({"low_air": {"classes": ["air_rotor"], "range_m": 650, "scatter_mrad": 0.5}});
    crate::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_heli",
        serde_json::json!({"capabilities": {"flares": {"capacity": 2, "reload_s": 30}}}),
    );
    let setup: ScenarioDefinition = serde_json::from_value(serde_json::json!({
        "map": {"size": [1200, 600], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35},
        "rules": rules,
        "units": [
            {"side": "blue", "kind": "test_at", "position": [40, 300]},
            {"side": "red", "kind": "test_heli", "position": [500, 300], "engagement": "return_fire_only"}
        ],
        "events": [], "scripts": []
    }))
    .unwrap();
    let mut battle = Battle::new(&setup, 7);
    for _ in 0..600 {
        battle.step();
        if battle
            .rounds
            .values()
            .any(|r| r.seeker_target == Some(Target::Unit(UnitId(1))))
        {
            break;
        }
    }
    assert!(
        battle
            .rounds
            .values()
            .any(|r| r.seeker_target == Some(Target::Unit(UnitId(1)))),
        "a seeker flies at the helicopter"
    );
    let ready = battle.units[1]
        .flares
        .expect("the helicopter carries flares");
    assert_eq!(ready.charges, 2, "no flare drawn before the lock");
    battle.units[1].retired = true;
    battle.step();
    assert_eq!(battle.units[1].flares, Some(ready));
}

/// The patched gun's lock: the jeep's own HMG is mount 0.
fn gun_lock(battle: &Battle) -> Option<Target> {
    battle.units[0].mounts[1].lock.as_ref().map(|l| l.target)
}

/// A round flying at the jeep head-on, 700 m off at 150 m/s, comes
/// first: the gun turns from the helicopter, shoots the round down as
/// soon as it has reloaded, well before the round arrives, and goes
/// back to the helicopter.
#[test]
fn a_gun_on_an_aircraft_shoots_down_a_round_closing_on_it_then_resumes() {
    let mut battle = interceptor_on_a_helicopter();
    let round = launch_red_round(
        &mut battle,
        v3(1700.0, 1000.0, 30.0),
        v3(-150.0, 0.0, 0.0),
        30.0,
        AltitudeLayer::LowAir,
    );
    let mut switched = false;
    for _ in 0..150 {
        battle.step();
        switched |= matches!(gun_lock(&battle), Some(Target::Round(r)) if r.id == round);
        if battle.projectiles.get(round).is_none() {
            break;
        }
    }
    assert!(switched, "the gun never turned to the round");
    assert!(
        battle.projectiles.get(round).is_none(),
        "the round was not shot down before it arrived"
    );
    for _ in 0..30 {
        battle.step();
    }
    assert_eq!(
        gun_lock(&battle),
        Some(Target::Unit(UnitId(1))),
        "back on the helicopter"
    );
}

/// Whether the gun ever turns from the helicopter to a round at `at`
/// flying at `velocity` over the next two seconds.
fn turns_to(at: V3, velocity: V3) -> bool {
    let mut battle = interceptor_on_a_helicopter();
    let round = launch_red_round(&mut battle, at, velocity, 30.0, AltitudeLayer::LowAir);
    (0..60).any(|_| {
        battle.step();
        matches!(gun_lock(&battle), Some(Target::Round(r)) if r.id == round)
    })
}

/// A round 100 m off speeding away at 900 m/s, faster than the gun's
/// 600 m/s shells, can't be caught: the gun keeps the helicopter.
#[test]
fn a_round_speeding_away_faster_than_the_shot_is_left_alone() {
    assert!(!turns_to(v3(1100.0, 1000.0, 30.0), v3(900.0, 0.0, 0.0)));
}

/// A round crossing 790 m off, inside the gun's 800 m, at 300 m/s: the
/// shells would meet it about 880 m off, past their reach, so the gun
/// keeps the helicopter.
#[test]
fn a_round_crossing_whose_meeting_point_lies_past_reach_is_left_alone() {
    assert!(!turns_to(v3(1790.0, 1000.0, 30.0), v3(0.0, 300.0, 0.0)));
    // The same round closing instead is taken: the reach, not the
    // crossing, decides.
    assert!(turns_to(v3(1790.0, 1000.0, 30.0), v3(-300.0, 0.0, 0.0)));
}

/// A ballistic gun that `targets` as given: 1,500 m, 40 rounds.
fn test_interceptor(mut targets: serde_json::Value) -> serde_json::Value {
    for fire in targets.as_object_mut().unwrap().values_mut() {
        let fire = fire.as_object_mut().unwrap();
        fire.entry("range_m").or_insert(serde_json::json!(1500));
        fire.entry("scatter_mrad").or_insert(serde_json::json!(16));
    }
    serde_json::json!({
        "name": "Test AA", "description": "A test row.", "icon": "autocannon", "appearance": "aa_cannon",
        "stationary": false, "speed_mps": 600, "gravity_scale": 0.5,
        "aim_s": 0.5, "reload_s": 3.5, "ammo": 40, "resupply_rounds_per_s": 1,
        "penetration": 65, "partial_from": 0.08,
        "damage": 100, "near_miss_suppression": 0.1,
        "suppression_radius_m": 6, "blast_radius_m": 2, "proximity_fuse_m": 5,
        "structural_damage": 8, "targets": targets
    })
}

/// An interceptor reaches a round as far as its row reaches the band the
/// round is assigned to, whatever height the round flies at: 300 m off,
/// a round assigned to the ground or low air is past this gun's 150 m,
/// and one assigned to high air is inside its 400 m.
#[test]
fn an_interceptor_reaches_a_round_by_its_assigned_band() {
    let row = test_interceptor(serde_json::json!({
        "ground": { "classes": ["missile"], "range_m": 150 },
        "low_air": { "classes": ["missile"], "range_m": 150 },
        "high_air": { "classes": ["missile"], "range_m": 400 }
    }));
    for (layer, reached) in [
        (AltitudeLayer::Ground, false),
        (AltitudeLayer::LowAir, false),
        (AltitudeLayer::HighAir, true),
    ] {
        let mut battle = interceptor_and_round(row.clone(), v3(400.0, 200.0, 40.0), layer);
        battle.step();
        // The jeep's own gun is mount 0; the patched AA mount follows it.
        let lock = battle.units[0].mounts[1].lock.as_ref().map(|l| l.target);
        match lock {
            Some(Target::Round(round)) => {
                assert!(reached, "{layer:?}: locked past its reach");
                assert_eq!(round.layer, layer, "aimed by the assigned band");
            }
            _ => assert!(!reached, "{layer:?}: no lock inside its reach: {lock:?}"),
        }
    }
}

/// An aircraft's gun that lists `missile` arms against a round as a
/// ground mount's does: a helicopter's, 300 m from a round assigned to
/// low air, locks on it.
#[test]
fn an_aircraft_arms_against_an_incoming_round() {
    let row = test_interceptor(serde_json::json!({
        "low_air": { "classes": ["missile"], "range_m": 400 }
    }));
    let (mut battle, round) = carried_interceptor_and_round(
        "test_heli",
        row,
        v3(400.0, 200.0, 30.0),
        v3(0.0, 0.0, 0.0),
        30.0,
        AltitudeLayer::LowAir,
        serde_json::json!([]),
    );
    battle.step();
    let locks: Vec<_> = battle.units[0]
        .mounts
        .iter()
        .filter_map(|m| m.lock.as_ref().map(|l| l.target))
        .collect();
    assert!(
        locks
            .iter()
            .any(|t| matches!(t, Target::Round(r) if r.id == round)),
        "{locks:?}"
    );
}

/// A gun shoots down a round only when its assigned layer lists `missile`.
/// A gun that fires at men and vehicles on the ground but lists `missile`
/// in high air alone fires at a round assigned to high air and lets an
/// identical one assigned to the ground fly
/// by. A gun listing `missile` on the ground fires at the ground round,
/// though it lists no soldier there: a round is no area (W10).
#[test]
fn a_gun_shoots_down_rounds_in_layers_that_list_missile() {
    let high_air = serde_json::json!({ "classes": ["air_rotor", "air_fixed_wing", "missile"] });
    let only_high = test_interceptor(serde_json::json!({
        "ground": { "classes": ["infantry", "vehicle"] },
        "high_air": high_air
    }));
    let ground_too = test_interceptor(serde_json::json!({
        "ground": { "classes": ["missile"] },
        "high_air": high_air
    }));
    for (row, layer, shoots) in [
        (&only_high, AltitudeLayer::HighAir, true),
        (&only_high, AltitudeLayer::Ground, false),
        (&ground_too, AltitudeLayer::Ground, true),
    ] {
        let mut battle = interceptor_and_round(row.clone(), v3(500.0, 200.0, 300.0), layer);
        let interceptor = battle
            .arsenal
            .weapons
            .iter()
            .position(|weapon| weapon.id == "test_interceptor")
            .unwrap();
        let mut fired = false;
        for _ in 0..90 {
            battle.step();
            fired |= battle
                .rounds
                .values()
                .any(|r| r.weapon == interceptor && r.unit == UnitId(0));
        }
        let spent = battle.units[0].mounts[1].ammo[0] < Some(40);
        let ground = &row["targets"]["ground"]["classes"];
        assert_eq!(
            (fired, spent),
            (shoots, shoots),
            "a round assigned to {layer:?}, the ground listing {ground}"
        );
    }
}

/// A guided row that lists `missile` in high air: 1,500 m, accelerating
/// from 100 to 290 m/s, turning 180°/s, a 5 m fuse; launcher-supported,
/// or homing on its own with a radar seeker.
fn test_guided_interceptor(seeker: bool) -> serde_json::Value {
    let mut row = serde_json::json!({
        "name": "Test SAM", "description": "A test row.", "icon": "aa_missile", "appearance": "sam",
        "stationary": true, "range_m": 1500, "min_range_m": 50,
        "speed_mps": 100, "accel_mps2": 95, "top_speed_mps": 290,
        "aim_s": 0.5, "reload_s": 4, "ammo": 6, "resupply_rounds_per_s": 1,
        "penetration": 300, "partial_from": 0.08,
        "damage": 100, "turn_deg_s": 180, "guidance": "stationary",
        "lifetime_s": 10, "near_miss_suppression": 0, "suppression_radius_m": 0,
        "blast_radius_m": 3, "proximity_fuse_m": 5, "structural_damage": 20,
        "targets": { "high_air": { "classes": ["air_rotor", "air_fixed_wing", "missile"], "range_m": 1500, "scatter_mrad": 6 } }
    });
    if seeker {
        row["seeker"] = serde_json::json!("radar");
        row["seeker_fov_deg"] = serde_json::json!(100);
    }
    row
}

/// The id of the first round the jeep's interceptor launched, if any.
fn interceptor_round(battle: &Battle) -> Option<ProjectileId> {
    let interceptor = battle
        .arsenal
        .weapons
        .iter()
        .position(|weapon| weapon.id == "test_interceptor")?;
    battle
        .rounds
        .iter()
        .find(|(_, r)| r.weapon == interceptor && r.unit == UnitId(0))
        .map(|(&id, _)| id)
}

/// A guided missile homes on a round as it homes on an aircraft: fired
/// at a round crossing towards the jeep from high air, both the
/// launcher-steered missile and the one with its own seeker follow it
/// and burst beside it, ending both in the same tick, long before
/// either round's flight would end.
#[test]
fn a_guided_missile_homes_on_a_round_and_bursts_beside_it() {
    for seeker in [false, true] {
        let (mut battle, incoming) = interceptor_and_flying_round(
            test_guided_interceptor(seeker),
            v3(800.0, 230.0, 300.0),
            v3(-40.0, 0.0, 0.0),
            30.0,
            AltitudeLayer::HighAir,
        );
        let mut missile = None;
        let mut ended = None;
        for tick in 0..300 {
            battle.step();
            missile = missile.or_else(|| interceptor_round(&battle));
            let gone = |id| battle.projectiles.get(id).is_none();
            if missile.is_some_and(gone) || gone(incoming) {
                ended = Some((tick, missile.is_some_and(gone), gone(incoming)));
                break;
            }
        }
        assert!(
            missile.is_some(),
            "seeker {seeker}: the missile was never launched"
        );
        assert!(
            matches!(ended, Some((tick, true, true)) if tick < 240),
            "seeker {seeker}: the missile and the round did not end together: {ended:?}"
        );
    }
}

/// A round that ends in flight is a target lost, as a unit that dies
/// is: one that strikes a wall 100 m beside it, which no lead foresees, ends
/// before the missile reaches it; the launcher lets its missile go, and
/// the missile flies on at a point it no longer renews, never at where
/// the round would have been.
#[test]
fn a_guided_missile_whose_round_ends_mid_flight_is_let_go() {
    for seeker in [false, true] {
        let wall = serde_json::json!([{ "kind": "wall", "center": [700, 330], "yaw": 0,
            "half_extents": [100, 1, 80] }]);
        let (mut battle, incoming) = carried_interceptor_and_round(
            "test_jeep",
            test_guided_interceptor(seeker),
            v3(700.0, 230.0, 40.0),
            v3(0.0, 40.0, 0.0),
            30.0,
            AltitudeLayer::HighAir,
            wall,
        );
        let mut missile = None;
        while battle.projectiles.get(incoming).is_some() {
            battle.step();
            missile = missile.or_else(|| interceptor_round(&battle));
        }
        let missile = missile.unwrap_or_else(|| panic!("seeker {seeker}: never launched"));
        battle.step();
        let point = |battle: &Battle| {
            let flying = battle.projectiles.get(missile).expect("still in flight");
            flying.guidance.expect("a guided round")
        };
        let released = point(&battle);
        assert!(!released.supported, "seeker {seeker}: still supported");
        assert!(
            released.point.z.abs() < 1e-6,
            "seeker {seeker}: a lost target releases towards ground, not its stale airborne aim: {:?}",
            released.point
        );
        assert!(
            battle.units[0].mounts[1].support.is_none(),
            "seeker {seeker}"
        );
        for _ in 0..5 {
            battle.step();
        }
        assert_eq!(
            point(&battle).point,
            released.point,
            "seeker {seeker}: it follows nothing"
        );
    }
}

/// A ground gun engaging a helicopter sends an air-layer round all the
/// way to impact; the muzzle starts on the ground, but the target is in air.
#[test]
fn a_ground_to_air_round_keeps_the_targets_layer_throughout_flight() {
    let mut battle = interceptor_on_a_helicopter();
    let id = (0..120)
        .find_map(|_| {
            battle.step();
            interceptor_round(&battle)
        })
        .expect("the gun fired at the helicopter");
    for _ in 0..300 {
        let Some(round) = battle.rounds.get(&id) else {
            return;
        };
        assert_eq!(round.layer, AltitudeLayer::LowAir);
        battle.step();
    }
    panic!("the shot never ended");
}

/// Deliberate lofting changes ground fire's engagement band. A direct
/// grenade's gravity and visible arc alone do not make it an air round.
#[test]
fn only_authored_indirect_or_top_attack_ground_fire_uses_low_air() {
    use serde_json::json;
    for (weapon, patch, expected) in [
        (
            "grenade",
            json!({"trajectory": "direct"}),
            AltitudeLayer::Ground,
        ),
        (
            "grenade",
            json!({"trajectory": "indirect"}),
            AltitudeLayer::LowAir,
        ),
        (
            "atgm",
            json!({"top_attack": {"loft_m":60,"dive_deg":40}}),
            AltitudeLayer::LowAir,
        ),
    ] {
        let mut rules = crate::fixtures::test_rules();
        for (key, value) in patch.as_object().unwrap() {
            rules["weapons"][weapon][key] = value.clone();
        }
        rules["weapons"][weapon]["damage"] = json!(0);
        rules["weapons"][weapon]["aim_s"] = json!(0.1);
        rules["weapons"][weapon]["min_range_m"] = json!(0);
        rules["weapons"][weapon]["targets"] = json!({
            "ground":{"classes":["vehicle"],"range_m":200,"scatter_mrad":16}
        });
        crate::fixtures::patch_catalog(
            &mut rules,
            "units",
            "test_jeep",
            json!({
                "sensors":{"ground_m":1000},
                "mounts":[{"id":"test","name":"Test","weapons":[weapon],"turret":true,
                    "pivot_m":[0,0,1],"muzzle_m":[1,0,0]}]
            }),
        );
        let setup = serde_json::from_value(json!({
            "map":{"size":[1200,600],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35},
            "rules":rules,
            "units":[
                {"side":"blue","kind":"test_jeep","position":[100,300]},
                {"side":"red","kind":"test_tank","position":[180,300],"engagement":"return_fire_only"}
            ],"events":[],"scripts":[]
        })).unwrap();
        let mut battle = Battle::new(&setup, 17);
        let id = (0..300)
            .find_map(|_| {
                battle.step();
                battle
                    .rounds
                    .iter()
                    .find(|(_, r)| {
                        r.unit == UnitId(0) && battle.arsenal.weapons[r.weapon].id == weapon
                    })
                    .map(|(&id, _)| id)
            })
            .unwrap_or_else(|| panic!("{weapon}: no launch"));
        assert_eq!(battle.rounds[&id].layer, expected, "{weapon}");
    }
}

/// One independently seeking round at a still ground hull, far enough away
/// to change its tracking conditions before any physical collision.
fn unit_missile(seeker: bool, patch: serde_json::Value) -> (Battle, ProjectileId) {
    let mut row = test_guided_interceptor(seeker);
    for (key, value) in patch.as_object().unwrap() {
        row[key] = value.clone();
    }
    row["targets"] =
        serde_json::json!({"ground":{"classes":["vehicle"],"range_m":1500,"scatter_mrad":0.5}});
    row["ammo"] = serde_json::json!(1);
    let (mut battle, _) = interceptor_and_flying_round(
        row,
        v3(2300.0, 300.0, 300.0),
        V3::default(),
        30.0,
        AltitudeLayer::HighAir,
    );
    battle.units[1].position = v3(700.0, 200.0, 0.0);
    battle.units[1].engagement = Engagement::ReturnFireOnly;
    let id = (0..300)
        .find_map(|_| {
            battle.step();
            interceptor_round(&battle)
        })
        .expect("the seeker launched");
    (battle, id)
}

#[test]
fn a_seeker_loses_a_target_outside_its_flight_cone_and_never_takes_it_back() {
    let (mut battle, id) = unit_missile(true, serde_json::json!({"turn_deg_s":180}));
    let original = battle.units[1].position;
    // The missile is heading east. The target moves well outside its
    // authored 100-degree cone, while remaining alive on the field.
    battle.units[1].position = v3(200.0, 390.0, 0.0);
    battle.step();
    let released = battle.projectiles.get(id).unwrap().guidance.unwrap().point;
    assert!(
        released.y < 250.0,
        "released ahead instead of following across its cone: {released:?}"
    );
    battle.units[1].position = original;
    for _ in 0..30 {
        battle.step();
        if let Some(p) = battle.projectiles.get(id) {
            if let Some(g) = p.guidance {
                assert_eq!(g.point, released, "returning to the cone cannot reacquire");
            }
            assert!(p.velocity.x > 0.0, "it flies onward");
        }
    }
}

#[test]
fn a_launcher_frees_its_mount_after_its_missile_passes_a_still_tracked_target() {
    let (mut battle, id) = unit_missile(false, serde_json::json!({"turn_deg_s":1}));
    battle.units[1].position.y += 60.0;
    for _ in 0..300 {
        battle.step();
        let p = battle
            .projectiles
            .get(id)
            .expect("the wide miss is still physical flight");
        if p.guidance.is_none() {
            battle.step();
            assert!(
                battle.units[0].mounts.iter().all(|m| m.support.is_none()),
                "a terminal miss no longer occupies its launcher"
            );
            return;
        }
    }
    panic!("the missile never ended its missed attempt");
}

#[test]
fn an_autonomous_top_attack_keeps_its_loft_and_hits_after_its_launcher_moves_and_loses_sight() {
    let (mut battle, id) = unit_missile(
        true,
        serde_json::json!({
            "turn_deg_s":360,"top_attack":{"loft_m":60,"dive_deg":40}
        }),
    );
    let hp = battle.units[1].hp;
    let launcher = battle.units[0].position;
    let ack = battle.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 1,
        queued: false,
        order: Order::Move {
            units: vec![UnitId(0)],
            gesture: 1,
            goal: [100.0, 50.0],
            route: RoutePolicy::Shortest,
            direction: contract::command::MoveDirection::Forward,
            facing: None,
        },
    });
    assert_eq!(ack.error, None);
    // The missile is east of this screen; the launcher stays west of it.
    // The seeker's own track must remain independent of the operator's LOS.
    battle.world.add_prop(
        &serde_json::from_value(serde_json::json!({
            "kind":"wall","center":[100.5,200],"yaw":0,"half_extents":[0.1,150,8]
        }))
        .unwrap(),
    );
    assert!(
        battle.projectiles.get(id).unwrap().position.x > 100.6,
        "screen behind the missile"
    );
    assert!(
        !weapons::guidance_clear(
            &battle.world,
            &battle.rules,
            &battle.units[0],
            &battle.units[0].mounts[1],
            battle.units[1].position
        ),
        "screen blocks launcher LOS"
    );
    let mut apex = 0.0_f64;
    for _ in 0..300 {
        battle.step();
        if let Some(p) = battle.projectiles.get(id) {
            apex = apex.max(p.position.z);
        } else {
            break;
        }
    }
    assert!(
        apex > 30.0,
        "the autonomous missile keeps its planned loft: {apex}"
    );
    assert_ne!(battle.units[0].position, launcher, "the launcher moved");
    assert!(
        battle.units[1].hp < hp,
        "the independent seeker hits through the roof"
    );
}

#[test]
fn losing_the_cone_does_not_erase_a_close_physical_hit() {
    let (mut battle, id) = unit_missile(true, serde_json::json!({"turn_deg_s":180}));
    let at = battle.projectiles.get(id).unwrap().position;
    // The target centre is outside the seeker's cone, but its hull already
    // crosses the live round. Losing homing cannot erase that physical hit.
    battle.units[1].position = v3(at.x + 0.1, at.y + 0.5, 0.0);
    let hp = battle.units[1].hp;
    battle.step();
    assert!(
        battle.projectiles.get(id).is_none(),
        "the physical hit consumes the missile"
    );
    assert!(
        battle.units[1].hp < hp,
        "normal impact damage survives tracking loss"
    );
    assert!(battle.flight_events.iter().any(|event| matches!(event,
        FlightEvent::Impact(hit) if hit.projectile == id && matches!(hit.struck, Struck::Body(_))
    )),"it physically struck the hull");
}
