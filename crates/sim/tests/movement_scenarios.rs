//! Movement scenarios as data: a small map, units, scripted orders and a
//! duration, each with the outcomes it must show. The tests below assert
//! them; `src/bin/movement_shots.rs` replays the same table through the same
//! [`run`] and draws it (`cargo run -p sim --release --features shots --bin
//! movement_shots`).
//!
//! A check whose rule has not landed yet carries `pending: Some("<slice>: why")`.
//! It is still evaluated and reported, never asserted; the slice that lands
//! the rule deletes the reason.
#![allow(dead_code)]

use contract::ids::UnitId;
use contract::map::PropKind;
use contract::observation::MoveState;
use contract::scenario::{GroundRules, ScenarioDefinition};
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::math::{v2, Obb2, V2};
use sim::world::Prop;

/// One named scenario. Unit ids are indices into `units`.
pub struct Scenario {
    pub name: &'static str,
    /// What the scenario shows, in one line.
    pub caption: &'static str,
    /// Map JSON (`contract::map::MapDefinition`).
    pub map: Value,
    /// Unit JSON (`contract::scenario::UnitSetup` rows).
    pub units: Value,
    /// Scenario events (`burst` for craters, `add_prop`).
    pub events: Value,
    /// Orders (`contract::scenario::ScriptedOrder` rows).
    pub scripts: Value,
    /// Merged over the village rules (JSON merge): per-scenario rule numbers.
    pub rules: Value,
    pub seconds: f64,
    pub seed: u64,
    pub checks: Vec<Check>,
}

pub struct Check {
    pub kind: CheckKind,
    /// `Some(reason)` until the rule the check needs has landed.
    pub pending: Option<&'static str>,
}

pub enum CheckKind {
    /// Unit ends idle within `within_m` of `at`.
    Arrive {
        unit: u32,
        at: [f64; 2],
        within_m: f64,
    },
    /// No living soldier's disc ever overlaps a solid body (every prop but a
    /// bridge deck: Q27).
    SoldiersClearOfProps,
    /// No vehicle hull ever overlaps a prop that blocks vehicles.
    VehiclesClearOfProps,
    /// At the end, every squad's soldiers stand at least `min_m` apart.
    Spacing { min_m: f64 },
    /// Soldiers of different squads never come closer than `min_m`.
    SquadsNeverOverlap { min_m: f64 },
    /// Every living soldier stands within `within_m` of the ground under him.
    OnGround { within_m: f64 },
    /// At the end, at least `min` soldiers of `unit` have a body within 1.5 m
    /// between them and `threat`'s position, or stand in a crater (Q20).
    InCover { unit: u32, threat: u32, min: usize },
    /// `unit` halts (attack-move) at least `min_m` short of `goal`, and is
    /// still halted at the end.
    HaltsShort {
        unit: u32,
        goal: [f64; 2],
        min_m: f64,
    },
    /// The prop nearest `near` at the start ends at least `min_m` from where it began.
    PropMoved { near: [f64; 2], min_m: f64 },
}

fn check(kind: CheckKind) -> Check {
    Check {
        kind,
        pending: None,
    }
}

fn pending(reason: &'static str, kind: CheckKind) -> Check {
    Check {
        kind,
        pending: Some(reason),
    }
}

/// A check's result after a run.
#[derive(Debug)]
pub struct Outcome {
    pub label: String,
    pub pending: Option<&'static str>,
    pub passed: bool,
    pub detail: String,
}

pub const SOLDIER_RADIUS_M: f64 = 0.3;
/// Q20's cover reach: a body within this of a soldier can cover him.
const COVER_REACH_M: f64 = 1.5;

// --- the scenarios -----------------------------------------------------------

fn flat(size: [f64; 2], extra: Value) -> Value {
    let mut map = json!({ "size": size, "height_grid_m": 4, "slope_cutoff_deg": 35 });
    merge(&mut map, &extra);
    map
}

