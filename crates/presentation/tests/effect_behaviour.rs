//! What a player sees the effects do, beyond the browser's vectors: nothing
//! without a published cause, a publication taken once, a tracer keeping its
//! chance through its flight, a wreck that burns, smoulders and goes out, a
//! paused clock that holds, a long gap that makes only what is still alive,
//! a smoke budget that thins every wreck alike, dust behind a moving hull
//! and off a hull whose heavy gun fires, smoke and dust drifting the way the
//! grass leans, rotors that show their blades still and smear when fast, and
//! wakes off whoever the side sees wading, decided by tick, not by frame.
//! Driven by synthetic publications over the fixture's presentation.
use presentation::effects::frame::{
    EffectBatch, EffectFrame, WakeImpulse, WakeKind, EFFECT_FLOATS,
};
use presentation::effects::publication::EffectPublication;
use presentation::effects::tables::EffectPresentation;
use presentation::effects::DrawnRotor;
use presentation::wind::WindSettings;
use std::f64::consts::FRAC_PI_2;

use serde_json::{json, Value};

const HZ: f64 = 30.0;
const DT: f64 = 1.0 / HZ;
const STREAK: f32 = 0.0;
const GLOW: f32 = 1.0;
const FLIPBOOK: f32 = 2.0;

fn vectors() -> Value {
    let at = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/effect-vectors.json"
    );
    serde_json::from_str(&std::fs::read_to_string(at).unwrap()).unwrap()
}

/// The fixture's presentation, edited by `edit`.
fn presentation_with(edit: impl FnOnce(&mut Value)) -> EffectPresentation {
    let mut p = vectors()["presentation"].clone();
    edit(&mut p);
    EffectPresentation::from_value(&p).unwrap()
}

fn frame() -> EffectFrame {
    EffectFrame::new(HZ, presentation_with(|_| {}))
}

/// Tick `tick`'s publication: nothing but what `fields` names.
fn publ(tick: f64, fields: Value) -> EffectPublication {
    let mut p = json!({"tick": tick, "segments": [], "blasts": [], "shooters": [], "smokes": []});
    for (k, v) in fields.as_object().unwrap() {
        p[k] = v.clone();
    }
    serde_json::from_value(p).unwrap()
}

/// What `f` draws at `clock` in still air: each instance's sixteen floats.
fn drawn(f: &mut EffectFrame, clock: f64) -> Vec<Vec<f32>> {
    drawn_in(f, clock, None)
}

/// What `f` draws at `clock` in `wind`.
fn drawn_in(f: &mut EffectFrame, clock: f64, wind: Option<&WindSettings>) -> Vec<Vec<f32>> {
    let mut batch = EffectBatch::new(f.presentation().capacity);
    f.build(clock, &mut batch, None, &[], &[], wind);
    assert_eq!(batch.dropped, 0, "a frame never exceeds capacity");
    batch.data[..batch.count * EFFECT_FLOATS]
        .chunks(EFFECT_FLOATS)
        .map(<[f32]>::to_vec)
        .collect()
}

fn shaped(d: &[Vec<f32>], shape: f32) -> Vec<&Vec<f32>> {
    d.iter().filter(|i| i[12] == shape).collect()
}

/// Flipbooks on the fire layer (flames) or the dust layer (smoke, dust).
fn layer(d: &[Vec<f32>], layer: f32) -> Vec<&Vec<f32>> {
    shaped(d, FLIPBOOK)
        .into_iter()
        .filter(|i| i[6] == layer)
        .collect()
}

/// A tank seen at (100, 50) with `shots` cannon rounds counted.
fn tank(key: f64, shots: f64) -> Value {
    json!({"key": key, "position": [100.0, 50.0, 0.0], "half": [3.5, 1.8, 1.2], "yaw": 0.0,
        "members": [], "mounts": [{"bearing": 0.0, "elevation": 0.0, "shots": shots,
        "kind": "tank_ap", "muzzle": {"id": "gun", "on": null, "pivot": [0.5, 0.0, 2.2],
        "ring": [0.0, 0.0, 0.0], "muzzle": [5.2, 0.0, 0.0]}}]})
}

fn wreck(x: f64, y: f64) -> Value {
    json!({"key": format!("wreck:{x},{y}"), "kind": "wreck", "center": [x, y, 0.2],
        "yaw": 0.4, "half": [3.5, 1.8, 1.2]})
}

