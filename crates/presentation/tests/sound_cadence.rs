//! Firing recordings must cover rounds the fielded weapon can actually fire.
use presentation::sound::tables::SoundTables;
use serde_json::{json, Value};

fn fixture(name: &str) -> Value {
    serde_json::from_str(
        &std::fs::read_to_string(format!(
            "{}/../../fixtures/{name}.json",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap(),
    )
    .unwrap()
}

fn field(magazine: Value, interval: f64, shots: usize) -> Result<SoundTables, String> {
    let mut base = fixture("rules")["weapons"]["rifle"].clone();
    base["appearance"] = json!("rifle");
    // The base uses a single report, independently of the shipped look's tuning.
    let weapons = json!({"base": base, "derived": {"extends":"base",
        "appearance":"cadence_test", "magazine":magazine}});
    let mut presentation = fixture("presentation");
    let mut catalog = fixture("sounds");
    catalog["sources"]["cadence-source"] = json!({
        "label":"Test source", "author":"Test", "license":"CC0-1.0",
        "url":"https://example.com/test.wav", "path":"assets/third-party/audio/test.wav",
        "sha256":"0".repeat(64), "notes":"Admission test; never decoded."
    });
    catalog["clips"]["cadence-clip"] = json!({
        "label":"Test reports", "category":"test", "role":"shot", "frames":48000,
        "sample_rate":48000, "sha256":"0".repeat(64), "url":"/audio/clips/cadence-clip.wav",
        "loop":false, "processing":"weighted-shot", "source":"cadence-source",
        "source_rate":48000, "source_frames":[0,48000]
    });
    if shots > 1 {
        catalog["clips"]["cadence-clip"]["burst"] = json!({
            "shots":(0..shots).map(|i| [i*100,i*100+100]).collect::<Vec<_>>(),
            "interval_s":interval
        });
    }
    catalog["sounds"]["cadence-recipe"] = json!({
        "label":"Test burst", "clips":["cadence-clip"], "synth":null,
        "synth_gain":0, "gain":1, "loop":false
    });
    let mut look = presentation["weapon_looks"]["rifle"].clone();
    look["sound"]["shot"]["near"] = json!("cadence-recipe");
    look["sound"]["shot"]["far"] = json!("cadence-recipe");
    presentation["weapon_looks"]["cadence_test"] = look;
    // Keep the inherited base valid even when this test's burst is inadmissible.
    catalog["sounds"]["single-recipe"] = json!({
        "label":"Test single", "clips":[], "synth":"rifle", "synth_gain":1,
        "gain":1, "loop":false
    });
    presentation["weapon_looks"]["rifle"]["sound"]["shot"]["near"] = json!("single-recipe");
    presentation["weapon_looks"]["rifle"]["sound"]["shot"]["far"] = json!("single-recipe");
    SoundTables::from_parts(fixture("audio"), catalog, &presentation, &weapons)
}

fn magazine(rounds: usize, interval: f64, burst: Option<usize>) -> Value {
    json!({"rounds":rounds, "shot_interval_s":interval,
        "burst":burst.map(|rounds| json!({"rounds":rounds,"aim_max_s":0}))})
}

#[test]
fn fielding_refuses_a_recorded_burst_off_the_derived_weapons_cadence() {
    let error = field(magazine(20, 0.2, Some(3)), 0.1, 3)
        .expect_err("a 100 ms recording cannot sound a 200 ms weapon");
    assert!(
        error.contains("derived") && error.contains("cadence_test") && error.contains("cadence"),
        "{error}"
    );
}

#[test]
fn fielding_bounds_a_recording_by_both_burst_and_magazine() {
    for magazine in [
        Value::Null,
        magazine(20, 0.1, Some(2)),
        magazine(2, 0.1, None),
    ] {
        let error = field(magazine, 0.1, 3)
            .expect_err("the recording must not sound rounds the weapon cannot supply");
        assert!(
            error.contains("derived") && error.contains("shot count"),
            "{error}"
        );
    }
    field(magazine(20, 0.1, Some(3)), 0.1, 3).unwrap();
    field(magazine(20, 0.1, None), 0.1, 3).unwrap();
    field(Value::Null, 0.0, 1).unwrap();
}

#[test]
fn fielding_allows_five_ms_of_recorded_spacing_error_but_no_more() {
    for interval in [0.095, 0.105] {
        field(magazine(20, 0.1, Some(3)), interval, 3).unwrap();
    }
    for interval in [0.0949, 0.1051] {
        let error = field(magazine(20, 0.1, Some(3)), interval, 3)
            .expect_err("more than five ms per shot is refused");
        assert!(error.contains("within 5 ms"), "{error}");
    }
}