fn wall(center: [f64; 2], yaw: f64, half: [f64; 3]) -> Value {
    json!({ "kind": "wall", "center": center, "yaw": yaw, "half_extents": half })
}

fn crate_at(center: [f64; 2]) -> Value {
    json!({ "kind": "crate", "center": center, "yaw": 0, "half_extents": [0.8, 0.8, 0.6] })
}

fn wreck(center: [f64; 2], yaw: f64, half: [f64; 3]) -> Value {
    json!({ "kind": "wreck", "center": center, "yaw": yaw, "half_extents": half })
}

fn rifle(side: &str, at: [f64; 2]) -> Value {
    json!({ "side": side, "kind": "rifle", "position": at, "engagement": "return_fire_only" })
}

fn go(unit: u32, goal: [f64; 2]) -> Value {
    order(
        json!({ "kind": "move", "units": [unit], "gesture": unit + 1, "goal": goal, "route": "shortest" }),
    )
}

fn order(order: Value) -> Value {
    json!({ "tick": 1, "side": "blue", "order": order })
}

/// The movement lane's scenario table, simple to hard (t0–t3).
pub fn scenarios() -> Vec<Scenario> {
    use CheckKind::*;
    let none = json!([]);
    // Wrecks don't block infantry yet.
    let soldiers_in_wreck = "34: every solid body blocks infantry (Q27)";
    vec![
        Scenario {
            name: "t0-open-ground",
            caption: "open ground: one squad walks 85 m east",
            map: flat([120.0, 80.0], json!({})),
            units: json!([rifle("blue", [15.0, 40.0])]),
            events: none.clone(),
            scripts: json!([go(0, [100.0, 40.0])]),
            rules: json!({}),
            seconds: 36.0,
            seed: 1,
            checks: vec![
                check(Arrive {
                    unit: 0,
                    at: [100.0, 40.0],
                    within_m: 1.5,
                }),
                check(Spacing { min_m: 2.0 }),
                check(SoldiersClearOfProps),
            ],
        },
        Scenario {
            name: "t0-around-wreck",
            caption: "a squad passes a tank wreck in its path",
            map: flat(
                [120.0, 80.0],
                json!({ "props": [wreck([57.0, 40.0], 0.3, [3.5, 1.8, 1.2])] }),
            ),
            units: json!([rifle("blue", [15.0, 40.0])]),
            events: none.clone(),
            scripts: json!([go(0, [100.0, 40.0])]),
            rules: json!({}),
            seconds: 36.0,
            seed: 1,
            checks: vec![
                check(Arrive {
                    unit: 0,
                    at: [100.0, 40.0],
                    within_m: 1.5,
                }),
                pending(soldiers_in_wreck, SoldiersClearOfProps),
            ],
        },
        Scenario {
            name: "t0-tank-around-wreck",
            caption: "a tank drives around a tank wreck in its path",
            map: flat(
                [120.0, 80.0],
                json!({ "props": [wreck([57.0, 40.0], 0.3, [3.5, 1.8, 1.2])] }),
            ),
            units: json!([{ "side": "blue", "kind": "tank", "position": [15, 40], "engagement": "return_fire_only" }]),
            events: none.clone(),
            scripts: json!([go(0, [100.0, 40.0])]),
            rules: json!({}),
            seconds: 20.0,
            seed: 1,
            checks: vec![
                check(Arrive {
                    unit: 0,
                    at: [100.0, 40.0],
                    within_m: 1.5,
                }),
                check(VehiclesClearOfProps),
            ],
        },
        Scenario {
            name: "t0-through-gap",
            caption: "a squad crosses a wall through a 5 m gap",
            map: flat(
                [120.0, 80.0],
                json!({ "props": [
                    wall([60.0, 18.75], 0.0, [0.5, 18.75, 1.5]),
                    wall([60.0, 61.25], 0.0, [0.5, 18.75, 1.5]),
                ] }),
            ),
            units: json!([rifle("blue", [15.0, 60.0])]),
            events: none.clone(),
            scripts: json!([go(0, [100.0, 20.0])]),
            rules: json!({}),
            seconds: 45.0,
            seed: 1,
            checks: vec![
                check(Arrive {
                    unit: 0,
                    at: [100.0, 20.0],
                    within_m: 1.5,
                }),
                check(SoldiersClearOfProps),
            ],
        },
        Scenario {
            name: "t1-into-cover",
            caption: "move beside low walls and crates, enemy north-east: take cover facing it",
            map: flat(
                [120.0, 90.0],
                json!({ "props": [
                    wall([73.0, 38.0], 0.0, [3.0, 0.4, 0.5]),
                    wall([79.0, 43.0], 0.0, [0.4, 2.5, 0.5]),
                    crate_at([62.0, 26.0]),
                    crate_at([65.0, 25.0]),
                ] }),
            ),
            units: json!([rifle("blue", [35.0, 18.0]), rifle("red", [92.0, 54.0])]),
            events: json!([
                { "tick": 1, "burst": { "point": [58.0, 46.0], "weapon": "tank_he" } },
                { "tick": 1, "burst": { "point": [84.0, 52.0], "weapon": "tank_he" } },
            ]),
            scripts: json!([go(0, [70.0, 34.0])]),
            rules: json!({}),
            seconds: 30.0,
            seed: 1,
            checks: vec![
                check(Arrive {
                    unit: 0,
                    at: [70.0, 34.0],
                    within_m: 1.5,
                }),
                pending(
                    "33: soldiers claim cover at the order (D4)",
                    InCover {
                        unit: 0,
                        threat: 1,
                        min: 6,
                    },
                ),
                check(SoldiersClearOfProps),
            ],
        },
        Scenario {
            name: "t1-not-enough-cover",
            caption: "only two crates near the goal: two take cover, six scatter",
            map: flat(
                [120.0, 90.0],
                json!({ "props": [crate_at([62.0, 26.0]), crate_at([65.0, 25.0])] }),
            ),
            units: json!([rifle("blue", [35.0, 18.0]), rifle("red", [92.0, 54.0])]),
            events: none.clone(),
            scripts: json!([go(0, [63.0, 22.0])]),
            rules: json!({}),
            seconds: 26.0,
            seed: 1,
            checks: vec![
                check(Arrive {
                    unit: 0,
                    at: [63.0, 22.0],
                    within_m: 1.5,
                }),
                pending(
                    "33: soldiers claim cover at the order (D4)",
                    InCover {
                        unit: 0,
                        threat: 1,
                        min: 2,
                    },
                ),
                check(Spacing { min_m: 2.0 }),
                check(SoldiersClearOfProps),
            ],
        },
        Scenario {
            name: "t1-attack-move-halt",
            caption: "attack-move at an enemy squad: halt on contact, into the crates' cover",
            map: flat(
                [140.0, 100.0],
                json!({ "props": [crate_at([75.0, 57.0]), crate_at([77.0, 43.0]), crate_at([73.0, 60.0])] }),
            ),
            units: json!([
                { "side": "blue", "kind": "rifle", "position": [20, 50] },
                rifle("red", [120.0, 50.0]),
            ]),
            events: none.clone(),
            scripts: json!([order(
                json!({ "kind": "attack_move", "units": [0], "gesture": 1, "goal": [120.0, 50.0] })
            )]),
            // Short sight so the halt lands on a map small enough to read; soldiers
            // too tough to fall, so the halt lasts and nobody resumes the advance.
            rules: json!({ "sensors": { "infantry_ground_m": 40 }, "health": { "soldier": 1.0e6 } }),
            seconds: 40.0,
            seed: 1,
            checks: vec![
                check(HaltsShort {
                    unit: 0,
                    goal: [120.0, 50.0],
                    min_m: 25.0,
                }),
                pending(
                    "33: an attack-move halts into cover (D4, D5)",
                    InCover {
                        unit: 0,
                        threat: 1,
                        min: 3,
                    },
                ),
            ],
        },
        Scenario {
            name: "t2-two-squads-crossing",
            caption: "two squads cross paths at right angles at the same time",
            map: flat([120.0, 120.0], json!({})),
            units: json!([rifle("blue", [10.0, 60.0]), rifle("blue", [60.0, 10.0])]),
            events: none.clone(),
            scripts: json!([go(0, [110.0, 60.0]), go(1, [60.0, 110.0])]),
            rules: json!({}),
            seconds: 45.0,
            seed: 1,
            checks: vec![
                check(Arrive {
                    unit: 0,
                    at: [110.0, 60.0],
                    within_m: 1.5,
                }),
                check(Arrive {
                    unit: 1,
                    at: [60.0, 110.0],
                    within_m: 1.5,
                }),
                pending(
                    "32: soft personal space; soldiers never jam (Q10)",
                    SquadsNeverOverlap {
                        min_m: 2.0 * SOLDIER_RADIUS_M,
                    },
                ),
            ],
        },
        Scenario {
            name: "t2-door-jam",
            caption: "two squads converge on one 5 m door in a long wall",
            map: flat(
                [120.0, 80.0],
                json!({ "props": [
                    wall([60.0, 18.75], 0.0, [0.5, 18.75, 1.5]),
                    wall([60.0, 61.25], 0.0, [0.5, 18.75, 1.5]),
                ] }),
            ),
            units: json!([rifle("blue", [20.0, 30.0]), rifle("blue", [20.0, 50.0])]),
            events: none.clone(),
            scripts: json!([go(0, [100.0, 50.0]), go(1, [100.0, 30.0])]),
            rules: json!({}),
            seconds: 50.0,
            seed: 1,
            checks: vec![
                check(Arrive {
                    unit: 0,
                    at: [100.0, 50.0],
                    within_m: 1.5,
                }),
                check(Arrive {
                    unit: 1,
                    at: [100.0, 30.0],
                    within_m: 1.5,
                }),
                check(SoldiersClearOfProps),
                pending(
                    "32: soft personal space; soldiers never jam (Q10)",
                    SquadsNeverOverlap {
                        min_m: 2.0 * SOLDIER_RADIUS_M,
                    },
                ),
            ],
        },
        Scenario {
            name: "t2-one-man-gap",
            caption: "a squad threads a 1.5 m gap one man at a time",
            map: flat(
                [120.0, 80.0],
                json!({ "props": [
                    wall([60.0, 19.625], 0.0, [0.5, 19.625, 1.5]),
                    wall([60.0, 60.375], 0.0, [0.5, 19.625, 1.5]),
                ] }),
            ),
            units: json!([rifle("blue", [20.0, 40.0])]),
            events: none.clone(),
            scripts: json!([go(0, [100.0, 40.0])]),
            rules: json!({}),
            seconds: 45.0,
            seed: 1,
            checks: vec![
                pending(
                    "32: two-resolution navigation; a squad passes wherever one man fits (Q27)",
                    Arrive {
                        unit: 0,
                        at: [100.0, 40.0],
                        within_m: 1.5,
                    },
                ),
                check(SoldiersClearOfProps),
            ],
        },
        Scenario {
            name: "t2-slope",
            caption: "a squad crosses a ridge: every soldier stands on the ground",
            map: flat(
                [120.0, 80.0],
                json!({ "relief": [{ "kind": "ridge", "center": [60, 40], "peak_m": 8, "radius_m": 30 }] }),
            ),
            units: json!([rifle("blue", [10.0, 36.0])]),
            events: none.clone(),
            scripts: json!([go(0, [110.0, 44.0])]),
            rules: json!({}),
            seconds: 45.0,
            seed: 1,
            checks: vec![
                check(Arrive {
                    unit: 0,
                    at: [110.0, 44.0],
                    within_m: 1.5,
                }),
                check(OnGround { within_m: 0.25 }),
            ],
        },
        Scenario {
            name: "t3-tank-pushes-wreck",
            caption: "a tank meets a jeep-sized wreck on a road and shoves it off",
            map: flat(
                [140.0, 60.0],
                json!({
                    "roads": [{ "points": [[0, 30], [140, 30]], "width_m": 10 }],
                    "props": [wreck([70.0, 30.0], 0.2, [2.0, 1.0, 0.9])],
                }),
            ),
            units: json!([{ "side": "blue", "kind": "tank", "position": [10, 30], "engagement": "return_fire_only" }]),
            events: none.clone(),
            scripts: json!([go(0, [130.0, 30.0])]),
            rules: json!({}),
            seconds: 24.0,
            seed: 1,
            checks: vec![
                check(Arrive {
                    unit: 0,
                    at: [130.0, 30.0],
                    within_m: 1.5,
                }),
                check(VehiclesClearOfProps),
                pending(
                    "34: a heavy push class shoves lighter bodies (Q2, Q4)",
                    PropMoved {
                        near: [70.0, 30.0],
                        min_m: 2.0,
                    },
                ),
            ],
        },
        Scenario {
            // Stand-ins until slice 34 adds the kinds: a supply truck for the
            // jeep and a thin wall for the fence.
            name: "t3-jeep-blocked-by-fence",
            caption: "a light vehicle meets a fence it cannot push and drives round it",
            map: flat(
                [120.0, 70.0],
                json!({ "props": [wall([60.0, 30.0], 0.0, [0.1, 20.0, 0.6])] }),
            ),
            units: json!([{ "side": "blue", "kind": "supply", "position": [20, 30], "engagement": "return_fire_only" }]),
            events: none.clone(),
            scripts: json!([go(0, [100.0, 30.0])]),
            rules: json!({}),
            seconds: 30.0,
            seed: 1,
            checks: vec![
                check(Arrive {
                    unit: 0,
                    at: [100.0, 30.0],
                    within_m: 1.5,
                }),
                check(VehiclesClearOfProps),
            ],
        },
    ]
}

