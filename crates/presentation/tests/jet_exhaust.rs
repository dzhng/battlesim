//! What a player sees jets' engines and wings leave behind: a glow in every
//! running nozzle, a flame only from an engine that has an afterburner and
//! only once the jet flies past its cruise, wingtip vapour that thickens in a
//! turn, and trails that are the same whatever the frame rate, hold on a
//! paused clock, clear on a rewind, break where a jet jumps, fade out after
//! a jet is gone, and never hold more than their life's worth of samples.
//! Driven by synthetic drawn jets over a test-owned exhaust style.
use presentation::effects::exhaust::{DrawnJet, Emitter};
use presentation::effects::frame::{EffectBatch, EffectFrame, EFFECT_FLOATS};
use presentation::effects::tables::EffectPresentation;
use serde_json::{json, Value};

const ENGINE_ALBEDO: f32 = 0.5;
const TIP_ALBEDO: f32 = 0.9;
const CRUISE: f64 = 200.0;
const LIFE: f64 = 2.0;
const HZ: f64 = 20.0;

fn style() -> Value {
    json!({
        "sample_hz": HZ, "max_jets": 16, "break_mps": 1500, "max_gap_s": 0.5,
        "speed_smoothing_s": 0.2, "reheat_share": [1.1, 1.4], "vapour_load": [1.05, 1.6],
        "engines": {
            "burner": {
                "glow": {"color": [1, 0.4, 0.1], "intensity": [1, 4], "reheat_intensity": 6, "radius": 2, "min_px": 1},
                "flame": {"color": [1, 0.4, 0.1], "core_color": [1, 0.8, 0.5], "intensity": 8, "core_intensity": 20,
                          "length": 8, "width": 1.6, "diamonds": 3, "diamond_intensity": 10, "flicker": 0.1},
                "trail": 1
            },
            "fan": {"glow": {"color": [1, 0.3, 0.1], "intensity": [0.2, 1], "reheat_intensity": 0, "radius": 1.5, "min_px": 0}, "trail": 1}
        },
        "engine_trail": {"albedo": [ENGINE_ALBEDO, ENGINE_ALBEDO, ENGINE_ALBEDO], "opacity": [0.05, 0.2], "life_s": LIFE, "width_m": [0.5, 4], "fade_in_s": 0.1},
        "wingtip_trail": {"albedo": [TIP_ALBEDO, TIP_ALBEDO, TIP_ALBEDO], "opacity": [0.05, 0.4], "life_s": LIFE, "width_m": [0.2, 2], "fade_in_s": 0.05},
        "airframes": {}
    })
}

fn frame() -> EffectFrame {
    let at = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/effect-vectors.json"
    );
    let v: Value = serde_json::from_str(&std::fs::read_to_string(at).unwrap()).unwrap();
    let mut p = v["presentation"].clone();
    p["jet_exhaust"] = style();
    EffectFrame::new(30.0, EffectPresentation::from_value(&p).unwrap())
}

/// A jet `key` at `at` heading along +X, flying level, its engine `engine`.
fn jet(key: u64, engine: &str, at: [f64; 3], load: f64) -> DrawnJet {
    let off = |dx: f64, dy: f64| [at[0] + dx, at[1] + dy, at[2]];
    DrawnJet {
        key,
        engine: engine.into(),
        at,
        aft: [-1.0, 0.0, 0.0],
        load,
        cruise_mps: CRUISE,
        emitters: vec![
            (Emitter::Nozzle { radius: 0.4 }, off(-8.0, 0.5)),
            (Emitter::Nozzle { radius: 0.4 }, off(-8.0, -0.5)),
            (Emitter::Wingtip, off(-3.0, 6.0)),
            (Emitter::Wingtip, off(-3.0, -6.0)),
        ],
    }
}

/// Every instance drawn at `clock` after the jets of that frame.
fn drawn(f: &mut EffectFrame, clock: f64, jets: &[DrawnJet]) -> Vec<Vec<f32>> {
    f.observe_jets(clock, jets);
    let mut batch = EffectBatch::new(f.presentation().capacity);
    f.build(clock, &mut batch, None, &[], &[], None);
    assert_eq!(batch.dropped, 0);
    batch.data[..batch.count * EFFECT_FLOATS]
        .chunks(EFFECT_FLOATS)
        .map(<[f32]>::to_vec)
        .collect()
}

