//! The reader decodes every sample of both fixture captures exactly as the
//! browser's decoder did (its decode is the fixture `<scene>-observations.json`).
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sim::publication::reader::Reader;

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

#[test]
fn both_fixture_captures_decode_as_the_browser_decoded_them() {
    for (scene, capture) in [
        ("paris-corner", "run/paris-corner/capture.json"),
        ("market-town", "publication/market-town-capture.json"),
    ] {
        let (capture, expected) = (
            read(capture),
            read(&format!("publication/{scene}-observations.json")),
        );
        let mut reader = Reader::new(capture["layout"].as_str().unwrap()).unwrap();
        let samples = capture["samples"].as_array().unwrap();
        assert_eq!(
            samples.len(),
            expected["observations"].as_array().unwrap().len()
        );
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
            let mut view = serde_json::to_value(&observation).unwrap();
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
            same(&view, expected, &format!("{scene} tick {}", sample["tick"]));
        }
    }
}