pub fn scenario(name: &str) -> Scenario {
    scenarios()
        .into_iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("no scenario {name}"))
}

// --- the runner ----------------------------------------------------------------

/// JSON merge: objects merge key by key, anything else replaces.
fn merge(base: &mut Value, patch: &Value) {
    match (base, patch) {
        (Value::Object(b), Value::Object(p)) => {
            for (k, v) in p {
                merge(b.entry(k.clone()).or_insert(Value::Null), v);
            }
        }
        (b, p) => *b = p.clone(),
    }
}

pub fn definition(s: &Scenario) -> ScenarioDefinition {
    let mut rules: Value =
        serde_json::from_str(include_str!("../../../fixtures/village.json")).unwrap();
    merge(&mut rules, &s.rules);
    serde_json::from_value(json!({
        "map": s.map, "rules": rules, "units": s.units, "events": s.events, "scripts": s.scripts,
    }))
    .unwrap()
}

pub fn tick_hz(s: &Scenario) -> u32 {
    definition(s).rules.tick_hz
}

/// Run `s` through the real battle for its duration, calling `each` with the
/// battle before the first step and after every step, and judge its checks.
pub fn run(s: &Scenario, mut each: impl FnMut(&Battle)) -> Vec<Outcome> {
    let setup = definition(s);
    let ground = setup.rules.ground.clone();
    let mut b = Battle::new(&setup, s.seed);
    let mut judges: Vec<Judge> = s.checks.iter().map(|c| Judge::new(&c.kind, &b)).collect();
    each(&b);
    for _ in 0..(s.seconds * setup.rules.tick_hz as f64).round() as u64 {
        b.step();
        for (j, c) in judges.iter_mut().zip(&s.checks) {
            j.watch(&c.kind, &b);
        }
        each(&b);
    }
    judges
        .into_iter()
        .zip(&s.checks)
        .map(|(j, c)| j.verdict(c, &b, &ground))
        .collect()
}

