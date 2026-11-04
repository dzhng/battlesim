//! Aircraft: a hull in the air is no ground body. Traffic drives under it,
//! soldiers neither step out of it nor lean on it, forest ground does not hide
//! it, and it is admitted against its own hull limits.

use crate::common::{json_ground_reach, Seat};
use contract::catalog::AltitudeLayer;
use contract::command::{
    CommandEnvelope, MoveDirection, Order, OrderError, RoutePolicy, TargetRef,
};
use contract::ids::{Side, UnitId};
use contract::observation::{ActionReason, ContactSource};
use contract::scenario::Rules;
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::fixtures;

/// Test rules whose catalog holds the test helicopter (`test_heli`,
/// `fixtures/units/test/aircraft.json`).
pub(crate) fn fixture() -> Value {
    fixtures::test_rules()
}

pub(crate) fn battle_with(fixture: &Value, map: Value, units: Value) -> Battle {
    scripted(fixture, map, units, json!([]), json!([]))
}

fn scripted(fixture: &Value, map: Value, units: Value, events: Value, scripts: Value) -> Battle {
    let setup = serde_json::from_value(json!({
        "map": map, "rules": fixture, "units": units, "events": events, "scripts": scripts,
    }))
    .unwrap();
    Battle::new(&setup, 1)
}

pub(crate) fn open_map() -> Value {
    json!({ "size": [600, 400], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 })
}

pub(crate) fn order(units: &[u32], goal: [f64; 2]) -> CommandEnvelope {
    CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 1,
        order: Order::Move {
            units: units.iter().map(|&u| UnitId(u)).collect(),
            gesture: 1,
            goal,
            route: RoutePolicy::Shortest,
            direction: MoveDirection::Forward,
            facing: None,
        },
        queued: false,
    }
}

#[test]
fn a_helicopter_enters_the_battle_at_cruise_height() {
    let fixture = fixture();
    let b = battle_with(
        &fixture,
        open_map(),
        json!([{ "side": "blue", "kind": "test_heli", "position": [300, 200] }]),
    );
    let heli = b.unit(UnitId(0)).unwrap();
    assert!(heli.airborne());
    assert!(heli.ground_footprint().is_none());
    let cruise = b.rules().air.low_air.cruise_agl_m;
    assert!(
        (heli.position.z - cruise).abs() < 1e-9,
        "z {}",
        heli.position.z
    );
}

#[test]
fn air_rules_refuse_a_high_air_floor_above_the_fixed_wing_ceilings() {
    let mut fixture = fixture();
    fixture["air"]["high_air"]["floor_agl_m"] = json!(250);
    let error = serde_json::from_value::<contract::scenario::Rules>(fixture)
        .expect_err("a high-air floor above a fixed-wing ceiling must be rejected");
    assert!(error.to_string().contains("high-air floor"), "{error}");
}

#[test]
fn authored_high_air_is_retained_in_runtime_layer() {
    let mut fixture = fixture();
    let doc = fixture["catalog"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|doc| doc["units"].get("test_heli").is_some())
        .unwrap();
    doc["units"]["test_heli"]["mobility"]["air"]["layer"] = json!("high_air");
    doc["units"]["test_heli"]["mobility"]["air"]["flight"] = json!("fixed_wing");
    let b = battle_with(
        &fixture,
        open_map(),
        json!([{ "side": "blue", "kind": "test_heli", "position": [300, 200] }]),
    );
    let jet = b.unit(UnitId(0)).unwrap();
    assert_eq!(jet.layer(), AltitudeLayer::HighAir);
    assert!(jet.airborne());
}

#[test]
fn a_jet_spawns_at_the_jet_cruise_height() {
    let mut fixture = fixture();
    let doc = fixture["catalog"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|doc| doc["units"].get("test_heli").is_some())
        .unwrap();
    doc["units"]["test_heli"]["mobility"]["air"]["layer"] = json!("high_air");
    doc["units"]["test_heli"]["mobility"]["air"]["flight"] = json!("fixed_wing");
    let b = battle_with(
        &fixture,
        open_map(),
        json!([{ "side": "blue", "kind": "test_heli", "position": [300, 200] }]),
    );
    let jet = b.unit(UnitId(0)).unwrap();
    let cruise = b.rules().air.high_air.cruise_agl_m;
    assert!(
        (jet.position.z - cruise).abs() < 1e-9,
        "z {}",
        jet.position.z
    );
}

/// D40: a row's own `cruise_agl_m` is where it flies. In high air it never
/// flies under the floor; in low air nothing caps it, even above high air,
/// and its layer still decides who can see and shoot it.
#[test]
fn a_cruise_override_is_the_flight_height_floored_in_high_air_and_uncapped_in_low_air() {
    let mut fixture = fixture();
    fixture["catalog"]
        .as_array_mut()
        .unwrap()
        .push(json!({ "units": {
            "test_jet_180": { "extends": "test_jet", "mobility": { "air": { "cruise_agl_m": 180 } } },
            "test_jet_50": { "extends": "test_jet", "mobility": { "air": { "cruise_agl_m": 50 } } },
            "test_heli_400": { "extends": "test_heli", "mobility": { "air": { "cruise_agl_m": 400 } } }
        } }));
    let mut b = battle_with(
        &fixture,
        open_map(),
        json!([
            { "side": "blue", "kind": "test_jet_180", "position": [300, 200] },
            { "side": "blue", "kind": "test_jet_50", "position": [300, 200] },
            { "side": "blue", "kind": "test_heli_400", "position": [100, 100] }
        ]),
    );
    let floor = b.rules().air.high_air.floor_agl_m;
    assert!(50.0 < floor, "the low override must lie under the floor");
    let heights = |b: &Battle| [0, 1, 2].map(|i| b.unit(UnitId(i)).unwrap().position.z);
    for (z, want) in heights(&b).into_iter().zip([180.0, floor, 400.0]) {
        assert!((z - want).abs() < 1e-9, "spawned at {z}, not {want}");
    }
    assert_eq!(b.accept(order(&[2], [500.0, 300.0])).error, None);
    for _ in 0..40 * b.rules().tick_hz {
        b.step();
    }
    for (z, want) in heights(&b).into_iter().zip([180.0, floor, 400.0]) {
        assert!((z - want).abs() < 1.0, "flies at {z:.1}, not {want}");
    }
    let heli = b.unit(UnitId(2)).unwrap();
    assert_eq!(heli.layer(), AltitudeLayer::LowAir);
    let at = heli.position.xy();
    assert!(
        (at.x - 500.0).hypot(at.y - 300.0) < 2.0,
        "the high-flying helicopter reached its goal: {at:?}"
    );
}

