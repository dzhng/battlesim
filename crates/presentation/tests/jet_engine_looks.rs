//! How jets' engines trail, by the layers the data gives: each layer of the
//! engine trail drawn and nothing else changed, thicker on the afterburner
//! by its reheat, smooth from the nozzle, and the continuous plumes and
//! billows, each fixed where it was laid and refusing malformed fields.
mod jets;
use jets::*;

use presentation::effects::tables::EffectPresentation;
use serde_json::{json, Value};

/// The test style trailing its engines in the test look `name` (`looks`).
fn styled(name: &str) -> Value {
    let mut s = style();
    s["engine_trail"] = looks()[name].clone();
    s
}

#[test]
fn every_layer_of_the_engine_trail_draws_and_nothing_else_changes() {
    let flight = circling;
    let plain = fly(&mut frame_with(styled("plain")), 60.0, 1.5, flight, flight);
    let layered = fly(
        &mut frame_with(styled("layered")),
        60.0,
        1.5,
        flight,
        flight,
    );
    assert!(!ribbons(&plain, ENGINE_ALBEDO).is_empty());
    assert!(ribbons(&plain, HAZE_ALBEDO).is_empty() && ribbons(&plain, SMOKE_ALBEDO).is_empty());
    assert!(
        ribbons(&layered, ENGINE_ALBEDO).is_empty(),
        "the other trail's layers are gone"
    );
    let (haze, smoke) = (
        ribbons(&layered, HAZE_ALBEDO),
        ribbons(&layered, SMOKE_ALBEDO),
    );
    assert!(
        !haze.is_empty() && !smoke.is_empty(),
        "every layer of the trail draws"
    );
    assert!(
        smoke.len() > haze.len(),
        "each layer keeps its own length: a short haze, a long trail"
    );
    // Nothing else changes: the glows, flames and wingtip vapour.
    assert_eq!(glows(&plain), glows(&layered));
    assert_eq!(flames(&plain), flames(&layered));
    assert_eq!(ribbons(&plain, TIP_ALBEDO), ribbons(&layered, TIP_ALBEDO));
    let mut empty = style();
    empty["engine_trail"] = json!([]);
    let at = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/effect-vectors.json"
    );
    let v: Value = serde_json::from_str(&std::fs::read_to_string(at).unwrap()).unwrap();
    let mut p = v["presentation"].clone();
    p["jet_exhaust"] = empty;
    assert!(
        EffectPresentation::from_value(&p).is_err(),
        "an engine trail with no layer is refused"
    );
}

#[test]
fn a_layer_thickens_on_the_afterburner_by_its_reheat() {
    // Past cruise the throttle is full either way; only the afterburner adds.
    let thickest = |speed: f64, albedo: f32| {
        let flight = straight(2.0, "burner", speed);
        let d = fly(
            &mut frame_with(styled("layered")),
            60.0,
            2.0,
            &flight,
            &flight,
        );
        ribbons(&d, albedo)
            .iter()
            .map(|r| r[11])
            .fold(0.0, f32::max)
    };
    let (cruise, burning) = (
        thickest(CRUISE, SMOKE_ALBEDO),
        thickest(1.5 * CRUISE, SMOKE_ALBEDO),
    );
    assert!(
        burning > 1.5 * cruise,
        "smoke thickens on the burner: {cruise} {burning}"
    );
    let (haze_cruise, haze_burning) = (
        thickest(CRUISE, HAZE_ALBEDO),
        thickest(1.5 * CRUISE, HAZE_ALBEDO),
    );
    assert!(
        (haze_burning - haze_cruise).abs() < 1e-4,
        "a layer without reheat does not: {haze_cruise} {haze_burning}"
    );
}

#[test]
fn a_trail_thickens_from_the_nozzle_without_a_step() {
    // A trail fading in over three ticks, drawn half a tick past one.
    let flight = straight(2.0, "burner", CRUISE);
    let d = fly(&mut frame(), 60.0, 1.0 + 0.5 / TICK_HZ, &flight, &flight);
    let one: Vec<f32> = {
        let r = ribbons(&d, ENGINE_ALBEDO);
        r[..r.len() / 2].iter().map(|r| r[11]).collect()
    };
    let full = one.iter().copied().fold(0.0, f32::max);
    // Newest last: from the nozzle back to where it is whole.
    let young: Vec<f32> = one
        .iter()
        .rev()
        .take_while(|&&o| o < 0.98 * full)
        .copied()
        .collect();
    assert!(
        young.len() > 4,
        "it thickens over several stretches: {young:?}"
    );
    for w in young.windows(2) {
        assert!(
            (w[1] - w[0]).abs() < 0.35 * full,
            "no step a tick long as it thickens: {young:?}"
        );
    }
}