/// Lit ribbons (blended: an opacity in the colour's alpha) of `albedo`.
fn ribbons(d: &[Vec<f32>], albedo: f32) -> Vec<&Vec<f32>> {
    d.iter()
        .filter(|i| i[12] == 0.0 && i[11] > 0.0 && i[8] == albedo)
        .collect()
}

/// Additive streaks (a flame's plume and core).
fn flames(d: &[Vec<f32>]) -> usize {
    d.iter().filter(|i| i[12] == 0.0 && i[11] == 0.0).count()
}

/// Additive glows, and the brightest one's red.
fn glows(d: &[Vec<f32>]) -> (usize, f32) {
    let g: Vec<_> = d.iter().filter(|i| i[12] == 1.0 && i[11] == 0.0).collect();
    (g.len(), g.iter().map(|i| i[8]).fold(0.0, f32::max))
}

fn span(r: &[f32]) -> f64 {
    let d: Vec<f64> = (0..3).map(|k| f64::from(r[4 + k] - r[k])).collect();
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
}

/// Where a jet circling at cruise is at `t`.
fn circling(t: f64) -> [f64; 3] {
    let r = 1500.0;
    let a = CRUISE * t / r;
    [r * libm::cos(a), r * libm::sin(a), 900.0]
}

/// Fly `jets(t)` at `fps` from 0 to `until` and return the last frame.
fn fly(
    f: &mut EffectFrame,
    fps: f64,
    until: f64,
    jets: impl Fn(f64) -> Vec<DrawnJet>,
) -> Vec<Vec<f32>> {
    let n = (until * fps).round() as u32;
    let mut out = Vec::new();
    for i in 0..=n {
        let t = f64::from(i) / fps;
        out = drawn(f, t, &jets(t));
    }
    out
}

#[test]
fn a_trail_is_the_same_whatever_the_frame_rate() {
    let jets = |t: f64| vec![jet(2, "burner", circling(t), 1.3)];
    let slow = fly(&mut frame(), 24.0, 1.5, jets);
    let fast = fly(&mut frame(), 144.0, 1.5, jets);
    for albedo in [ENGINE_ALBEDO, TIP_ALBEDO] {
        let (a, b) = (ribbons(&slow, albedo), ribbons(&fast, albedo));
        assert!(a.len() > 20, "a trail of samples behind the jet");
        assert_eq!(a.len(), b.len(), "as many stretches at 24 fps as at 144");
        for (x, y) in a.iter().zip(&b) {
            for k in 0..8 {
                assert!(
                    (x[k] - y[k]).abs() < 0.05,
                    "same place and width: {x:?} vs {y:?}"
                );
            }
            assert!((x[11] - y[11]).abs() < 1e-3, "same opacity");
        }
    }
}

#[test]
fn a_paused_clock_holds_the_trails_and_a_rewind_clears_them() {
    let mut f = frame();
    let flying = |t: f64| vec![jet(2, "burner", [CRUISE * t, 0.0, 900.0], 1.0)];
    let before = fly(&mut f, 60.0, 1.0, flying);
    let held = drawn(&mut f, 1.0, &flying(1.0));
    assert_eq!(before, held, "a held clock draws the same frame");
    let rewound = drawn(&mut f, 0.2, &flying(0.2));
    assert!(
        ribbons(&rewound, ENGINE_ALBEDO).is_empty(),
        "a rewind starts every trail over"
    );
    assert!(ribbons(&rewound, TIP_ALBEDO).is_empty());
    assert_eq!(glows(&rewound).0, 2, "its nozzles still glow");
}

#[test]
fn a_jet_that_jumps_breaks_its_trail_instead_of_streaking_across() {
    let mut f = frame();
    let at = |t: f64| {
        let x = CRUISE * t + if t >= 1.0 { 5000.0 } else { 0.0 };
        vec![jet(2, "burner", [x, 0.0, 900.0], 1.0)]
    };
    let d = fly(&mut f, 60.0, 1.5, at);
    let longest = ribbons(&d, ENGINE_ALBEDO)
        .iter()
        .chain(&ribbons(&d, TIP_ALBEDO))
        .map(|r| span(r))
        .fold(0.0, f64::max);
    assert!(
        longest < 2.0 * CRUISE / HZ,
        "no stretch spans the jump: {longest} m"
    );
}