#[test]
fn air_sensor_ranges_use_the_target_layer_without_extending_ground_vision() {
    let mut fixture = fixture();
    let doc = fixture["catalog"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|doc| doc["units"].get("test_heli").is_some())
        .unwrap();
    let mut high = doc["units"]["test_heli"].clone();
    high["mobility"]["air"]["layer"] = json!("high_air");
    high["mobility"]["air"]["flight"] = json!("fixed_wing");
    doc["units"]["test_high_air"] = high;
    fixtures::patch_catalog(
        &mut fixture,
        "units",
        "test_jeep",
        json!({
            "sensors": { "ground_m": 300, "low_air_m": 1200, "high_air_m": 3000,
                "sight_shape": { "front": 1, "side": 1, "rear": 1 } }
        }),
    );
    let battle = battle_with(
        &fixture,
        json!({"size":[4000,600],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35}),
        json!([
            {"side":"blue","kind":"test_jeep","position":[100,300]},
            {"side":"red","kind":"test_high_air","position":[2500,300]},
            {"side":"red","kind":"test_heli","position":[1000,300]},
            {"side":"red","kind":"test_jeep","position":[1000,350]}
        ]),
    );
    let blue = battle.observe(Side::Blue);
    let mut kinds: Vec<_> = blue
        .identified
        .iter()
        .map(|u| battle.rules().catalog.id(u.kind))
        .collect();
    kinds.sort();
    assert_eq!(kinds, ["test_heli", "test_high_air"]);
    assert!(!blue.ground_visibility.visible(1000.0, 350.0));
}

#[test]
fn high_air_has_no_ground_vision_but_can_identify_low_air() {
    let mut fixture = fixture();
    let doc = fixture["catalog"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|doc| doc["units"].get("test_heli").is_some())
        .unwrap();
    let mut high = doc["units"]["test_heli"].clone();
    high["mobility"]["air"]["layer"] = json!("high_air");
    high["mobility"]["air"]["flight"] = json!("fixed_wing");
    doc["units"]["test_high_air"] = high;
    let b = battle_with(
        &fixture,
        open_map(),
        json!([
            { "side": "blue", "kind": "test_high_air", "position": [300, 200], "engagement": "return_fire_only" },
            { "side": "red", "kind": "test_rifle", "position": [500, 200], "engagement": "return_fire_only" },
            { "side": "red", "kind": "test_heli", "position": [500, 240], "engagement": "return_fire_only" }
        ]),
    );
    let blue = b.observe(Side::Blue);
    assert_eq!(
        blue.own[0].sees.len(),
        1,
        "the aircraft sees the low-air helicopter"
    );
    assert_eq!(
        blue.identified.len(),
        1,
        "the aircraft does not identify ground units"
    );
    assert_eq!(
        blue.identified[0].kind,
        b.rules().catalog.index("test_heli").unwrap()
    );
    assert!(
        !blue.ground_visibility.visible(300.0, 200.0),
        "high-air aircraft do not illuminate ground fog"
    );
}

/// Red's layered air defence on the air lab's ground, against `blue`: a
/// short gun, a medium SAM and a long SAM (test jeeps carrying the game's
/// three air-defence weapon rows), and with `hidden` a second gun and a
/// second medium SAM that a tower hides from blue's approach.
fn air_defence_battle(blue: Value, hidden: bool) -> Battle {
    let mut fixture = fixture();
    let aa = |name: &str, weapon: &str, high_air_m: u32, radar_emitter: bool| {
        json!({
            "extends": "test_jeep", "name": name,
            "mounts": [{ "id": "HMG", "weapons": [weapon] }],
            "sensors": { "low_air_m": 900, "high_air_m": high_air_m },
            "capabilities": {
                "radar_emitter": radar_emitter
            }
        })
    };
    fixture["catalog"]
        .as_array_mut()
        .unwrap()
        .push(json!({ "units": {
            "test_aa_short": aa("Short AA", "aa_ballistic", 900, false),
            "test_aa_medium": aa("Medium SAM", "aa_medium", 3200, true),
            "test_aa_long": aa("Long SAM", "aa_long", 7000, true),
        }}));
    let red = |kind: &str, at: [f64; 2]| json!({ "side": "red", "kind": kind, "position": at, "yaw": std::f64::consts::PI });
    let mut units = vec![
        red("test_aa_short", [470.0, 260.0]),
        red("test_aa_medium", [540.0, 200.0]),
        red("test_aa_long", [560.0, 320.0]),
    ];
    if hidden {
        units.push(red("test_aa_short", [385.0, 460.0]));
        units.push(red("test_aa_medium", [385.0, 452.0]));
    }
    units.extend(blue.as_array().unwrap().iter().cloned());
    let map = sim::maps::load("air").unwrap().definition;
    let encounter: contract::scenario::EncounterDefinition =
        serde_json::from_value(json!({ "units": units })).unwrap();
    Battle::new(
        &encounter
            .on(map, serde_json::from_value(fixture).unwrap())
            .unwrap(),
        6,
    )
}

