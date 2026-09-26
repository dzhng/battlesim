//! Uncertain evidence: firing areas, last-seen areas, sound cues, known props.
use contract::ids::Side;
use contract::observation::{ContactSource, ObservationFrame, SoundBand, SoundCategory};
use serde_json::{json, Value};
use sim::battle::Battle;

mod common;
const MAP: &str = include_str!("../../../fixtures/sensors-lab.json");

fn battle(units: Value, events: Value, scripts: Value) -> Battle {
    Battle::new(&common::scenario_with(MAP, units, events, scripts), 11)
}

fn blue(b: &Battle) -> &ObservationFrame {
    b.observe(Side::Blue)
}

fn run(b: &mut Battle, ticks: u64) {
    for _ in 0..ticks {
        b.step();
    }
}

fn fires(unit: u32, ticks: &[u64]) -> Value {
    json!(ticks
        .iter()
        .map(|t| json!({ "tick": t, "fire": { "unit": unit } }))
        .collect::<Vec<_>>())
}

fn lifetime_ticks() -> u64 {
    (common::village()["sensors"]["contact_lifetime_s"]
        .as_f64()
        .unwrap()
        * 30.0) as u64
}

fn radius() -> f64 {
    common::village()["sensors"]["contact_radius_m"]
        .as_f64()
        .unwrap()
}

// Blue's scout looks east at the ridge; red hides behind it at (840, 480).
fn hidden_shooter() -> Value {
    json!([
        { "side": "blue", "kind": "recon", "position": [560, 480] },
        { "side": "red", "kind": "rifle", "position": [840, 480] },
    ])
}

#[test]
fn a_shot_from_hiding_reveals_an_area_but_not_the_shooter() {
    let mut b = battle(hidden_shooter(), fires(1, &[5]), json!([]));
    run(&mut b, 5);
    let f = blue(&b);
    assert!(f.identified.is_empty(), "the ridge still hides it");
    assert_eq!(f.contacts.len(), 1);
    let c = &f.contacts[0];
    assert_eq!(c.source, ContactSource::Firing);
    let d = (c.center[0] - 840.0).hypot(c.center[1] - 480.0);
    assert!(
        d <= radius(),
        "the shooter lies inside its area ({d} m from centre)"
    );
}

#[test]
fn repeated_shots_refresh_one_report_without_new_samples() {
    let mut b = battle(hidden_shooter(), fires(1, &[5, 40, 80]), json!([]));
    run(&mut b, 5);
    let first = blue(&b).contacts[0].clone();
    run(&mut b, 75);
    let later = &blue(&b).contacts;
    assert_eq!(later.len(), 1);
    assert_eq!(
        (later[0].id, later[0].center),
        (first.id, first.center),
        "same report, same place"
    );
    assert!(
        later[0].expires_tick > first.expires_tick,
        "expiry refreshed"
    );
}

#[test]
fn an_area_never_follows_hidden_movement_and_only_a_shot_outside_it_starts_another() {
    // A hidden tank fires, drives 170 m inside the ridge's shadow, fires again.
    let units = json!([
        { "side": "blue", "kind": "recon", "position": [560, 480] },
        { "side": "red", "kind": "tank", "position": [900, 420], "yaw": std::f64::consts::PI },
    ]);
    let scripts = json!([{ "tick": 10, "side": "red", "order":
        { "kind": "move", "units": [1], "gesture": 1, "goal": [900, 590], "route": "shortest" } }]);
    let mut b = battle(units, fires(1, &[5, 1200]), scripts);
    run(&mut b, 5);
    let first = blue(&b).contacts[0].clone();
    for _ in 5..1199 {
        b.step();
        assert!(blue(&b).identified.is_empty(), "the tank stays hidden");
        if let Some(now) = blue(&b).contacts.iter().find(|c| c.id == first.id) {
            assert_eq!(
                now.center, first.center,
                "the area stays where it was reported"
            );
        }
    }
    b.step();
    let shooter = b.observe(Side::Red).own[0].position;
    let outside = (shooter[0] - first.center[0]).hypot(shooter[1] - first.center[1]) > radius();
    let firing: Vec<_> = blue(&b)
        .contacts
        .iter()
        .filter(|c| c.source == ContactSource::Firing)
        .collect();
    assert_eq!(firing.len(), 1, "old area expired; one current report");
    assert_eq!(
        firing[0].id != first.id,
        outside,
        "new report exactly when fired from outside the old area"
    );
}

#[test]
fn areas_expire_without_fresh_evidence() {
    let mut b = battle(hidden_shooter(), fires(1, &[5]), json!([]));
    run(&mut b, 5 + lifetime_ticks());
    assert_eq!(blue(&b).contacts.len(), 1, "alive through its lifetime");
    run(&mut b, 2);
    assert!(blue(&b).contacts.is_empty(), "then gone");
}

