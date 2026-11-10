//! What a player sees jets' engines and wings leave behind: a glow in every
//! running nozzle, a flame only from an engine that has an afterburner and
//! only once the jet flies past its cruise, wingtip vapour that thickens in a
//! turn, and trails laid where the publications put the jet: the same
//! whatever the frame rate, whole in a replay framed mid-flight, held on a
//! paused clock, cleared on a rewind, broken where a jet jumps, fading out
//! after a jet is gone or downed, and never holding more than their life.
//! Driven by synthetic publications and drawn jets over a test-owned style.
use presentation::effects::exhaust::{DrawnJet, Emitter};
use presentation::effects::frame::{EffectBatch, EffectFrame, EFFECT_FLOATS};
use presentation::effects::publication::EffectPublication;
use presentation::effects::tables::EffectPresentation;
use serde_json::{json, Value};

const ENGINE_ALBEDO: f32 = 0.5;
const TIP_ALBEDO: f32 = 0.9;
const CRUISE: f64 = 200.0;
const LIFE: f64 = 2.0;
const TICK_HZ: f64 = 30.0;
const MAX_AIRCRAFT: usize = 16;
const MAX_GAP_S: f64 = 0.5;

fn style() -> Value {
    json!({
        "max_aircraft": MAX_AIRCRAFT, "break_mps": 1500, "max_gap_s": MAX_GAP_S,
        "speed_window_s": 0.5, "reheat_share": [1.1, 1.4], "vapour_load": [1.05, 1.6],
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
    EffectFrame::new(TICK_HZ, EffectPresentation::from_value(&p).unwrap())
}

/// One aircraft at a moment: its key, where it is and its heading.
#[derive(Clone, Copy)]
struct Craft {
    key: f64,
    at: [f64; 3],
    yaw: f64,
    engine: &'static str,
    load: f64,
}

/// `c` drawn: two nozzles 8 m aft and two wingtips, turned with its heading.
fn drawn_jet(c: &Craft) -> DrawnJet {
    let (s, k) = (c.yaw.sin(), c.yaw.cos());
    let off = |x: f64, y: f64| [c.at[0] + x * k - y * s, c.at[1] + x * s + y * k, c.at[2]];
    DrawnJet {
        key: c.key.to_bits(),
        engine: c.engine.into(),
        at: c.at,
        yaw: c.yaw,
        aft: [-k, -s, 0.0],
        load: c.load,
        cruise_mps: CRUISE,
        emitters: vec![
            (Emitter::Nozzle { radius: 0.4 }, off(-8.0, 0.5)),
            (Emitter::Nozzle { radius: 0.4 }, off(-8.0, -0.5)),
            (Emitter::Wingtip, off(-3.0, 6.0)),
            (Emitter::Wingtip, off(-3.0, -6.0)),
        ],
    }
}

/// Tick `tick`'s publication of `crafts`, airborne.
fn publ(tick: u32, crafts: &[Craft]) -> EffectPublication {
    let shooters: Vec<Value> = crafts
        .iter()
        .map(|c| {
            json!({"key": c.key, "position": c.at, "half": [7, 3, 1.8], "airborne": true,
                        "yaw": c.yaw, "members": [], "mounts": []})
        })
        .collect();
    serde_json::from_value(
        json!({"tick": tick, "segments": [], "blasts": [], "shooters": shooters, "smokes": []}),
    )
    .unwrap()
}

/// What `f` draws at `clock` with `drawn` jets drawn.
fn draw(f: &mut EffectFrame, clock: f64, drawn: &[Craft]) -> Vec<Vec<f32>> {
    f.observe_jets(clock, &drawn.iter().map(drawn_jet).collect::<Vec<_>>());
    let mut batch = EffectBatch::new(f.presentation().capacity);
    f.build(clock, &mut batch, None, &[], &[], None);
    assert_eq!(batch.dropped, 0);
    batch.data[..batch.count * EFFECT_FLOATS]
        .chunks(EFFECT_FLOATS)
        .map(<[f32]>::to_vec)
        .collect()
}

/// A battle: `flight(t)` published every tick (the page taking each tick a
/// tick ahead of the clock it draws), and `drawn(t)` drawn at `fps` from 0
/// to `until`; the last frame.
fn fly(
    f: &mut EffectFrame,
    fps: f64,
    until: f64,
    flight: impl Fn(f64) -> Vec<Craft>,
    drawn: impl Fn(f64) -> Vec<Craft>,
) -> Vec<Vec<f32>> {
    let mut noted = 0u32;
    let mut out = Vec::new();
    for i in 0..=(until * fps).round() as u32 {
        let t = f64::from(i) / fps;
        while f64::from(noted) / TICK_HZ <= t + 1.0 / TICK_HZ + 1e-9 {
            f.note(&publ(noted, &flight(f64::from(noted) / TICK_HZ)));
            noted += 1;
        }
        out = draw(f, t, &drawn(t));
    }
    out
}

fn craft(key: f64, engine: &'static str, at: [f64; 3], yaw: f64, load: f64) -> Craft {
    Craft {
        key,
        at,
        yaw,
        engine,
        load,
    }
}

/// A jet flying straight along +X at `speed`.
fn straight(key: f64, engine: &'static str, speed: f64) -> impl Fn(f64) -> Vec<Craft> {
    move |t| vec![craft(key, engine, [speed * t, 0.0, 900.0], 0.0, 1.0)]
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

/// A jet circling at cruise, banked.
fn circling(t: f64) -> Vec<Craft> {
    let r = 1500.0;
    let a = CRUISE * t / r;
    vec![craft(
        2.0,
        "burner",
        [r * a.cos(), r * a.sin(), 900.0],
        a + std::f64::consts::FRAC_PI_2,
        1.3,
    )]
}

#[test]
fn a_trail_is_the_same_whatever_the_frame_rate() {
    let slow = fly(&mut frame(), 24.0, 1.5, circling, circling);
    let fast = fly(&mut frame(), 144.0, 1.5, circling, circling);
    for albedo in [ENGINE_ALBEDO, TIP_ALBEDO] {
        let (a, b) = (ribbons(&slow, albedo), ribbons(&fast, albedo));
        assert!(a.len() > 20, "a trail of samples behind the jet");
        assert_eq!(a.len(), b.len(), "as many stretches at 24 fps as at 144");
        for (x, y) in a.iter().zip(&b) {
            for k in 0..8 {
                assert!(
                    (x[k] - y[k]).abs() < 1e-3,
                    "same place and width: {x:?} vs {y:?}"
                );
            }
            assert!((x[11] - y[11]).abs() < 1e-4, "same opacity");
        }
    }
}

#[test]
fn a_replay_framed_mid_flight_shows_the_whole_trail() {
    let mut f = frame();
    let flight = straight(2.0, "burner", CRUISE);
    for tick in 0..=45 {
        f.note(&publ(tick, &flight(f64::from(tick) / TICK_HZ)));
    }
    let seek = draw(&mut f, 1.5, &flight(1.5));
    let flown = fly(&mut frame(), 60.0, 1.5, &flight, &flight);
    for albedo in [ENGINE_ALBEDO, TIP_ALBEDO] {
        assert!(
            ribbons(&seek, albedo).len() > 20,
            "the trail is there on the first frame"
        );
        assert_eq!(ribbons(&seek, albedo).len(), ribbons(&flown, albedo).len());
    }
}

#[test]
fn a_paused_clock_holds_the_trails_and_a_rewind_clears_them() {
    let mut f = frame();
    let flight = straight(2.0, "burner", CRUISE);
    let before = fly(&mut f, 60.0, 1.0, &flight, &flight);
    let held = draw(&mut f, 1.0, &flight(1.0));
    assert_eq!(before, held, "a held clock draws the same frame");
    f.note(&publ(6, &flight(0.2)));
    let rewound = draw(&mut f, 0.2, &flight(0.2));
    assert!(
        ribbons(&rewound, ENGINE_ALBEDO).is_empty(),
        "a rewind starts every trail over"
    );
    assert!(ribbons(&rewound, TIP_ALBEDO).is_empty());
    assert_eq!(glows(&rewound).0, 2, "its nozzles still glow");
}

#[test]
fn a_jet_that_jumps_breaks_its_trail_instead_of_streaking_across() {
    let jumping = |t: f64| {
        let x = CRUISE * t + if t >= 1.0 { 5000.0 } else { 0.0 };
        vec![craft(2.0, "burner", [x, 0.0, 900.0], 0.0, 1.0)]
    };
    let d = fly(&mut frame(), 60.0, 1.5, jumping, jumping);
    let longest = [ENGINE_ALBEDO, TIP_ALBEDO]
        .iter()
        .flat_map(|&a| ribbons(&d, a))
        .map(|r| span(r))
        .fold(0.0, f64::max);
    assert!(
        longest < 2.0 * CRUISE / TICK_HZ,
        "no stretch spans the jump: {longest} m"
    );
}

#[test]
fn a_jet_gone_stops_glowing_at_once_and_its_trail_fades_out() {
    let mut f = frame();
    let flight = straight(2.0, "burner", CRUISE);
    let until_one = |t: f64| if t <= 1.0 { flight(t) } else { vec![] };
    fly(&mut f, 60.0, 1.0, until_one, until_one);
    let gone = fly(&mut f, 60.0, 1.0 + 1.0 / 60.0, until_one, until_one);
    assert_eq!(glows(&gone).0, 0, "no glow without a running jet");
    assert_eq!(flames(&gone), 0);
    assert!(
        !ribbons(&gone, ENGINE_ALBEDO).is_empty(),
        "its trail stays a while"
    );
    f.note(&publ(31 + (LIFE * TICK_HZ) as u32 + 2, &[]));
    assert!(
        draw(&mut f, 1.0 + LIFE + 0.1, &[]).is_empty(),
        "then fades out entirely"
    );
    assert_eq!(f.jet_samples(), 0, "and holds nothing");
}

#[test]
fn a_downed_jet_still_falling_lays_no_more_trail() {
    // Published all the way down, drawn running only until it is hit.
    let mut f = frame();
    let flight = straight(2.0, "burner", CRUISE);
    let running = |t: f64| if t < 1.0 { flight(t) } else { vec![] };
    let late = fly(&mut f, 60.0, 1.0 + MAX_GAP_S + LIFE + 0.1, &flight, running);
    assert!(
        late.is_empty(),
        "nothing laid after its engines went out survives: {}",
        late.len()
    );
}

#[test]
fn trails_hold_no_more_than_their_life_whatever_the_flight_or_the_crowd() {
    let crowd = |t: f64| -> Vec<Craft> {
        (0..40u32)
            .map(|k| {
                let mut c = circling(t + f64::from(k) * 3.0)[0];
                c.key = f64::from(2 * k);
                c.at[2] += f64::from(k) * 50.0;
                c
            })
            .collect()
    };
    let mut f = frame();
    let per_tick_life = (LIFE * TICK_HZ + 2.0) as usize;
    let bound = MAX_AIRCRAFT * 5 * per_tick_life;
    let mut most = 0;
    let mut noted = 0;
    for i in 0..=(30 * 60) {
        let t = f64::from(i) / 60.0;
        while f64::from(noted) / TICK_HZ <= t {
            f.note(&publ(noted, &crowd(f64::from(noted) / TICK_HZ)));
            noted += 1;
        }
        draw(&mut f, t, &crowd(t));
        most = most.max(f.jet_samples());
    }
    assert!(
        most <= bound,
        "at most max_aircraft tracks of a life each: {most} > {bound}"
    );
}

#[test]
fn only_an_afterburning_engine_burns_and_only_past_its_cruise() {
    let flames_at = |engine: &'static str, speed: f64| {
        let flight = straight(2.0, engine, speed);
        flames(&fly(&mut frame(), 60.0, 2.0, &flight, &flight))
    };
    assert_eq!(flames_at("burner", CRUISE), 0, "no flame at cruise");
    assert!(flames_at("burner", 1.5 * CRUISE) > 0, "a flame past it");
    assert_eq!(flames_at("fan", 1.5 * CRUISE), 0, "a turbofan never burns");
    let glow_at = |speed: f64| {
        let flight = straight(2.0, "burner", speed);
        glows(&fly(&mut frame(), 60.0, 2.0, &flight, &flight)).1
    };
    let (slow, cruise, fast) = (
        glow_at(0.5 * CRUISE),
        glow_at(CRUISE),
        glow_at(1.5 * CRUISE),
    );
    assert!(
        slow < cruise && cruise < fast,
        "brighter the faster: {slow} {cruise} {fast}"
    );
}

#[test]
fn wingtip_vapour_thickens_in_a_turn() {
    let opacity = |load: f64| {
        let flight = move |t: f64| vec![craft(2.0, "burner", [CRUISE * t, 0.0, 900.0], 0.0, load)];
        let d = fly(&mut frame(), 60.0, 1.0, flight, flight);
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
    let flight = straight(2.0, "burner", CRUISE);
    let d = fly(&mut frame(), 60.0, 1.5, &flight, &flight);
    let r = ribbons(&d, ENGINE_ALBEDO);
    // One nozzle's trail, oldest first: wider and fainter far behind than
    // near the jet, past the near end's fade-in.
    let half = r.len() / 2;
    let (old, young) = (r[0], r[half - 4]);
    assert!(old[3] > young[3], "wider far behind");
    assert!(old[11] < young[11], "fainter far behind");
}
