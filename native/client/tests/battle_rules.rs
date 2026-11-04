//! Two consecutive battles field and hear their own encounter rules.
use std::path::Path;

use contract::ids::Side;
use native_client::audio;
use presentation::{
    effects::{frame::EffectFrame, publication::EffectFeed, tables::EffectPresentation},
    fielded,
    sound::frame::{Command, Heard, Listener, Motion, Sound},
};
use serde_json::{json, Value};
use sim::{
    battle::Battle,
    publication::{reader::Reader, Publisher},
};

fn fixture(root: &Path, name: &str) -> Value {
    serde_json::from_slice(&std::fs::read(root.join(format!("fixtures/{name}.json"))).unwrap())
        .unwrap()
}

#[test]
fn native_fielding_and_sound_require_firing_parts_from_mounts() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let base: contract::scenario::Rules =
        serde_json::from_value(sim::fixtures::test_rules()).unwrap();
    let mut settings = fixture(&root, "presentation");
    let look = &settings["weapon_looks"]["tank_he"];
    settings["weapon_looks"]["blast_only"] = json!({
        "blast": look["blast"], "sound": {"blast": look["sound"]["blast"]}
    });
    let scratch = std::env::temp_dir().join(format!("native-firing-parts-{}", std::process::id()));
    std::fs::create_dir_all(scratch.join("fixtures")).unwrap();
    std::fs::create_dir_all(scratch.join("assets/runtime")).unwrap();
    std::fs::write(
        scratch.join("fixtures/presentation.json"),
        settings.to_string(),
    )
    .unwrap();
    std::fs::write(
        scratch.join("assets/runtime/catalog.json"),
        json!({"sides": {"blue": [0, 0, 1], "red": [1, 0, 0]}}).to_string(),
    )
    .unwrap();
    let mut results = Vec::new();
    for weapons in [
        json!([]),
        json!(["ambient_blast"]),
        json!(["tank_ap", "ambient_blast"]),
    ] {
        let rules = base.clone().overridden(json!({
            "weapons": {"ambient_blast": {"extends": "tank_he", "name": "Ambient blast", "appearance": "blast_only"}},
            "units": {"test_tank": {"mounts": [{"id": "cannon", "weapons": weapons}]}}
        }).as_object().unwrap()).unwrap();
        let visual = fielded::fielded(&scratch, &rules, &[]).err();
        let setup = audio::setup(
            &rules,
            &settings,
            &fixture(&root, "audio"),
            &fixture(&root, "sounds"),
        )
        .unwrap();
        let sound = Sound::from_json(&setup).err();
        results.push((visual, sound));
    }
    std::fs::remove_dir_all(scratch).unwrap();
    let (visual, sound) = &results[0];
    assert!(
        visual.is_none() && sound.is_none(),
        "unmounted: {visual:?}, {sound:?}"
    );
    for (visual, sound) in &results[1..] {
        for result in [visual, sound] {
            let error = result
                .as_ref()
                .expect("mounted blast-only weapon was admitted");
            assert!(
                error.contains("weapons.ambient_blast.appearance: missing firing tracer"),
                "{error}"
            );
        }
    }
}

