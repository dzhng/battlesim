//! Sound uses named distance profiles, independently of recordings and base gains.
use presentation::sound::frame::{
    Command, Listener, Motion, Scripted, Sound, SoundSoldier, SoundVehicle,
};
use serde_json::{json, Value};

fn sound(fire_rolloff: f64) -> Sound {
    sound_with(fire_rolloff, |_| {})
}

fn sound_with(fire_rolloff: f64, edit: impl FnOnce(&mut Value)) -> Sound {
    let fixture = |text| serde_json::from_str::<Value>(text).unwrap();
    let mut audio = fixture(include_str!("../../../fixtures/audio.json"));
    // Fixed test mix: both sources start equally loud, only fire fades quickly.
    audio["distance"]["weapons"] = json!({
        "ref_m": 150, "rolloff": 1.2, "max_m": 3000, "floor": 0.15, "cull": 0.004
    });
    audio["focus"] = json!({"share": 0.3, "inner_deg": 35, "outer_deg": 55});
    audio["fires"]["default"] = json!({
        "sound": "fire", "gain": 0.25, "smoulder_gain": 0.06
    });
    audio["distance"]["burning"] = json!({
        "ref_m": 10, "rolloff": fire_rolloff, "max_m": 3000, "floor": 0.005, "cull": 0.001
    });
    edit(&mut audio);
    let mut presentation = fixture(include_str!("../../../fixtures/presentation.json"));
    presentation["effects"]["smoke"]["wreck"]["burn_s"] = json!(60);
    let game = fixture(include_str!("../../../fixtures/rules.json"));
    let setup = json!({
        "audio": audio, "presentation": presentation,
        "catalog": fixture(include_str!("../../../fixtures/sounds.json")),
        "weapons": game["weapons"], "unit_catalog": {"units": [], "soldiers": {}},
        "tick_hz": 30,
    });
    let mut s = Sound::from_json(&setup.to_string()).unwrap();
    let durations = s
        .tables()
        .battle_sounds()
        .into_iter()
        .map(|n| (n, vec![2.0; 8]))
        .collect();
    s.set_durations(durations).unwrap();
    s
}

fn listener(distance: f64) -> Listener {
    Listener::of_camera(
        [0.0, -0.6 * distance, 0.8 * distance],
        [0.0; 3],
        [0.0, 0.0, 1.0],
    )
}

fn publication() -> Scripted {
    serde_json::from_value(json!({
        "effects": {"tick": 1, "segments": [], "blasts": [], "shooters": [],
            "smokes": [{"key": "wreck", "kind": "wreck", "center": [0,0,0],
                        "half": [1,1,0], "yaw": 0}]},
        "launches": [{"shooter": 1, "mount": 0, "soldier": null,
                      "x": 0, "y": 0, "z": 0, "kind": "tank_ap"}]
    }))
    .unwrap()
}

#[test]
fn burning_is_full_close_faint_at_rts_distance_and_fades_as_the_camera_moves() {
    let mut s = sound(12.0);
    let publication = publication();
    let launches = publication.launches();
    s.note(&publication.heard(&launches), 0.0);
    let commands = s.update(1.0 / 30.0, 0.0, 0.0, &Motion::default(), listener(25.0));
    let (id, near) = commands
        .iter()
        .find_map(|c| match c {
            Command::Start { id, voice } if voice.sound == "fire" => Some((*id, voice.gain)),
            _ => None,
        })
        .expect("a close fire is audible");
    assert_eq!(near, 0.25, "close-up fire keeps its authored volume");
    let mut previous = near;
    for (i, distance) in [65.0, 200.0, 1000.0].into_iter().enumerate() {
        let clock = (i as f64 + 2.0) / 30.0;
        let commands = s.update(clock, clock, clock, &Motion::default(), listener(distance));
        let gain = commands
            .iter()
            .find_map(|c| match c {
                Command::Set {
                    id: voice, params, ..
                } if *voice == id => Some(params.gain),
                _ => None,
            })
            .expect("a faint fire remains audible");
        assert!(
            gain < previous && gain < near * 0.1,
            "distance {distance}: {gain} after {previous}"
        );
        previous = gain;
    }
    let commands = s.update(0.2, 0.2, 0.2, &Motion::default(), listener(11000.0));
    assert!(
        commands
            .iter()
            .any(|c| matches!(c, Command::Stop { id: voice, .. } if *voice == id)),
        "a fire beyond its maximum reach stops"
    );
    let commands = s.update(0.3, 0.3, 0.3, &Motion::default(), listener(25.0));
    assert!(
        commands
            .iter()
            .any(|c| matches!(c, Command::Start { voice, .. }
        if voice.sound == "fire" && voice.gain == near)),
        "zooming back in restores the fire"
    );
}

