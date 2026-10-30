//! Sound follows the presentation document's effect clock.
use presentation::sound::frame::{Command, Heard, Listener, Motion, Sound};
use serde_json::{json, Value};

fn setup() -> Value {
    let fixture = |text| serde_json::from_str::<Value>(text).unwrap();
    json!({
        "audio": fixture(include_str!("../../../fixtures/audio.json")),
        "catalog": fixture(include_str!("../../../fixtures/sounds.json")),
        "weapons": fixture(include_str!("../../../fixtures/rules.json"))["weapons"],
        "presentation": fixture(include_str!("../../../fixtures/presentation.json")),
        "tick_hz": 30
    })
}

#[test]
fn sound_uses_the_presentation_smoke_clock_without_a_second_timing_document() {
    let mut setup = setup();
    setup["presentation"]["effects"]["smoke"]["default"]["burn_s"] = json!(1);
    setup["presentation"]["effects"]["smoke"]["default"]["smoulder_s"] = json!(1);
    let mut sound = Sound::from_json(&setup.to_string()).unwrap();
    sound
        .set_durations(
            sound
                .tables()
                .battle_sounds()
                .into_iter()
                .map(|s| (s, vec![1.0]))
                .collect(),
        )
        .unwrap();
    let effects = serde_json::from_value(json!({
        "tick": 1, "segments": [], "blasts": [], "shooters": [],
        "smokes": [{"key": "test_fire", "kind": "default", "center": [0,0,0], "yaw": 0, "half": [1,1,1]}]
    })).unwrap();
    sound.note(
        &Heard {
            effects: &effects,
            launches: &[],
            audible: vec![],
            shouts: vec![],
        },
        0.0,
    );
    let listener = Listener {
        position: [0.0, 0.0, 0.0],
        forward: [0.0, 1.0, 0.0],
    };
    let commands = sound.update(0.5, 0.5, 0.5, &Motion::default(), listener);
    let fire = commands
        .iter()
        .find_map(|c| match c {
            Command::Start { id, voice } if voice.looping && voice.position.is_some() => Some(*id),
            _ => None,
        })
        .expect("the known fire burns");
    let commands = sound.update(2.5, 2.5, 2.5, &Motion::default(), listener);
    assert!(
        commands
            .iter()
            .any(|c| matches!(c, Command::Stop { id, .. } if *id == fire)),
        "the fire stops after the document's burn and smoulder"
    );

    setup["presentation"]["effects"]["cook_off"]["rise_m"] = json!(0);
    let error = Sound::from_json(&setup.to_string())
        .err()
        .expect("invalid cook-off must still be refused");
    assert!(
        error.contains("cook_off") && error.contains("rise_m"),
        "{error}"
    );
}
