//! A downed airframe striking a building throws sparks off its skin and dust
//! off the face where it struck, fewer for a slower strike, and no fire and
//! no light: its fireball is its crash on the ground. A presentation without
//! the strike style throws nothing. Driven by synthetic publications over the
//! effect vectors' presentation, given the shipped strike style.
use presentation::effects::frame::{EffectBatch, EffectFrame, EFFECT_FLOATS, EFFECT_KINDS};
use presentation::effects::publication::EffectPublication;
use presentation::effects::tables::EffectPresentation;
use serde_json::{json, Value};

const HZ: f64 = 30.0;
const DT: f64 = 1.0 / HZ;
/// Where the strike lands: on a wall's west face.
const POINT: [f64; 3] = [110.0, 200.0, 20.0];

fn read(path: &str) -> Value {
    let at = format!("{}/{path}", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(at).unwrap()).unwrap()
}

/// The vectors' presentation, with the shipped strike style or without one.
fn frame(styled: bool) -> EffectFrame {
    let mut p = read("tests/fixtures/effect-vectors.json")["presentation"].clone();
    if styled {
        let shipped = read("../../fixtures/presentation.json");
        p["airframe_strike"] = shipped["effects"]["airframe_strike"].clone();
        assert!(
            p["airframe_strike"].is_object(),
            "the game ships a strike style"
        );
    }
    EffectFrame::new(HZ, EffectPresentation::from_value(&p).unwrap())
}

/// Tick 1's publication: one strike on the west face at `POINT`, closing at
/// `speed` m/s.
fn strike(speed: f64) -> EffectPublication {
    serde_json::from_value(json!({"tick": 1.0, "segments": [], "blasts": [], "shooters": [],
        "smokes": [], "strikes": [{"point": POINT, "normal": [-1.0, 0.0, 0.0], "speed_mps": speed}]}))
    .unwrap()
}

/// What `f` draws at `clock`: each instance's floats, the effects running by
/// kind, and the lights.
fn drawn(f: &mut EffectFrame, clock: f64) -> (Vec<Vec<f32>>, [usize; 8], usize) {
    let mut batch = EffectBatch::new(f.presentation().capacity);
    f.build(clock, &mut batch, None, &[], &[], None);
    let counts = f.counts(clock, &batch);
    let instances = batch.data[..batch.count * EFFECT_FLOATS]
        .chunks(EFFECT_FLOATS)
        .map(<[f32]>::to_vec)
        .collect();
    (instances, counts.running, counts.lights)
}

fn kind(name: &str) -> usize {
    EFFECT_KINDS.iter().position(|k| *k == name).unwrap()
}

#[test]
fn a_strike_throws_sparks_and_dust_where_it_struck_and_no_fire() {
    let mut f = frame(true);
    f.note(&strike(30.0));
    let (instances, running, lights) = drawn(&mut f, 1.5 * DT);
    assert!(running[kind("sparks")] > 0, "no sparks: {running:?}");
    assert!(running[kind("puff")] > 0, "no dust: {running:?}");
    for fire in ["blast", "flash", "flame"] {
        assert_eq!(running[kind(fire)], 0, "a strike throws no {fire}");
    }
    assert_eq!(lights, 0, "a strike lights nothing");
    for i in &instances {
        let at = [i[0], i[1], i[2]].map(f64::from);
        let off = (0..3)
            .map(|k| (at[k] - POINT[k]).powi(2))
            .sum::<f64>()
            .sqrt();
        assert!(off < 4.0, "an effect {off:.1} m from the strike: {i:?}");
        assert!(at[0] < POINT[0] + 0.5, "an effect inside the wall: {i:?}");
    }
}

#[test]
fn a_slower_strike_throws_fewer_sparks_and_less_dust() {
    // Late enough for all its dust to be born, early enough for its sparks to fly.
    let at = DT + 0.26;
    let count = |speed: f64| {
        let mut f = frame(true);
        f.note(&strike(speed));
        drawn(&mut f, at).0.len()
    };
    let (hard, soft) = (count(30.0), count(6.0));
    assert!(
        soft > 0 && soft < hard,
        "{soft} effects from a soft strike, {hard} from a hard one"
    );
}

#[test]
fn without_a_strike_style_a_strike_throws_nothing() {
    let mut f = frame(false);
    f.note(&strike(30.0));
    let (instances, _, _) = drawn(&mut f, 1.5 * DT);
    assert!(instances.is_empty(), "{instances:?}");
}
