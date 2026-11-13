//! Shared helpers for the jet exhaust tests: a test-owned style, synthetic
//! publications and drawn jets, and the flights they fly.
#![allow(dead_code)]
use presentation::effects::exhaust::{DrawnJet, Emitter};
use presentation::effects::frame::{EffectBatch, EffectFrame, EFFECT_FLOATS};
use presentation::effects::publication::EffectPublication;
use presentation::effects::tables::EffectPresentation;
use serde_json::{json, Value};

pub const ENGINE_ALBEDO: f32 = 0.5;
pub const TIP_ALBEDO: f32 = 0.9;
pub const SHEET_ALBEDO: f32 = 0.7;
pub const HAZE_ALBEDO: f32 = 0.4;
pub const SMOKE_ALBEDO: f32 = 0.1;
pub const CRUISE: f64 = 200.0;
pub const LIFE: f64 = 2.0;
pub const TICK_HZ: f64 = 30.0;
pub const MAX_AIRCRAFT: usize = 16;
pub const MAX_GAP_S: f64 = 0.5;
pub const ON_LOAD: f64 = 1.8;
pub const OFF_LOAD: f64 = 1.5;
pub const RELEASE_S: f64 = 1.5;
pub const G: f64 = 9.81;
/// The share of the wingtip vapour the wing-top sheet starts at.
pub const SHEET_FROM: f64 = 0.7;

pub fn style() -> Value {
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
        "engine_style": "plain",
        "engine_styles": {
            "plain": [{"albedo": [ENGINE_ALBEDO, ENGINE_ALBEDO, ENGINE_ALBEDO], "opacity": [0.05, 0.2], "life_s": LIFE, "width_m": [0.5, 4], "fade_in_s": 0.1}],
            "plumed": [{"albedo": [SMOKE_ALBEDO, SMOKE_ALBEDO, SMOKE_ALBEDO], "opacity": [0.0, 0.8], "life_s": LIFE, "width_m": [1, 6], "fade_in_s": 0.02, "plume": {"density": 5, "length_m": 10}}],
            "mixed": [{"albedo": [0.5, 0.5, 0.5], "opacity": [0.0, 0.8], "life_s": LIFE, "width_m": [1, 6], "fade_in_s": 0.02, "plume": {"density": 5, "length_m": 10, "mix": [0.02, 0.95]}}],
            "billowed": [{"albedo": [SMOKE_ALBEDO, SMOKE_ALBEDO, SMOKE_ALBEDO], "opacity": [0.0, 0.8], "life_s": LIFE, "width_m": [1, 6], "fade_in_s": 0.02, "puffs": {"spacing_m": 2, "density": 6}}],
            "layered": [
                {"albedo": [HAZE_ALBEDO, HAZE_ALBEDO, HAZE_ALBEDO], "opacity": [0.0, 0.3], "life_s": 0.5, "width_m": [0.5, 1.5], "fade_in_s": 0.02},
                {"albedo": [SMOKE_ALBEDO, SMOKE_ALBEDO, SMOKE_ALBEDO], "opacity": [0.0, 0.2], "reheat": 0.6, "life_s": LIFE, "width_m": [0.5, 3], "fade_in_s": 0.02}
            ]
        },
        "wingtip_trail": {"albedo": [TIP_ALBEDO, TIP_ALBEDO, TIP_ALBEDO], "opacity": [0.0, 0.4], "life_s": LIFE, "width_m": [0.2, 2], "fade_in_s": 0.05},
        "sheet_from": SHEET_FROM,
        "wing_sheet": [],
        "wing_sheet_rule": {"span": [0, 0.5], "rows": [[0.2, 0.2]], "spacing_m": 1, "lift_m": 0.2},
        "airframes": {}
    })
}

pub fn frame() -> EffectFrame {
    frame_with(style())
}

pub fn frame_with(style: Value) -> EffectFrame {
    let at = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/effect-vectors.json"
    );
    let v: Value = serde_json::from_str(&std::fs::read_to_string(at).unwrap()).unwrap();
    let mut p = v["presentation"].clone();
    p["jet_exhaust"] = style;
    EffectFrame::new(TICK_HZ, EffectPresentation::from_value(&p).unwrap())
}

/// The test style with short, thin, sharply tapered wingtip wisps.
pub fn wispy() -> Value {
    let mut s = style();
    s["wingtip_trail"] = json!({"albedo": [TIP_ALBEDO, TIP_ALBEDO, TIP_ALBEDO], "opacity": [0.0, 0.6],
        "life_s": 0.4, "width_m": [0.1, 0.8], "fade_in_s": 0.08, "taper": 3,
        "breakup": {"amount": 0.85, "length_m": 4}});
    s
}

/// One aircraft at a moment: its key, where it is and its heading.
#[derive(Clone, Copy)]
pub struct Craft {
    pub key: f64,
    pub at: [f64; 3],
    pub yaw: f64,
    pub engine: &'static str,
}