#[test]
fn consecutive_battles_present_their_encounters_rules() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let base = presentation::static_world::files::rules(&root, "air").unwrap();
    let map = sim::maps::load("air").unwrap().definition;
    let encounter = json!({
        "units": [{"side":"blue", "kind":"test_tank", "position":[100,200]}, {"side":"red", "kind":"test_supply", "position":[250,200], "engagement":"return_fire_only"}],
        "events": [], "scripts": []
    });
    for changed in [false, true] {
        let mut encounter = encounter.clone();
        let weapon = if changed { "encounter_gun" } else { "tank_ap" };
        encounter["overrides"] = json!({
            "units": {
                "test_tank": {"mounts":[{"id":"cannon","weapons":[weapon]}], "sensors":{"ground_m":3000,"low_air_m":3000,"high_air_m":3000,"sight_shape":{"front":1,"side":1,"rear":1}}},
                "test_supply": {"mounts":[],"body":{"hull":{"hp":1000000}}}
            }
        });
        if changed {
            encounter["overrides"]["weapons"] = json!({"encounter_gun":{"extends":"tank_ap","name":"Encounter gun","appearance":"tank_he"}});
            encounter["overrides"]["tick_hz"] = json!(20);
            encounter["overrides"]["air"] = json!({"low_air":{"cruise_agl_m":25}});
        }
        let encounter: contract::scenario::EncounterDefinition =
            serde_json::from_value(encounter).unwrap();
        let scenario = encounter.on(map.clone(), base.clone()).unwrap();
        sim::admit(&scenario.rules).unwrap();
        let mut battle = Battle::new(&scenario, 7);
        let fielded = fielded::fielded(
            &root,
            battle.rules(),
            &["test_tank".into(), "test_supply".into()],
        )
        .unwrap();
        assert_eq!(
            fielded.units.types["test_tank"].mounts[0].weapons[0].kind,
            weapon
        );
        let effects = EffectPresentation::from_value(&fielded.effects.presentation).unwrap();
        let look = if changed {
            "tank_he"
        } else {
            &base.weapons["tank_ap"].appearance
        };
        assert_eq!(effects.looks.name(weapon), look);
        let setup = audio::setup(
            &scenario.rules,
            &fixture(&root, "presentation"),
            &fixture(&root, "audio"),
            &fixture(&root, "sounds"),
        )
        .unwrap();
        let mut sound = Sound::from_json(&setup).unwrap();
        let expected = effects
            .looks
            .get(weapon)
            .sound
            .shot
            .as_ref()
            .unwrap()
            .near
            .clone();
        let mut effects = EffectFrame::new(f64::from(scenario.rules.tick_hz), effects);
        let mut publisher = Publisher::new();
        let mut reader = Reader::new(&sim::publication::layout_json(&battle)).unwrap();
        let mut feed = EffectFeed::new("blue");
        let mut heard_shot = false;
        for _ in 0..300 {
            let words: Vec<_> = publisher
                .publish(&battle, Side::Blue)
                .unwrap()
                .iter()
                .map(|v| v.to_bits())
                .collect();
            let observation = reader.decode(&words).unwrap().unwrap();
            let publication =
                feed.publication(&observation, &fielded.units, &fielded.effects, None);
            let launches = effects.note(&publication);
            if launches.iter().any(|l| l.kind.as_ref() == weapon) {
                let at = (observation.tick - 1.0) / f64::from(scenario.rules.tick_hz);
                sound.note(
                    &Heard {
                        effects: &publication,
                        launches: &launches,
                        audible: vec![],
                        shouts: vec![],
                    },
                    at - 0.01,
                );
                let commands = sound.update(
                    at - 0.01,
                    at - 0.01,
                    at - 0.01,
                    &Motion::default(),
                    Listener {
                        position: [100.0, 200.0, 2.0],
                        forward: [1.0, 0.0, 0.0],
                    },
                );
                let voice = commands
                    .iter()
                    .find_map(|c| match c {
                        Command::Start { voice, .. } if voice.sound == expected => Some(voice),
                        _ => None,
                    })
                    .expect("the battle's fired weapon sounds its own look");
                assert!(
                    (voice.at - at).abs() < 1e-12,
                    "the shot follows this battle's tick rate"
                );
                heard_shot = true;
                break;
            }
            battle.step();
        }
        assert!(heard_shot, "the encounter's gun never presented a shot");
        let helicopter = scenario.rules.catalog.by_id("test_transport_heli");
        assert_eq!(
            scenario.rules.air.cruise_agl_m(&helicopter.mobility),
            if changed { 25.0 } else { 30.0 }
        );
    }
}
