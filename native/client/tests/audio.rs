//! The native player, rendered to a buffer (nothing plays aloud): it sounds
//! what the presentation crate's sound decides, at its level, and frees what
//! it stops. Needs the recordings (`git lfs pull --include="assets/runtime/audio/clips/**"`).
use std::path::Path;

use native_client::audio::{self, Audio, OutputSettings};
use native_client::bundle::RUNTIME;
use presentation::effects::launches::Launch;
use presentation::effects::publication::EffectPublication;
use presentation::sound::frame::{Heard, Listener, Motion};
use serde_json::Value;

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
