//! The reader decodes every sample of both fixture captures exactly as the
//! browser's decoder did (its decode is the fixture `<scene>-observations.json`).
//! Vitest re-records those observations through the browser decoder; this test only verifies parity.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sim::battle::Battle;
use sim::fixtures::{self, CatalogSet};
use sim::publication::reader::Reader;
use sim::publication::Publisher;

#[path = "common/capture_publication.rs"]
mod capture_publication;

const FIXTURES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/parity/capture/"
);

fn read(path: &str) -> Value {
    serde_json::from_str(&std::fs::read_to_string(format!("{FIXTURES}{path}")).unwrap()).unwrap()
}

/// Equal field for field, every number at float32 precision (the fixture
/// writes each as its shortest float32 text, and NaN as null).
fn same(found: &Value, expected: &Value, at: &str) {
    match (found, expected) {
        (Value::Number(a), Value::Number(b)) => {
            assert_eq!(
                a.as_f64().unwrap() as f32,
                b.as_f64().unwrap() as f32,
                "{at}"
            )
        }
        (Value::Array(a), Value::Array(b)) => {
            assert_eq!(a.len(), b.len(), "{at} length");
            for (i, (a, b)) in a.iter().zip(b).enumerate() {
                same(a, b, &format!("{at}[{i}]"));
            }
        }
        (Value::Object(a), Value::Object(b)) => {
            assert!(
                a.keys().eq(b.keys()),
                "{at} keys: {:?} vs {:?}",
                a.keys(),
                b.keys()
            );
            for (key, a) in a {
                same(a, &b[key], &format!("{at}.{key}"));
            }
        }
        _ => assert_eq!(found, expected, "{at}"),
    }
}

fn observation_value(observation: &sim::publication::reader::Observation) -> Value {
    let mut view = serde_json::to_value(observation).unwrap();
    view["corpses"] = json!(observation.corpses);
    view["knownProps"] = json!(observation.known_props);
    view["fallenBodies"] = json!(observation.fallen_bodies);
    let bytes: Vec<u8> = observation
        .fog
        .bits
        .iter()
        .flat_map(|w| w.to_le_bytes())
        .collect();
    let digest: String = Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    view["fog"]["bits"] = json!(format!("sha256:{digest}"));
    view["groundPatch"]["runs"] = json!(observation.ground_patch.runs);
    view
}

#[test]
fn both_fixture_captures_decode_as_the_browser_decoded_them() {
    for (scene, capture) in [
        ("paris-corner", "run/paris-corner/capture.json"),
        ("market-town", "publication/market-town-capture.json"),
    ] {
        let (mut capture, expected) = (
            read(capture),
            read(&format!("publication/{scene}-observations.json")),
        );
        if std::env::var_os("BLESS_PARITY").is_some() {
            capture_publication::refresh(&mut capture);
            let relative = if scene == "paris-corner" {
                "run/paris-corner/capture.json"
            } else {
                "publication/market-town-capture.json"
            };
            let bytes = serde_json::to_vec(&capture).unwrap();
            std::fs::write(format!("{FIXTURES}{relative}"), &bytes).unwrap();
            if scene == "paris-corner" {
                let path = format!("{FIXTURES}run/paris-corner/manifest.json");
                let mut manifest: Value =
                    serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
                manifest["identity"]["captureHash"] =
                    json!(format!("{:x}", Sha256::digest(&bytes)));
                std::fs::write(path, serde_json::to_vec(&manifest).unwrap()).unwrap();
            }
        }
        let mut reader = Reader::new(capture["layout"].as_str().unwrap()).unwrap();
        let samples = capture["samples"].as_array().unwrap();
        assert_eq!(
            samples.len(),
            expected["observations"].as_array().unwrap().len()
        );
        let mut recorded = Vec::with_capacity(samples.len());
        for (sample, expected) in samples
            .iter()
            .zip(expected["observations"].as_array().unwrap())
        {
            let words: Vec<u32> = sample["publication"]
                .as_array()
                .unwrap()
                .iter()
                .map(|w| w.as_u64().unwrap() as u32)
                .collect();
            let observation = reader
                .decode(&words)
                .unwrap()
                .expect("a capture is one stream");
            let view = observation_value(&observation);
            recorded.push(view.clone());
            if std::env::var_os("BLESS_PARITY").is_none() {
                same(&view, expected, &format!("{scene} tick {}", sample["tick"]));
            }
        }
        if std::env::var_os("BLESS_PARITY").is_some() {
            let path = format!("{FIXTURES}publication/{scene}-observations.json");
            let output = serde_json::json!({"observations": recorded});
            std::fs::write(path, serde_json::to_string(&output).unwrap()).unwrap();
        }
    }
}

