//! The effects draw what the browser's `EffectFrame` draws for the same
//! causes, seeded as it seeds them: every instance's sixteen floats and every
//! cast light at fixed clocks after a few publications that exercise every
//! kind of effect, and a fixture run's plate's effect counts replayed from
//! its capture (the browser's TypeScript wrote both before it was deleted).
use std::path::PathBuf;

use presentation::effects::frame::{
    EffectBatch, EffectCounts, EffectFrame, EFFECT_FLOATS, EFFECT_KINDS, SHAPE_NAMES,
};
use presentation::effects::lights::CAST_LIGHT_FLOATS;
use presentation::effects::publication::{EffectFeed, EffectPublication};
use presentation::effects::tables::EffectPresentation;
use presentation::effects::{DrawnRotor, MuzzleSource};
use presentation::seed::js_round;
use presentation::world::{EffectsSection, UnitsSection, Vec3};
use serde_json::Value;
use sim::publication::reader::Reader;

fn fixture(path: &str) -> Value {
    let at = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/parity/capture")
        .join(path);
    let text = std::fs::read_to_string(&at).unwrap_or_else(|e| panic!("{}: {e}", at.display()));
    serde_json::from_str(&text).expect("fixture JSON")
}

/// The browser's vectors regenerated with the shot's kick (`tests/fixtures`).
fn kicked(name: &str) -> Value {
    let at = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    serde_json::from_str(&std::fs::read_to_string(at).unwrap()).expect("fixture JSON")
}

/// The fixture's rounding: `Math.round(v * 1e6) / 1e6`.
fn round(v: f32) -> f64 {
    js_round(f64::from(v) * 1e6) / 1e6
}

fn counts_of(want: &Value) -> EffectCounts {
    let of = |group: &str, name: &str| want[group][name].as_u64().expect("count") as usize;
    EffectCounts {
        running: EFFECT_KINDS.map(|k| of("running", k)),
        instances: SHAPE_NAMES.map(|k| of("instances", k)),
        lights: want["lights"].as_u64().expect("lights") as usize,
    }
}

/// The drawn muzzles the vectors name: (shooter, mount, soldier) → point.
struct Muzzles(Vec<(f64, usize, Option<f64>, Vec3)>);

impl MuzzleSource for Muzzles {
    fn muzzle(&mut self, shooter: f64, mount: usize, soldier: Option<f64>) -> Option<Vec3> {
        let drawn = self
            .0
            .iter()
            .find(|m| (m.0, m.1, m.2) == (shooter, mount, soldier));
        drawn.map(|m| m.3)
    }
}

#[test]
fn effects_match_the_browsers_vectors() {
    let v = kicked("effect-vectors.json");
    let p = EffectPresentation::from_value(&v["presentation"]).expect("presentation");
    let mut batch = EffectBatch::new(p.capacity);
    let mut frame = EffectFrame::new(v["tickHz"].as_f64().unwrap(), p);
    let publications: Vec<EffectPublication> =
        serde_json::from_value(v["publications"].clone()).expect("publications");
    let mut muzzles = Muzzles(serde_json::from_value(v["muzzles"].clone()).expect("muzzles"));
    let rotor: DrawnRotor = serde_json::from_value(v["rotor"].clone()).expect("rotor");
    let mut noted = 0;
    for want in v["frames"].as_array().unwrap() {
        let clock = want["clock"].as_f64().unwrap();
        while noted < want["noted"].as_u64().unwrap() as usize {
            frame.note(&publications[noted]);
            noted += 1;
        }
        frame.build(
            clock,
            &mut batch,
            Some(&mut muzzles),
            std::slice::from_ref(&rotor),
        );
        let instances: Vec<f64> = serde_json::from_value(want["instances"].clone()).unwrap();
        assert_eq!(
            batch.count,
            instances.len() / EFFECT_FLOATS,
            "at {clock} s: instances"
        );
        for (k, &theirs) in instances.iter().enumerate() {
            let mine = round(batch.data[k]);
            assert!(
                mine == theirs,
                "at {clock} s: instance {} float {} is {mine}, not the browser's {theirs}",
                k / EFFECT_FLOATS,
                k % EFFECT_FLOATS
            );
        }
        let lights = want["lights"].as_array().unwrap();
        let list = &batch.lights;
        assert_eq!(list.count, lights.len(), "at {clock} s: lights");
        for (i, light) in lights.iter().enumerate() {
            let row = &list.data[i * CAST_LIGHT_FLOATS..i * CAST_LIGHT_FLOATS + 7];
            let theirs: Vec<f64> = ["at", "radius", "rgb"]
                .iter()
                .flat_map(|k| match &light[k] {
                    Value::Array(a) => a.iter().map(|x| x.as_f64().unwrap()).collect(),
                    x => vec![x.as_f64().unwrap()],
                })
                .collect();
            let mine: Vec<f64> = row.iter().map(|&x| round(x)).collect();
            assert_eq!(mine, theirs, "at {clock} s: light {i}");
            assert_eq!(
                &*list.causes[i],
                light["cause"].as_str().unwrap(),
                "at {clock} s: light {i}"
            );
        }
        assert_eq!(
            frame.counts(clock, &batch),
            counts_of(&want["counts"]),
            "at {clock} s"
        );
    }
}

#[test]
fn a_fixture_runs_plate_counts_are_the_pages_replay_of_its_capture() {
    let manifest = fixture("run/paris-corner/manifest.json");
    let capture = fixture("run/paris-corner/capture.json");
    let units: UnitsSection = serde_json::from_value(manifest["units"].clone()).expect("units");
    let effects: EffectsSection =
        serde_json::from_value(manifest["effects"].clone()).expect("effects");
    let p = EffectPresentation::from_value(&effects.presentation).expect("presentation");
    let mut batch = EffectBatch::new(p.capacity);
    let mut frame = EffectFrame::new(capture["tickHz"].as_f64().unwrap(), p);
    let mut reader = Reader::new(capture["layout"].as_str().unwrap()).expect("layout");
    let mut feed = EffectFeed::new(capture["side"].as_str().unwrap());
    let samples = capture["samples"].as_array().unwrap();
    let mut counts = Vec::new();
    let mut at = 0;
    for f in capture["frames"].as_array().unwrap() {
        let tick = f["tick"].as_f64().unwrap();
        while at < samples.len() && samples[at]["tick"].as_f64().unwrap() <= tick {
            let words: Vec<u32> =
                serde_json::from_value(samples[at]["publication"].clone()).unwrap();
            at += 1;
            let o = reader
                .decode(&words)
                .expect("decodes")
                .expect("an observation");
            frame.note(&feed.publication(&o, &units, &effects));
        }
        let Some(plate) = f["plate"].as_str() else {
            continue;
        };
        let clock = f["simTime"].as_f64().unwrap();
        frame.build(clock, &mut batch, None, &[]);
        counts.push((plate.to_owned(), frame.counts(clock, &batch)));
    }
    let want = manifest["effectCounts"].as_array().unwrap();
    assert_eq!(counts.len(), want.len());
    for ((plate, mine), theirs) in counts.iter().zip(want) {
        assert_eq!(plate, theirs["plate"].as_str().unwrap());
        assert_eq!(*mine, counts_of(theirs), "{plate}");
    }
}