fn shots(b: &Battle, id: UnitId) -> u32 {
    b.unit(id).unwrap().mounts.iter().map(|m| m.shots).sum()
}

/// Jeeps carrying the game's air-reaching rows fire at the layers each row
/// reaches. Behind a tower, a guided SAM holds without its own line to the
/// jet, as a guided missile does at any target; a ballistic gun is direct
/// fire like any other, with no rule of its own for the air.
#[test]
fn air_defence_follows_the_target_layer_and_only_guided_fire_needs_its_own_sight() {
    let mut helicopters = air_defence_battle(
        json!([
            { "side": "blue", "kind": "test_heli", "position": [250, 200], "engagement": "return_fire_only" },
            { "side": "blue", "kind": "test_heli", "position": [250, 330], "engagement": "return_fire_only" },
        ]),
        false,
    );
    for _ in 0..300 {
        helicopters.step();
    }
    assert!(
        shots(&helicopters, UnitId(1)) > 0,
        "medium AA fires at LowAir"
    );
    assert!(
        shots(&helicopters, UnitId(0)) > 0,
        "short AA fires at LowAir"
    );
    assert_eq!(
        shots(&helicopters, UnitId(2)),
        0,
        "long AA never fires at LowAir"
    );

    // The hidden pair may see the circling jet at times. The SAM never
    // launches on a tick it has no line of its own; the gun never holds for
    // want of one.
    let mut jet = air_defence_battle(
        json!([{ "side": "blue", "kind": "test_jet", "position": [20, 300], "engagement": "return_fire_only" }]),
        true,
    );
    let (gun, sam) = (UnitId(3), UnitId(4));
    let (mut sam_blind, mut gun_blind) = (false, false);
    for _ in 0..300 {
        let before = shots(&jet, sam);
        jet.step();
        let own = &jet.observe(Side::Red).own;
        let blind = |id: UnitId| own[id.0 as usize].mounts[0].reason == ActionReason::NoOwnSight;
        gun_blind |= blind(gun);
        if blind(sam) {
            sam_blind = true;
            assert_eq!(shots(&jet, sam), before, "a SAM without its own line holds");
        }
    }
    for id in [UnitId(0), UnitId(1), UnitId(2)] {
        assert!(shots(&jet, id) > 0, "AA {id:?} fires at HighAir");
    }
    assert!(sam_blind, "the hidden SAM explains its hold");
    assert!(!gun_blind, "a ballistic gun has no air-only sight rule");
}

/// An air-defence gun reaches as far as its weapon row says at each layer: a
/// helicopter inside its low-air reach draws fire, and one past it does not
/// though the gun sees it; a jet passing at cruise height is inside its
/// high-air reach and draws fire.
#[test]
fn an_air_defence_gun_engages_air_within_its_rows_reach_at_each_layer() {
    let (low_reach, high_reach) = (300.0, 600.0);
    let mut rules = fixture();
    rules["weapons"]["test_aa_gun"] = json!({
        "extends": "aa_ballistic",
        "range_m": high_reach,
        "targets": {
            "low_air": { "range_m": low_reach },
            "high_air": { "range_m": high_reach }
        }
    });
    json_ground_reach(&mut rules["weapons"]["test_aa_gun"], high_reach);
    rules["weapons"]["test_aa_gun"]["round_cost"] =
        rules["weapons"]["aa_ballistic"]["round_cost"].clone();
    fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jeep",
        json!({
            "mounts": [{ "id": "HMG", "weapons": ["test_aa_gun"] }],
            "sensors": { "low_air_m": 4.0 * low_reach, "high_air_m": 4.0 * high_reach }
        }),
    );
    let cruise = serde_json::from_value::<Rules>(rules.clone())
        .unwrap()
        .air
        .high_air
        .cruise_agl_m;
    assert!(
        cruise < high_reach,
        "a jet at {cruise} m is inside the {high_reach} m high-air reach"
    );
    let span = 1.5 * high_reach;
    let map = json!({ "size": [4.0 * span, 2.0 * span], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 });
    let at = [2.0 * span, span];
    // The gun faces east; the aircraft starts east of it, and a mover flies
    // west over it.
    let duel = |kind: &str, offset_m: f64, pass: bool| {
        let mut b = battle_with(
            &rules,
            map.clone(),
            json!([
                { "side": "red", "kind": "test_jeep", "position": at, "yaw": 0 },
                { "side": "blue", "kind": kind, "position": [at[0] + offset_m, at[1]],
                  "yaw": std::f64::consts::PI, "engagement": "return_fire_only" },
            ]),
        );
        if pass {
            b.step();
            let ack = b.accept(order(&[1], [at[0] - offset_m, at[1]]));
            assert_eq!(ack.error, None, "{ack:?}");
        }
        let (mut farthest_shot_m, mut seen): (f64, bool) = (0.0, false);
        for _ in 0..300 {
            let before = shots(&b, UnitId(0));
            b.step();
            seen |= !b.observe(Side::Red).identified.is_empty();
            if shots(&b, UnitId(0)) > before {
                let (gun, target) = (b.unit(UnitId(0)).unwrap(), b.unit(UnitId(1)).unwrap());
                farthest_shot_m = farthest_shot_m.max((target.position - gun.position).length());
            }
        }
        (shots(&b, UnitId(0)), farthest_shot_m, seen)
    };

    let (near, near_m, _) = duel("test_heli", 0.5 * low_reach, false);
    assert!(
        near > 0,
        "a helicopter at half the low-air reach draws fire"
    );
    assert!(near_m <= low_reach, "fired at a helicopter {near_m} m away");
    let (past, _, seen) = duel("test_heli", 1.5 * low_reach, false);
    assert!(seen, "the gun sees the helicopter past its reach");
    assert_eq!(past, 0, "a helicopter past the low-air reach draws no fire");

    let (jet, jet_m, _) = duel("test_jet", span, true);
    assert!(jet > 0, "a jet passing at cruise height draws fire");
    assert!(jet_m <= high_reach, "fired at a jet {jet_m} m away");
}