/// Assert every active check of the named scenario; report the pending ones.
pub fn assert_scenario(name: &str) {
    let s = scenario(name);
    let outcomes = run(&s, |_| {});
    let mut failed = Vec::new();
    for o in &outcomes {
        let state = match (o.pending, o.passed) {
            (None, true) => "pass",
            (None, false) => "FAIL",
            (Some(_), true) => "pending (passes: un-pend it?)",
            (Some(_), false) => "pending (fails)",
        };
        println!("{name}: {} — {state}: {}", o.label, o.detail);
        if o.pending.is_none() && !o.passed {
            failed.push(format!("{}: {}", o.label, o.detail));
        }
    }
    assert!(failed.is_empty(), "{name}: {failed:#?}");
}

pub fn units(b: &Battle) -> impl Iterator<Item = &sim::units::Unit> {
    (0u32..)
        .map(move |i| b.unit(UnitId(i)))
        .take_while(|u| u.is_some())
        .flatten()
}

fn solid(p: &Prop) -> bool {
    p.kind != PropKind::BridgeDeck
}

/// Distance from `p` to a rectangle (0 inside).
pub fn distance_to_box(r: &Obb2, p: V2) -> f64 {
    let d = r.to_local(p);
    let (x, y) = (
        (d.x.abs() - r.half.x).max(0.0),
        (d.y.abs() - r.half.y).max(0.0),
    );
    x.hypot(y)
}

