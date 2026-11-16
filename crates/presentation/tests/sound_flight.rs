//! A round heard in flight: its loop follows it until it lands, and a
//! falling bomb's whistle drops in pitch as it falls.
use presentation::effects::publication::{EffectPublication, Segment};
use presentation::sound::frame::{Command, Heard, Listener, Motion, Sound};
use serde_json::{json, Value};

fn fixture(name: &str) -> Value {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/");
    serde_json::from_str(&std::fs::read_to_string(format!("{root}{name}")).unwrap()).unwrap()
}

/// The game's sound as a player sets it up, every battle sound two seconds long.
fn ready() -> Sound {
    let game = fixture("rules.json");
    let setup = json!({
        "audio": fixture("audio.json"),
        "presentation": fixture("presentation.json"),
        "catalog": fixture("sounds.json"),
        "weapons": game["weapons"],
        "unit_catalog": {"units": [], "soldiers": {}},
        "tick_hz": game["tick_hz"],
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

const HERE: Listener = Listener {
    position: [0.0, 0.0, 0.0],
    forward: [0.0, 1.0, 0.0],
    look: [0.0; 3],
};

#[test]
fn a_falling_bomb_whistles_where_it_is_its_pitch_falling_until_it_lands() {
    let mut s = ready();
    let dt = 1.0 / fixture("rules.json")["tick_hz"].as_f64().unwrap();
    let fall = [
        [0.0, 50.0, 200.0],
        [6.0, 50.0, 150.0],
        [12.0, 50.0, 90.0],
        [18.0, 50.0, 0.0],
    ];
    let mut whistle = None;
    let mut heard: Vec<(f64, f64)> = Vec::new();
    let mut stopped = false;
    for (i, w) in fall.windows(2).enumerate() {
        let tick = i as f64 + 1.0;
        let lands = i == fall.len() - 2;
        let e = EffectPublication {
            tick,
            segments: vec![Segment {
                path: w.to_vec(),
                ricochets: Vec::new(),
                kind: "air_bomb".into(),
                shooter: None,
                hit: if lands { "ground" } else { "none" }.into(),
                normal: None,
            }],
            blasts: Vec::new(),
            shooters: Vec::new(),
            smokes: Vec::new(),
            cook_offs: Vec::new(),
            flare_deploys: Vec::new(),
            grinds: Vec::new(),
            strikes: Vec::new(),
            smoke_clouds: Vec::new(),
        };
        let heard_now = Heard {
            effects: &e,
            launches: &[],
            audible: Vec::new(),
            shouts: Vec::new(),
        };
        s.note(&heard_now, 0.0);
        let clock = (tick - 1.0) * dt + dt / 2.0;
        for c in s.update(clock, clock, clock, &Motion::default(), HERE) {
            match c {
                Command::Start { id, voice } if voice.looping && voice.position.is_some() => {
                    assert!(whistle.is_none(), "one whistle for one bomb");
                    whistle = Some(*id);
                    heard.push((voice.rate, voice.position.unwrap()[2]));
                }
                Command::Set { id, params, .. } if Some(*id) == whistle => {
                    heard.push((params.rate, params.position.unwrap()[2]));
                }
                Command::Stop { id, .. } if Some(*id) == whistle => stopped = true,
                _ => {}
            }
        }
    }
    assert!(whistle.is_some(), "a falling bomb is heard in flight");
    assert_eq!(heard.len(), 2, "heard while it falls: {heard:?}");
    let [(high, up), (low, down)] = heard[..] else {
        unreachable!()
    };
    assert!(down < up, "the whistle falls with the bomb: {heard:?}");
    assert!(low < high, "its pitch drops as it falls: {heard:?}");
    assert!(stopped, "the whistle ends where the bomb lands");
}
