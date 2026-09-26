//! Sensing and shared identification against crafted geometry.
use contract::ids::Side;
use contract::observation::ObservationFrame;
use serde_json::json;
use sim::battle::Battle;

mod common;
const MAP: &str = include_str!("../../../fixtures/sensors-lab.json");

fn battle(units: serde_json::Value) -> Battle {
    Battle::new(&common::scenario(MAP, units, json!([])), 1)
}

fn blue(b: &Battle) -> &ObservationFrame {
    b.observe(Side::Blue)
}

/// Whether blue identifies the red unit standing nearest to `at`.
fn identifies(b: &Battle, at: [f64; 2]) -> bool {
    blue(b)
        .identified
        .iter()
        .any(|e| (e.position[0] - at[0]).hypot(e.position[1] - at[1]) < 10.0)
}

#[test]
fn ground_range_bounds_identification_by_observer_class() {
    // Each observer faces east (the default yaw), so the target is dead ahead:
    // the ground range times the shape's front (infantry have no shape).
    let s = common::village()["sensors"].clone();
    let front = s["sight_shape"]["tank"]["front"].as_f64().unwrap();
    for (kind, range) in [
        ("recon", s["recon_ground_m"].as_f64().unwrap()),
        ("rifle", s["infantry_ground_m"].as_f64().unwrap()),
        ("tank", s["tank_ground_m"].as_f64().unwrap() * front),
    ] {
        let near = battle(json!([
            { "side": "blue", "kind": kind, "position": [20, 580] },
            { "side": "red", "kind": "tank", "position": [20.0 + range - 5.0, 580] },
        ]));
        let far = battle(json!([
            { "side": "blue", "kind": kind, "position": [20, 580] },
            { "side": "red", "kind": "tank", "position": [20.0 + range + 5.0, 580] },
        ]));
        assert_eq!(
            blue(&near).identified.len(),
            1,
            "{kind} sees inside {range} m"
        );
        assert_eq!(
            blue(&far).identified.len(),
            0,
            "{kind} does not see past {range} m"
        );
    }
}

#[test]
fn edge_infantry_hide_sooner_than_vehicles() {
    // Both a few metres inside the thin forest's west edge, 330 m from a rifle
    // squad. Every soldier of the red squad stands inside the forest.
    let b = battle(json!([
        { "side": "blue", "kind": "rifle", "position": [78, 60] },
        { "side": "red", "kind": "rifle", "position": [408, 60] },
        { "side": "red", "kind": "tank", "position": [404, 140] },
    ]));
    assert!(
        !identifies(&b, [408.0, 60.0]),
        "infantry near the edge are concealed"
    );
    assert!(
        identifies(&b, [404.0, 140.0]),
        "a vehicle needs more depth to hide"
    );
}

#[test]
fn thin_forest_lets_vehicles_be_seen_beyond_it_and_thick_forest_blocks() {
    let b = battle(json!([
        { "side": "blue", "kind": "rifle", "position": [240, 100] },
        { "side": "red", "kind": "tank", "position": [520, 100] },
        { "side": "blue", "kind": "rifle", "position": [240, 300] },
        { "side": "red", "kind": "tank", "position": [620, 300] },
    ]));
    assert!(
        identifies(&b, [520.0, 100.0]),
        "seen through 40 m of foliage"
    );
    assert!(
        !identifies(&b, [620.0, 300.0]),
        "160 m of foliage blocks outright"
    );
}

#[test]
fn hills_and_buildings_block_sight() {
    let hill = battle(json!([
        { "side": "blue", "kind": "recon", "position": [560, 480] },
        { "side": "red", "kind": "tank", "position": [840, 480] },
        { "side": "red", "kind": "tank", "position": [700, 200] },
    ]));
    assert!(!identifies(&hill, [840.0, 480.0]), "behind the ridge");
    assert!(
        identifies(&hill, [700.0, 200.0]),
        "clear of the ridge at a similar range"
    );
    let building = battle(json!([
        { "side": "blue", "kind": "recon", "position": [900, 100] },
        { "side": "red", "kind": "tank", "position": [1100, 100] },
        { "side": "red", "kind": "tank", "position": [1100, 160] },
    ]));
    assert!(
        !identifies(&building, [1100.0, 100.0]),
        "behind the building"
    );
    assert!(identifies(&building, [1100.0, 160.0]), "clear of it");
}

#[test]
fn shared_identification_extends_a_tank_but_not_its_own_sensor() {
    let b = battle(json!([
        { "side": "blue", "kind": "recon", "position": [20, 560] },
        { "side": "blue", "kind": "tank", "position": [200, 560] },
        { "side": "red", "kind": "tank", "position": [820, 560] },
    ]));
    let frame = blue(&b);
    assert_eq!(
        frame.identified.len(),
        1,
        "the scout identifies it for the team"
    );
    let id = frame.identified[0].id;
    let scout = frame
        .own
        .iter()
        .find(|u| u.kind == contract::scenario::UnitKind::Recon)
        .unwrap();
    let tank = frame
        .own
        .iter()
        .find(|u| u.kind == contract::scenario::UnitKind::Tank)
        .unwrap();
    assert_eq!(scout.sees, vec![id]);
    assert!(
        tank.sees.is_empty(),
        "620 m is beyond the tank's own optics"
    );
}

