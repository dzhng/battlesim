//! The native player, rendered to a buffer (nothing plays aloud): it sounds
//! what the presentation crate's sound decides, at its level, and frees what
//! it stops. Needs the recordings (`git lfs pull --include="assets/runtime/audio/clips/**"`);
//! the score and its voice line need one track and one pack's bark
//! (`--include="assets/runtime/audio/music/tension_sparse_1.*,assets/runtime/audio/voices/us_tank_crew/pack.json,assets/runtime/audio/voices/us_tank_crew/soldier-under_fire-stressed-*"`).
use std::path::Path;

use native_client::audio::{self, Audio, OutputSettings};
use native_client::bundle::RUNTIME;
use presentation::effects::launches::Launch;
use presentation::effects::publication::EffectPublication;
use presentation::sound::frame::{Heard, Listener, Motion, Shout};
use presentation::sound::voices::Speakers;
use serde_json::{json, Value};

fn read(name: &str) -> Value {
    let path = format!("{}/../../fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn player() -> Audio {
    let setup = audio::setup(&read("game.json"), &read("sounds.json"));
    let output = OutputSettings::Buffer(audio::SAMPLE_RATE);
    Audio::new(&setup, Path::new(RUNTIME), output).unwrap()
}

const HERE: Listener = Listener {
    position: [0.0, 0.0, 2.0],
    forward: [0.0, 1.0, 0.0],
};

/// A cannon fired at `at` on tick 1, heard for `seconds`: the rendered
/// samples, after the bed's first second (heard alone, as a floor).
fn cannon(at: [f64; 3], seconds: f64) -> (Vec<f32>, Audio) {
    let mut a = player();
    let effects = EffectPublication {
        tick: 1.0,
        segments: Vec::new(),
        blasts: Vec::new(),
        shooters: Vec::new(),
        smokes: Vec::new(),
        cook_offs: Vec::new(),
    };
    let launches = [Launch {
        shooter: 2.0,
        mount: 0,
        soldier: None,
        at,
        dir: [0.0, 1.0, 0.0],
        kind: "tank_ap".into(),
        hull: true,
    }];
    let mut now = 0.0;
    let dt = 1.0 / 60.0;
    while now < seconds {
        if (now - 1.0).abs() < dt / 2.0 {
            let heard = Heard {
                effects: &effects,
                launches: &launches,
                audible: Vec::new(),
                shouts: Vec::new(),
            };
            let c = a.sound_mut().note(&heard, now).to_vec();
            a.run(&c, now);
        }
        let clock = (now - 1.0).max(0.0);
        let c = a
            .sound_mut()
            .update(clock, now, now, &Motion::default(), HERE)
            .to_vec();
        a.run(&c, now);
        now += dt;
    }
    a.render(now);
    (a.samples().to_vec(), a)
}

fn rms(samples: &[f32]) -> f64 {
    (samples.iter().map(|s| f64::from(*s).powi(2)).sum::<f64>() / samples.len() as f64).sqrt()
}

/// Interleaved stereo seconds `from..to`.
fn span(samples: &[f32], from: f64, to: f64) -> &[f32] {
    let at = |t: f64| (t * f64::from(audio::SAMPLE_RATE)) as usize * 2;
    &samples[at(from)..at(to).min(samples.len())]
}

#[test]
fn a_cannon_sounds_over_the_bed_louder_near_than_far_and_ends() {
    let (near, a) = cannon([0.0, 20.0, 1.0], 4.0);
    let (far, _) = cannon([0.0, 1500.0, 1.0], 4.0);
    let bed = rms(span(&near, 0.2, 0.9));
    let shot = |s: &[f32]| rms(span(s, 1.0, 1.4));
    assert!(bed > 0.0, "the countryside bed sounds");
    assert!(
        shot(&near) > 4.0 * bed,
        "near {} over bed {bed}",
        shot(&near)
    );
    assert!(
        shot(&near) > 2.0 * shot(&far),
        "near {} far {}",
        shot(&near),
        shot(&far)
    );
    assert!(near.iter().all(|s| s.abs() <= 1.0), "the limiter holds");
    let played = a.played();
    assert_eq!(played.started, 2, "the bed and the shot");
    assert_eq!(played.live, 1, "the shot played out; the bed loops on");
}

#[test]
fn a_shot_to_the_right_is_heard_on_the_right() {
    let (east, _) = cannon([60.0, 0.0, 1.0], 2.0);
    let (west, _) = cannon([-60.0, 0.0, 1.0], 2.0);
    let side = |s: &[f32], ch: usize| {
        let w = span(s, 1.0, 1.4);
        rms(&w.iter().skip(ch).step_by(2).copied().collect::<Vec<_>>())
    };
    assert!(side(&east, 1) > 2.0 * side(&east, 0));
    assert!(side(&west, 0) > 2.0 * side(&west, 1));
}

/// The one track the score test's tension mood holds, and the set and unit
/// kind whose bark it hears.
const TRACK: &str = "tension_sparse_1";
const SET: &str = "us_tank_crew";
const KIND: &str = "us_m1_abrams_sep_v2";
const SHOUT_S: f64 = 6.0;

/// Ten seconds of a battle whose score's tension mood is [`TRACK`] (or
/// nothing, `music` false), heard at [`HERE`], where a seen enemy tank
/// shouts "under fire" 36 m off at [`SHOUT_S`] (`shout`), with the voices
/// bus at `voices` times its level.
fn scored(music: bool, shout: bool, voices: f64) -> (Vec<f32>, Audio) {
    let mut game = read("game.json");
    let bus = &mut game["presentation"]["audio"]["buses"]["voices"];
    *bus = json!(bus.as_f64().unwrap() * voices);
    let mut catalog = read("sounds.json");
    catalog["music"]["moods"]["tension"] = json!(if music { vec![TRACK] } else { vec![] });
    catalog["music"]["moods"]["combat"] = json!([]);
    let setup = audio::setup(&game, &catalog);
    let output = OutputSettings::Buffer(audio::SAMPLE_RATE);
    let mut a = Audio::new(&setup, Path::new(RUNTIME), output).unwrap();
    a.speakers(Speakers {
        own: Default::default(),
        enemy: [(KIND.to_owned(), SET.to_owned())].into(),
    });
    let effects = |tick| EffectPublication {
        tick,
        segments: Vec::new(),
        blasts: Vec::new(),
        shooters: Vec::new(),
        smokes: Vec::new(),
        cook_offs: Vec::new(),
    };
    let hz = read("game.json")["tick_hz"].as_f64().unwrap();
    let dt = 1.0 / 60.0;
    let mut now = 0.0;
    while now < 10.0 {
        let c = a
            .sound_mut()
            .update(now, now, now, &Motion::default(), HERE)
            .to_vec();
        a.run(&c, now);
        if shout && (now - SHOUT_S).abs() < dt / 2.0 {
            let shouts = vec![Shout {
                own: false,
                id: 9.0,
                kind: KIND.into(),
                position: [-20.0, 30.0, 0.0],
                trigger: "b5_under_fire".into(),
                intensity: "stressed".into(),
            }];
            let fx = effects((now * hz).round() + 1.0);
            let heard = Heard {
                effects: &fx,
                launches: &[],
                audible: Vec::new(),
                shouts,
            };
            let c = a.sound_mut().note(&heard, now).to_vec();
            a.run(&c, now);
        }
        a.speak(now);
        now += dt;
    }
    a.render(now);
    assert!(a.errors().is_empty(), "{:?}", a.errors());
    (a.samples().to_vec(), a)
}

/// `a − b`, sample by sample: what one run adds over another.
fn minus(a: &[f32], b: &[f32]) -> Vec<f32> {
    a.iter().zip(b).map(|(a, b)| a - b).collect()
}

#[test]
fn the_score_streams_a_voice_line_sounds_and_the_score_dips_under_it() {
    let (bed, _) = scored(false, false, 1.0);
    let (score, a) = scored(true, false, 1.0);
    let (unheard, _) = scored(true, true, 0.0);
    let (heard, b) = scored(true, true, 1.0);
    assert_eq!((a.played().tracks, a.played().lines), (1, 0));
    assert_eq!((b.played().tracks, b.played().lines), (1, 1));
    // How long the line lasts: its takes' longest.
    let pack: Value = serde_json::from_str(
        &std::fs::read_to_string(format!("{RUNTIME}/audio/voices/{SET}/pack.json")).unwrap(),
    )
    .unwrap();
    let line = pack["takes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|t| {
            t["voice"] == "soldier" && t["trigger"] == "under_fire" && t["intensity"] == "stressed"
        })
        .map(|t| t["duration_s"].as_f64().unwrap())
        .fold(0.0, f64::max);
    assert!(line > 0.5, "the line lasts {line} s");

    // The score: the track fades in from silence, then streams on.
    let music = minus(&score, &bed);
    let full = rms(span(&music, 5.0, SHOUT_S));
    assert!(full > 1e-3, "the track sounds: {full}");
    assert!(rms(span(&music, 0.0, 0.3)) < 0.2 * full, "it fades in");

    // The line: heard where the lines run differ, and nowhere before it.
    let voice = minus(&heard, &unheard);
    let spoken = rms(span(&voice, SHOUT_S, SHOUT_S + line));
    assert!(spoken > 1e-3, "the line sounds: {spoken}");
    assert!(rms(span(&voice, 4.0, SHOUT_S)) < 1e-6, "nothing before it");

    // Under the line the music dips by the duck depth, and recovers after.
    let depth = read("game.json")["presentation"]["audio"]["music"]["duck"]["depth"]
        .as_f64()
        .unwrap();
    let dipped = minus(&unheard, &bed);
    let share = |from: f64, to: f64| rms(span(&dipped, from, to)) / rms(span(&music, from, to));
    let under = share(SHOUT_S + 0.2, SHOUT_S + line - 0.1);
    assert!(
        (under - (1.0 - depth)).abs() < 0.05,
        "under the line {under}"
    );
    let after = share(SHOUT_S + line + 1.5, 10.0);
    assert!(after > 0.97, "after it {after}");
    assert!(heard.iter().all(|s| s.abs() <= 1.0), "the limiter holds");
}