#[test]
fn an_identified_shooter_adds_no_area() {
    let units = json!([
        { "side": "blue", "kind": "recon", "position": [560, 480] },
        { "side": "red", "kind": "rifle", "position": [560, 560] },
    ]);
    let mut b = battle(units, fires(1, &[5]), json!([]));
    run(&mut b, 6);
    assert_eq!(blue(&b).identified.len(), 1);
    assert!(blue(&b).contacts.is_empty());
}

#[test]
fn losing_sight_leaves_a_fixed_last_seen_area_that_reidentification_retires() {
    // Red's tank drives behind the ridge and, much later, back out.
    let units = json!([
        { "side": "blue", "kind": "recon", "position": [560, 400] },
        { "side": "red", "kind": "tank", "position": [820, 330] },
    ]);
    let scripts = json!([
        { "tick": 1, "side": "red", "order": { "kind": "move", "units": [1], "gesture": 1, "goal": [820, 460], "route": "shortest" } },
        { "tick": 1, "side": "red", "queued": true, "order": { "kind": "move", "units": [1], "gesture": 1, "goal": [820, 330], "route": "shortest" } },
    ]);
    let mut b = battle(units, json!([]), scripts);
    let mut last = None;
    let mut lost = None;
    for _ in 0..2000 {
        b.step();
        let f = blue(&b);
        if let Some(e) = f.identified.first() {
            if lost.is_some() {
                assert!(
                    f.contacts.is_empty(),
                    "identification retires the last-seen area"
                );
                return;
            }
            last = Some(e.position);
        } else if let Some(c) = f
            .contacts
            .iter()
            .find(|c| c.source == ContactSource::LastSeen)
        {
            let at = last.expect("seen before lost");
            assert_eq!(
                c.center,
                [at[0], at[1]],
                "centred on the last sighting, never moved"
            );
            lost.get_or_insert(b.tick());
        }
    }
    panic!("the tank never came back into view");
}

#[test]
fn listeners_hear_unseen_enemies_by_category_range_and_direction() {
    // A hidden tank 280 m east of blue's scout moves; a hidden rifle squad
    // 300 m away idles (beyond infantry hearing).
    let units = json!([
        { "side": "blue", "kind": "recon", "position": [560, 480] },
        { "side": "red", "kind": "tank", "position": [840, 480] },
        { "side": "red", "kind": "rifle", "position": [860, 470] },
    ]);
    let scripts = json!([{ "tick": 1, "side": "red", "order":
        { "kind": "move", "units": [1], "gesture": 1, "goal": [840, 520], "route": "shortest" } }]);
    let mut b = battle(units, fires(2, &[20]), scripts);
    let mut heard = Vec::new();
    for _ in 0..30 {
        b.step();
        heard.extend(blue(&b).audible.clone());
    }
    let engine = heard
        .iter()
        .find(|c| c.category == SoundCategory::Vehicle)
        .expect("the tank is heard");
    assert_eq!(engine.sector, 0, "east");
    assert_eq!(engine.band, SoundBand::Near);
    assert!(engine.moving);
    assert!(
        heard.iter().any(|c| c.category == SoundCategory::Shot),
        "the shot is heard at 300 m"
    );
    assert!(
        !heard.iter().any(|c| c.category == SoundCategory::Infantry),
        "idle infantry at 300 m is not"
    );
    // Cues arrive once per bucket, one per source and category.
    let per_bucket = heard
        .iter()
        .filter(|c| c.category == SoundCategory::Vehicle)
        .count();
    assert_eq!(per_bucket, 2, "30 ticks span two 15-tick buckets");
}

#[test]
fn obstacles_become_known_by_sight() {
    // Two walls appear at tick 10: one in the scout's view, one behind the ridge.
    let events = json!([
        { "tick": 10, "add_prop": { "kind": "wall", "center": [620, 470], "yaw": 0, "half_extents": [0.5, 3, 2] } },
        { "tick": 10, "add_prop": { "kind": "wall", "center": [860, 400], "yaw": 0, "half_extents": [0.5, 3, 2] } },
    ]);
    let units = json!([{ "side": "blue", "kind": "recon", "position": [560, 480] }]);
    let mut b = battle(units, events, json!([]));
    run(&mut b, 30);
    let known: Vec<_> = blue(&b).known_props.iter().map(|p| p.center).collect();
    assert_eq!(known, vec![[620.0, 470.0]]);
    assert!(b.observe(Side::Red).known_props.is_empty());
}