/// `c` drawn: two nozzles 8 m aft and two wingtips, turned with its heading.
pub fn drawn_jet(c: &Craft) -> DrawnJet {
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
pub fn publ(tick: u32, crafts: &[Craft]) -> EffectPublication {
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
pub fn draw(f: &mut EffectFrame, clock: f64, drawn: &[Craft]) -> Vec<Vec<f32>> {
    draw_jets(f, clock, &drawn.iter().map(drawn_jet).collect::<Vec<_>>())
}

/// What `f` draws at `clock` with `jets` drawn as they are given.
pub fn draw_jets(f: &mut EffectFrame, clock: f64, jets: &[DrawnJet]) -> Vec<Vec<f32>> {
    f.observe_jets(clock, jets);
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
pub fn fly(
    f: &mut EffectFrame,
    fps: f64,
    until: f64,
    flight: impl Fn(f64) -> Vec<Craft>,
    drawn: impl Fn(f64) -> Vec<Craft>,
) -> Vec<Vec<f32>> {
    fly_jets(f, fps, until, flight, |t| {
        drawn(t).iter().map(drawn_jet).collect()
    })
}

/// [`fly`], with the jets drawn at `t` given whole.
pub fn fly_jets(
    f: &mut EffectFrame,
    fps: f64,
    until: f64,
    flight: impl Fn(f64) -> Vec<Craft>,
    drawn: impl Fn(f64) -> Vec<DrawnJet>,
) -> Vec<Vec<f32>> {
    let mut noted = 0u32;
    let mut out = Vec::new();
    for i in 0..=(until * fps).round() as u32 {
        let t = f64::from(i) / fps;
        while f64::from(noted) / TICK_HZ <= t + 1.0 / TICK_HZ + 1e-9 {
            f.note(&publ(noted, &flight(f64::from(noted) / TICK_HZ)));
            noted += 1;
        }
        out = draw_jets(f, t, &drawn(t));
    }
    out
}

pub fn craft(key: f64, engine: &'static str, at: [f64; 3], yaw: f64) -> Craft {
    Craft {
        key,
        at,
        yaw,
        engine,
    }
}

/// A jet flying straight along +X at `speed`.
pub fn straight(key: f64, engine: &'static str, speed: f64) -> impl Fn(f64) -> Vec<Craft> {
    move |t| vec![craft(key, engine, [speed * t, 0.0, 900.0], 0.0)]
}

/// Billows of `albedo`: sheet sprites (shape 2) flagged (misc.w 1) to draw
/// noise, as a trail with `puffs` draws.
pub fn billows(d: &[Vec<f32>], albedo: f32) -> Vec<&Vec<f32>> {
    d.iter()
        .filter(|i| i[12] == 2.0 && i[15] == 1.0 && i[8] == albedo)
        .collect()
}

/// Lit ribbons (blended: an opacity in the colour's alpha) of `albedo`.
pub fn ribbons(d: &[Vec<f32>], albedo: f32) -> Vec<&Vec<f32>> {
    d.iter()
        .filter(|i| i[12] == 0.0 && i[11] > 0.0 && i[8] == albedo)
        .collect()
}

/// Additive streaks (a flame's plume and core).
pub fn flames(d: &[Vec<f32>]) -> usize {
    d.iter().filter(|i| i[12] == 0.0 && i[11] == 0.0).count()
}

/// Additive glows, and the brightest one's red.
pub fn glows(d: &[Vec<f32>]) -> (usize, f32) {
    let g: Vec<_> = d.iter().filter(|i| i[12] == 1.0 && i[11] == 0.0).collect();
    (g.len(), g.iter().map(|i| i[8]).fold(0.0, f32::max))
}

pub fn span(r: &[f32]) -> f64 {
    let d: Vec<f64> = (0..3).map(|k| f64::from(r[4 + k] - r[k])).collect();
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
}

/// A jet circling at cruise, its wings pulling past `ON_LOAD`.
pub fn circling(t: f64) -> Vec<Craft> {
    let r = 1500.0;
    let a = CRUISE * t / r;
    vec![craft(
        2.0,
        "burner",
        [r * a.cos(), r * a.sin(), 900.0],
        a + std::f64::consts::FRAC_PI_2,
    )]
}

/// A jet flown along `path(t)` (its position), its heading along its way.
pub fn along(path: impl Fn(f64) -> [f64; 3]) -> impl Fn(f64) -> Vec<Craft> {
    move |t| {
        let (p, q) = (path(t), path(t + 1e-3));
        let yaw = (q[1] - p[1]).atan2(q[0] - p[0]);
        vec![craft(2.0, "burner", p, yaw)]
    }
}

/// A level turn at `speed` whose rate is `rate(t)` rad/s, from heading +X,
/// integrated finely so its published path bends as it flies.
pub fn turning(speed: f64, rate: impl Fn(f64) -> f64) -> impl Fn(f64) -> [f64; 3] {
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
pub fn rate_for(speed: f64, load: f64) -> f64 {
    G * (load * load - 1.0).sqrt() / speed
}

/// The wingtip ribbons' opacities, oldest first.
pub fn vapour(d: &[Vec<f32>]) -> Vec<f32> {
    ribbons(d, TIP_ALBEDO).iter().map(|r| r[11]).collect()
}

pub fn thickest(d: &[Vec<f32>]) -> f32 {
    vapour(d).into_iter().fold(0.0, f32::max)
}