/// Whether the segment `a`→`b` crosses the rectangle (sampled at 5 cm).
fn crosses(r: &Obb2, a: V2, b: V2) -> bool {
    let n = ((b - a).length() / 0.05).ceil().max(1.0) as usize;
    (0..=n).any(|k| r.contains(a + (b - a) * (k as f64 / n as f64), 0.0))
}

/// Does a body within reach lie between the soldier at `p` and `threat`, or
/// does he stand in a crater?
pub fn covered(b: &Battle, ground: &GroundRules, p: V2, threat: V2) -> bool {
    if b.ground().crater_fill(p.x, p.y, ground) > 0.0 {
        return true;
    }
    let toward = (threat - p).normalized();
    let reach = p + toward * (COVER_REACH_M + SOLDIER_RADIUS_M);
    b.world()
        .props_near(p, COVER_REACH_M + 10.0)
        .into_iter()
        .filter(|q| solid(q) && distance_to_box(&q.footprint(), p) <= COVER_REACH_M)
        .any(|q| crosses(&q.footprint(), p, reach))
}

/// Per-check state gathered while the battle runs.
struct Judge {
    worst: f64,
    at: String,
    /// Distance to the goal when the unit first halted.
    halted_short: Option<f64>,
    prop: Option<(u32, V2)>,
}

