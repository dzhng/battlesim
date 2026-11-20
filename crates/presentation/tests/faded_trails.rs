//! An expired smoke ribbon must disappear rather than become additive light.
use presentation::effects::frame::{EffectBatch, EffectFrame, EFFECT_FLOATS};
use presentation::effects::publication::EffectPublication;
use presentation::effects::tables::EffectPresentation;
use serde_json::{json, Value};

#[test]
fn a_faded_smoke_trail_does_not_glow() {
    let vectors: Value =
        serde_json::from_str(include_str!("fixtures/effect-vectors.json")).unwrap();
    let mut p = vectors["presentation"].clone();
    p["looks"]["looks"]["bomb"]["tracer"]["smoke"] = json!({
        "albedo": [0.9, 0.9, 0.9], "ribbon": 0.5, "opacity": 0.2,
        "size_m": [0.1, 0.7], "life_s": 2, "rise_mps": 0.1,
        "spread_mps": 0.1, "fade_in_s": 0.25, "spacing_m": 1
    });
    let p = EffectPresentation::from_value(&p).unwrap();
    let mut batch = EffectBatch::new(p.capacity);
    let mut frame = EffectFrame::new(30.0, p);
    let publication: EffectPublication = serde_json::from_value(json!({
        "tick": 1, "segments": [{"path": [[0, 0, 100], [3, 0, 100]],
            "ricochets": [], "kind": "air_bomb", "shooter": null, "hit": "none", "normal": null}],
        "blasts": [], "shooters": [], "smokes": []
    }))
    .unwrap();
    frame.note(&publication);
    // The ribbon's start has faded fully; its final end has not expired yet.
    frame.build(2.02, &mut batch, None, &[], &[], None);
    assert!(
        batch.data[..batch.count * EFFECT_FLOATS]
            .chunks(EFFECT_FLOATS)
            .all(|r| r[12] != 0.0 || r[11] > 0.0),
        "faded smoke must not turn into an emissive streak"
    );
}