#[test]
fn a_jet_draws_fire_only_from_a_row_listing_fixed_wing_aircraft() {
    let reach = 600.0;
    let passes = |classes: Value| {
        let mut rules = fixture();
        rules["weapons"]["test_aa_gun"] = json!({
            "extends": "aa_ballistic",
            "range_m": reach,
            "targets": { "high_air": { "range_m": reach, "scatter_mrad": 6, "classes": classes } }
        });
        json_ground_reach(&mut rules["weapons"]["test_aa_gun"], reach);
        rules["weapons"]["test_aa_gun"]["round_cost"] =
            rules["weapons"]["aa_ballistic"]["round_cost"].clone();
        fixtures::patch_catalog(
            &mut rules,
            "units",
            "test_jeep",
            json!({
                "mounts": [{ "id": "HMG", "weapons": ["test_aa_gun"] }],
                "sensors": { "high_air_m": 4.0 * reach }
            }),
        );
        let span = 1.5 * reach;
        let at = [2.0 * span, span];
        let mut b = battle_with(
            &rules,
            json!({ "size": [4.0 * span, 2.0 * span], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 }),
            json!([
                { "side": "red", "kind": "test_jeep", "position": at, "yaw": 0 },
                { "side": "blue", "kind": "test_jet", "position": [at[0] + span, at[1]],
                  "yaw": std::f64::consts::PI, "engagement": "return_fire_only" },
            ]),
        );
        b.step();
        let ack = b.accept(order(&[1], [at[0] - span, at[1]]));
        assert_eq!(ack.error, None, "{ack:?}");
        for _ in 0..300 {
            b.step();
        }
        shots(&b, UnitId(0))
    };
    assert_eq!(
        passes(json!(["air_rotor"])),
        0,
        "a rotor-only row lets the jet pass"
    );
    assert!(
        passes(json!(["air_fixed_wing"])) > 0,
        "a fixed-wing row fires at it"
    );
}

/// A round keeps the band its shooter fired it from for its whole flight,
/// never one read from its height: a ground launcher's missile lofting 60 m
/// over the field is a ground round throughout, and a helicopter's bullets
/// falling to the ground stay low-air rounds.
#[test]
fn a_round_keeps_the_band_it_was_fired_from_for_its_whole_flight() {
    let mut rules = fixture();
    rules["weapons"]["atgm"]["top_attack"] = json!({ "loft_m": 60, "dive_deg": 40 });
    rules["weapons"]["atgm"]["turn_deg_s"] = json!(360);
    let mut b = battle_with(
        &rules,
        json!({ "size": [1200, 600], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 }),
        json!([
            { "side": "blue", "kind": "test_at", "position": [100, 300] },
            { "side": "red", "kind": "test_tank", "position": [600, 300], "engagement": "return_fire_only" },
            { "side": "blue", "kind": "test_heli", "position": [100, 100] },
            { "side": "red", "kind": "test_rifle", "position": [250, 100], "engagement": "return_fire_only" },
        ]),
    );
    let (mut missile_top, mut heli_rounds) = (0.0f64, 0);
    for _ in 0..600 {
        b.step();
        for (p, r) in b.rounds() {
            let ground = b.world().height_at(p.position.x, p.position.y).unwrap();
            match r.unit {
                UnitId(0) => {
                    assert_eq!(
                        r.layer,
                        AltitudeLayer::Ground,
                        "at {:.0} m up",
                        p.position.z - ground
                    );
                    missile_top = missile_top.max(p.position.z - ground);
                }
                UnitId(2) => {
                    assert_eq!(r.layer, AltitudeLayer::LowAir);
                    heli_rounds += 1;
                }
                _ => {}
            }
        }
    }
    assert!(missile_top > 40.0, "the missile lofted {missile_top:.0} m");
    assert!(heli_rounds > 0, "the helicopter fired");
}

#[test]
fn an_attack_on_a_target_the_side_has_never_identified_is_refused() {
    let rules = serde_json::from_value(fixtures::rules()).unwrap();
    let mut b = Battle::new(&sim::maps::scenario("air", "ground", rules).unwrap(), 6);
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 1,
        order: Order::Attack {
            units: vec![UnitId(2)],
            target: TargetRef::Identified {
                id: contract::observation::ObservedTargetId(9999),
            },
        },
        queued: false,
    });
    assert_eq!(ack.error, Some(OrderError::UnknownTarget));
}

#[test]
fn authored_aircraft_fuel_starts_full_and_burns_on_map() {
    let mut fixture = fixture();
    fixtures::patch_catalog(
        &mut fixture,
        "units",
        "test_heli",
        json!({
            "capabilities": { "air": {
                "fuel_capacity_s": 180.0,
                "refund": { "health": 0.34, "ammunition": 0.33, "fuel": 0.33 }
            }}
        }),
    );
    let mut b = battle_with(
        &fixture,
        open_map(),
        json!([{ "side": "blue", "kind": "test_heli", "position": [300, 200] }]),
    );
    assert_eq!(b.unit(UnitId(0)).unwrap().air.unwrap().fuel_s, 180.0);
    for _ in 0..b.rules().tick_hz {
        b.step();
    }
    let fuel = b.unit(UnitId(0)).unwrap().air.unwrap().fuel_s;
    assert!(
        (fuel - 179.0).abs() < 1e-9,
        "one second of fuel burned: {fuel}"
    );
}

