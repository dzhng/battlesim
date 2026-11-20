//! A side sees flight and its burst in the round's band, without ground vision.

use super::*;
use serde_json::json;

#[test]
fn a_height_bounded_artillery_rocket_is_seen_through_low_air_sight() {
    for sees_low_air in [true, false] {
        let mut rules = crate::fixtures::test_rules();
        rules["weapons"]["test_rocket"] = json!({
            "extends": "artillery_rocket_light",
            "arc_height_m": 65,
            "speed_mps": 200,
            "targets": {"ground": {"range_m": 1000, "scatter_mrad": 12}}
        });
        crate::fixtures::patch_catalog(
            &mut rules,
            "units",
            "test_jeep",
            json!({"mounts": [{"id": "HMG", "weapons": ["test_rocket"]}]}),
        );
        crate::fixtures::patch_catalog(
            &mut rules,
            "units",
            "test_tank",
            json!({"sensors": {"ground_m": 20, "low_air_m": if sees_low_air {1500} else {0}, "high_air_m": 1500}}),
        );
        let setup = serde_json::from_value(json!({
            "map": {"size": [1400, 500], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35},
            "rules": rules,
            "units": [
                {"side": "red", "kind": "test_jeep", "position": [100, 200], "engagement": "return_fire_only"},
                {"side": "blue", "kind": "test_tank", "position": [550, 300], "engagement": "return_fire_only"}
            ]
        })).unwrap();
        let mut battle = Battle::new(&setup, 17);
        let ack = battle.accept(CommandEnvelope {
            side: Side::Red,
            player: None,
            seq: 1,
            queued: false,
            order: Order::Attack {
                units: vec![UnitId(0)],
                target: TargetRef::Ground {
                    point: [1000.0, 200.0, 0.0],
                },
            },
        });
        assert_eq!(ack.error, None);
        let mut launch_tick = None;
        let mut apex: f64 = 0.0;
        let mut landed = false;
        for _ in 0..450 {
            battle.step();
            let owner = battle.observe(Side::Red);
            let enemy = battle.observe(Side::Blue);
            for segment in owner.projectiles.iter().filter(|segment| segment.own) {
                launch_tick.get_or_insert(owner.tick);
                let top = segment.path.iter().map(|p| p[2]).fold(0.0, f64::max);
                apex = apex.max(top);
                if top > battle.rules.air.rotor.ceiling_agl_m && sees_low_air {
                    assert!(enemy
                        .projectiles
                        .iter()
                        .any(|p| !p.own && p.path == segment.path));
                }
            }
            assert!(sees_low_air || enemy.projectiles.is_empty());
            if let Some(impact) = owner.blasts.first() {
                assert!((impact.point[0] - 1000.0).abs() < 36.0, "{impact:?}");
                assert!((impact.point[1] - 200.0).abs() < 36.0, "{impact:?}");
                assert!(apex > battle.rules.air.rotor.ceiling_agl_m);
                assert!(apex < battle.rules.air.high_air.floor_agl_m);
                assert!(owner.tick - launch_tick.unwrap() < 12 * u64::from(battle.rules.tick_hz));
                assert_eq!(
                    enemy.blasts.first().map(|blast| blast.point),
                    sees_low_air.then_some(impact.point)
                );
                assert!(!enemy
                    .ground_visibility
                    .visible(impact.point[0], impact.point[1]));
                landed = true;
                break;
            }
        }
        assert!(landed, "the rocket burst at its ordered target");
    }
}

fn airborne_round(
    layer: AltitudeLayer,
    sensors: [f64; 3],
    height: f64,
    point: Option<V3>,
) -> Battle {
    let mut rules = crate::fixtures::test_rules();
    crate::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jeep",
        json!({"sensors": {"ground_m": sensors[0], "low_air_m": sensors[1], "high_air_m": sensors[2]}}),
    );
    rules["weapons"]["grenade"]["damage"] = json!(0);
    rules["weapons"]["grenade"]["structural_damage"] = json!(0);
    let setup: ScenarioDefinition = serde_json::from_value(json!({
        "map": {"size": [2000, 500], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35},
        "rules": rules,
        "units": [
            {"side": "blue", "kind": "test_jeep", "position": [100, 200], "engagement": "return_fire_only"},
            {"side": "red", "kind": "test_tank", "position": [1800, 350], "engagement": "return_fire_only"}
        ]
    })).unwrap();
    let mut battle = Battle::new(&setup, 17);
    let weapon = battle
        .arsenal
        .weapons
        .iter()
        .position(|w| w.id == "grenade")
        .unwrap();
    let id = battle.projectiles.launch(flight::Launch {
        origin: v3(500.0, 200.0, height),
        carrier: V3::default(),
        velocity: v3(if point.is_some() { -60.0 } else { 60.0 }, 0.0, -20.0),
        gravity_scale: 0.0,
        lifetime_s: 10.0,
        suppression_radius_m: 0.0,
        shooter: None,
        guidance: point.map(|point| flight::Guidance {
            point,
            turn_rad_s: 0.0,
            supported: false,
            launch_hold_s: 0.0,
            loft: None,
        }),
        motor: None,
        fall: None,
    });
    battle.rounds.insert(
        id,
        Round {
            weapon,
            unit: UnitId(1),
            side: Side::Red,
            seeker_target: None,
            aimed_at: None,
            layer,
            expected_arrival_tick: 0.0,
        },
    );
    battle
}

