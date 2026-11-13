//! A smoke screen as it is drawn (`specs/smoke-screen`, slice 07): its puffs
//! stay inside the circle `contract::smoke` grows and fades in steps and
//! below the cloud's top for its whole life, whatever the wind; the drawn
//! cloud stands at the law's first step from its pop; they cost wreck smoke
//! nothing; and a capability naming a look the presentation lacks is refused
//! when its unit is fielded. Every number is the shipped presentation's and
//! the test catalog's.
use contract::smoke::{state, SmokeCloud};
use presentation::assets::Runtime;
use presentation::effects::frame::{EffectBatch, EffectFrame, EFFECT_FLOATS};
use presentation::effects::publication::EffectPublication;
use presentation::effects::tables::EffectPresentation;
use presentation::fielded::Fielding;
use presentation::settings::PresentationSettings;
use presentation::wind::WindSettings;
use serde_json::{json, Value};
use std::path::PathBuf;

const HZ: u32 = 30;
const DT: f64 = 1.0 / HZ as f64;
const FLIPBOOK: f32 = 2.0;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn shipped() -> Value {
    serde_json::from_slice(&std::fs::read(root().join("fixtures/presentation.json")).unwrap())
        .unwrap()
}

/// The shipped effects with the shipped smoke looks, as a battle fields them.
fn presentation() -> EffectPresentation {
    let settings = PresentationSettings::from_value(&shipped()).unwrap();
    let mut p = settings.effects.clone();
    p.smoke_screen_looks = settings.smoke_screen_looks.clone();
    p
}

/// The cloud the test catalog's `kind` pops: cloud `id`, at `centre`, on `tick`.
fn cloud(kind: &str, id: u32, centre: [f64; 3], tick: u64) -> SmokeCloud {
    let rules: contract::scenario::Rules =
        serde_json::from_value(sim::fixtures::test_rules()).unwrap();
    let smoke = rules.catalog.by_id(kind).capabilities.smoke.clone();
    SmokeCloud {
        id,
        centre,
        popped: tick,
        spec: smoke.expect("a smoke carrier").cloud,
    }
}

/// Tick `tick`'s publication of `clouds`, and of `smokes` (wrecks).
fn publ(tick: u64, clouds: &[SmokeCloud], smokes: &Value) -> EffectPublication {
    serde_json::from_value(json!({
        "tick": tick as f64, "segments": [], "blasts": [], "shooters": [], "smokes": smokes,
        "smokeClouds": clouds,
    }))
    .unwrap()
}

/// Every instance drawn at `clock`, sixteen floats each.
fn drawn(f: &mut EffectFrame, clock: f64, wind: Option<&WindSettings>) -> Vec<Vec<f32>> {
    let mut batch = EffectBatch::new(f.presentation().capacity);
    f.build(clock, &mut batch, None, &[], &[], wind);
    batch.data[..batch.count * EFFECT_FLOATS]
        .chunks(EFFECT_FLOATS)
        .map(<[f32]>::to_vec)
        .collect()
}

#[test]
fn every_puff_stays_in_its_cloud_below_its_top_for_its_whole_life_in_any_wind() {
    let clouds = [
        cloud("test_smoke_tank", 0, [100.0, 50.0, 2.0], 10),
        cloud("test_smoke_squad", 1, [160.0, 50.0, -1.0], 40),
    ];
    let end = clouds
        .iter()
        .map(|c| c.popped + (c.spec.life_s * f64::from(HZ)).ceil() as u64)
        .max()
        .unwrap();
    let gale: Option<WindSettings> = serde_json::from_value(
        json!({"heading_deg": 30.0, "drift_mps": 6.0, "gust_m": 40.0, "gust_mps": 4.0}),
    )
    .ok();
    assert!(gale.is_some());
    let mut f = EffectFrame::new(f64::from(HZ), presentation());
    let mut seen = [false; 2];
    for tick in 1..end + 5 {
        f.note(&publ(tick, &clouds, &json!([])));
        // On the tick, and between it and the next.
        for clock in [tick as f64 * DT, (tick as f64 + 0.5) * DT] {
            let still = drawn(&mut f, clock, None);
            let windy = drawn(&mut f, clock, gale.as_ref());
            assert_eq!(still, windy, "no wind moves a smoke screen");
            let at = (clock / DT + 1e-6).floor() as u64;
            let states: Vec<_> = clouds.iter().map(|c| state(c, at, HZ)).collect();
            for i in still.iter().filter(|i| i[12] == FLIPBOOK) {
                let (x, y, z, r) = (
                    f64::from(i[0]),
                    f64::from(i[1]),
                    f64::from(i[2]),
                    f64::from(i[3]),
                );
                let k = clouds
                    .iter()
                    .position(|c| (c.centre[0] - x).abs() < 40.0)
                    .unwrap();
                let (c, s) = (
                    &clouds[k],
                    states[k].as_ref().expect("a puff of a live cloud"),
                );
                seen[k] = true;
                let out = (x - c.centre[0]).hypot(y - c.centre[1]);
                assert!(
                    out + r <= s.radius_m + 1e-3,
                    "tick {at}: a puff of radius {r} at {out} m leaves its {} m circle",
                    s.radius_m
                );
                assert!(
                    z + r <= c.centre[2] + s.height_m + 1e-3,
                    "tick {at}: a puff tops out at {} over a {} m cloud",
                    z + r - c.centre[2],
                    s.height_m
                );
            }
            if states.iter().all(Option::is_none) {
                assert!(still.is_empty(), "a cloud past its life draws nothing");
            }
        }
    }
    assert_eq!(seen, [true, true], "both clouds drew");
}