#[test]
fn a_skirmish_aircraft_with_low_fuel_turns_home_and_is_refunded() {
    let mut fixture = fixture();
    fixtures::patch_catalog(
        &mut fixture,
        "units",
        "test_heli",
        json!({
            "roster": { "factions": ["us"], "category": "air", "variant": "Fuel test", "family_name": "Fuel test" },
            "capabilities": { "air": {
                "fuel_capacity_s": 2.0,
                "refund": { "health": 0.34, "ammunition": 0.33, "fuel": 0.33 }
            }}
        }),
    );
    let setup: contract::scenario::ScenarioDefinition = serde_json::from_value(json!({
        "map": { "size": [800, 600], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": fixture,
        "units": [], "events": [], "scripts": [],
        "players": crate::common::human_duel(), "skirmish": {"rules": { "credits_per_minute": 200, "starting_minutes": 5, "preparation_s": 0, "max_units": 30, "max_players_per_side": 10, "dispatch_interval_s": 1, "capture_s": 20, "majority_s": 1800, "victory_score": 1000 },
            "sites": { "entries": [
                { "center": [400, 10], "side": "blue", "yaw": std::f64::consts::FRAC_PI_2 },
                { "center": [400, 590], "side": "red", "yaw": -std::f64::consts::FRAC_PI_2 }
            ], "objectives": [
                { "id": "center", "center": [400, 300], "radius_m": 50, "kind": "junction", "counterpart": null },
                { "id": "south", "center": [200, 200], "radius_m": 50, "kind": "field", "counterpart": "north" },
                { "id": "north", "center": [600, 400], "radius_m": 50, "kind": "field", "counterpart": "south" }
            ] }
        }
    })).unwrap();
    let mut battle = Battle::new(&setup, 1);
    let ready = |side, seq| contract::command::CommandEnvelope {
        side,
        player: Some(
            match side {
                Side::Blue => "blue-1",
                Side::Red => "red-1",
                Side::Neutral => "neutral",
            }
            .into(),
        ),
        seq,
        queued: false,
        order: Order::Ready,
    };
    assert_eq!(battle.accept(ready(Side::Blue, 1)).error, None);
    assert_eq!(battle.accept(ready(Side::Red, 1)).error, None);
    battle.step();
    assert_eq!(
        battle
            .accept(contract::command::CommandEnvelope {
                side: Side::Blue,
                player: Some("blue-1".into()),
                seq: 2,
                queued: false,
                order: Order::ConfirmPurchase {
                    variant: "test_heli".into(),
                    destination: [400.0, 300.0]
                }
            })
            .error,
        None
    );
    for _ in 0..1200 {
        battle.step();
    }
    let frame = battle.observe(Side::Blue);
    assert!(
        frame.own.is_empty(),
        "fuel-starved aircraft remained on map"
    );
    assert!(frame.seat().credits > 1000.0, "no fuel withdrawal refund");
}

#[test]
fn a_tank_drives_under_a_hovering_helicopter() {
    let fixture = fixture();
    let mut b = battle_with(
        &fixture,
        open_map(),
        json!([
            { "side": "blue", "kind": "test_tank", "position": [150, 200] },
            { "side": "blue", "kind": "test_heli", "position": [300, 200] },
        ]),
    );
    assert_eq!(b.accept(order(&[0], [450.0, 200.0])).error, None);
    for _ in 0..60 * b.rules().tick_hz {
        b.step();
    }
    let tank = b.unit(UnitId(0)).unwrap().position.xy();
    let short = (tank - sim::math::v2(450.0, 200.0)).length();
    assert!(
        short < 5.0,
        "the tank waited for the helicopter: {short:.1} m short"
    );
}

#[test]
fn soldiers_are_not_shoved_from_under_it() {
    let fixture = fixture();
    let mut b = battle_with(
        &fixture,
        open_map(),
        json!([
            { "side": "blue", "kind": "test_rifle", "position": [300, 200] },
            { "side": "blue", "kind": "test_heli", "position": [300, 200] },
        ]),
    );
    let before: Vec<_> = b.unit(UnitId(0)).unwrap().member_positions().collect();
    for _ in 0..10 * b.rules().tick_hz {
        b.step();
    }
    let after: Vec<_> = b.unit(UnitId(0)).unwrap().member_positions().collect();
    for (a, z) in before.iter().zip(&after) {
        assert!(
            (a.xy() - z.xy()).length() < 1e-9,
            "a soldier was shoved from {a:?} to {z:?}"
        );
    }
}

#[test]
fn a_helicopter_over_forest_is_not_concealed() {
    let fixture = fixture();
    let forest_map = json!({ "size": [600, 400], "fog_cell_m": 8, "height_grid_m": 4,
        "slope_cutoff_deg": 35,
        "forests": [{ "shape": { "kind": "polygon",
            "ring": [[250, 150], [350, 150], [350, 250], [250, 250]] } }] });
    let b = battle_with(
        &fixture,
        forest_map,
        json!([
            { "side": "blue", "kind": "test_heli", "position": [300, 200] },
            { "side": "blue", "kind": "test_tank", "position": [300, 180] },
        ]),
    );
    let rules = b.rules();
    assert!(!sim::sensing::concealed(
        b.world(),
        b.unit(UnitId(0)).unwrap(),
        rules
    ));
    assert!(
        sim::sensing::concealed(b.world(), b.unit(UnitId(1)).unwrap(), rules),
        "the same forest hides a tank on its ground"
    );
}

#[test]
fn an_identified_enemy_helicopter_below_half_hp_publishes_smoking() {
    // Two blue helicopters in red's plain sight, one under half its health
    // and one just above, and a tank as damaged: red sees which helicopter
    // smokes (a coarse bit, never its health), and blue sees its own alike.
    let fixture = fixture();
    let hp = |kind: &str| {
        let rules: Rules = serde_json::from_value(fixture.clone()).unwrap();
        let kind = rules.catalog.index(kind).unwrap();
        rules.catalog.get(kind).hull().unwrap().hp
    };
    let (heli, tank) = (hp("test_heli"), hp("test_tank"));
    let mut b = battle_with(
        &fixture,
        open_map(),
        json!([
            { "side": "blue", "kind": "test_heli", "position": [300, 200], "engagement": "return_fire_only",
                "condition": { "hp": heli * 0.49 } },
            { "side": "blue", "kind": "test_heli", "position": [300, 230], "engagement": "return_fire_only",
                "condition": { "hp": heli * 0.51 } },
            { "side": "blue", "kind": "test_tank", "position": [300, 170], "engagement": "return_fire_only",
                "condition": { "hp": tank * 0.2 } },
            { "side": "red", "kind": "test_recon", "position": [330, 215], "engagement": "return_fire_only" },
        ]),
    );
    for _ in 0..30 {
        b.step();
    }
    let red = b.observe(Side::Red);
    let mut seen: Vec<(f64, bool)> = red
        .identified
        .iter()
        .map(|e| (e.position[1], e.smoking))
        .collect();
    seen.sort_by(|a, b| a.0.total_cmp(&b.0));
    assert_eq!(
        seen.iter().map(|s| s.1).collect::<Vec<_>>(),
        [false, true, false],
        "the tank at y 170, the damaged helicopter at 200, the other at 230: {seen:?}"
    );
    let own: Vec<bool> = b
        .observe(Side::Blue)
        .own
        .iter()
        .map(|u| u.smoking)
        .collect();
    assert_eq!(own, [true, false, false]);
}

/// An aircraft carries as many weapon rows as it has mounts for: there is no
/// cap on how many distinct rows it fields.
#[test]
fn test_an_aircraft_with_five_distinct_weapon_rows_is_admitted() {
    let mut fixture = fixture();
    let mount = |id: &str, row: &str| {
        json!({ "id": id, "name": id.to_uppercase(), "weapons": [row], "turret": true,
            "pivot_m": [1.0, 0.0, 1.0], "muzzle_m": [1.0, 0.0, 0.0] })
    };
    fixtures::patch_catalog(
        &mut fixture,
        "units",
        "test_heli",
        json!({
            "capabilities": { "air": { "fuel_capacity_s": 180,
                "refund": { "health": 0.4, "ammunition": 0.4, "fuel": 0.2 } } },
            "mounts": [
                mount("a", "hmg"), mount("b", "rifle"), mount("c", "lmg"),
                mount("d", "autocannon"), mount("e", "rocket_pod"),
            ]
        }),
    );
    serde_json::from_value::<Rules>(fixture).expect("five rows on five mounts");
}

/// An aircraft authored in the ground layer would sit at 0 m where nothing
/// that reaches the air or the ground could target it: refused at load.
#[test]
fn test_an_aircraft_in_the_ground_layer_is_refused() {
    let mut fixture = fixture();
    fixtures::patch_catalog(
        &mut fixture,
        "units",
        "test_heli",
        json!({ "mobility": { "air": { "layer": "ground" } } }),
    );
    let error = serde_json::from_value::<Rules>(fixture).expect_err("an aircraft on the ground");
    assert!(error.to_string().contains("ground layer"), "{error}");
}

#[test]
fn a_battle_with_a_helicopter_replays_to_the_same_digest() {
    let fixture = fixture();
    let run = || {
        let mut b = battle_with(
            &fixture,
            open_map(),
            json!([
                { "side": "blue", "kind": "test_heli", "position": [200, 200] },
                { "side": "red", "kind": "test_rifle", "position": [400, 200] },
            ]),
        );
        for _ in 0..20 * b.rules().tick_hz {
            b.step();
        }
        b.digest()
    };
    assert_eq!(run(), run());
}

/// A long strip: an observer at one end loses sight of what stands past its
/// sensor range at the other.
fn strip() -> Value {
    json!({ "size": [1600, 400], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 })
}

#[test]
fn a_lost_helicopter_leaves_an_airborne_contact() {
    // Red's squad sees the hovering helicopter 550 m off, then walks out of
    // its 600 m sight: the area it leaves hangs where the airframe was, in
    // the air, not on the ground below it.
    let fixture = fixture();
    let mut b = scripted(
        &fixture,
        strip(),
        json!([
            { "side": "red", "kind": "test_rifle", "position": [200, 200], "engagement": "return_fire_only" },
            { "side": "blue", "kind": "test_heli", "position": [750, 200], "engagement": "return_fire_only" },
        ]),
        json!([]),
        json!([{ "tick": 20, "side": "red", "order":
            { "kind": "move", "units": [0], "gesture": 1, "goal": [20, 200], "route": "shortest" } }]),
    );
    let heli = b.unit(UnitId(1)).unwrap().position;
    let mut seen = false;
    for _ in 0..240 * b.rules().tick_hz {
        b.step();
        let red = b.observe(Side::Red);
        seen |= !red.identified.is_empty();
        if let Some(c) = red.contacts.first() {
            assert!(seen, "the helicopter was identified before it was lost");
            assert_eq!(c.source, ContactSource::LastSeen);
            assert_eq!(c.layer, AltitudeLayer::LowAir);
            assert_eq!(
                c.center,
                [heli.x, heli.y, heli.z],
                "the area is where it hovered"
            );
            assert!(c.aloft, "at cruise, well over the low hover: it floats");
            return;
        }
    }
    panic!("the squad never lost the helicopter (seen: {seen})");
}

/// Blue's squad and, 800 m off and out of its sight, red's helicopter,
/// which fires once at tick 5.
fn heard_helicopter(fixture: &Value) -> Battle {
    let mut b = scripted(
        fixture,
        strip(),
        json!([
            { "side": "blue", "kind": "test_rifle", "position": [100, 200] },
            { "side": "red", "kind": "test_heli", "position": [900, 200], "engagement": "return_fire_only" },
        ]),
        json!([{ "tick": 5, "fire": { "unit": 1 } }]),
        json!([]),
    );
    for _ in 0..5 {
        b.step();
    }
    b
}

#[test]
fn a_heard_helicopter_report_is_airborne() {
    // A shot from an unseen helicopter is heard overhead: the report's area
    // is at the airframe's height, in the air band.
    let fixture = fixture();
    let b = heard_helicopter(&fixture);
    let heli = b.unit(UnitId(1)).unwrap().position;
    let blue = b.observe(Side::Blue);
    assert!(blue.identified.is_empty(), "the helicopter is out of sight");
    let [c] = blue.contacts.as_slice() else {
        panic!("one report: {:?}", blue.contacts);
    };
    assert_eq!(c.source, ContactSource::Firing);
    assert_eq!(c.layer, AltitudeLayer::LowAir);
    assert_eq!(c.center[2], heli.z, "heard at the airframe's height");
    assert!(c.aloft, "heard from cruise: it floats");
    let off = (c.center[0] - heli.x).hypot(c.center[1] - heli.y);
    assert!(
        off <= c.radius,
        "the airframe lies inside its area: {off:.1} m"
    );
}

/// Blue's tank and, 720 m off and beyond each other's sight, a red `kind`
/// that fires every second: blue holds only its firing area.
fn hidden_fire_at_a_tank(fixture: &Value, kind: &str) -> Battle {
    let fire: Vec<Value> = (0..20)
        .map(|k| json!({ "tick": 5 + k * 30, "fire": { "unit": 1 } }))
        .collect();
    scripted(
        fixture,
        strip(),
        json!([
            { "side": "blue", "kind": "test_tank", "position": [100, 200] },
            { "side": "red", "kind": kind, "position": [820, 200], "engagement": "return_fire_only" },
        ]),
        json!(fire),
        json!([]),
    )
}

/// Rounds blue's tank launches over `ticks`.
fn blue_rounds(b: &mut Battle, ticks: u64) -> usize {
    let mut fired = std::collections::BTreeSet::new();
    for _ in 0..ticks {
        b.step();
        fired.extend(
            b.rounds()
                .filter(|(_, r)| r.unit == UnitId(0))
                .map(|(p, _)| p.id),
        );
    }
    fired.len()
}

#[test]
fn area_fire_refuses_an_air_contact() {
    // A tank crew hears a helicopter's gun but can't see it: shelling the
    // field under where the sound came from would be silly, so neither the
    // player's attack order nor the crew's own fire aims at the report. A
    // squad's report from the same place draws HE as before.
    let fixture = fixture();
    let mut b = hidden_fire_at_a_tank(&fixture, "test_heli");
    let mut ground = hidden_fire_at_a_tank(&fixture, "test_rifle");
    let (air_rounds, ground_rounds) = (blue_rounds(&mut b, 600), blue_rounds(&mut ground, 600));
    assert!(b.observe(Side::Blue).identified.is_empty(), "never seen");
    let [c] = b.observe(Side::Blue).contacts.as_slice() else {
        panic!("one report: {:?}", b.observe(Side::Blue).contacts);
    };
    assert_eq!(c.layer, AltitudeLayer::LowAir);
    assert_eq!(air_rounds, 0, "the crew never shells an air contact");
    assert!(ground_rounds > 0, "a ground report still draws area fire");
    let attack = |id| CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 1,
        order: Order::Attack {
            units: vec![UnitId(0)],
            target: TargetRef::Contact { id },
        },
        queued: false,
    };
    assert_eq!(
        b.accept(attack(c.id)).error,
        Some(OrderError::AirContact),
        "an attack order on an air contact is refused"
    );
    let area = &ground.observe(Side::Blue).contacts[0];
    assert!(!area.aloft, "a ground report lies on the ground");
    assert_eq!(ground.accept(attack(area.id)).error, None);
}