/// Notes ticks `from..=to`, each publishing `smokes`.
fn burn(f: &mut EffectFrame, from: u32, to: u32, smokes: &Value) {
    for t in from..=to {
        f.note(&publ(f64::from(t), json!({ "smokes": smokes })));
    }
}

fn wreck_style(key: &str) -> f64 {
    vectors()["presentation"]["smoke"]["wreck"][key]
        .as_f64()
        .unwrap()
}

#[test]
fn no_effect_without_a_published_cause_and_a_counter_s_rise_is_a_flash() {
    let mut f = frame();
    // A unit first seen with rounds already counted shows no shot.
    f.note(&publ(5.0, json!({"shooters": [tank(2.0, 7.0)]})));
    f.note(&publ(6.0, json!({"shooters": [tank(2.0, 7.0)]})));
    for t in [4.5 * DT, 6.0 * DT, 5.0] {
        assert!(drawn(&mut f, t).is_empty(), "nothing at {t}");
    }
    // Its counter rising is a shot: one flash, at the cannon's muzzle.
    f.note(&publ(7.0, json!({"shooters": [tank(2.0, 8.0)]})));
    let d = drawn(&mut f, 6.01 * DT);
    let flash = shaped(&d, GLOW);
    assert_eq!(flash.len(), 1, "one flash: {d:?}");
    let reach = f64::from(flash[0][0] - 100.0).hypot(f64::from(flash[0][1] - 50.0));
    assert!(
        reach > 4.0 && reach < 7.0,
        "at the muzzle, not the hull: {reach} m off"
    );
}

#[test]
fn a_publication_is_taken_once_and_an_earlier_tick_is_a_new_battle() {
    let busy = publ(
        2.0,
        json!({"segments": [{"path": [[0.0, 0.0, 1.0], [28.0, 0.0, 1.0]], "ricochets": [],
            "kind": "rifle", "shooter": null, "hit": "ground", "normal": [0.0, 0.0, 1.0]}],
            "blasts": [{"point": [28.0, 0.0, 0.0], "radius": 6.0, "kind": "grenade"}]}),
    );
    let (mut once, mut twice) = (frame(), frame());
    once.note(&publ(1.0, json!({})));
    once.note(&busy);
    twice.note(&publ(1.0, json!({})));
    for _ in 0..3 {
        twice.note(&busy);
    }
    for t in [1.5 * DT, 2.0 * DT, 2.5 * DT, 0.5] {
        assert_eq!(drawn(&mut twice, t), drawn(&mut once, t), "at {t}");
    }
    assert!(!drawn(&mut once, 2.0 * DT).is_empty());
    twice.note(&publ(1.0, json!({})));
    assert!(
        drawn(&mut twice, 2.0 * DT).is_empty(),
        "everything before is forgotten"
    );
}

#[test]
fn a_round_s_tracer_chance_is_drawn_once_and_carried_through_its_flight() {
    let mut f = frame(); // rifle rounds show as tracers one in five
    let mut chosen: Option<Vec<i64>> = None;
    for tick in 1..=3 {
        let x = f64::from(tick) * 10.0;
        let rounds: Vec<Value> = (0..400)
            .map(|id| json!({"path": [[x - 10.0, f64::from(id) * 5.0, 2.0], [x, f64::from(id) * 5.0, 2.0]],
                "ricochets": [], "kind": "rifle", "shooter": null, "hit": "none", "normal": null}))
            .collect();
        f.note(&publ(f64::from(tick), json!({ "segments": rounds })));
        let d = drawn(&mut f, (f64::from(tick) - 0.5) * DT);
        let mut shown: Vec<i64> = shaped(&d, STREAK).iter().map(|i| i[1] as i64).collect();
        shown.sort_unstable();
        shown.dedup();
        assert!(
            (60..=100).contains(&shown.len()),
            "about one in five: {}",
            shown.len()
        );
        if let Some(before) = &chosen {
            assert_eq!(&shown, before, "the same rounds stay tracers");
        }
        chosen = Some(shown);
    }
}