impl Judge {
    fn new(kind: &CheckKind, b: &Battle) -> Self {
        let prop = match kind {
            CheckKind::PropMoved { near, .. } => {
                let near = v2(near[0], near[1]);
                b.world()
                    .props()
                    .filter(|p| solid(p))
                    .min_by(|x, y| {
                        (x.center - near)
                            .length()
                            .total_cmp(&(y.center - near).length())
                    })
                    .map(|p| (p.id, p.center))
            }
            _ => None,
        };
        let mut j = Judge {
            worst: f64::INFINITY,
            at: String::new(),
            halted_short: None,
            prop,
        };
        j.watch(kind, b);
        j
    }

    /// Track the smallest margin seen: negative is a violation.
    fn note(&mut self, margin: f64, b: &Battle, what: impl FnOnce() -> String) {
        if margin < self.worst {
            self.worst = margin;
            self.at = format!("{} at tick {}", what(), b.tick());
        }
    }

    fn watch(&mut self, kind: &CheckKind, b: &Battle) {
        match kind {
            CheckKind::SoldiersClearOfProps => {
                for u in units(b).filter(|u| !u.is_vehicle() && !u.garrisoned()) {
                    for p in u.member_positions() {
                        for q in b.world().props_near(p.xy(), 12.0) {
                            if solid(q) {
                                let m = distance_to_box(&q.footprint(), p.xy()) - SOLDIER_RADIUS_M;
                                self.note(m, b, || {
                                    format!("unit {} soldier in {:?} {}", u.id.0, q.kind, q.id)
                                });
                            }
                        }
                    }
                }
            }
            CheckKind::VehiclesClearOfProps => {
                for u in units(b).filter(|u| u.alive()) {
                    let Some(hull) = u.hull_box() else { continue };
                    for q in b.world().props_near(hull.center, 12.0) {
                        if q.kind.blocks(contract::map::MoverClass::Vehicle)
                            && hull.overlaps(&q.footprint())
                        {
                            self.note(-1.0, b, || {
                                format!("unit {} hull in {:?} {}", u.id.0, q.kind, q.id)
                            });
                        }
                    }
                }
            }
            CheckKind::SquadsNeverOverlap { min_m } => {
                let squads: Vec<_> = units(b).filter(|u| !u.is_vehicle()).collect();
                for (i, a) in squads.iter().enumerate() {
                    for c in &squads[i + 1..] {
                        for p in a.member_positions() {
                            for q in c.member_positions() {
                                let m = (p.xy() - q.xy()).length() - min_m;
                                self.note(m, b, || format!("units {} and {}", a.id.0, c.id.0));
                            }
                        }
                    }
                }
            }
            CheckKind::OnGround { within_m } => {
                for u in units(b).filter(|u| !u.is_vehicle() && !u.garrisoned()) {
                    for p in u.member_positions() {
                        if let Some(ground) = b.world().height_at(p.x, p.y) {
                            let m = within_m - (p.z - ground).abs();
                            self.note(m, b, || format!("unit {} soldier off the ground", u.id.0));
                        }
                    }
                }
            }
            CheckKind::HaltsShort { unit, goal, .. } => {
                let u = b.unit(UnitId(*unit)).unwrap();
                if u.state == MoveState::Halted && self.halted_short.is_none() {
                    self.halted_short = Some((u.position.xy() - v2(goal[0], goal[1])).length());
                }
            }
            _ => {}
        }
    }