/// Engine billows: sheet sprites (shape 2) flagged (misc.w 1) to draw noise,
/// carrying their optical depth (misc.y).
fn billows(d: &[Vec<f32>]) -> Vec<&Vec<f32>> {
    d.iter().filter(|i| i[12] == 2.0 && i[15] == 1.0).collect()
}

#[test]
fn a_puff_layer_draws_engine_billows_fixed_where_laid_and_nothing_else_does() {
    let flight = circling;
    let mut f = frame_with(styled("billowed"));
    let a = fly(&mut f, 60.0, 1.2, flight, flight);
    let b = fly(&mut f, 60.0, 1.2 + 1.0 / 60.0, flight, flight);
    let (ba, bb) = (billows(&a), billows(&b));
    assert!(ba.len() > 30, "a trail of billows: {}", ba.len());
    for r in &ba {
        assert_eq!(r[13], 6.0, "each carries its optical depth");
        assert_eq!(r[8], SMOKE_ALBEDO);
    }
    // A billow already laid stays where it was laid a frame on, but for
    // spreading as it grows: no jump, no slide along the trail.
    let near = |r: &&&Vec<f32>| {
        bb.iter().any(|s| {
            let d: f32 = (0..3).map(|k| (s[k] - r[k]).powi(2)).sum();
            d.sqrt() < 0.1
        })
    };
    let kept = ba.iter().filter(near).count();
    assert!(
        kept > ba.len() / 2,
        "billows hang where they were laid: {kept}"
    );
    // No other look and no wingtip draws them.
    let plain = fly(&mut frame_with(styled("plain")), 60.0, 1.2, flight, flight);
    assert!(billows(&plain).is_empty());
    assert_eq!(
        ribbons(&a, TIP_ALBEDO),
        ribbons(&plain, TIP_ALBEDO),
        "wingtips unchanged"
    );
    // A billow layer without optical depth is refused.
    let mut bad = styled("billowed");
    bad["engine_trail"][0]["puffs"]["density"] = json!(0);
    let at = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/effect-vectors.json"
    );
    let v: Value = serde_json::from_str(&std::fs::read_to_string(at).unwrap()).unwrap();
    let mut p = v["presentation"].clone();
    p["jet_exhaust"] = bad;
    assert!(EffectPresentation::from_value(&p).is_err());
}

#[test]
fn a_plume_layer_draws_one_continuous_dense_ribbon_and_is_never_also_a_wisp() {
    let flight = circling;
    let d = fly(&mut frame_with(styled("plumed")), 60.0, 1.2, flight, flight);
    // Plume ribbons: shape 0, an opacity, misc.w 2 + its density.
    let plumes: Vec<&Vec<f32>> = d
        .iter()
        .filter(|i| i[12] == 0.0 && i[11] > 0.0 && i[15] > 1.5)
        .collect();
    assert!(
        plumes.len() > 20,
        "a trail of plume stretches: {}",
        plumes.len()
    );
    for r in &plumes {
        assert_eq!(r[15], 7.0, "each carries 2 + its optical depth");
    }
    // One nozzle's stretches meet end to end with the same noise place.
    let mut joined = 0;
    for w in plumes.windows(2) {
        if w[0][4..7] == w[1][..3] {
            assert!((w[0][14] - w[1][13]).abs() < 1e-4, "no seam in its noise");
            assert!(
                (w[0][7] - w[1][3]).abs() < 1e-4,
                "nor in its width: {} then {}",
                w[0][7],
                w[1][3]
            );
            joined += 1;
        }
    }
    assert!(joined > 10, "it is continuous: {joined}");
    // A wingtip wisp is never mistaken for a plume, nor the plain look.
    for r in ribbons(&d, TIP_ALBEDO) {
        assert!(r[15] <= 1.0);
    }
    let plain = fly(&mut frame_with(styled("plain")), 60.0, 1.2, flight, flight);
    assert!(plain.iter().all(|i| i[15] <= 1.5 || i[12] != 0.0));
    // A plume with no depth, or also puffed or broken up, is refused.
    let at = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/effect-vectors.json"
    );
    let v: Value = serde_json::from_str(&std::fs::read_to_string(at).unwrap()).unwrap();
    for bad in [
        json!({"density": 0, "length_m": 10}),
        json!({"density": 5, "length_m": 0}),
    ] {
        let mut s = styled("plumed");
        s["engine_trail"][0]["plume"] = bad;
        let mut p = v["presentation"].clone();
        p["jet_exhaust"] = s;
        assert!(EffectPresentation::from_value(&p).is_err());
    }
    let mut s = styled("plumed");
    s["engine_trail"][0]["breakup"] = json!({"amount": 0.5, "length_m": 4});
    let mut p = v["presentation"].clone();
    p["jet_exhaust"] = s;
    assert!(
        EffectPresentation::from_value(&p).is_err(),
        "a plume is not also a wisp"
    );
}
