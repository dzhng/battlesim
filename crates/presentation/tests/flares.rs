//! Physical countermeasure paths stay visible at their published height.
use presentation::effects::frame::{EffectBatch, EffectFrame};
use presentation::effects::publication::EffectPublication;
use presentation::effects::tables::EffectPresentation;
use serde_json::json;

#[test]
fn a_visible_flare_draws_along_its_published_path_and_holds_while_paused() {
    let mut effects = effects();
    let publication: EffectPublication = serde_json::from_value(json!({
        "tick": 1, "segments": [], "blasts": [], "smokes": [],
        "flares": [{"from": [0, 0, 20], "to": [0, 0.1, 19.9]}]
    }))
    .unwrap();
    effects.note(&publication);
    let clock = 1.0 / 30.0;
    let mut batch = EffectBatch::new(512);
    effects.build(clock, &mut batch, None, &[], &[], None);
    assert!(batch.count > 0, "the physical flare is visible");
    let heads: Vec<_> = batch.data[..batch.count * 16]
        .as_chunks::<16>()
        .0
        .iter()
        .filter(|r| r[12] == 1.0)
        .collect();
    assert!(
        heads.len() > 8,
        "a deployment draws a shower of distinct burning pellets"
    );
    assert!(
        heads.iter().any(|a| heads.iter().any(|b| a[..3] != b[..3])),
        "pellets spread instead of stacking on one glow"
    );
    let first = batch.data[..batch.count * 16].to_vec();
    for row in first.as_chunks::<16>().0 {
        assert!(
            (19.0..=20.0).contains(&row[2]),
            "the glow stays airborne: {row:?}"
        );
        assert!(
            row[12] != 0.0 || (19.0..=20.0).contains(&row[6]),
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

fn effects() -> EffectFrame {
    EffectFrame::new(30.0, style())
}

fn style() -> EffectPresentation {
    let vectors: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/effect-vectors.json")).unwrap();
    let mut style = vectors["presentation"].clone();
    style["flare"] = json!({
        "pellets": 24, "spread_m": 16, "spread_s": 0.6,
        "tracer": {
        "tail_s": 0.1, "tail_m": [0.1, 2.0],
        "glow": {"color": [1, 0.8, 0.4], "intensity": 4, "width_m": 0.1, "min_px": 3},
        "head_glow": {"color": [1, 0.8, 0.4], "intensity": 4, "width_m": 0.1, "min_px": 3}
        }
    });
    EffectPresentation::from_value(&style).unwrap()
}

#[test]
fn a_shower_keeps_its_motion_across_ticks_and_settles_slowly() {
    let mut effects = effects();
    let mut batch = EffectBatch::new(1024);
    let point = |tick: f64| {
        let t = tick / 30.0;
        [2.0 * t, 0.0, 60.0 - 3.0 * t]
    };
    let mut last: Vec<[f32; 3]> = Vec::new();
    for tick in 1..=60 {
        let publication: EffectPublication = serde_json::from_value(json!({
            "tick": tick, "segments": [], "blasts": [], "smokes": [],
            "flares": [{"from": point(f64::from(tick - 1)), "to": point(f64::from(tick))}]
        }))
        .unwrap();
        effects.note(&publication);
        effects.build(f64::from(tick) / 30.0, &mut batch, None, &[], &[], None);
        let pellets: Vec<[f32; 3]> = batch.data[..batch.count * 16]
            .as_chunks::<16>()
            .0
            .iter()
            .filter(|r| r[12] == 1.0)
            .skip(1)
            .map(|r| [r[0], r[1], r[2]])
            .collect();
        assert!(!pellets.is_empty());
        if tick == 60 {
            assert_eq!(pellets.len(), last.len(), "the same shower continues");
            for (a, b) in last.iter().zip(&pellets) {
                let fall = a[2] - b[2];
                assert!(
                    fall > 0.0 && fall < 0.2,
                    "a burning pellet settles slowly instead of dropping ballistically: {fall}"
                );
                assert!(
                    (b[0] - a[0]).abs() < 0.2 && (b[1] - a[1]).abs() < 0.2,
                    "drag slows the outward ejection"
                );
            }
            let across = pellets
                .iter()
                .map(|p| p[1])
                .fold(f32::NEG_INFINITY, f32::max)
                - pellets.iter().map(|p| p[1]).fold(f32::INFINITY, f32::min);
            assert!(
                across > 5.0,
                "the shower stays spread out instead of restarting at every tick"
            );
        }
        last = pellets;
    }
}

#[test]
fn fast_flares_keep_their_smoke_close_to_the_burning_pellets() {
    let mut style = style();
    let flare = std::sync::Arc::make_mut(style.flare.as_mut().unwrap());
    std::sync::Arc::make_mut(&mut flare.tracer).smoke = Some(
        serde_json::from_value(json!({
            "albedo": [1, 1, 1], "opacity": 0.5, "size_m": [0.2, 0.5],
            "life_s": 0.5, "rise_mps": 0, "spread_mps": 0,
            "ribbon": 0.8, "spacing_m": 0.2, "max_length_m": 3
        }))
        .unwrap(),
    );
    let mut effects = EffectFrame::new(30.0, style);
    let publication: EffectPublication = serde_json::from_value(json!({
        "tick": 1, "segments": [], "blasts": [], "smokes": [],
        "flares": [{"from": [0, 0, 60], "to": [30, 0, 59.9]}]
    }))
    .unwrap();
    effects.note(&publication);
    let mut batch = EffectBatch::new(2048);
    effects.build(1.0 / 30.0, &mut batch, None, &[], &[], None);
    let mut smoke = 0;
    for row in batch.data[..batch.count * 16].as_chunks::<16>().0 {
        if row[12] == 0.0 && row[11] > 0.0 {
            smoke += 1;
            assert!(row[0] > 25.0 && row[4] > 25.0,
                "fast ejection leaves a short wisp by the pellet, not a long line back to the aircraft: {row:?}");
        }
    }
    assert!(smoke > 0, "the speed cap preserves visible smoke");
}

#[test]
fn the_missiles_decoy_stays_inside_the_flare_shower() {
    let mut effects = effects();
    let mut batch = EffectBatch::new(1024);
    let point = |tick: f64| {
        let t = tick / 30.0;
        [80.0 * t, 0.0, 100.0 - 3.0 * t]
    };
    for tick in 1..=60 {
        let publication: EffectPublication = serde_json::from_value(json!({
            "tick": tick, "segments": [], "blasts": [], "smokes": [],
            "flares": [{"from": point(f64::from(tick - 1)), "to": point(f64::from(tick))}]
        }))
        .unwrap();
        effects.note(&publication);
        effects.build(f64::from(tick) / 30.0, &mut batch, None, &[], &[], None);
    }
    let target = point(60.0);
    let mut heads = 0;
    for row in batch.data[..batch.count * 16].as_chunks::<16>().0 {
        if row[12] != 1.0 {
            continue;
        }
        heads += 1;
        let distance = (0..3)
            .map(|k| (f64::from(row[k]) - target[k]).powi(2))
            .sum::<f64>()
            .sqrt();
        assert!(
            distance < 25.0,
            "the missile's decoy and the visible shower stay together through flight: {distance}"
        );
    }
    assert!(heads > 8);
}