#[test]
fn a_report_from_the_low_hover_lies_on_the_ground() {
    // A helicopter idle at its supply truck sinks to the low hover, just over
    // the tallest ground hull. A shot it fires from there is heard at that
    // height, in the air band, but no higher than a tank's roof: its report
    // lies on the ground (D33), where one heard from cruise floats.
    let fixture = fixture();
    let fire = 30 * fixture["tick_hz"].as_u64().unwrap();
    let mut b = scripted(
        &fixture,
        strip(),
        json!([
            { "side": "blue", "kind": "test_rifle", "position": [100, 200] },
            { "side": "red", "kind": "test_supply", "position": [900, 200] },
            { "side": "red", "kind": "test_heli", "position": [910, 200], "engagement": "return_fire_only" },
        ]),
        json!([{ "tick": fire, "fire": { "unit": 2 } }]),
        json!([]),
    );
    for _ in 0..=fire {
        b.step();
    }
    let heli = b.unit(UnitId(2)).unwrap().position;
    assert!(
        heli.z < b.rules().air.low_air.cruise_agl_m / 2.0,
        "sank to the low hover before firing: {:.1} m",
        heli.z
    );
    let blue = b.observe(Side::Blue);
    assert!(blue.identified.is_empty(), "the helicopter is out of sight");
    let [c] = blue.contacts.as_slice() else {
        panic!("one report: {:?}", blue.contacts);
    };
    assert_eq!(c.layer, AltitudeLayer::LowAir);
    assert!(!c.aloft, "heard from the low hover: on the ground");
}

