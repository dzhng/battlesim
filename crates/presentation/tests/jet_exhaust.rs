//! What a player sees jets' engines and wings leave behind: a glow in every
//! running nozzle, a flame only from an engine that has an afterburner and
//! only once the jet flies past its cruise, wingtip vapour only while the
//! wings pull hard (a tight turn, a pull-up; never level flight, however
//! fast), forming without flicker and thinning over its release, and trails
//! laid where the publications put the jet: the same
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
const ON_LOAD: f64 = 1.8;
const OFF_LOAD: f64 = 1.5;
const RELEASE_S: f64 = 1.5;
const G: f64 = 9.81;

fn style() -> Value {
    json!({
        "max_aircraft": MAX_AIRCRAFT, "break_mps": 1500, "max_gap_s": MAX_GAP_S,
        "speed_window_s": 0.5, "reheat_share": [1.1, 1.4],
        "vapour": {"load": [ON_LOAD, 3.5], "off_load": OFF_LOAD, "onset": 0.4, "attack_s": 0.2, "release_s": RELEASE_S},
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
        "wingtip_trail": {"albedo": [TIP_ALBEDO, TIP_ALBEDO, TIP_ALBEDO], "opacity": [0.0, 0.4], "life_s": LIFE, "width_m": [0.2, 2], "fade_in_s": 0.05},
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

fn craft(key: f64, engine: &'static str, at: [f64; 3], yaw: f64) -> Craft {
    Craft {
        key,
        at,
        yaw,
        engine,
    }
}

/// A jet flying straight along +X at `speed`.
fn straight(key: f64, engine: &'static str, speed: f64) -> impl Fn(f64) -> Vec<Craft> {
    move |t| vec![craft(key, engine, [speed * t, 0.0, 900.0], 0.0)]
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

/// A jet circling at cruise, its wings pulling past `ON_LOAD`.
fn circling(t: f64) -> Vec<Craft> {
    let r = 1500.0;
    let a = CRUISE * t / r;
    vec![craft(
        2.0,
        "burner",
        [r * a.cos(), r * a.sin(), 900.0],
        a + std::f64::consts::FRAC_PI_2,
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
    let flight = circling;
    for tick in 0..=45 {
        f.note(&publ(tick, &flight(f64::from(tick) / TICK_HZ)));
    }
    let seek = draw(&mut f, 1.5, &flight(1.5));
    let flown = fly(&mut frame(), 60.0, 1.5, flight, flight);
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
    let flight = circling;
    let before = fly(&mut f, 60.0, 1.0, flight, flight);
    assert!(
        !ribbons(&before, TIP_ALBEDO).is_empty(),
        "turning, it vapours"
    );
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
        vec![craft(2.0, "burner", [x, 0.0, 900.0], 0.0)]
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

/// A jet flown along `path(t)` (its position), its heading along its way.
fn along(path: impl Fn(f64) -> [f64; 3]) -> impl Fn(f64) -> Vec<Craft> {
    move |t| {
        let (p, q) = (path(t), path(t + 1e-3));
        let yaw = (q[1] - p[1]).atan2(q[0] - p[0]);
        vec![craft(2.0, "burner", p, yaw)]
    }
}

/// A level turn at `speed` whose rate is `rate(t)` rad/s, from heading +X,
/// integrated finely so its published path bends as it flies.
fn turning(speed: f64, rate: impl Fn(f64) -> f64) -> impl Fn(f64) -> [f64; 3] {
    move |t| {
        let steps = (t * 2000.0).round() as u32;
        let dt = if steps == 0 {
            0.0
        } else {
            t / f64::from(steps)
        };
        let (mut p, mut h) = ([0.0, 0.0, 900.0], 0.0);
        for i in 0..steps {
            let mid = (f64::from(i) + 0.5) * dt;
            let k = h + rate(mid) * dt / 2.0;
            p[0] += speed * k.cos() * dt;
            p[1] += speed * k.sin() * dt;
            h += rate(mid) * dt;
        }
        p
    }
}

/// The rate a level turn at `speed` turns at to load its wings `load`.
fn rate_for(speed: f64, load: f64) -> f64 {
    G * (load * load - 1.0).sqrt() / speed
}

/// The wingtip ribbons' opacities, oldest first.
fn vapour(d: &[Vec<f32>]) -> Vec<f32> {
    ribbons(d, TIP_ALBEDO).iter().map(|r| r[11]).collect()
}

fn thickest(d: &[Vec<f32>]) -> f32 {
    vapour(d).into_iter().fold(0.0, f32::max)
}

#[test]
fn level_flight_leaves_no_wingtip_vapour_however_fast_or_hard_it_accelerates() {
    for speed in [0.5 * CRUISE, CRUISE, 1.5 * CRUISE] {
        let flight = straight(2.0, "burner", speed);
        let d = fly(&mut frame(), 60.0, 2.0, &flight, &flight);
        assert!(vapour(&d).is_empty(), "level at {speed} m/s");
        assert!(
            !ribbons(&d, ENGINE_ALBEDO).is_empty(),
            "its engines still trail"
        );
    }
    // Straight on, its afterburner shoving it from cruise to half again.
    let shove = along(|t: f64| [CRUISE * t + 0.25 * CRUISE * t * t, 0.0, 900.0]);
    let d = fly(&mut frame(), 60.0, 2.0, &shove, &shove);
    assert!(flames(&d) > 0, "it burns");
    assert!(
        vapour(&d).is_empty(),
        "but its wings carry no more than its weight"
    );
}

#[test]
fn a_hard_turn_blooms_wingtip_vapour_and_a_gentle_one_does_not() {
    let at = |load: f64| {
        let flight = along(turning(CRUISE, move |_| rate_for(CRUISE, load)));
        thickest(&fly(&mut frame(), 60.0, 2.0, &flight, &flight))
    };
    let (gentle, hard, harder) = (at(1.3), at(2.5), at(3.5));
    assert_eq!(gentle, 0.0, "a gentle turn leaves none");
    assert!(
        hard > 0.0 && harder > hard,
        "thicker the harder: {hard} {harder}"
    );
}

#[test]
fn the_same_turn_flown_faster_pulls_more_vapour() {
    // One rate of turn: the faster jet's wings carry more.
    let rate = rate_for(CRUISE, 2.2);
    let at = |speed: f64| {
        let flight = along(turning(speed, move |_| rate));
        thickest(&fly(&mut frame(), 60.0, 2.0, &flight, &flight))
    };
    let (slow, fast) = (at(0.75 * CRUISE), at(1.3 * CRUISE));
    assert!(fast > slow, "slow {slow}, fast {fast}");
}

#[test]
fn a_pull_up_out_of_level_flight_blooms_vapour_too() {
    // Level, then a 1500 m vertical arc upward at cruise: about 3.7 g.
    let r = 1500.0;
    let pull = along(move |t: f64| {
        let a = (CRUISE * (t - 0.6) / r).max(0.0);
        let level = CRUISE * t.min(0.6);
        [level + r * a.sin(), 0.0, 900.0 + r * (1.0 - a.cos())]
    });
    let level = fly(&mut frame(), 60.0, 0.55, &pull, &pull);
    assert!(vapour(&level).is_empty(), "none before the pull");
    let pulled = fly(&mut frame(), 60.0, 1.6, &pull, &pull);
    assert!(thickest(&pulled) > 0.0, "it forms in the pull");
}

#[test]
fn vapour_does_not_flicker_while_the_load_hovers_round_where_it_forms() {
    // Into a hard turn, then a load wavering slowly between 1.6 and 2.0:
    // above where it stops, and back and forth across where it forms;
    // beside the same turn held at 1.8.
    let laid = |waver: f64| {
        let rate = move |t: f64| {
            let load = if t < 1.0 {
                2.6
            } else {
                1.8 + waver * (t * 3.0).cos()
            };
            rate_for(CRUISE, load)
        };
        let flight = along(turning(CRUISE, rate));
        vapour(&fly(&mut frame(), 60.0, 3.8, &flight, &flight))
    };
    let (wavering, held) = (laid(0.2), laid(0.0));
    assert_eq!(wavering.len(), held.len());
    // Laid while it wavered: each stretch against the same age's held one.
    let n = wavering.len() / 2;
    for k in n - 40..n - 4 {
        let share = wavering[k] / held[k];
        assert!(
            (0.75..1.5).contains(&share),
            "steady, not pulsing with the load: {share} at {k}"
        );
    }
}

#[test]
fn vapour_thins_out_over_its_release_after_the_pull_ends_whatever_the_frame_rate() {
    // A hard turn for a second and a half, then level again.
    let rate = |t: f64| if t < 1.5 { rate_for(CRUISE, 3.0) } else { 0.0 };
    let flight = along(turning(CRUISE, rate));
    // The thickest vapour within 30 m of the jet: what it lays now.
    let head = |fps: f64, until: f64| {
        let d = fly(&mut frame(), fps, until, &flight, &flight);
        let at = flight(until)[0].at;
        ribbons(&d, TIP_ALBEDO)
            .iter()
            .filter(|r| {
                let d: f64 = (0..3).map(|k| (f64::from(r[4 + k]) - at[k]).powi(2)).sum();
                d.sqrt() < 30.0
            })
            .map(|r| r[11])
            .fold(0.0, f32::max)
    };
    let pulling = head(60.0, 1.5);
    let soon = head(60.0, 1.5 + RELEASE_S / 3.0);
    // Its load is read over half a second of flight, so it lets go that late.
    let later = head(60.0, 1.5 + 0.5 + RELEASE_S);
    assert!(pulling > 0.0);
    assert!(
        soon > 0.0 && soon < pulling,
        "it thins, not stops: {pulling} {soon}"
    );
    assert!(
        later < 0.1 * pulling,
        "and is all but gone after its release: {pulling} {later}"
    );
    let slow = fly(&mut frame(), 24.0, 2.5, &flight, &flight);
    let fast = fly(&mut frame(), 144.0, 2.5, &flight, &flight);
    let (a, b) = (vapour(&slow), vapour(&fast));
    assert_eq!(a.len(), b.len(), "as many stretches at 24 fps as at 144");
    for (x, y) in a.iter().zip(&b) {
        assert!((x - y).abs() < 1e-4, "the same release: {x} vs {y}");
    }
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