#[test]
fn a_known_wreck_burns_then_smoulders_then_is_out() {
    let (burn_s, smoulder_s) = (wreck_style("burn_s"), wreck_style("smoulder_s"));
    let src = json!([wreck(100.0, 200.0)]);
    let mut f = frame();
    let mut batch = EffectBatch::new(f.presentation().capacity);
    let burning = 20 * 30;
    burn(&mut f, 1, burning, &src);
    let d = drawn(&mut f, f64::from(burning) * DT);
    let (flames, smoke) = (layer(&d, 0.0), layer(&d, 1.0));
    assert!(flames.len() > 3, "flames lick over the hull");
    for i in &flames {
        assert!(f64::from(i[0] - 100.0).hypot(f64::from(i[1] - 200.0)) < 5.0);
    }
    assert!(
        smoke.len() > 20 && smoke.iter().any(|i| i[2] > 15.0),
        "a column of smoke"
    );
    f.build(f64::from(burning) * DT, &mut batch, None, &[], &[], None);
    assert!(batch.lights.count > 0, "the fire lights the ground");
    assert_eq!(f.stats().sources, 1);

    let smouldering = ((burn_s + 20.0) * HZ) as u32;
    burn(&mut f, burning + 1, smouldering, &src);
    let d = drawn(&mut f, f64::from(smouldering) * DT);
    assert!(layer(&d, 0.0).is_empty(), "no fire left");
    let thin = layer(&d, 1.0).len();
    assert!(thin > 0 && thin < smoke.len(), "thinner smoke: {thin}");
    f.build(
        f64::from(smouldering) * DT,
        &mut batch,
        None,
        &[],
        &[],
        None,
    );
    assert_eq!(batch.lights.count, 0);

    // Out, though the wreck is still known: nothing left.
    let out = ((burn_s + smoulder_s + 60.0) * HZ) as u32;
    burn(&mut f, smouldering + 1, out, &src);
    assert!(drawn(&mut f, f64::from(out) * DT).is_empty());
    assert_eq!((f.stats().live, f.stats().sources), (0, 0));
}

#[test]
fn paused_the_smoke_holds_a_reset_clears_it_and_a_long_gap_makes_only_the_living() {
    let src = json!([wreck(50.0, 50.0)]);
    let mut f = frame();
    burn(&mut f, 1, 90, &src);
    let held = drawn(&mut f, 90.0 * DT);
    assert!(!held.is_empty());
    burn(&mut f, 90, 90, &src); // a republished tick is ignored
    assert_eq!(
        drawn(&mut f, 90.0 * DT),
        held,
        "paused, the same frame again"
    );
    f.reset();
    assert!(drawn(&mut f, 90.0 * DT).is_empty());
    assert_eq!((f.stats().live, f.stats().sources), (0, 0));

    // Ten minutes on, long after the wreck went out: nothing to catch up on.
    let mut g = frame();
    burn(&mut g, 1, 1, &src);
    let later = 1 + 600 * 30;
    burn(&mut g, later, later, &src);
    assert!(drawn(&mut g, f64::from(later) * DT).is_empty());
    assert_eq!((g.stats().live, g.stats().sources), (0, 0));
}

#[test]
fn past_the_smoke_budget_every_wreck_thins_alike_and_none_goes_missing() {
    let budget = vectors()["presentation"]["smoke_budget"].as_f64().unwrap();
    // An aftermath: 400 wrecks learned at once, 50 m apart.
    let wrecks: Vec<Value> = (0..400)
        .map(|i| wreck(f64::from(i % 20) * 50.0, f64::from(i / 20) * 50.0))
        .collect();
    let mut f = frame();
    burn(&mut f, 1, 15 * 30, &Value::from(wrecks.clone()));
    let d = drawn(&mut f, 15.0 * 30.0 * DT);
    let smoke = shaped(&d, FLIPBOOK);
    assert!(
        smoke.len() as f64 <= budget * 1.1,
        "{} sprites",
        smoke.len()
    );
    for w in &wrecks {
        let (x, y) = (
            w["center"][0].as_f64().unwrap(),
            w["center"][1].as_f64().unwrap(),
        );
        let near = smoke
            .iter()
            .any(|i| (f64::from(i[0]) - x).hypot(f64::from(i[1]) - y) < 25.0);
        assert!(near, "the wreck at ({x}, {y}) still smokes");
    }
}

#[test]
fn a_moving_hull_raises_dust_behind_it_and_an_aircraft_none() {
    let dust = |speed: f64, airborne: bool| {
        let mut f = frame();
        let x = |t: u32| speed * f64::from(t) * DT;
        for t in 1..=60 {
            let mut hull = tank(3.0, 0.0);
            hull["position"] = json!([x(t), 0.0, 0.0]);
            hull["airborne"] = airborne.into();
            f.note(&publ(f64::from(t), json!({ "shooters": [hull] })));
        }
        let d = drawn(&mut f, 60.0 * DT);
        let puffs: Vec<f32> = layer(&d, 1.0).iter().map(|i| i[0]).collect();
        (puffs, x(60) as f32)
    };
    let (puffs, at) = dust(8.0, false);
    assert!(puffs.len() > 10, "{} puffs", puffs.len());
    assert!(puffs.iter().all(|&px| px < at), "all behind it");
    assert!(dust(0.0, false).0.is_empty(), "standing still, none");
    assert!(dust(8.0, true).0.is_empty(), "flying over the ground, none");
}