#[test]
fn a_helicopter_ordered_onto_an_identified_tank_takes_the_order() {
    // The player picks a tank his helicopter can see and orders the attack:
    // the order lands like any other unit's.
    let fixture = fixture();
    let mut b = battle_with(
        &fixture,
        open_map(),
        json!([
            { "side": "blue", "kind": "test_heli", "position": [300, 200], "engagement": "return_fire_only" },
            { "side": "red", "kind": "test_tank", "position": [330, 215], "engagement": "return_fire_only" },
        ]),
    );
    for _ in 0..30 {
        b.step();
    }
    let [tank] = b.observe(Side::Blue).identified.as_slice() else {
        panic!(
            "the tank in plain sight: {:?}",
            b.observe(Side::Blue).identified
        );
    };
    let attack = CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 1,
        order: Order::Attack {
            units: vec![UnitId(0)],
            target: TargetRef::Identified { id: tank.id },
        },
        queued: false,
    };
    assert_eq!(b.accept(attack).error, None);
    for _ in 0..5 {
        b.step();
    }
    assert_eq!(
        b.observe(Side::Blue).own[0].engagement,
        contract::command::Engagement::FireAtWill,
        "an attack order frees the helicopter to fire"
    );
}

#[test]
fn back_edge_entry_and_crossing_follow_the_entry_site_side() {
    let fixture = fixture();
    let setup = serde_json::from_value::<contract::scenario::ScenarioDefinition>(serde_json::json!({
        "map": {"size":[600,400],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35},
        "rules": fixture, "units": [],
        "players": [{"id":"blue-1","side":"blue","faction":"us","controller":"human","human_capable":true},{"id":"red-1","side":"red","faction":"eastern","controller":"passive_ai"}], "skirmish": {"rules":{"credits_per_minute":1,"starting_minutes":1,"preparation_s":1,"max_units":1,"max_players_per_side":10,"dispatch_interval_s":1, "capture_s": 20, "majority_s": 1800, "victory_score": 1000},"sites":{"entries":[{"side":"blue","center":[300,5],"yaw":1.57},{"side":"red","center":[300,395],"yaw":-1.57}],"objectives":[{"id":"center","center":[300,200],"radius_m":50,"kind":"junction","counterpart":null},{"id":"south","center":[200,100],"radius_m":50,"kind":"field","counterpart":"north"},{"id":"north","center":[400,300],"radius_m":50,"kind":"field","counterpart":"south"}]}}
    })).unwrap();
    let world = sim::encounter::PreparedMap::new(&setup.map, &setup.rules).world;
    let edge = sim::air::edge::back_edge(
        contract::ids::Side::Blue,
        &setup.skirmish.as_ref().unwrap().sites,
        &world,
    );
    let (point, heading) = sim::air::edge::entry(&edge, sim::math::v2(100.0, 0.0));
    assert!((point.y - 0.0).abs() < 1e-9);
    assert!((heading - std::f64::consts::FRAC_PI_2).abs() < 1e-9);
    assert!(sim::air::edge::crossed(
        &edge,
        sim::math::v2(100.0, 1.0),
        sim::math::v2(100.0, -1.0)
    ));
}