#[test]
fn a_drawn_cloud_already_fills_its_first_bloom_step_on_the_tick_it_pops() {
    // On its pop tick the law already conceals out to the first step's
    // circle, so the drawn cloud stands there too rather than growing from
    // a point.
    let c = cloud("test_smoke_tank", 0, [100.0, 50.0, 0.0], 10);
    let first = state(&c, c.popped, HZ).unwrap();
    assert!(first.radius_m < c.spec.radius_m, "the law blooms in steps");
    let mut f = EffectFrame::new(f64::from(HZ), presentation());
    f.note(&publ(c.popped, std::slice::from_ref(&c), &json!([])));
    let puffs = drawn(&mut f, c.popped as f64 * DT, None);
    let reach = (puffs.iter())
        .map(|i| f64::from(i[0] - 100.0).hypot(f64::from(i[1] - 50.0)) + f64::from(i[3]))
        .fold(0.0, f64::max);
    assert!(
        reach > 0.75 * first.radius_m,
        "on its pop tick the drawn cloud reaches {reach} m of its first step's {} m",
        first.radius_m
    );
}

#[test]
fn smoke_screens_cost_the_wrecks_nothing_and_keep_to_their_own_budget() {
    let p = presentation();
    let wrecks = json!((0..40)
        .map(|k| json!({"key": format!("w{k}"), "kind": "wreck", "center": [k as f64 * 15.0, 0, 0], "yaw": 0, "half": [3, 1.6, 1.2]}))
        .collect::<Vec<_>>());
    // Enough clouds to pass the screens' own budget, in a row of their own
    // north of the wrecks'.
    let per = f64::from(p.smoke_screen_looks["discharger"].puffs);
    let n = (p.smoke_screen_budget / per).ceil() as u32 * 2;
    let clouds: Vec<_> = (0..n)
        .map(|k| cloud("test_smoke_tank", k, [f64::from(k) * 40.0, 200.0, 0.0], 1))
        .collect();
    let run = |clouds: &[SmokeCloud]| {
        let mut f = EffectFrame::new(f64::from(HZ), p.clone());
        for tick in 1..=300 {
            f.note(&publ(tick, clouds, &wrecks));
        }
        drawn(&mut f, 300.0 * DT, None)
    };
    let alone = run(&[]);
    let together = run(&clouds);
    let (screens, rest): (Vec<_>, Vec<_>) =
        (together.into_iter()).partition(|i| i[12] == FLIPBOOK && f64::from(i[1]) > 100.0);
    assert_eq!(
        rest, alone,
        "the wrecks draw exactly as they do without screens"
    );
    assert!(!screens.is_empty());
    assert!(
        screens.len() as f64 <= p.smoke_screen_budget,
        "{} screen puffs past a budget of {}",
        screens.len(),
        p.smoke_screen_budget
    );
}

#[test]
fn a_smoke_capability_naming_a_look_the_presentation_lacks_is_refused_when_fielded() {
    let rules: contract::scenario::Rules =
        serde_json::from_value(sim::fixtures::test_rules()).unwrap();
    let mut catalog = rules.catalog.view();
    let rules = serde_json::to_value(rules).unwrap();
    let unit = catalog["units"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|u| u["id"] == "test_smoke_tank")
        .unwrap();
    unit["capabilities"]["smoke"]["cloud"]["look"] = json!("no_such_look");
    let runtime = Runtime::open(&root().join("assets/runtime"))
        .unwrap()
        .catalog;
    let kinds = vec!["test_smoke_tank".to_owned()];
    let error = Fielding {
        presentation: &shipped(),
        rules: &rules,
        catalog: &catalog,
        runtime: &runtime,
        kinds: &kinds,
        factions: None,
    }
    .resolve(&|_| Err("unexpected bundle read".into()))
    .err()
    .expect("the unknown look is refused");
    assert!(
        error.contains("test_smoke_tank") && error.contains("no_such_look"),
        "{error}"
    );
}