#[test]
fn a_missile_warning_uses_the_incoming_rounds_air_band() {
    use contract::observation::{ShoutSpeaker, ShoutTrigger};
    let mut battle = airborne_round(
        AltitudeLayer::HighAir,
        [50.0, 0.0, 1500.0],
        100.0,
        Some(v3(100.0, 200.0, 0.0)),
    );
    battle.step();
    let frame = battle.observe(Side::Blue);
    assert!(
        frame.projectiles.iter().any(|p| !p.own),
        "the round is actually visible"
    );
    assert!(
        frame
            .shouts
            .iter()
            .any(|s| s.speaker == ShoutSpeaker::Own(UnitId(0))
                && s.trigger == ShoutTrigger::B15IncomingMissile),
        "the threatened crew sees the HighAir round and calls it"
    );
}

#[test]
fn an_airborne_round_and_its_ground_burst_are_seen_without_revealing_ground() {
    let mut battle = airborne_round(AltitudeLayer::HighAir, [50.0, 0.0, 1500.0], 100.0, None);
    battle.step();
    let frame = battle.observe(Side::Blue);
    assert!(
        frame.projectiles.iter().any(|p| !p.own),
        "high-air sight sees the enemy round over unseen ground"
    );
    assert!(!frame.ground_visibility.visible(500.0, 200.0));
    assert!(frame.identified.is_empty(), "the launcher remains unknown");
    let mut burst = None;
    for _ in 0..400 {
        battle.step();
        let frame = battle.observe(Side::Blue);
        if let Some(actual) = battle.observe(Side::Red).blasts.first() {
            assert_eq!(
                frame.blasts.first().map(|b| b.point),
                Some(actual.point),
                "the side sees the same physical burst"
            );
        }
        if let Some(blast) = frame.blasts.first() {
            assert!(
                frame
                    .projectiles
                    .iter()
                    .any(|p| p.hit == SegmentHit::Ground),
                "the visible flight reaches its burst"
            );
            assert!(!frame
                .ground_visibility
                .visible(blast.point[0], blast.point[1]));
            assert!(frame.identified.is_empty());
            burst = Some(blast.point);
            break;
        }
    }
    let at = burst.expect("the high-air round's ground impact remains visible");
    assert!(
        at[0] > 700.0 && at[2].abs() < 0.01,
        "the burst is on unseen terrain: {at:?}"
    );
}

#[test]
fn seen_ground_does_not_reveal_a_round_in_an_air_band_the_side_cannot_sense() {
    for (layer, sensors) in [
        (AltitudeLayer::LowAir, [1500.0, 0.0, 1500.0]),
        (AltitudeLayer::HighAir, [1500.0, 1500.0, 0.0]),
    ] {
        let mut battle = airborne_round(layer, sensors, 100.0, None);
        let mut landed = false;
        for _ in 0..400 {
            battle.step();
            let frame = battle.observe(Side::Blue);
            assert!(frame.ground_visibility.visible(500.0, 200.0));
            assert!(
                frame.projectiles.is_empty(),
                "{layer:?}: ground sight and the other air band cannot reveal flight"
            );
            assert!(
                frame.blasts.is_empty(),
                "{layer:?}: the burst keeps its round's band"
            );
            if !battle.observe(Side::Red).blasts.is_empty() {
                landed = true;
                break;
            }
        }
        assert!(landed, "the unseen projectile actually hit the ground");
    }
}

#[test]
fn a_round_consumed_on_its_first_tick_still_publishes_its_air_band_burst() {
    for (layer, sensors) in [
        (AltitudeLayer::LowAir, [50.0, 1500.0, 0.0]),
        (AltitudeLayer::HighAir, [50.0, 0.0, 1500.0]),
    ] {
        let mut battle = airborne_round(layer, sensors, 0.5, None);
        battle.step();
        assert!(
            battle.rounds.is_empty(),
            "the impact consumed the round this tick"
        );
        let owner = battle.observe(Side::Red);
        let frame = battle.observe(Side::Blue);
        let actual = owner
            .blasts
            .first()
            .expect("an actual first-tick burst")
            .point;
        assert_eq!(
            frame.blasts.first().map(|b| b.point),
            Some(actual),
            "{layer:?}: visibility survives round removal"
        );
        assert!(frame
            .projectiles
            .iter()
            .any(|p| p.hit == SegmentHit::Ground));
        assert!(!frame.ground_visibility.visible(actual[0], actual[1]));
        assert!(frame.identified.is_empty());
    }
}