#[test]
fn a_downed_jet_crossing_the_edge_just_ends() {
    let b = battle_with(
        &fixture(),
        open_map(),
        json!([{"side":"blue","kind":"test_jet","position":[300,200]}]),
    );
    let unit = b.unit(UnitId(0)).unwrap();
    let mut crash = sim::crash::Crash::new(
        UnitId(0),
        sim::math::v3(599.0, 200.0, 10.0),
        sim::math::v2(1000.0, 0.0),
        0.0,
        None,
        vec![],
    );
    let fell = crash.fall(
        b.world(),
        sim::math::v3(0.0, 0.0, -9.81),
        unit.hull.unwrap(),
        1.0 / f64::from(b.rules().tick_hz),
    );
    assert_eq!(fell, sim::crash::Fall::LeftMap);
}

#[test]
fn a_high_air_unit_lights_no_ground_fog_in_its_sides_picture() {
    // High air never sees the ground, so its own side's drawn fog gets no
    // eye from it; a helicopter, low over the ground, still gives one.
    let fixture = fixture();
    let b = battle_with(
        &fixture,
        open_map(),
        json!([
            { "side": "blue", "kind": "test_jet", "position": [300, 200], "yaw": 0.0 },
            { "side": "blue", "kind": "test_heli", "position": [200, 200], "yaw": 0.0 },
        ]),
    );
    let frame = b.observe(Side::Blue);
    let eyes = |id: u32| {
        frame
            .own
            .iter()
            .find(|u| u.id == UnitId(id))
            .unwrap_or_else(|| panic!("unit {id} is fielded"))
            .sight
            .eyes
            .len()
    };
    assert_eq!(eyes(0), 0, "a high-air unit lights no ground fog");
    assert!(eyes(1) > 0, "a low-air unit still does");
}

#[path = "air/limits.rs"]
mod limits;