    fn verdict(self, c: &Check, b: &Battle, ground: &GroundRules) -> Outcome {
        let (label, passed, detail) = match &c.kind {
            CheckKind::Arrive { unit, at, within_m } => {
                let u = b.unit(UnitId(*unit)).unwrap();
                let d = (u.position.xy() - v2(at[0], at[1])).length();
                (
                    format!("unit {unit} arrives"),
                    u.state == MoveState::Idle && d <= *within_m,
                    format!("{:?}, {d:.2} m from the goal", u.state),
                )
            }
            CheckKind::Spacing { min_m } => {
                let mut worst = f64::INFINITY;
                for u in units(b).filter(|u| !u.is_vehicle()) {
                    let ps: Vec<_> = u.member_positions().collect();
                    for (i, p) in ps.iter().enumerate() {
                        for q in &ps[i + 1..] {
                            worst = worst.min((p.xy() - q.xy()).length());
                        }
                    }
                }
                (
                    format!("soldiers at least {min_m} m apart"),
                    worst >= *min_m,
                    format!("closest pair {worst:.2} m"),
                )
            }
            CheckKind::InCover { unit, threat, min } => {
                let u = b.unit(UnitId(*unit)).unwrap();
                let t = b.unit(UnitId(*threat)).unwrap().position.xy();
                let n = u
                    .member_positions()
                    .filter(|p| covered(b, ground, p.xy(), t))
                    .count();
                (
                    format!("at least {min} of unit {unit} in cover"),
                    n >= *min,
                    format!("{n} in cover"),
                )
            }
            CheckKind::HaltsShort { unit, min_m, .. } => {
                let u = b.unit(UnitId(*unit)).unwrap();
                (
                    format!("unit {unit} halts at least {min_m} m short and holds"),
                    self.halted_short.is_some_and(|d| d >= *min_m) && u.state == MoveState::Halted,
                    format!(
                        "first halt {} m short, {:?} at the end",
                        self.halted_short
                            .map_or("never".into(), |d| format!("{d:.1}")),
                        u.state
                    ),
                )
            }
            CheckKind::PropMoved { min_m, .. } => {
                let (id, start) = self.prop.expect("a prop near the point");
                let moved = b
                    .world()
                    .prop(id)
                    .map_or(0.0, |p| (p.center - start).length());
                (
                    format!("prop {id} moved at least {min_m} m"),
                    moved >= *min_m,
                    format!("moved {moved:.2} m"),
                )
            }
            kind => {
                let label = match kind {
                    CheckKind::SoldiersClearOfProps => "no soldier inside a body".into(),
                    CheckKind::VehiclesClearOfProps => "no hull inside a blocking body".into(),
                    CheckKind::SquadsNeverOverlap { min_m } => {
                        format!("squads' soldiers never within {min_m} m")
                    }
                    CheckKind::OnGround { within_m } => {
                        format!("soldiers within {within_m} m of the ground")
                    }
                    _ => unreachable!(),
                };
                let passed = self.worst >= 0.0;
                let detail = if passed {
                    format!("worst margin {:.2} m", self.worst)
                } else {
                    format!("worst {:.2} m: {}", self.worst, self.at)
                };
                (label, passed, detail)
            }
        };
        Outcome {
            label,
            pending: c.pending,
            passed,
            detail,
        }
    }
}