#[test]
fn a_jet_gone_or_downed_stops_glowing_at_once_and_its_trail_fades_out() {
    let mut f = frame();
    let flying = |t: f64| vec![jet(2, "burner", [CRUISE * t, 0.0, 900.0], 1.0)];
    fly(&mut f, 60.0, 1.0, flying);
    let gone = drawn(&mut f, 1.0 + 1.0 / 60.0, &[]);
    assert_eq!(glows(&gone).0, 0, "no glow without a running jet");
    assert_eq!(flames(&gone), 0);
    assert!(
        !ribbons(&gone, ENGINE_ALBEDO).is_empty(),
        "its trail stays a while"
    );
    let later = drawn(&mut f, 1.0 + LIFE + 0.01, &[]);
    assert!(later.is_empty(), "then fades out entirely");
    assert_eq!(f.jet_samples(), 0, "and holds nothing");
}

#[test]
fn trails_hold_no_more_than_their_life_whatever_the_flight_or_the_crowd() {
    let mut f = frame();
    let per_jet = 4.0 * (LIFE * HZ + 2.0);
    let crowd = |t: f64| -> Vec<DrawnJet> {
        (0..20u64)
            .map(|k| {
                let mut p = circling(t + k as f64 * 3.0);
                p[2] += k as f64 * 50.0;
                jet(2 * k, "burner", p, 1.0 + (k % 3) as f64 * 0.4)
            })
            .collect()
    };
    let mut most = 0;
    for i in 0..=(30 * 60) {
        drawn(&mut f, f64::from(i) / 60.0, &crowd(f64::from(i) / 60.0));
        most = most.max(f.jet_samples());
    }
    assert!(
        most as f64 <= 16.0 * per_jet,
        "at most max_jets tracks of a life each: {most}"
    );
}

#[test]
fn only_an_afterburning_engine_burns_and_only_past_its_cruise() {
    let both = |speed: f64| {
        move |t: f64| {
            vec![
                jet(2, "burner", [speed * t, 0.0, 900.0], 1.0),
                jet(4, "fan", [speed * t, 300.0, 900.0], 1.0),
            ]
        }
    };
    let burner = |speed: f64| move |t: f64| vec![jet(2, "burner", [speed * t, 0.0, 900.0], 1.0)];
    let fan = |speed: f64| move |t: f64| vec![jet(4, "fan", [speed * t, 0.0, 900.0], 1.0)];
    assert_eq!(
        flames(&fly(&mut frame(), 60.0, 2.0, both(CRUISE))),
        0,
        "no flame at cruise"
    );
    assert!(
        flames(&fly(&mut frame(), 60.0, 2.0, burner(1.5 * CRUISE))) > 0,
        "a flame past it"
    );
    assert_eq!(
        flames(&fly(&mut frame(), 60.0, 2.0, fan(1.5 * CRUISE))),
        0,
        "a turbofan never burns"
    );
    let (_, slow) = glows(&fly(&mut frame(), 60.0, 2.0, burner(0.5 * CRUISE)));
    let (_, cruise) = glows(&fly(&mut frame(), 60.0, 2.0, burner(CRUISE)));
    let (_, fast) = glows(&fly(&mut frame(), 60.0, 2.0, burner(1.5 * CRUISE)));
    assert!(
        slow < cruise && cruise < fast,
        "brighter the faster: {slow} {cruise} {fast}"
    );
}

#[test]
fn wingtip_vapour_thickens_in_a_turn() {
    let opacity = |load: f64| {
        let d = fly(&mut frame(), 60.0, 1.0, move |t| {
            vec![jet(2, "burner", [CRUISE * t, 0.0, 900.0], load)]
        });
        ribbons(&d, TIP_ALBEDO)
            .iter()
            .map(|r| r[11])
            .fold(0.0, f32::max)
    };
    let (level, turning) = (opacity(1.0), opacity(2.0));
    assert!(
        level > 0.0 && turning > 2.0 * level,
        "level {level}, turning {turning}"
    );
}

#[test]
fn a_trail_widens_and_fades_toward_its_end() {
    let d = fly(&mut frame(), 60.0, 1.5, |t| {
        vec![jet(2, "burner", [CRUISE * t, 0.0, 900.0], 1.0)]
    });
    let r = ribbons(&d, ENGINE_ALBEDO);
    // Oldest first: wider and fainter at the far end than near the jet,
    // past the near end's fade-in.
    let (old, young) = (r[0], r[r.len() - 4]);
    assert!(old[3] > young[3], "wider far behind");
    assert!(old[11] < young[11], "fainter far behind");
}
