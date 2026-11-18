//! Smoke may exhaust the visual budget, but rounds in flight stay readable.
use presentation::effects::frame::{EffectBatch, EffectFrame, EFFECT_FLOATS};
use presentation::effects::publication::EffectPublication;
use presentation::effects::tables::EffectPresentation;
use serde_json::{json, Value};

#[test]
fn smoke_cannot_crowd_out_a_rounds_flight_glow() {
    let vectors: Value =
        serde_json::from_str(include_str!("fixtures/effect-vectors.json")).unwrap();
    let mut p = vectors["presentation"].clone();
    p["capacity"] = json!(64);
    let mut look = p["looks"]["looks"]["rifle"].clone();
    look["tracer"] = json!({
        "tail_s": 0.05, "tail_m": [1, 10],
        "glow": {"color": [1, 0.3, 0.1], "intensity": 8, "width_m": 0.4, "min_px": 4},
        "smoke": {"albedo": [0.9, 0.9, 0.9], "ribbon": 0.5, "opacity": 0.2,
            "size_m": [0.1, 0.7], "life_s": 2, "rise_mps": 0.1, "spread_mps": 0.1, "spacing_m": 1}
    });
    p["looks"]["looks"]["test_flight"] = look;
    p["looks"]["weapons"]["test_flight"] = json!("test_flight");
    let p = EffectPresentation::from_value(&p).unwrap();
    let mut batch = EffectBatch::new(p.capacity);
    let mut frame = EffectFrame::new(30.0, p);
    let mut exhausted = false;
    for tick in 1..=60 {
        let x = f64::from(tick) * 5.0;
        let publication: EffectPublication = serde_json::from_value(json!({
            "tick": tick, "segments": [{"path": [[x - 5.0, 0, 100], [x, 0, 100]],
                "ricochets": [], "kind": "test_flight", "shooter": null, "hit": "none", "normal": null}],
            "blasts": [], "shooters": [], "smokes": []
        })).unwrap();
        frame.note(&publication);
        frame.build(
            (f64::from(tick) - 0.5) / 30.0,
            &mut batch,
            None,
            &[],
            &[],
            None,
        );
        exhausted |= batch.dropped > 0;
        assert!(
            batch.data[..batch.count * EFFECT_FLOATS]
                .chunks(EFFECT_FLOATS)
                .any(|r| r[12] == 0.0
                    && r[11] == 0.0
                    && (r[4] - (x as f32 - 2.5)).abs() < 0.001
                    && r[8] > 0.0),
            "the round's current head lost its glow at tick {tick} ({} effects dropped)",
            batch.dropped
        );
    }
    assert!(exhausted, "the smoke actually exhausted the visual budget");
}