/// The biome's wind block, blowing toward `heading_deg` at `drift_mps`.
fn wind(heading_deg: f64, drift_mps: f64) -> WindSettings {
    let block = json!({"heading_deg": heading_deg, "drift_mps": drift_mps, "gust_m": 40.0, "gust_mps": 4.0});
    serde_json::from_value(block).unwrap()
}

/// What a cause draws in a wind: its puffs' floats.
type Puffs = fn(&WindSettings) -> Vec<Vec<f32>>;

#[test]
fn smoke_and_dust_drift_the_way_the_grass_leans_whatever_the_heading() {
    // A wreck's smoke, and the dust a hull raises driving north.
    let smoke = |w: &WindSettings| {
        let mut f = frame();
        burn(&mut f, 1, 150, &json!([wreck(100.0, 200.0)]));
        layer(&drawn_in(&mut f, 150.0 * DT, Some(w)), 1.0)
            .into_iter()
            .cloned()
            .collect::<Vec<_>>()
    };
    let dust = |w: &WindSettings| {
        let mut f = frame();
        for t in 1..=60 {
            let mut hull = tank(3.0, 0.0);
            hull["position"] = json!([0.0, 8.0 * f64::from(t) * DT, 0.0]);
            f.note(&publ(f64::from(t), json!({ "shooters": [hull] })));
        }
        layer(&drawn_in(&mut f, 60.0 * DT, Some(w)), 1.0)
            .into_iter()
            .cloned()
            .collect::<Vec<_>>()
    };
    for heading in [0.0, 35.0, 100.0, 190.0, 260.0, 330.0] {
        // Grass leans along the record's `toward` (`grassVertex.ts`).
        let blowing = wind(heading, 2.0);
        let lean = blowing.record().toward.map(f64::from);
        let causes: [(&str, Puffs); 2] = [("smoke", smoke), ("dust", dust)];
        for (cause, puffs) in causes {
            let (moved, still) = (puffs(&blowing), puffs(&wind(heading, 0.0)));
            assert!(
                moved.len() > 10 && moved.len() == still.len(),
                "{cause}: the same puffs"
            );
            let mut drifted = 0;
            for (a, b) in moved.iter().zip(&still) {
                let d: [f64; 3] = std::array::from_fn(|i| f64::from(a[i] - b[i]));
                let (along, across) = (
                    d[0] * lean[0] + d[1] * lean[1],
                    d[1] * lean[0] - d[0] * lean[1],
                );
                assert!(
                    along >= -1e-3 && across.abs() < 1e-3 + 1e-3 * along,
                    "{cause} at {heading}°: {d:?}"
                );
                assert!(
                    d[2].abs() < 1e-3,
                    "{cause} at {heading}°: the wind is level"
                );
                drifted += usize::from(along > 0.5);
            }
            assert!(
                drifted > moved.len() / 2,
                "{cause} at {heading}°: most puffs drift downwind"
            );
        }
    }
}

#[test]
fn a_still_rotor_shows_its_blades_and_a_fast_one_smears_them_never_past_a_turn() {
    let rotor = |turned: f64| {
        let mut r = vectors()["rotor"].clone();
        r["turned"] = turned.into();
        let r: DrawnRotor = serde_json::from_value(r).unwrap();
        let mut f = frame();
        let mut batch = EffectBatch::new(f.presentation().capacity);
        f.build(0.0, &mut batch, None, &[r], &[], None);
        let d = batch.data[..batch.count * EFFECT_FLOATS].to_vec();
        d.chunks(EFFECT_FLOATS)
            .map(<[f32]>::to_vec)
            .collect::<Vec<_>>()
    };
    // Still: its blades (swept through nothing), its hub and its mast; no disc.
    let still = rotor(0.0);
    assert_eq!(still.len(), 3);
    assert_eq!(still[0][7], 0.0);
    // Turning: the blades sweep the shutter's share; fast, at most a whole turn.
    let shutter = vectors()["presentation"]["rotor_blur"]["shutter"]
        .as_f64()
        .unwrap();
    let turning = rotor(0.8);
    assert_eq!(turning.len(), 4, "blades, a faint disc, hub and mast");
    assert!((f64::from(turning[0][7]) - 0.8 * shutter).abs() < 1e-6);
    let fast = rotor(100.0);
    assert!((f64::from(fast[0][7]) - std::f64::consts::TAU).abs() < 1e-6);
}

