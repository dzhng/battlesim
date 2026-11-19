//! Rotor loops follow the speed carried by the drawn pose's emitter.
use presentation::records::{Emitter, NONE};
use presentation::sound::frame::{Command, Listener, Motion, Sound};
use presentation::sound::idle::idle_loops;
use serde_json::{json, Value};

fn sound() -> Sound {
    let fixture = |text| serde_json::from_str::<Value>(text).unwrap();
    let rules = fixture(include_str!("../../../fixtures/rules.json"));
    Sound::from_json(
        &json!({
            "audio": fixture(include_str!("../../../fixtures/audio.json")),
            "catalog": fixture(include_str!("../../../fixtures/sounds.json")),
            "presentation": fixture(include_str!("../../../fixtures/presentation.json")),
            "weapons": rules["weapons"], "tick_hz": rules["tick_hz"],
            "unit_catalog": {"units": [], "soldiers": {}}
        })
        .to_string(),
    )
    .unwrap()
}

const HERE: Listener = Listener {
    position: [0.0; 3],
    forward: [0.0, 1.0, 0.0],
    look: [0.0; 3],
};

fn engine(class: &str, speed: f32) -> (f64, f64) {
    let mut s = sound();
    let emitter = Emitter {
        unit: 1,
        member: NONE,
        class: 0,
        flags: 0,
        position: [0.0; 4],
        motion: [0.0, 0.0, 0.0, speed],
    };
    let motion = Motion::of(&[emitter], |_| Some(class));
    s.update(0.0, 0.0, 0.0, &motion, HERE)
        .iter()
        .find_map(|c| match c {
            Command::Start { voice, .. } if voice.looping && voice.position.is_some() => {
                Some((voice.gain, voice.rate))
            }
            _ => None,
        })
        .unwrap_or((0.0, 0.0))
}

#[test]
fn rotor_gain_and_pitch_follow_the_emitter_to_silence() {
    for class in [
        "air_light",
        "air_light_logistics",
        "air_medium",
        "air_heavy",
    ] {
        assert_eq!(engine(class, 1.0).1, 1.0, "{class} plays at recorded pitch");
    }
    let levels: Vec<_> = [1.0, 0.75, 0.5, 0.25, 0.0]
        .map(|s| engine("air_light", s))
        .into();
    assert!(levels[0].0 > 0.0);
    assert_eq!(
        levels.last().unwrap().0,
        0.0,
        "stopped blades are silent: {levels:?}"
    );
    for pair in levels.windows(2) {
        assert!(
            pair[0].0 > pair[1].0,
            "gain falls with rotor speed: {levels:?}"
        );
        assert!(
            pair[0].1 > pair[1].1,
            "pitch falls with rotor speed: {levels:?}"
        );
    }
}

#[test]
fn full_rotor_and_non_rotor_engines_keep_their_authored_levels() {
    let s = sound();
    for (class, speed) in [
        ("air_light", 1.0),
        ("tracked_heavy", -1.0),
        ("high_air_light", -1.0),
    ] {
        let baseline = idle_loops(s.tables(), Some(class), false)
            .into_iter()
            .find(|l| l.slot == "engine")
            .unwrap();
        assert_eq!(
            engine(class, speed),
            (baseline.gain, baseline.rate),
            "{class}"
        );
    }
}

#[test]
fn a_published_landing_carries_the_pose_speed_to_sound() {
    use contract::ids::Side;
    use presentation::presentation::{Presentation, Ui};
    use presentation::view::Camera;
    use presentation::world::{BattleInput, SoldierBody, WorldInput, Xray};
    use sim::{
        battle::Battle,
        publication::{self, Publisher},
    };

    let kinds = [
        "test_supply_heli",
        "test_transport_heli",
        "test_jet",
        "test_tank",
    ];
    let scenario = serde_json::from_value(json!({
        "map": {"size": [1000, 1000], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35},
        "rules": sim::fixtures::test_rules(),
        "units": kinds.iter().enumerate().map(|(i, kind)| json!({
            "side": "blue", "kind": kind, "position": [200 + i * 150, 400]
        })).collect::<Vec<_>>(), "events": [], "scripts": []
    }))
    .unwrap();
    let mut battle = Battle::new(&scenario, 1);
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fielded =
        presentation::fielded::fielded(&root, battle.rules(), &kinds.map(String::from)).unwrap();
    let hz = f64::from(battle.rules().tick_hz);
    let mut p = Presentation::new(Some(&publication::layout_json(&battle)), hz).unwrap();
    p.load_world(
        WorldInput {
            battle: Some(BattleInput {
                units: fielded.units,
                effects: fielded.effects,
                side: "blue".into(),
                tick_hz: hz,
                soldier: SoldierBody {
                    radius_m: 0.2,
                    height_m: 1.8,
                },
                xray: Xray {
                    own: [0.0; 4],
                    selected: [0.0; 4],
                    ally: [0.0; 4],
                },
                blocked: [0.0; 4],
                stand_in: None,
            }),
            sun_azimuth: 0.0,
            sun_elevation: 0.7,
            buildings: None,
        },
        fielded.tables,
        None,
    )
    .unwrap();
    let camera = Camera {
        eye: [500.0, -300.0, 900.0],
        target: [500.0, 400.0, 0.0],
        up: [0.0, 0.0, 1.0],
        fov_y: 0.9,
        near: 0.1,
        size: [1280, 720],
    };
    let mut publisher = Publisher::new();
    let mut mix = sound().tables().audio.clone();
    mix.vehicle_class_overrides
        .insert("test_transport_heli".into(), "air_heavy".into());
    let mut speeds = Vec::new();
    let mut settled = false;
    for _ in 0..900 {
        let now = battle.tick() as f64 / hz;
        let words: Vec<_> = publisher
            .publish(&battle, Side::Blue)
            .unwrap()
            .iter()
            .map(|f| f.to_bits())
            .collect();
        p.push_publication(&words, now).unwrap();
        p.frame(now + 1.0 / hz, &camera, &Ui::default(), 0).unwrap();
        let motion = p.motion(&mix);
        let vehicle = |id| motion.vehicles.iter().find(|v| v.key == id * 2.0).unwrap();
        speeds.push(vehicle(0.0).rotor_speed.unwrap());
        assert_eq!(
            vehicle(1.0).rotor_speed,
            Some(1.0),
            "transport stays at flight speed"
        );
        assert_eq!(vehicle(1.0).vehicle_class, "air_heavy");
        assert_eq!(vehicle(2.0).rotor_speed, None, "jets have no rotor input");
        assert_eq!(
            vehicle(3.0).rotor_speed,
            None,
            "ground vehicles have no rotor input"
        );
        if speeds.last() == Some(&0.0) {
            settled = true;
            break;
        }
        battle.step();
    }
    assert!(settled, "the helicopter lands and stops its rotor");
    assert_eq!(speeds[0], 1.0);
    assert!(
        speeds.iter().any(|s| *s > 0.0 && *s < 1.0),
        "wind-down is gradual"
    );
    assert!(speeds.windows(2).all(|w| w[1] <= w[0]));
}
