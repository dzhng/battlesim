//! The last-fired row is sufficient even when flight cannot identify a launch.
use presentation::effects::{
    frame::{EffectBatch, EffectFrame},
    publication::EffectFeed,
    tables::EffectPresentation,
};
use serde_json::{json, Value};
use sim::{
    battle::Battle,
    publication::{reader::Reader, Publisher},
};

#[path = "../../sim/tests/common/mod.rs"]
mod common;

fn second_round(case: &str) {
    let mut scenario = common::scenario(
        r#"{"size":[500,500],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35}"#,
        json!([{"side":"blue","kind":"test_tank","position":[100,100]}]),
        json!([]),
    );
    scenario.rules = scenario
        .rules
        .overridden(
            json!({"units":{"test_tank":{"mounts":[
                {"id":"cannon","weapons":["tank_ap","tank_he"],"pivot_m":[0,0,2],"muzzle_m":[5,0,0]}
            ]}}})
            .as_object()
            .unwrap(),
        )
        .unwrap();
    let battle = Battle::new(&scenario, 1);
    let runtime =
        presentation::assets::Runtime::open(&sim::fixtures::dir().join("../assets/runtime"))
            .unwrap();
    let settings: Value =
        serde_json::from_str(include_str!("../../../fixtures/presentation.json")).unwrap();
    let rules = serde_json::to_value(battle.rules()).unwrap();
    let catalog = battle.rules().catalog.view();
    let fielded = presentation::fielded::Fielding {
        presentation: &settings,
        rules: &rules,
        catalog: &catalog,
        runtime: &runtime.catalog,
        kinds: &["test_tank".into()],
        factions: None,
    }
    .resolve(&|hash| runtime.read(hash, "bundle.bin"))
    .unwrap()
    .typed()
    .unwrap();
    let p = EffectPresentation::from_value(&fielded.effects.presentation).unwrap();
    let cause = format!("flash:{}", p.looks.name("tank_he"));
    let mut batch = EffectBatch::new(p.capacity);
    let mut effects = EffectFrame::new(30.0, p);
    let mut publisher = Publisher::new();
    let words = publisher
        .publish(&battle, contract::ids::Side::Blue)
        .unwrap()
        .iter()
        .map(|w| w.to_bits())
        .collect::<Vec<_>>();
    let mut observation = Reader::new(&sim::publication::layout_json(&battle))
        .unwrap()
        .decode(&words)
        .unwrap()
        .unwrap();
    let mut feed = EffectFeed::new("blue");
    assert!(effects
        .note(&feed.publication(&observation, &fielded.units, &fielded.effects, None))
        .is_empty());
    observation.tick += if case == "gap" { 4.0 } else { 1.0 };
    let hull = &mut observation.own[0];
    hull.position = [110.0, 105.0, 0.0];
    hull.yaw = std::f64::consts::FRAC_PI_2;
    let mount = &mut hull.weapon_poses[0];
    mount.bearing = std::f64::consts::FRAC_PI_2;
    mount.elevation = 0.0;
    mount.round = 1.0;
    mount.shots = Some(1.0);
    let mut publication = feed.publication(&observation, &fielded.units, &fielded.effects, None);
    if case == "fog" || case == "impact" {
        publication.segments = serde_json::from_value(json!([{
            "path": if case == "fog" { json!([[200,200,2],[210,200,2]]) } else { json!([[110,110,2],[110,111,0]]) },
            "ricochets":[], "kind":"tank_he", "shooter":null,
            "hit":if case == "impact" {"ground"} else {"none"}, "normal":null
        }])).unwrap();
    }
    let launches = effects.note(&publication);
    assert_eq!(
        launches.len(),
        1,
        "{case}: exactly one launch on the counter-rise tick"
    );
    let launch = &launches[0];
    assert_eq!(&*launch.kind, "tank_he");
    assert!(
        launch
            .at
            .iter()
            .zip([110.0, 110.0, 2.0])
            .all(|(a, b)| (a - b).abs() < 1e-9),
        "muzzle follows the moving, turning hull: {:?}",
        launch.at
    );
    assert!(launch.dir[0].abs() < 1e-9 && (launch.dir[1] - 1.0).abs() < 1e-9);
    effects.build(observation.tick / 30.0, &mut batch, None, &[], &[], None);
    assert!(batch.lights.causes.iter().any(|c| c.as_ref() == cause));
    observation.tick += 1.0;
    assert!(
        effects
            .note(&feed.publication(&observation, &fielded.units, &fielded.effects, None))
            .is_empty(),
        "no delayed or duplicate launch"
    );
}

#[test]
fn second_round_on_a_moving_turning_hull() {
    second_round("moving");
}
#[test]
fn second_round_after_a_skipped_publication() {
    second_round("gap");
}
#[test]
fn second_round_with_a_fog_clipped_first_stretch() {
    second_round("fog");
}
#[test]
fn second_round_with_an_immediate_impact() {
    second_round("impact");
}

/// A soldier's round flies a tick after his counter rises; by then his mount
/// may name the next row he fired.
#[test]
fn a_soldier_s_round_launches_as_the_row_he_fired_after_he_switches_rows() {
    let publication = |tick: f64, shots: f64, kind: &str, segments: Value| {
        serde_json::from_value::<presentation::effects::publication::EffectPublication>(json!({
            "tick": tick, "segments": segments, "blasts": [], "smokes": [],
            "shooters": [{"key": 1, "position": [0, 0, 0], "half": null, "yaw": 0,
                "members": [7],
                "mounts": [{"bearing": 0, "elevation": 0, "shots": shots, "kind": kind, "muzzle": null}]}]
        }))
        .unwrap()
    };
    let flight = |kind: &str, y: f64| {
        json!([{"path": [[0, y, 1], [5, y, 1]], "ricochets": [], "kind": kind,
            "shooter": 7, "hit": "none", "normal": null}])
    };
    let mut tracker = presentation::effects::launches::LaunchTracker::default();
    assert!(tracker
        .note(&publication(1.0, 0.0, "grenade", json!([])), false, None)
        .is_empty());
    assert!(tracker
        .note(&publication(2.0, 1.0, "grenade", json!([])), false, None)
        .is_empty());
    let kinds = |launches: Vec<presentation::effects::launches::Launch>| {
        launches
            .into_iter()
            .map(|l| (l.kind.to_string(), l.soldier))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        kinds(tracker.note(
            &publication(3.0, 2.0, "rifle", flight("grenade", 0.0)),
            false,
            None
        )),
        [("grenade".to_string(), Some(7.0))],
        "the grenade launches although his mount now names the rifle"
    );
    assert_eq!(
        kinds(tracker.note(
            &publication(4.0, 2.0, "rifle", flight("rifle", 3.0)),
            false,
            None
        )),
        [("rifle".to_string(), Some(7.0))]
    );
}