#[test]
fn a_heavy_gun_s_shot_shakes_dust_off_its_whole_hull_and_blows_it_off_the_ground_under_its_muzzle()
{
    // The test's own look: a tank round shakes a full shot's dust, an
    // autocannon's a third of it, an HMG's none; puffs that stay put.
    let p = presentation_with(|p| {
        p["looks"]["looks"]["tank_sabot"]["flash"]["dust"] = 1.0.into();
        p["looks"]["looks"]["ifv_cannon"]["flash"]["dust"] = (1.0 / 3.0).into();
        p["looks"]["looks"]["heavy_machine_gun"]["flash"]["dust"] = 0.0.into();
        let mut puff = p["dust"].clone();
        for unused in ["spacing_m", "min_speed_mps", "full_speed_mps"] {
            puff.as_object_mut().unwrap().remove(unused);
        }
        for (k, v) in [
            ("rise_mps", json!(0)),
            ("spread_mps", json!(0)),
            ("life_s", json!(1.5)),
        ] {
            puff[k] = v;
        }
        puff["size_m"] = json!([0.5, 1.0]);
        let (mut hull, mut ground) = (puff.clone(), puff);
        (hull["count"], hull["over_s"]) = (12.into(), 0.1.into());
        (ground["count"], ground["over_s"], ground["reach_m"]) = (9.into(), 0.1.into(), 4.into());
        p["shot_dust"] = json!({"hull": hull, "ground": ground});
    });
    // The puffs (x, y, z) drawn just after a hull at (100, 50) facing east
    // fires its first mount, a `kind`, north.
    let shot_dust = |kind: &str, airborne: bool| {
        let mut f = EffectFrame::new(HZ, p.clone());
        let shooter = |shots: f64| {
            let mut t = tank(2.0, shots);
            t["airborne"] = airborne.into();
            t["mounts"][0]["kind"] = kind.into();
            t["mounts"][0]["bearing"] = FRAC_PI_2.into();
            t
        };
        f.note(&publ(5.0, json!({"shooters": [shooter(3.0)]})));
        f.note(&publ(6.0, json!({"shooters": [shooter(4.0)]})));
        let d = drawn(&mut f, 5.0 * DT + 0.15);
        layer(&d, 1.0)
            .iter()
            .map(|i| [i[0], i[1], i[2]].map(f64::from))
            .collect::<Vec<_>>()
    };
    let dust = shot_dust("tank_ap", false);
    let (hl, hw, top) = (3.5, 1.8, 2.4);
    let on_hull = |i: &&[f64; 3]| (i[0] - 100.0).abs() <= hl && (i[1] - 50.0).abs() <= hw;
    let (over_hull, on_ground): (Vec<&[f64; 3]>, Vec<_>) = dust.iter().partition(on_hull);
    // Shaken off the whole hull: end to end, off its roof and down its flanks…
    assert!(!over_hull.is_empty());
    assert!(over_hull.iter().all(|i| i[2] <= top));
    assert!(over_hull.iter().any(|i| i[0] > 100.0 + hl / 3.0));
    assert!(over_hull.iter().any(|i| i[0] < 100.0 - hl / 3.0));
    assert!(over_hull.iter().any(|i| i[2] > top * 0.7));
    assert!(over_hull.iter().any(|i| i[2] < top * 0.5));
    // …and blown off the ground out where the muzzle points, past the hull's side.
    assert!(!on_ground.is_empty());
    assert!(on_ground.iter().all(|i| i[2] < top * 0.5 && i[1] > 50.0));
    let middle = on_ground.iter().map(|i| i[1]).sum::<f64>() / on_ground.len() as f64;
    assert!(middle > 50.0 + 5.2, "ahead of the muzzle: {middle}");
    // An autocannon's shot raises less; an HMG's none, nor anything in the air.
    let light = shot_dust("autocannon", false);
    assert!(!light.is_empty() && light.len() < dust.len() / 2);
    assert!(shot_dust("hmg", false).is_empty());
    assert!(shot_dust("tank_ap", true).is_empty());
}