// --- the tests: one per scenario -------------------------------------------------

#[test]
fn t0_a_squad_crosses_open_ground() {
    assert_scenario("t0-open-ground");
}

#[test]
fn t0_a_squad_passes_a_wreck() {
    assert_scenario("t0-around-wreck");
}

#[test]
fn t0_a_tank_drives_around_a_wreck() {
    assert_scenario("t0-tank-around-wreck");
}

#[test]
fn t0_a_squad_crosses_a_wall_through_a_gap() {
    assert_scenario("t0-through-gap");
}

#[test]
fn t1_a_squad_moves_into_cover() {
    assert_scenario("t1-into-cover");
}

#[test]
fn t1_too_little_cover_leaves_the_rest_scattered() {
    assert_scenario("t1-not-enough-cover");
}

#[test]
fn t1_an_attack_move_halts_on_contact() {
    assert_scenario("t1-attack-move-halt");
}

#[test]
fn t2_two_squads_cross_without_jamming() {
    assert_scenario("t2-two-squads-crossing");
}

#[test]
fn t2_two_squads_share_one_door() {
    assert_scenario("t2-door-jam");
}

#[test]
fn t2_a_squad_threads_a_one_man_gap() {
    assert_scenario("t2-one-man-gap");
}

#[test]
fn t2_a_squad_crosses_a_ridge() {
    assert_scenario("t2-slope");
}

#[test]
fn t3_a_tank_meets_a_wreck_on_a_road() {
    assert_scenario("t3-tank-pushes-wreck");
}

#[test]
fn t3_a_light_vehicle_drives_round_a_fence() {
    assert_scenario("t3-jeep-blocked-by-fence");
}

#[test]
fn every_scenario_has_a_test() {
    let src = include_str!("movement_scenarios.rs");
    for s in scenarios() {
        assert!(
            src.contains(&format!("assert_scenario(\"{}\")", s.name)),
            "{} has no test",
            s.name
        );
    }
}
