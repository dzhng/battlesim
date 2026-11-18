//! Physical countermeasure paths stay visible at their published height.
use presentation::effects::frame::{EffectBatch, EffectFrame};
use presentation::effects::publication::EffectPublication;
use presentation::effects::tables::EffectPresentation;
use serde_json::json;

#[test]
fn a_visible_flare_draws_along_its_published_path_and_holds_while_paused() {
    let vectors: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/effect-vectors.json")).unwrap();
    let mut style = vectors["presentation"].clone();
    style["flare"] = json!({
        "tail_s": 0.1, "tail_m": [0.1, 2.0],
        "glow": {"color": [1, 0.8, 0.4], "intensity": 4, "width_m": 0.1, "min_px": 3}
    });
    let style = EffectPresentation::from_value(&style).unwrap();
    let mut effects = EffectFrame::new(30.0, style);
    let publication: EffectPublication = serde_json::from_value(json!({
        "tick": 1, "segments": [], "blasts": [], "smokes": [],
        "flares": [{"from": [0, 0, 20], "to": [0, 1, 19]}]
    }))
    .unwrap();
    effects.note(&publication);
    let clock = 1.0 / 30.0;
    let mut batch = EffectBatch::new(64);
    effects.build(clock, &mut batch, None, &[], &[], None);
    assert!(batch.count > 0, "the physical flare is visible");
    let first = batch.data[..batch.count * 16].to_vec();
    for row in first.as_chunks::<16>().0 {
        assert!(
            (19.0..=20.0).contains(&row[2]),
            "the glow stays airborne: {row:?}"
        );
        assert!(
            (19.0..=20.0).contains(&row[6]),
            "the trail follows the falling path: {row:?}"
        );
    }
    effects.build(clock, &mut batch, None, &[], &[], None);
    assert_eq!(
        &batch.data[..batch.count * 16],
        &first,
        "a paused flare holds still"
    );
    effects.build(2.0, &mut batch, None, &[], &[], None);
    assert_eq!(
        batch.count, 0,
        "a flare with no further observed path fades out"
    );
}