/// `tank`'s hull at (`x`, 50), wading in water 0.6 m deep wherever `wet`.
fn hull_at(x: f64, wet: bool) -> Value {
    let mut hull = tank(3.0, 0.0);
    hull["position"] = json!([x, 50.0, 0.0]);
    if wet {
        hull["wading"] = json!([{ "at": [x, 50.0, 0.0], "depth_m": 0.6 }]);
    }
    hull
}

/// A squad of two at (`x`, 20), both wading.
fn squad_at(x: f64) -> Value {
    json!({"key": 5.0, "position": [x, 20.0, 0.0], "half": null, "yaw": 0.0,
        "members": [7.0, 8.0], "mounts": [],
        "wading": [{ "at": [x, 19.0, 0.0], "depth_m": 0.6 }, { "at": [x, 21.0, 0.0], "depth_m": 0.6 }]})
}

/// Every wake impulse `f` made over ticks `1..=ticks` of `unit(t)`, taken
/// tick by tick as a renderer takes them.
fn wakes(f: &mut EffectFrame, ticks: u32, unit: impl Fn(u32) -> Value) -> Vec<WakeImpulse> {
    let mut made = vec![];
    for t in 1..=ticks {
        f.note(&publ(f64::from(t), json!({ "shooters": [unit(t)] })));
        made.extend(f.wakes(f64::from(t - 1) * DT, f64::from(t) * DT).cloned());
    }
    made
}

#[test]
fn a_hull_moving_in_standing_water_makes_a_wake_and_no_dust() {
    let run = |wet: bool| {
        let mut f = frame();
        let made = wakes(&mut f, 60, |t| {
            hull_at(100.0 + 8.0 * f64::from(t) * DT, wet)
        });
        (made, layer(&drawn(&mut f, 60.0 * DT), 1.0).len())
    };
    let (made, dust) = run(true);
    assert!(made.len() > 5, "a wake behind it: {made:?}");
    assert_eq!(dust, 0, "no dust off the water");
    let (made, dust) = run(false);
    assert!(made.is_empty(), "no wake on dry ground");
    assert!(dust > 10, "the dust it raised there");
}

#[test]
fn a_still_wader_makes_rings_and_a_moving_one_drags_a_v() {
    let still = wakes(&mut frame(), 60, |_| squad_at(40.0));
    assert!(!still.is_empty(), "a still squad rings the water");
    assert!(still.iter().all(|w| w.kind == WakeKind::Rings), "{still:?}");
    let moving = wakes(&mut frame(), 60, |t| {
        squad_at(40.0 + 3.0 * f64::from(t) * DT)
    });
    assert!(!moving.is_empty(), "a moving squad drags a wake");
    assert!(moving.iter().all(|w| w.kind == WakeKind::Vee), "{moving:?}");
    // Each wader's own: both men's lines, along their path.
    for y in [19.0, 21.0] {
        assert!(
            moving.iter().any(|w| (w.xy[1] - y).abs() < 1e-9),
            "the man at y {y}"
        );
    }
}

#[test]
fn a_unit_out_of_sight_makes_no_wake() {
    let mut f = frame();
    // Seen wading for a second, then gone from the publication.
    let made = wakes(&mut f, 60, |t| {
        if t <= 30 {
            squad_at(40.0)
        } else {
            json!({"key": 9.0, "position": [0.0, 0.0, 0.0], "half": null, "yaw": 0.0,
                "members": [], "mounts": []})
        }
    });
    assert!(!made.is_empty());
    let last_seen = 30.0 * DT;
    assert!(
        made.iter().all(|w| w.at <= last_seen),
        "none after it hid: {made:?}"
    );
}

#[test]
fn wakes_do_not_depend_on_frame_rate() {
    // The same ticks, drawn at 24 and at 144 frames a second, each
    // publication taken as the clock reaches the tick it covers.
    let at = |fps: f64| {
        let mut f = frame();
        let mut made = vec![];
        let (mut clock, mut tick) = (0.0, 0);
        while clock < 2.0 {
            let next = clock + 1.0 / fps;
            while f64::from(tick) * DT <= next {
                tick += 1;
                let x = 100.0 + 8.0 * f64::from(tick) * DT;
                let units = json!([hull_at(x, true), squad_at(40.0)]);
                f.note(&publ(f64::from(tick), json!({ "shooters": units })));
            }
            drawn(&mut f, next);
            made.extend(f.wakes(clock, next).cloned());
            clock = next;
        }
        made
    };
    let slow = at(24.0);
    assert!(slow.len() > 10, "{slow:?}");
    assert_eq!(format!("{slow:?}"), format!("{:?}", at(144.0)));
}