#[test]
fn a_partly_hidden_squad_exports_only_its_seen_soldiers() {
    // A squad straddling the building's shadow: some members are behind it.
    let b = battle(json!([
        { "side": "blue", "kind": "recon", "position": [900, 104] },
        { "side": "red", "kind": "rifle", "position": [1016, 112] },
    ]));
    let frame = blue(&b);
    assert_eq!(frame.identified.len(), 1);
    let seen = &frame.identified[0];
    assert!(
        !seen.members.is_empty() && seen.members.len() < 8,
        "{} of 8 seen",
        seen.members.len()
    );
    // The reported position is the centroid of what was seen.
    let n = seen.members.len() as f64;
    let cy = seen.members.iter().map(|m| m[1]).sum::<f64>() / n;
    assert!((seen.position[1] - cy).abs() < 1e-9);
}

#[test]
fn hidden_enemy_changes_leave_the_side_view_identical() {
    // Red's tank behind the ridge, beyond vehicle hearing range, moves; blue's
    // frame must not change. (Within hearing range the sound is evidence.)
    let units = json!([
        { "side": "blue", "kind": "recon", "position": [100, 480] },
        { "side": "red", "kind": "tank", "position": [840, 480] },
    ]);
    let mut a = Battle::new(&common::scenario(MAP, units.clone(), json!([])), 1);
    let scripts = json!([{ "tick": 1, "side": "red", "order": { "kind": "move", "units": [1], "gesture": 1, "goal": [860, 470], "route": "shortest" } }]);
    let mut b = Battle::new(&common::scenario_with(MAP, units, json!([]), scripts), 1);
    for _ in 0..90 {
        a.step();
        b.step();
        assert_eq!(
            serde_json::to_string(blue(&a)).unwrap(),
            serde_json::to_string(blue(&b)).unwrap()
        );
    }
}

/// Blue's identification handles, tick by tick, while red follows `path`.
fn handles_along(
    scout: [f64; 2],
    start: [f64; 2],
    path: &[[f64; 2]],
) -> Vec<Option<contract::observation::ObservedTargetId>> {
    let units = json!([
        { "side": "blue", "kind": "recon", "position": scout },
        { "side": "red", "kind": "tank", "position": start },
    ]);
    let scripts: Vec<_> = path
        .iter()
        .map(|goal| json!({ "tick": 1, "side": "red", "queued": true,
            "order": { "kind": "move", "units": [1], "gesture": 1, "goal": goal, "route": "shortest" } }))
        .collect();
    let mut b = Battle::new(
        &common::scenario_with(MAP, units, json!([]), json!(scripts)),
        1,
    );
    (0..1500)
        .map(|_| {
            b.step();
            blue(&b).identified.first().map(|e| e.id)
        })
        .collect()
}

/// The first hidden spell: (ticks hidden, handle before, handle after).
fn first_gap(
    seen: &[Option<contract::observation::ObservedTargetId>],
) -> (
    usize,
    Option<contract::observation::ObservedTargetId>,
    Option<contract::observation::ObservedTargetId>,
) {
    let gap = seen
        .iter()
        .position(|s| s.is_none())
        .expect("it passes out of sight");
    let back = gap
        + seen[gap..]
            .iter()
            .position(|s| s.is_some())
            .expect("and back into sight");
    (back - gap, seen[gap - 1], seen[back])
}

#[test]
fn a_brief_loss_of_sight_keeps_the_identification_handle() {
    // The tank crosses behind a 3 m wall: hidden for well under the grace.
    let (hidden, before, after) = first_gap(&handles_along(
        [900.0, 300.0],
        [1100.0, 270.0],
        &[[1100.0, 330.0]],
    ));
    let grace = (common::village()["sensors"]["acquisition_grace_s"]
        .as_f64()
        .unwrap()
        * 30.0) as usize;
    assert!(hidden <= grace, "hidden {hidden} ticks");
    assert_eq!(before, after);
}

#[test]
fn a_lapsed_identification_gets_a_new_handle() {
    // The tank drives behind the ridge and back out, hidden for many seconds.
    let (hidden, before, after) = first_gap(&handles_along(
        [560.0, 400.0],
        [820.0, 330.0],
        &[[820.0, 440.0], [820.0, 330.0]],
    ));
    assert!(hidden > 45, "hidden {hidden} ticks");
    assert_ne!(before, after);
}

#[test]
fn the_ground_field_marks_open_ground_visible_and_hidden_ground_fogged() {
    let b = battle(json!([{ "side": "blue", "kind": "recon", "position": [560, 480] }]));
    let fog = &blue(&b).ground_visibility;
    assert!(fog.visible(620.0, 480.0), "open ground in front");
    assert!(!fog.visible(820.0, 480.0), "the far side of the ridge");
    assert!(!fog.visible(1300.0, 480.0), "beyond range");
    assert!(
        !b.observe(Side::Red).ground_visibility.visible(560.0, 480.0),
        "red has no eyes there"
    );
}