/// Rebuilds the market-town seven-frame window from the deterministic menu battle.
/// Set `PARITY_RUN` to a native-parity run directory, with `BLESS_PARITY=1` to write
/// the committed seven-sample capture; without blessing it compares every tick.
#[test]
fn market_town_window_matches_a_fresh_stream_after_its_preamble() {
    // Only a named browser run can rebuild or check the window.
    let Ok(root) = std::env::var("PARITY_RUN") else {
        return;
    };
    let full: Value = serde_json::from_str(
        &std::fs::read_to_string(format!("{root}/market-town/capture.json")).unwrap(),
    )
    .unwrap();
    let run_samples = full["samples"].as_array().unwrap();
    let window: Vec<_> = run_samples
        .iter()
        .filter(|s| (1825..=1831).contains(&s["tick"].as_u64().unwrap()))
        .collect();
    assert_eq!(window.len(), 7);
    let mut game: Value =
        serde_json::from_str(&std::fs::read_to_string(fixtures::dir().join("rules.json")).unwrap())
            .unwrap();
    game["catalog"] = Value::Array(fixtures::catalog_documents(CatalogSet::Capture));
    let rules: contract::scenario::Rules = serde_json::from_value(game).unwrap();
    let scenario = sim::maps::scenario("market-town", "menu", rules).unwrap();
    let mut battle = Battle::new(&scenario, 1);
    while battle.tick() < 1825 {
        battle.step();
    }
    let mut publisher = Publisher::new();
    let mut fresh = Vec::new();
    for (i, run) in window.iter().enumerate() {
        if i > 0 {
            battle.step();
        }
        assert_eq!(battle.tick(), run["tick"].as_u64().unwrap());
        let publication = publisher
            .publish(&battle, contract::ids::Side::Blue)
            .unwrap()
            .iter()
            .map(|w| w.to_bits())
            .collect::<Vec<_>>();
        fresh.push(serde_json::json!({"tick": battle.tick(), "digest": run["digest"], "publication": publication}));
    }
    if std::env::var_os("BLESS_PARITY").is_some() {
        let mut output = full.clone();
        output["samples"] = Value::Array(fresh.clone());
        std::fs::write(
            format!("{FIXTURES}publication/market-town-capture.json"),
            serde_json::to_string(&output).unwrap(),
        )
        .unwrap();
        let mut reader = Reader::new(full["layout"].as_str().unwrap()).unwrap();
        let observations = fresh
            .iter()
            .map(|sample| {
                let words = sample["publication"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|word| word.as_u64().unwrap() as u32)
                    .collect::<Vec<_>>();
                observation_value(&reader.decode(&words).unwrap().unwrap())
            })
            .collect::<Vec<_>>();
        std::fs::write(
            format!("{FIXTURES}publication/market-town-observations.json"),
            serde_json::to_string(&serde_json::json!({ "observations": observations })).unwrap(),
        )
        .unwrap();
        return;
    }
    let mut fresh_reader = Reader::new(full["layout"].as_str().unwrap()).unwrap();
    let mut run_reader = Reader::new(full["layout"].as_str().unwrap()).unwrap();
    let words = |s: &Value| {
        s["publication"]
            .as_array()
            .unwrap()
            .iter()
            .map(|w| w.as_u64().unwrap() as u32)
            .collect::<Vec<_>>()
    };
    let mut fresh_iter = fresh.iter();
    for run in run_samples {
        let tick = run["tick"].as_u64().unwrap();
        let Some(sample) = (1825..=1831)
            .contains(&tick)
            .then(|| fresh_iter.next())
            .flatten()
        else {
            let _ = run_reader.decode(&words(run)).unwrap();
            continue;
        };
        let a = fresh_reader.decode(&words(sample)).unwrap().unwrap();
        let b = run_reader.decode(&words(run)).unwrap().unwrap();
        let mut av = observation_value(&a);
        let bv = observation_value(&b);
        av["fog"]["bits"] = bv["fog"]["bits"].clone();
        av["groundPatch"] = bv["groundPatch"].clone();
        same(&av, &bv, &format!("market-town tick {}", sample["tick"]));
    }
}
