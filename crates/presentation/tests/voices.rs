//! Which voice lines play: equal to the browser's.
//!
//! `fixtures/chatter-vectors.json` was written by the browser's `Chatter`
//! (TypeScript, before it moved here): each case its set table, its loaded
//! takes and a script of requests, take loads, resets and arbitrations, with
//! every line each arbitration started (voice, take, moment, length) and
//! dropped (why). The cases cover the four-line cap, tier then distance
//! priority (a request without a position nearest), the late drop, a missing
//! take, self-overlap, group collapse, seeded takes, and the workbench's
//! chatter preview.
use presentation::sound::frame::Command;
use presentation::sound::tables::Bus;
use presentation::sound::voices::{line_sound, Chatter, ChatterRules, Request, LINE_GAIN};
use serde_json::{json, Value};

/// `v` with every number as a float, so `0` and `0.0` compare equal and
/// nothing else does.
fn floats(v: &Value) -> Value {
    match v {
        Value::Number(n) => json!(n.as_f64().unwrap()),
        Value::Array(a) => a.iter().map(floats).collect(),
        Value::Object(o) => o.iter().map(|(k, v)| (k.clone(), floats(v))).collect(),
        _ => v.clone(),
    }
}

#[test]
fn every_line_equals_the_browsers() {
    let v: Value = serde_json::from_str(include_str!("fixtures/chatter-vectors.json")).unwrap();
    let mut lines = 0;
    let mut drops = 0;
    for case in v["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let rules: ChatterRules = serde_json::from_value(case["rules"].clone()).unwrap();
        let mut chatter = Chatter::new(rules, case["seed"].as_f64().unwrap());
        for (set, voices) in case["voices"].as_object().unwrap() {
            chatter.set_voices(set, serde_json::from_value(voices.clone()).unwrap());
        }
        let load = |chatter: &mut Chatter, t: &Value| {
            let s = |k: &str| t[k].as_str().unwrap().to_owned();
            chatter.loaded(
                &s("set"),
                &s("voice"),
                &s("trigger"),
                &s("intensity"),
                t["take"].as_u64().unwrap() as u32,
                t["duration"].as_f64().unwrap(),
            );
        };
        for t in case["takes"].as_array().unwrap() {
            load(&mut chatter, t);
        }
        for (i, step) in case["steps"].as_array().unwrap().iter().enumerate() {
            let at = format!("{name}, step {i}");
            if let Some(r) = step.get("request") {
                let request: Request = serde_json::from_value(r.clone()).unwrap();
                assert_eq!(
                    floats(&serde_json::to_value(&request).unwrap()),
                    floats(r),
                    "{at}: request"
                );
                chatter.request(request, step["now"].as_f64().unwrap());
            } else if let Some(t) = step.get("take") {
                load(&mut chatter, t);
            } else if step.get("reset").is_some() {
                chatter.reset();
            } else {
                let u = &step["update"];
                let l: Vec<f64> = serde_json::from_value(u["listener"].clone()).unwrap();
                let got = chatter.update(u["now"].as_f64().unwrap(), [l[0], l[1], l[2]]);
                let started: Vec<Value> = got
                    .started
                    .iter()
                    .map(|s| {
                        json!({ "request": s.request, "voice": s.voice, "take": s.take,
                                "at": s.at, "duration": s.duration })
                    })
                    .collect();
                assert_eq!(
                    floats(&Value::from(started)),
                    floats(&step["started"]),
                    "{at}: started"
                );
                assert_eq!(
                    floats(&serde_json::to_value(&got.dropped).unwrap()),
                    floats(&step["dropped"]),
                    "{at}: dropped"
                );
                // Each line started is one start command on the voices bus.
                assert_eq!(got.commands.len(), got.started.len(), "{at}: commands");
                for (c, s) in got.commands.iter().zip(&got.started) {
                    let Command::Start { id, voice } = c else {
                        panic!("{at}: a start")
                    };
                    let r = &s.request;
                    assert_eq!(*id, s.id);
                    assert_eq!(
                        voice.sound,
                        line_sound(&r.set, &s.voice, &r.trigger, &r.intensity)
                    );
                    assert_eq!(
                        (voice.variant, voice.bus, voice.at, voice.gain),
                        (s.take, Bus::Voices, s.at, LINE_GAIN)
                    );
                    assert!(voice.position.is_none() && voice.pan.is_none() && !voice.looping);
                }
                lines += got.started.len();
                drops += got.dropped.len();
            }
        }
    }
    assert!(
        lines > 200 && drops > 10,
        "{lines} lines, {drops} drops compared"
    );
}

#[test]
fn rules_out_of_range_are_refused() {
    for rules in [
        json!({ "lines": 0, "wait_s": 1, "group_s": 0.25 }),
        json!({ "lines": 4, "wait_s": -1, "group_s": 0.25 }),
        json!({ "lines": 4, "wait_s": 1, "group_s": -0.25 }),
    ] {
        let rules: ChatterRules = serde_json::from_value(rules).unwrap();
        assert!(rules.check().is_err(), "{rules:?}");
    }
}