#[test]
fn each_source_uses_its_profiles_for_playback_and_voice_placement() {
    let mut s = sound_with(12.0, |audio| {
        for (name, radius) in [
            ("ground_engines", 10),
            ("aircraft_engines", 20),
            ("small_details", 30),
            ("weapons", 40),
            ("world_voices", 50),
            ("burning", 60),
        ] {
            audio["distance"][name] = json!({
                "ref_m": radius, "rolloff": 1, "max_m": 1000, "floor": 0, "cull": 0.00001
            });
        }
        let vehicle = |engine, profile| {
            json!({
                "engine": engine, "engine_distance": profile,
                "idle_rate": 1, "load_rate": 1, "idle_gain": 1, "load_gain": 1,
                "running": "wheels", "running_gain": 1, "full_speed_mps": 1,
                "turret": "turret", "turret_gain": 1, "full_traverse_rps": 1,
                "reverse": "reverse_beep", "reverse_gain": 1
            })
        };
        audio["vehicles"]["test_ground"] = vehicle("engine_small", "ground_engines");
        audio["vehicles"]["test_air"] = vehicle("engine_diesel", "aircraft_engines");
        audio["footsteps"] =
            json!({"sound": "footstep", "gain": 1, "stride_m": 1, "max_step_m": 4});
    });
    let at = Listener {
        position: [0.0, -100.0, 0.0],
        forward: [0.0, 1.0, 0.0],
        look: [0.0; 3],
    };
    let mut motion = Motion {
        vehicles: ["test_ground", "test_air"]
            .into_iter()
            .enumerate()
            .map(|(i, class)| SoundVehicle {
                key: i as f64,
                vehicle_class: class.into(),
                position: [0.0; 3],
                travel_l: 0.0,
                travel_r: 0.0,
                turret: 0.0,
                reverse: true,
                rotor_speed: None,
            })
            .collect(),
        soldiers: vec![SoundSoldier {
            id: 1.0,
            position: [-2.0, 0.0, 0.0],
        }],
    };
    let publication = publication();
    let launches = publication.launches();
    s.note(&publication.heard(&launches), 0.0);
    let commands = s.update(0.0, 0.0, 0.0, &motion, at).to_vec();
    let voices: Vec<_> = commands
        .iter()
        .filter_map(|c| match c {
            Command::Start { voice, .. } => Some(voice),
            _ => None,
        })
        .collect();
    for (sound, expected) in [
        ("engine_small", 0.1),
        ("engine_diesel", 0.2),
        ("fire", 0.15),
    ] {
        let gain = voices
            .iter()
            .find(|v| v.sound == sound)
            .expect("the source sounds")
            .gain;
        assert!(
            (gain - expected).abs() < 1e-12,
            "{sound}: {gain} != {expected}"
        );
    }
    let report = voices
        .iter()
        .find(|v| !v.looping)
        .expect("the weapon fired");
    assert!((report.gain / s.tables().shot("tank_ap").gain - 0.4).abs() < 1e-12);
    let mut bark = (*report).clone();
    bark.gain = 1.0;
    s.place(&mut bark, [0.0; 3]);
    assert_eq!(bark.gain, 0.5, "world voices have their own profile");
    for vehicle in &mut motion.vehicles {
        vehicle.travel_l = 1.0;
        vehicle.travel_r = 1.0;
        vehicle.turret = 1.0;
    }
    motion.soldiers[0].position = [0.0; 3];
    let footstep = s.tables().effect("footstep").to_owned();
    let commands = s.update(1.0 / 30.0, 1.0 / 30.0, 1.0 / 30.0, &motion, at);
    for sound in ["reverse_beep", "wheels", "turret", &footstep] {
        let voice = commands
            .iter()
            .find_map(|c| match c {
                Command::Start { voice, .. } if voice.sound == sound => Some(voice),
                _ => None,
            })
            .expect("the detail sounds");
        assert_eq!(voice.gain, 0.3, "{sound} uses the small-details profile");
    }
}
