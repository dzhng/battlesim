//! Movement scenarios as data: a small map, units, scripted orders and a
//! duration, each with the outcomes it must show. The tests below assert
//! them; `examples/movement_shots.rs` replays the same table through the
//! same [`run`] and draws it (`cargo run -p sim --release --example
//! movement_shots`).
//!
//! A check whose rule has not landed yet carries `pending: Some("<slice>: why")`.
//! It is still evaluated and reported, never asserted; the slice that lands
//! the rule deletes the reason.
#![allow(dead_code)]

use contract::ids::{Side, UnitId};
use contract::map::MoverClass;
use contract::observation::MoveState;
use contract::scenario::ScenarioDefinition;
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
    /// Its `catalog` holds patches by section and id, merged into the unit
    /// catalog's entries (`sim::fixtures::patch_catalog`).
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
    /// Unit ends idle within `within_m` of `at`: a vehicle's centre, a
    /// squad's anchor (27d: its soldiers spread over the area round it).
    Arrive {
        unit: u32,
        at: [f64; 2],
        within_m: f64,
    },
    /// No living soldier's disc ever overlaps a solid body (every prop that
    /// blocks infantry: all but the ground kinds, Q27).
    SoldiersClearOfProps,
    /// No vehicle hull ever overlaps a prop that blocks vehicles.
    VehiclesClearOfProps,
    /// At the end, every squad's soldiers stand at least `min_m` apart.
    Spacing { min_m: f64 },
    /// Soldiers of different squads never come closer than `min_m`.
    SquadsNeverOverlap { min_m: f64 },
    /// Every living soldier stands within `within_m` of the ground under him.
    OnGround { within_m: f64 },
    /// At the end, at least `min` soldiers of `unit` have cover against
    /// `threat`'s position (Q20): on the far side of a body from it.
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
    /// The prop nearest `near` at the start never moves: nothing here can shove it.
    PropStays { near: [f64; 2] },
    /// No living soldier's disc ever overlaps a live vehicle's hull (Q23).
    SoldiersClearOfHulls,
    /// Every soldier ends with the health he started with.
    NobodyHurt,
    /// No two live hulls ever overlap, whatever their sides (Q14).
    VehiclesNeverOverlap,
    /// `unit` waits for another vehicle at some point (the wait before a detour).
    Waits { unit: u32 },
    /// At the end, at least `min` soldiers of `unit` are each covered
    /// against `threat` by a body no squadmate shares: one tooth each.
    OneBodyEach { unit: u32, threat: u32, min: usize },
    /// At the end, at least `min` soldiers of `unit` have a clear straight
    /// line from their muzzle to one of `threat`'s soldiers (D3: blocked ones
    /// step out).
    ClearLines { unit: u32, threat: u32, min: usize },
    /// `unit` has turned at least `min_deg` before its centre moves half a
    /// metre: it pivots on the spot (Q29, tracks).
    PivotsInPlace { unit: u32, min_deg: f64 },
    /// `unit` never turns tighter than its turning radius, nor standing
    /// still (Q29, wheels).
    WithinRadius { unit: u32 },
    /// `unit` drives backwards at some point.
    Reverses { unit: u32 },
    /// `unit` never drives backwards.
    NeverReverses { unit: u32 },
    /// `unit` keeps the facing it starts with, within a degree (Q31).
    FacingHeld { unit: u32 },
    /// At the end, `unit` is at rest facing `deg` (a right-drag's facing, Q9).
    EndsFacing { unit: u32, deg: f64 },
    /// `first` arrives before `second`, and `second` arrives.
    ArrivesFirst { first: u32, second: u32 },
    /// At the end, at least `min` trees fewer stand than at the start (Q16).
    KnocksTrees { min: usize },
    /// No tree ever falls.
    TreesStand,
    /// At the end, at least `min_share` of `rect`'s metre cells (`[x0, y0,
    /// x1, y1]`) are open ground: neither forest ground nor under foliage.
    OpenGround { rect: [f64; 4], min_share: f64 },
    /// The solid prop nearest `near` at the start ends destroyed, a prop of
    /// `into` standing in its place (Q17).
    PropBecomes { near: [f64; 2], into: &'static str },
    /// While inside `rect` (`[x0, y0, x1, y1]`), `unit` averages at least `min_mps`.
    FastThrough {
        unit: u32,
        rect: [f64; 4],
        min_mps: f64,
    },
    /// `side` identifies `unit` on some tick while it is inside `rect`.
    SpottedIn {
        unit: u32,
        side: Side,
        rect: [f64; 4],
    },
    /// `observer`'s side first identifies `first` at least `ratio` times
    /// farther from `observer` than it first identifies `then`.
    SpottedFarther {
        first: u32,
        then: u32,
        observer: u32,
        ratio: f64,
    },
    /// Soldiers of `unit` lean out round tall cover to fire (27d): at least
    /// `min` of them lean out on some tick, and none ever leans from inside
    /// a body.
    Leans { unit: u32, min: usize },
    /// No soldier twitches while he has somewhere to go (his spot or post
    /// more than half a metre off, set off, his squad neither waiting nor
    /// halted by an attack-move): at most
    /// `max_reversals` reversals of his step (a step more than 135° off his
    /// last one), and at most `max_still_s` without leaving a metre's circle.
    /// Every scenario carries it (`scenarios`).
    NoTwitch {
        max_reversals: u32,
        max_still_s: f64,
    },
}

/// The no-twitch bound every scenario gets. A soldier twitching past a
/// corridor corner reversed 490 times in 40 s and stood 17 s within a metre
/// (slice 37b); a clean walk reverses a handful of times, jostling in a crowd.
const NO_TWITCH: CheckKind = CheckKind::NoTwitch {
    max_reversals: 20,
    max_still_s: 8.0,
};

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

/// A wreck of `kind` (`tank_wreck`, `supply_wreck`, `jeep_wreck`) the size
/// of its vehicle's hull.
fn wreck(kind: &str, center: [f64; 2], yaw: f64) -> Value {
    let half = match kind {
        "tank_wreck" => [3.5, 1.8, 1.2],
        "supply_wreck" => [3.0, 1.4, 1.8],
        _ => [2.2, 1.0, 0.95],
    };
    json!({ "kind": kind, "center": center, "yaw": yaw, "half_extents": half })
}

fn prop(kind: &str, center: [f64; 2], yaw: f64, half: [f64; 3]) -> Value {
    json!({ "kind": kind, "center": center, "yaw": yaw, "half_extents": half })
}

/// Dragon's teeth: 1.2 m anti-tank teeth at x, every 2.4 m from `y0` up to
/// `y1`, each gap a one-man opening (Q18).
fn teeth_at(x: f64, y0: f64, y1: f64) -> Vec<Value> {
    let n = ((y1 - y0) / 2.4).floor() as usize + 1;
    (0..n)
        .map(|k| prop("tooth", [x, y0 + 2.4 * k as f64], 0.0, [0.6, 0.6, 0.6]))
        .collect()
}

fn rifle(side: &str, at: [f64; 2]) -> Value {
    json!({ "side": side, "kind": "rifle", "position": at, "engagement": "return_fire_only" })
}

fn vehicle(side: &str, kind: &str, at: [f64; 2], yaw: f64) -> Value {
    json!({ "side": side, "kind": kind, "position": at, "yaw": yaw, "engagement": "return_fire_only" })
}

fn go(unit: u32, goal: [f64; 2]) -> Value {
    order(
        json!({ "kind": "move", "units": [unit], "gesture": unit + 1, "goal": goal, "route": "shortest" }),
    )
}

/// `go` on the fastest route: a vehicle keeps to the road.
fn drive(side: &str, unit: u32, goal: [f64; 2]) -> Value {
    json!({ "tick": 1, "side": side, "order":
        { "kind": "move", "units": [unit], "gesture": unit + 1, "goal": goal, "route": "fastest" } })
}

/// `go` backwards: a reverse move, facing held (Q31).
fn back(unit: u32, goal: [f64; 2]) -> Value {
    order(
        json!({ "kind": "move", "units": [unit], "gesture": unit + 1, "goal": goal,
        "route": "shortest", "direction": "reverse" }),
    )
}

/// `go` with a right-drag's facing (Q9), in degrees.
fn go_facing(unit: u32, goal: [f64; 2], deg: f64) -> Value {
    order(
        json!({ "kind": "move", "units": [unit], "gesture": unit + 1, "goal": goal,
        "route": "shortest", "facing": deg.to_radians() }),
    )
}

/// A forest of `density` over `rect` (`[x, y, w, h]`), as the maps author one.
fn forest(rect: [f64; 4], density: &str) -> Value {
    json!({ "rect": rect, "density": density, "canopy_height_m": 12, "trunk_radius_m": 0.35,
            "trunk_height_m": 10, "trunk_clearance_m": 2 })
}

/// The village fixture's own map (its ground, roads and forests) with only
/// its props whose centre lies inside `window` (`[x0, y0, x1, y1]`), so the
/// drawing frames that corner of the encounter.
fn village(window: [f64; 4]) -> Value {
    let fixture: Value =
        serde_json::from_str(include_str!("../../../fixtures/village.json")).unwrap();
    let mut map = fixture["map"].clone();
    let props: Vec<Value> = map["props"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| {
            let c = &p["center"];
            inside(&window, v2(c[0].as_f64().unwrap(), c[1].as_f64().unwrap()))
        })
        .cloned()
        .collect();
    map["props"] = Value::Array(props);
    map
}

/// A barrage: `n` × `n` HE bursts `step` metres apart about `center`, in
/// each wave at `ticks`.
fn barrage(center: [f64; 2], n: usize, step: f64, ticks: &[u64]) -> Vec<Value> {
    let half = (n as f64 - 1.0) / 2.0;
    let mut out = Vec::new();
    for &tick in ticks {
        for i in 0..n {
            for j in 0..n {
                let at = [
                    center[0] + (i as f64 - half) * step,
                    center[1] + (j as f64 - half) * step,
                ];
                out.push(json!({ "tick": tick, "burst": { "point": at, "weapon": "tank_he" } }));
            }
        }
    }
    out
}

fn order(order: Value) -> Value {
    json!({ "tick": 1, "side": "blue", "order": order })
}

/// The movement lane's scenario table, simple to hard (t0–t3).
pub fn scenarios() -> Vec<Scenario> {
    let mut all = authored();
    for s in &mut all {
        s.checks.push(check(NO_TWITCH));
    }
    all
}

fn authored() -> Vec<Scenario> {
    use CheckKind::*;
    let none = json!([]);
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
                json!({ "props": [wreck("tank_wreck", [57.0, 40.0], 0.3)] }),
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
                check(SoldiersClearOfProps),
            ],
        },
        Scenario {
            name: "t0-tank-around-wreck",
            caption: "a tank drives around a tank wreck in its path",
            map: flat(
                [120.0, 80.0],
                json!({ "props": [wreck("tank_wreck", [57.0, 40.0], 0.3)] }),
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
            caption: "move beside sandbags and crates, enemy north-east: take cover facing it",
            map: flat(
                [120.0, 90.0],
                json!({ "props": [
                    prop("sandbags", [73.0, 38.0], 0.0, [3.0, 0.4, 0.5]),
                    prop("sandbags", [79.0, 43.0], 0.0, [0.4, 2.5, 0.5]),
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
                // The long wall holds four at 2 m, a crate one; the short
                // wall and the far crate lie beyond 8 m of the rest (Q7).
                check(InCover {
                    unit: 0,
                    threat: 1,
                    min: 5,
                }),
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
                check(InCover {
                    unit: 0,
                    threat: 1,
                    min: 2,
                }),
                check(Spacing { min_m: 2.0 }),
                check(SoldiersClearOfProps),
            ],
        },
        Scenario {
            name: "t1-attack-move-halt",
            caption: "attack-move at an enemy squad: halt on contact, into the crates' cover",
            map: flat(
                [140.0, 100.0],
                json!({ "props": [
                    crate_at([75.0, 57.0]), crate_at([77.0, 43.0]), crate_at([73.0, 60.0]),
                    // A field wall on the line where the squad halts.
                    wall([84.0, 50.0], 0.0, [0.4, 3.0, 0.5]),
                ] }),
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
            rules: json!({ "catalog": { "units": { "squad": { "sensors": { "ground_m": 40 } } }, "soldiers": { "rifleman": { "hp": 1.0e6 } } } }),
            seconds: 40.0,
            seed: 1,
            checks: vec![
                check(HaltsShort {
                    unit: 0,
                    goal: [120.0, 50.0],
                    min_m: 25.0,
                }),
                check(InCover {
                    unit: 0,
                    threat: 1,
                    min: 3,
                }),
            ],
        },
        Scenario {
            name: "t1-cover-destroyed",
            caption: "a squad at rest in a firefight behind a wall; the wall is shelled to rubble at 10 s: they re-cover",
            map: flat(
                [140.0, 90.0],
                json!({ "props": [
                    wall([64.0, 45.0], 0.0, [0.4, 5.0, 0.6]),
                    crate_at([57.0, 38.0]), crate_at([57.0, 52.0]), crate_at([59.0, 45.0]),
                ] }),
            ),
            units: json!([
                { "side": "blue", "kind": "rifle", "position": [58, 45] },
                { "side": "red", "kind": "rifle", "position": [110, 45] },
            ]),
            // Three ATGM bursts on its middle bring the wall down to rubble.
            events: json!([{ "tick": 300, "burst": { "point": [64.0, 45.0], "weapon": "atgm" } }, { "tick": 300, "burst": { "point": [64.0, 45.0], "weapon": "atgm" } }, { "tick": 300, "burst": { "point": [64.0, 45.0], "weapon": "atgm" } }]),
            scripts: none.clone(),
            // Soldiers too tough to fall, so the fight lasts.
            rules: json!({ "catalog": { "soldiers": { "rifleman": { "hp": 1.0e6 } } } }),
            seconds: 30.0,
            seed: 1,
            checks: vec![
                check(InCover {
                    unit: 0,
                    threat: 1,
                    min: 3,
                }),
                check(SoldiersClearOfProps),
            ],
        },
        Scenario {
            name: "t1-cover-shot-away",
            caption: "a squad behind chest-high sandbags under a jeep's HMG: the fire grinds the sandbags to rubble; the squad holds the rubble strip (light cover) and the teeth (27c)",
            map: flat(
                [140.0, 90.0],
                json!({ "props": [
                    prop("sandbags", [64.0, 45.0], 0.0, [0.4, 5.0, 0.6]),
                    prop("tooth", [55.0, 36.0], 0.0, [0.6, 0.6, 0.6]),
                    prop("tooth", [55.0, 54.0], 0.0, [0.6, 0.6, 0.6]),
                    prop("tooth", [52.0, 40.0], 0.0, [0.6, 0.6, 0.6]),
                    prop("tooth", [52.0, 50.0], 0.0, [0.6, 0.6, 0.6]),
                ] }),
            ),
            units: json!([
                { "side": "blue", "kind": "rifle", "position": [60, 45] },
                { "side": "red", "kind": "jeep", "position": [110, 45], "yaw": std::f64::consts::PI },
            ]),
            events: none.clone(),
            scripts: none.clone(),
            // Soldiers and the jeep too tough to fall, so the fight lasts: the
            // squad firing back past its cover would kill the jeep otherwise.
            rules: json!({ "catalog": { "units": { "jeep": { "body": { "hull": { "hp": 1.0e6 } } } }, "soldiers": { "rifleman": { "hp": 1.0e6 } } } }),
            seconds: 60.0,
            seed: 1,
            checks: vec![
                check(PropBecomes {
                    near: [64.0, 45.0],
                    into: "rubble",
                }),
                check(InCover {
                    unit: 0,
                    threat: 1,
                    min: 3,
                }),
                check(SoldiersClearOfProps),
            ],
        },
        Scenario {
            name: "t1-behind-parked-tank",
            caption: "a squad sent beside a parked tank, enemy beyond it: they take cover behind the hull",
            map: flat([140.0, 80.0], json!({})),
            units: json!([
                { "side": "blue", "kind": "tank", "position": [64, 40], "yaw": std::f64::consts::FRAC_PI_2, "engagement": "return_fire_only" },
                rifle("blue", [20.0, 40.0]),
                rifle("red", [115.0, 42.0]),
            ]),
            events: none.clone(),
            scripts: json!([go(1, [59.0, 40.0])]),
            rules: json!({}),
            seconds: 30.0,
            seed: 1,
            checks: vec![
                check(InCover {
                    unit: 1,
                    threat: 2,
                    min: 4,
                }),
                check(SoldiersClearOfHulls),
            ],
        },
        Scenario {
            name: "t1-step-out-corner",
            caption: "at rest behind a tall wall in a firefight: soldiers without a line step out round its end",
            map: flat(
                [140.0, 100.0],
                json!({ "props": [wall([62.0, 45.0], 0.0, [0.4, 5.0, 1.5])] }),
            ),
            units: json!([
                { "side": "blue", "kind": "rifle", "position": [57, 47] },
                { "side": "red", "kind": "rifle", "position": [100, 72] },
            ]),
            events: none.clone(),
            scripts: none.clone(),
            rules: json!({ "catalog": { "soldiers": { "rifleman": { "hp": 1.0e6 } } } }),
            seconds: 30.0,
            seed: 1,
            checks: vec![
                // The men near an end step round it; the one who spawned
                // deepest behind the wall has no clear place within 4 m
                // and sits out (D3).
                check(ClearLines {
                    unit: 0,
                    threat: 1,
                    min: 7,
                }),
                check(SoldiersClearOfProps),
            ],
        },
        Scenario {
            name: "t1-wood-lean-out",
            caption: "a squad at rest in a wood trades fire with a squad in the open: men lean out from their trees, fire and tuck back (27d)",
            map: flat(
                [140.0, 90.0],
                json!({ "props": (0..16)
                    .map(|k| {
                        let (i, j) = ((k % 4) as f64, (k / 4) as f64);
                        let x = 52.0 + 4.0 * i + if k / 4 % 2 == 1 { 2.0 } else { 0.0 };
                        prop("trunk", [x, 38.0 + 4.0 * j], 0.0, [0.35, 0.35, 6.0])
                    })
                    .collect::<Vec<_>>() }),
            ),
            units: json!([
                { "side": "blue", "kind": "rifle", "position": [58, 44] },
                { "side": "red", "kind": "rifle", "position": [108, 46] },
            ]),
            events: none.clone(),
            scripts: none.clone(),
            rules: json!({ "catalog": { "soldiers": { "rifleman": { "hp": 1.0e6 } } } }),
            seconds: 30.0,
            seed: 1,
            checks: vec![
                check(Leans { unit: 0, min: 2 }),
                check(ClearLines {
                    unit: 0,
                    threat: 1,
                    min: 7,
                }),
                check(SoldiersClearOfProps),
            ],
        },
        Scenario {
            name: "t1-building-corner-lean-out",
            caption: "a squad at rest beside a house, the enemy beyond its corner: the man at each corner leans out; the rest find a line (27d)",
            map: flat(
                [140.0, 110.0],
                json!({ "props": [prop("building", [60.0, 40.0], 0.0, [6.0, 6.0, 4.0])] }),
            ),
            units: json!([
                { "side": "blue", "kind": "rifle", "position": [49, 42] },
                { "side": "red", "kind": "rifle", "position": [95, 85] },
            ]),
            events: none.clone(),
            scripts: none.clone(),
            rules: json!({ "catalog": { "soldiers": { "rifleman": { "hp": 1.0e6 } } } }),
            seconds: 30.0,
            seed: 1,
            checks: vec![
                check(Leans { unit: 0, min: 1 }),
                check(ClearLines {
                    unit: 0,
                    threat: 1,
                    min: 7,
                }),
                check(SoldiersClearOfProps),
            ],
        },
        Scenario {
            name: "t1-parked-tank-lean-out",
            caption: "a squad at rest behind its parked tank trades fire with a squad beyond it: men lean out past the hull's ends (27d)",
            map: flat([140.0, 90.0], json!({})),
            units: json!([
                { "side": "blue", "kind": "rifle", "position": [59, 45] },
                { "side": "red", "kind": "rifle", "position": [110, 45] },
                vehicle("blue", "tank", [64.0, 45.0], std::f64::consts::FRAC_PI_2),
            ]),
            events: none.clone(),
            scripts: none.clone(),
            rules: json!({ "catalog": { "soldiers": { "rifleman": { "hp": 1.0e6 } } } }),
            seconds: 30.0,
            seed: 1,
            checks: vec![
                check(Leans { unit: 0, min: 1 }),
                check(ClearLines {
                    unit: 0,
                    threat: 1,
                    min: 7,
                }),
                check(SoldiersClearOfHulls),
            ],
        },
        Scenario {
            name: "t1-corner-three",
            caption: "three men at a house's corner, the enemy beyond it: one leans out at the corner, the two stacked behind him step out to fire (27d)",
            map: flat(
                [140.0, 110.0],
                json!({ "props": [prop("building", [60.0, 40.0], 0.0, [6.0, 6.0, 4.0])] }),
            ),
            units: json!([
                { "side": "blue", "kind": "rifle", "position": [51.5, 45.0] },
                { "side": "red", "kind": "rifle", "position": [95, 85] },
            ]),
            events: none.clone(),
            scripts: none.clone(),
            rules: json!({ "catalog": { "units": { "rifle": { "body": { "squad": { "slots": ["grenadier", "rifleman", "rifleman"] } } } }, "soldiers": { "rifleman": { "hp": 1.0e6 } } } }),
            seconds: 30.0,
            seed: 1,
            checks: vec![
                check(Leans { unit: 0, min: 1 }),
                // All three end able to engage.
                check(ClearLines {
                    unit: 0,
                    threat: 1,
                    min: 3,
                }),
                check(SoldiersClearOfProps),
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
                check(SquadsNeverOverlap {
                    min_m: 2.0 * SOLDIER_RADIUS_M,
                }),
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
                check(SquadsNeverOverlap {
                    min_m: 2.0 * SOLDIER_RADIUS_M,
                }),
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
                check(Arrive {
                    unit: 0,
                    at: [100.0, 40.0],
                    within_m: 1.5,
                }),
                check(SoldiersClearOfProps),
            ],
        },
        Scenario {
            name: "t2-line-of-teeth",
            caption: "a squad threads a line of dragon's teeth, 1.2 m apart, one man at a time",
            map: flat([120.0, 80.0], json!({ "props": teeth_at(60.0, 1.0, 79.0) })),
            units: json!([rifle("blue", [20.0, 40.0])]),
            events: none.clone(),
            scripts: json!([go(0, [100.0, 40.0])]),
            rules: json!({}),
            seconds: 45.0,
            seed: 1,
            checks: vec![
                check(Arrive {
                    unit: 0,
                    at: [100.0, 40.0],
                    within_m: 1.5,
                }),
                check(SoldiersClearOfProps),
            ],
        },
        Scenario {
            name: "t2-around-crate-stack",
            caption: "a crate stack stands across the squad's lanes: they walk round it",
            map: flat(
                [120.0, 80.0],
                json!({ "props": [
                    crate_at([56.0, 38.4]), crate_at([56.0, 40.0]), crate_at([56.0, 41.6]),
                    crate_at([57.6, 38.4]), crate_at([57.6, 40.0]), crate_at([57.6, 41.6]),
                    crate_at([80.0, 44.0]), crate_at([81.6, 44.0]), crate_at([80.0, 45.6]),
                ] }),
            ),
            units: json!([rifle("blue", [20.0, 40.0])]),
            events: none.clone(),
            scripts: json!([go(0, [84.0, 40.0])]),
            rules: json!({}),
            seconds: 36.0,
            seed: 1,
            checks: vec![
                check(Arrive {
                    unit: 0,
                    at: [84.0, 40.0],
                    within_m: 1.5,
                }),
                check(SoldiersClearOfProps),
            ],
        },
        Scenario {
            name: "t2-crate-dropped-in-lane",
            caption:
                "a sandbag line appears across the squad's way mid-walk: they replan round it",
            map: flat([120.0, 80.0], json!({})),
            units: json!([rifle("blue", [15.0, 40.0])]),
            events: json!([
                { "tick": 150, "add_prop": { "kind": "sandbags", "center": [52.0, 40.0], "yaw": 0, "half_extents": [0.8, 6.0, 0.6] } },
            ]),
            scripts: json!([go(0, [95.0, 40.0])]),
            rules: json!({}),
            seconds: 40.0,
            seed: 1,
            checks: vec![
                check(Arrive {
                    unit: 0,
                    at: [95.0, 40.0],
                    within_m: 1.5,
                }),
                check(SoldiersClearOfProps),
            ],
        },
        Scenario {
            name: "t2-around-parked-tank",
            caption: "a parked tank stands on the squad's corridor: they walk round it",
            map: flat([140.0, 60.0], json!({})),
            units: json!([
                { "side": "blue", "kind": "tank", "position": [70, 30], "yaw": std::f64::consts::FRAC_PI_2, "engagement": "return_fire_only" },
                rifle("blue", [30.0, 30.0]),
            ]),
            events: none.clone(),
            scripts: json!([go(1, [110.0, 30.0])]),
            rules: json!({}),
            seconds: 36.0,
            seed: 1,
            checks: vec![
                check(Arrive {
                    unit: 1,
                    at: [110.0, 30.0],
                    within_m: 1.5,
                }),
                check(SoldiersClearOfHulls),
            ],
        },
        Scenario {
            name: "t3-tank-through-resting-squad",
            caption: "a tank drives through a resting squad: they step aside, it never stops",
            map: flat([140.0, 70.0], json!({})),
            units: json!([
                rifle("blue", [70.0, 35.0]),
                { "side": "blue", "kind": "tank", "position": [15, 36], "engagement": "return_fire_only" },
            ]),
            events: none.clone(),
            scripts: json!([go(1, [125.0, 35.0])]),
            rules: json!({}),
            seconds: 22.0,
            seed: 1,
            checks: vec![
                check(Arrive {
                    unit: 1,
                    at: [125.0, 35.0],
                    within_m: 1.5,
                }),
                check(SoldiersClearOfHulls),
                check(NobodyHurt),
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
            seconds: 60.0,
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
            caption: "a tank meets a jeep wreck across a road and shoves it aside",
            map: flat(
                [140.0, 60.0],
                json!({
                    "roads": [{ "points": [[0, 30], [140, 30]], "width_m": 10 }],
                    "props": [wreck("jeep_wreck", [70.0, 31.0], 1.4)],
                }),
            ),
            units: json!([vehicle("blue", "tank", [10.0, 30.0], 0.0)]),
            events: none.clone(),
            scripts: json!([drive("blue", 0, [130.0, 30.0])]),
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
                check(PropMoved {
                    near: [70.0, 30.0],
                    min_m: 2.0,
                }),
            ],
        },
        Scenario {
            name: "t3-jeep-blocked-by-fence",
            caption: "a jeep meets a fence it cannot push and drives round it",
            map: flat(
                [120.0, 70.0],
                json!({ "props": [prop("fence", [60.0, 30.0], 0.0, [0.1, 20.0, 0.6])] }),
            ),
            units: json!([vehicle("blue", "jeep", [20.0, 30.0], 0.0)]),
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
                check(PropStays { near: [60.0, 30.0] }),
            ],
        },
        Scenario {
            name: "t3-truck-pushes-crate",
            caption: "a supply truck meets a crate in a walled lane and shoves it along",
            map: flat(
                [140.0, 60.0],
                json!({ "props": [
                    prop("wall", [60.0, 25.2], 0.0, [20.0, 0.4, 1.5]),
                    prop("wall", [60.0, 34.8], 0.0, [20.0, 0.4, 1.5]),
                    prop("crate", [55.0, 30.0], 0.0, [0.8, 0.8, 0.6]),
                ] }),
            ),
            units: json!([vehicle("blue", "supply", [15.0, 30.0], 0.0)]),
            events: none.clone(),
            scripts: json!([go(0, [125.0, 30.0])]),
            rules: json!({}),
            seconds: 30.0,
            seed: 1,
            checks: vec![
                check(Arrive {
                    unit: 0,
                    at: [125.0, 30.0],
                    within_m: 1.5,
                }),
                check(VehiclesClearOfProps),
                check(PropMoved {
                    near: [55.0, 30.0],
                    min_m: 2.0,
                }),
            ],
        },
        Scenario {
            name: "t3-infantry-cross-teeth-tank-goes-round",
            caption: "dragon's teeth: the squad threads the gaps, the tank is stopped and routes round the end",
            map: flat([120.0, 90.0], json!({ "props": teeth_at(60.0, 1.0, 66.0) })),
            units: json!([
                rifle("blue", [20.0, 35.0]),
                vehicle("blue", "tank", [20.0, 50.0], 0.0),
            ]),
            events: none.clone(),
            scripts: json!([go(0, [100.0, 35.0]), go(1, [100.0, 50.0])]),
            rules: json!({}),
            seconds: 45.0,
            seed: 1,
            checks: vec![
                check(Arrive {
                    unit: 0,
                    at: [100.0, 35.0],
                    within_m: 1.5,
                }),
                check(Arrive {
                    unit: 1,
                    at: [100.0, 50.0],
                    within_m: 1.5,
                }),
                check(SoldiersClearOfProps),
                check(VehiclesClearOfProps),
                check(PropStays { near: [60.0, 49.0] }),
            ],
        },
        Scenario {
            name: "t1-one-tooth-each",
            caption: "a squad sent to dragon's teeth, enemy beyond: one man behind each tooth",
            map: flat([120.0, 80.0], json!({ "props": teeth_at(60.0, 20.0, 60.0) })),
            units: json!([rifle("blue", [25.0, 40.0]), rifle("red", [100.0, 42.0])]),
            events: none.clone(),
            scripts: json!([go(0, [57.0, 40.0])]),
            rules: json!({}),
            seconds: 30.0,
            seed: 1,
            checks: vec![
                check(SoldiersClearOfProps),
                check(OneBodyEach {
                    unit: 0,
                    threat: 1,
                    min: 6,
                }),
            ],
        },
        Scenario {
            name: "t3-tanks-meet-head-on",
            caption: "a blue and a red tank meet head-on on a road: one waits, one detours, neither overlaps",
            map: flat(
                [160.0, 60.0],
                json!({ "roads": [{ "points": [[0, 30], [160, 30]], "width_m": 10 }] }),
            ),
            units: json!([
                vehicle("blue", "tank", [15.0, 30.0], 0.0),
                vehicle("red", "tank", [145.0, 30.0], std::f64::consts::PI),
            ]),
            events: none.clone(),
            scripts: json!([drive("blue", 0, [145.0, 30.0]), drive("red", 1, [15.0, 30.0])]),
            rules: json!({}),
            seconds: 40.0,
            seed: 1,
            checks: vec![
                check(Arrive {
                    unit: 0,
                    at: [145.0, 30.0],
                    within_m: 1.5,
                }),
                check(Arrive {
                    unit: 1,
                    at: [15.0, 30.0],
                    within_m: 1.5,
                }),
                check(VehiclesNeverOverlap),
                check(Waits { unit: 1 }),
            ],
        },
        Scenario {
            name: "t3-column-through-wreck-field",
            caption: "three tanks cross a field of wrecks that appears ahead: each learns them and drives round, touching nothing",
            map: flat([160.0, 80.0], json!({})),
            units: json!([
                vehicle("blue", "tank", [12.0, 40.0], 0.0),
                vehicle("blue", "tank", [12.0, 28.0], 0.0),
                vehicle("blue", "tank", [12.0, 52.0], 0.0),
            ]),
            events: json!([
                { "tick": 1, "add_prop": wreck("tank_wreck", [70.0, 40.0], 0.4) },
                { "tick": 1, "add_prop": wreck("tank_wreck", [84.0, 55.0], -0.3) },
                { "tick": 1, "add_prop": wreck("supply_wreck", [78.0, 29.0], 1.2) },
                { "tick": 1, "add_prop": wreck("jeep_wreck", [95.0, 42.0], 0.8) },
                { "tick": 1, "add_prop": wreck("jeep_wreck", [62.0, 50.0], -0.6) },
                { "tick": 1, "add_prop": wreck("supply_wreck", [100.0, 30.0], 0.1) },
            ]),
            scripts: json!([go(0, [145.0, 40.0]), go(1, [145.0, 28.0]), go(2, [145.0, 52.0])]),
            rules: json!({}),
            seconds: 40.0,
            seed: 1,
            checks: vec![
                check(Arrive {
                    unit: 0,
                    at: [145.0, 40.0],
                    within_m: 2.5,
                }),
                check(Arrive {
                    unit: 1,
                    at: [145.0, 28.0],
                    within_m: 2.5,
                }),
                check(Arrive {
                    unit: 2,
                    at: [145.0, 52.0],
                    within_m: 2.5,
                }),
                check(VehiclesClearOfProps),
                check(VehiclesNeverOverlap),
            ],
        },
        Scenario {
            name: "t4-tank-pivots",
            caption: "a tank ordered to a point behind it pivots on the spot, then drives",
            map: flat([120.0, 80.0], json!({})),
            units: json!([vehicle("blue", "tank", [80.0, 40.0], 0.0)]),
            events: none.clone(),
            scripts: json!([go(0, [30.0, 40.0])]),
            rules: json!({}),
            seconds: 16.0,
            seed: 1,
            checks: vec![
                check(PivotsInPlace { unit: 0, min_deg: 110.0 }),
                check(Arrive {
                    unit: 0,
                    at: [30.0, 40.0],
                    within_m: 1.5,
                }),
                check(NeverReverses { unit: 0 }),
            ],
        },
        Scenario {
            name: "t4-truck-u-turn",
            caption: "a truck ordered to a point behind it U-turns on its radius in the open",
            map: flat([120.0, 80.0], json!({})),
            units: json!([vehicle("blue", "supply", [80.0, 40.0], 0.0)]),
            events: none.clone(),
            scripts: json!([go(0, [30.0, 40.0])]),
            rules: json!({}),
            seconds: 20.0,
            seed: 1,
            checks: vec![
                check(Arrive {
                    unit: 0,
                    at: [30.0, 40.0],
                    within_m: 1.5,
                }),
                check(WithinRadius { unit: 0 }),
                check(NeverReverses { unit: 0 }),
            ],
        },
        Scenario {
            name: "t4-truck-three-point-turn",
            caption: "a truck turns round in a 10 m walled lane: forward, back, forward",
            map: flat(
                [140.0, 60.0],
                json!({ "props": [
                    wall([70.0, 24.5], 0.0, [40.0, 0.5, 1.5]),
                    wall([70.0, 35.5], 0.0, [40.0, 0.5, 1.5]),
                ] }),
            ),
            units: json!([vehicle("blue", "supply", [70.0, 30.0], 0.0)]),
            events: none.clone(),
            scripts: json!([go(0, [40.0, 30.0])]),
            rules: json!({}),
            seconds: 40.0,
            seed: 1,
            checks: vec![
                check(Arrive {
                    unit: 0,
                    at: [40.0, 30.0],
                    within_m: 1.5,
                }),
                check(VehiclesClearOfProps),
                check(WithinRadius { unit: 0 }),
                check(Reverses { unit: 0 }),
            ],
        },
        Scenario {
            name: "t4-tank-reverses-out-of-gap",
            caption: "a tank nosed into a dead-end gap backs straight out, facing held",
            map: flat(
                [120.0, 60.0],
                json!({ "props": [
                    wall([60.0, 25.4], 0.0, [8.0, 0.4, 1.5]),
                    wall([60.0, 34.6], 0.0, [8.0, 0.4, 1.5]),
                    wall([68.4, 30.0], 0.0, [0.4, 5.0, 1.5]),
                ] }),
            ),
            units: json!([vehicle("blue", "tank", [62.0, 30.0], 0.0)]),
            events: none.clone(),
            scripts: json!([back(0, [30.0, 30.0])]),
            rules: json!({}),
            seconds: 25.0,
            seed: 1,
            checks: vec![
                check(Arrive {
                    unit: 0,
                    at: [30.0, 30.0],
                    within_m: 1.5,
                }),
                check(FacingHeld { unit: 0 }),
                check(Reverses { unit: 0 }),
                check(VehiclesClearOfProps),
            ],
        },
        Scenario {
            name: "t5-right-drag-facing",
            caption: "a squad, a tank and a jeep right-dragged east-to-north: the squad and the tank end facing north, the jeep (wheels never pivot) keeps its heading",
            map: flat([120.0, 100.0], json!({})),
            units: json!([
                rifle("blue", [20.0, 20.0]),
                vehicle("blue", "tank", [20.0, 50.0], 0.0),
                vehicle("blue", "jeep", [20.0, 80.0], 0.0),
            ]),
            events: none.clone(),
            scripts: json!([
                go_facing(0, [90.0, 20.0], 90.0),
                go_facing(1, [90.0, 50.0], 90.0),
                go_facing(2, [90.0, 80.0], 90.0),
            ]),
            rules: json!({}),
            seconds: 30.0,
            seed: 1,
            checks: vec![
                check(EndsFacing { unit: 0, deg: 90.0 }),
                check(EndsFacing { unit: 1, deg: 90.0 }),
                check(EndsFacing { unit: 2, deg: 0.0 }),
                check(VehiclesNeverOverlap),
            ],
        },
        Scenario {
            name: "t5-drag-sets-the-cover-side",
            caption: "a squad sent to a wall, right-dragged east across it, takes the wall's west face",
            map: flat([120.0, 80.0], json!({ "props": [
                { "kind": "wall", "center": [70.0, 40.0], "yaw": 0, "half_extents": [0.4, 10.0, 0.6] }
            ] })),
            units: json!([rifle("blue", [20.0, 40.0])]),
            events: none.clone(),
            scripts: json!([go_facing(0, [66.0, 40.0], 0.0)]),
            rules: json!({}),
            seconds: 25.0,
            seed: 2,
            checks: vec![
                check(EndsFacing { unit: 0, deg: 0.0 }),
                check(SoldiersClearOfProps),
                check(Spacing { min_m: 0.8 }),
            ],
        },
        Scenario {
            name: "t4-reverse-is-slower",
            caption: "two tanks drive 60 m east: the upper forwards, the lower in reverse",
            map: flat([120.0, 80.0], json!({})),
            units: json!([
                vehicle("blue", "tank", [20.0, 55.0], 0.0),
                vehicle("blue", "tank", [20.0, 25.0], std::f64::consts::PI),
            ]),
            events: none.clone(),
            scripts: json!([go(0, [80.0, 55.0]), back(1, [80.0, 25.0])]),
            rules: json!({}),
            seconds: 30.0,
            seed: 1,
            checks: vec![
                check(ArrivesFirst {
                    first: 0,
                    second: 1,
                }),
                check(FacingHeld { unit: 1 }),
            ],
        },
        Scenario {
            name: "t3-tank-carves-lane",
            caption: "a tank knocks a lane through medium forest; a jeep follows it at open-ground speed and the red squad down the lane spots it",
            map: flat([220.0, 80.0], json!({ "forests": [forest([70.0, 0.0, 60.0, 80.0], "medium")] })),
            units: json!([
                vehicle("blue", "tank", [15.0, 40.0], 0.0),
                vehicle("blue", "jeep", [12.0, 20.0], 0.0),
                rifle("red", [210.0, 40.0]),
            ]),
            events: none.clone(),
            scripts: json!([
                go(0, [185.0, 40.0]),
                { "tick": 1350, "side": "blue", "order":
                    { "kind": "move", "units": [1], "gesture": 2, "goal": [165.0, 40.0], "route": "fastest" } },
            ]),
            // Red's eyes reach 150 m: down the open lane, not through the trees.
            rules: json!({ "catalog": { "units": { "squad": { "sensors": { "ground_m": 150 } } } } }),
            seconds: 80.0,
            seed: 1,
            checks: vec![
                check(Arrive {
                    unit: 0,
                    at: [185.0, 40.0],
                    within_m: 2.5,
                }),
                check(Arrive {
                    unit: 1,
                    at: [165.0, 40.0],
                    within_m: 2.5,
                }),
                check(VehiclesClearOfProps),
                check(KnocksTrees { min: 4 }),
                check(FastThrough {
                    unit: 1,
                    rect: [74.0, 34.0, 126.0, 46.0],
                    min_mps: 7.0,
                }),
                check(SpottedIn {
                    unit: 1,
                    side: Side::Red,
                    rect: [74.0, 30.0, 126.0, 50.0],
                }),
            ],
        },
        Scenario {
            name: "t3-jeep-through-light-forest",
            caption: "a jeep threads light forest between the trunks, knocking none",
            map: flat([200.0, 80.0], json!({ "forests": [forest([60.0, 0.0, 80.0, 80.0], "light")] })),
            units: json!([vehicle("blue", "jeep", [15.0, 40.0], 0.0)]),
            events: none.clone(),
            scripts: json!([go(0, [185.0, 40.0])]),
            rules: json!({}),
            seconds: 50.0,
            seed: 1,
            checks: vec![
                check(Arrive {
                    unit: 0,
                    at: [185.0, 40.0],
                    within_m: 2.0,
                }),
                check(VehiclesClearOfProps),
                check(TreesStand),
            ],
        },
        Scenario {
            name: "t1-spotted-light-vs-dense",
            caption: "a squad walks toward two hidden squads: it spots the one in light forest from far farther than the one in dense",
            map: flat(
                [400.0, 100.0],
                json!({ "forests": [
                    forest([300.0, 0.0, 60.0, 45.0], "light"),
                    forest([300.0, 55.0, 60.0, 45.0], "dense"),
                ] }),
            ),
            units: json!([
                rifle("red", [318.0, 22.0]),
                rifle("red", [318.0, 78.0]),
                rifle("blue", [40.0, 50.0]),
            ]),
            events: none.clone(),
            scripts: json!([go(2, [290.0, 50.0])]),
            rules: json!({}),
            seconds: 90.0,
            seed: 1,
            checks: vec![check(SpottedFarther {
                first: 0,
                then: 1,
                observer: 2,
                ratio: 1.3,
            })],
        },
        Scenario {
            name: "t3-barrage-clears-forest",
            caption: "a barrage of HE falls on medium forest: the trees in the patch fall and it reads as open ground",
            map: flat([200.0, 80.0], json!({ "forests": [forest([60.0, 0.0, 80.0, 80.0], "medium")] })),
            units: json!([rifle("blue", [15.0, 40.0])]),
            events: json!(barrage([100.0, 40.0], 3, 7.0, &[20, 80])),
            scripts: none.clone(),
            rules: json!({}),
            seconds: 12.0,
            seed: 1,
            checks: vec![
                check(KnocksTrees { min: 6 }),
                check(OpenGround {
                    rect: [94.0, 34.0, 106.0, 46.0],
                    min_share: 0.9,
                }),
            ],
        },
        Scenario {
            name: "t3-tank-shells-house-through-works",
            caption: "a tank ordered to shell a house fires through a fence panel and into the sandbags on its line: the fence falls, the sandbags turn to rubble, the house to a ruin (27c)",
            map: flat(
                [260.0, 80.0],
                json!({ "props": [
                    prop("fence", [120.0, 40.0], 0.0, [0.1, 3.0, 0.6]),
                    prop("sandbags", [190.0, 40.0], 0.0, [0.4, 4.0, 0.5]),
                    prop("building", [230.0, 40.0], 0.0, [10.0, 10.0, 4.0]),
                ] }),
            ),
            units: json!([vehicle("blue", "tank", [20.0, 40.0], 0.0)]),
            events: none.clone(),
            scripts: json!([order(
                json!({ "kind": "attack", "units": [0], "target": { "kind": "ground", "point": [230, 40, 0] } })
            )]),
            rules: json!({}),
            seconds: 90.0,
            seed: 1,
            checks: vec![
                check(PropBecomes {
                    near: [190.0, 40.0],
                    into: "rubble",
                }),
                check(PropBecomes {
                    near: [230.0, 40.0],
                    into: "ruin",
                }),
            ],
        },
        Scenario {
            name: "t1-sandbags-shot-to-rubble",
            caption: "a squad at rest behind sandbags in a firefight; missiles burst on them at 10 s: rubble, and the squad re-covers at the crates",
            map: flat(
                [140.0, 90.0],
                json!({ "props": [
                    prop("sandbags", [64.0, 45.0], 0.0, [0.4, 5.0, 0.5]),
                    crate_at([57.0, 38.0]), crate_at([57.0, 52.0]), crate_at([58.0, 45.0]),
                ] }),
            ),
            units: json!([
                { "side": "blue", "kind": "rifle", "position": [58, 45] },
                { "side": "red", "kind": "rifle", "position": [110, 45] },
            ]),
            // ATGM bursts on the sandbags' far face: a 4 m blast that spares the crates.
            events: json!([
                { "tick": 300, "burst": { "point": [64.5, 43.0], "weapon": "atgm" } },
                { "tick": 301, "burst": { "point": [64.5, 47.0], "weapon": "atgm" } },
                { "tick": 302, "burst": { "point": [64.5, 45.0], "weapon": "atgm" } },
                { "tick": 303, "burst": { "point": [64.5, 45.0], "weapon": "atgm" } },
            ]),
            scripts: none.clone(),
            // Soldiers too tough to fall, so the fight lasts.
            rules: json!({ "catalog": { "soldiers": { "rifleman": { "hp": 1.0e6 } } } }),
            seconds: 30.0,
            seed: 1,
            checks: vec![
                check(PropBecomes {
                    near: [64.0, 45.0],
                    into: "rubble",
                }),
                check(InCover {
                    unit: 0,
                    threat: 1,
                    min: 6,
                }),
                check(SoldiersClearOfProps),
            ],
        },
        Scenario {
            name: "t2-squad-crosses-medium-wood",
            caption: "a squad walks across medium forest between the trunks, each soldier round every trunk (Q27)",
            map: flat([220.0, 80.0], json!({ "forests": [forest([50.0, 0.0, 120.0, 80.0], "medium")] })),
            units: json!([rifle("blue", [20.0, 40.0])]),
            events: none.clone(),
            scripts: json!([go(0, [200.0, 40.0])]),
            rules: json!({}),
            seconds: 120.0,
            seed: 1,
            checks: vec![
                check(Arrive {
                    unit: 0,
                    at: [200.0, 40.0],
                    within_m: 2.0,
                }),
                check(SoldiersClearOfProps),
            ],
        },
        // --- the village's own field works (slice 37), on its real map --------
        Scenario {
            name: "v-teeth-roadblock",
            caption: "village road block: the tank leaves the road round the teeth, the squad threads their gaps",
            map: village([860.0, 760.0, 960.0, 850.0]),
            units: json!([
                vehicle("blue", "tank", [835.0, 791.0], 0.0),
                rifle("blue", [850.0, 812.0]),
            ]),
            events: none.clone(),
            scripts: json!([go(0, [950.0, 797.0]), go(1, [935.0, 812.0])]),
            rules: json!({}),
            seconds: 60.0,
            seed: 1,
            checks: vec![
                check(Arrive {
                    unit: 0,
                    at: [950.0, 797.0],
                    within_m: 1.5,
                }),
                check(Arrive {
                    unit: 1,
                    at: [935.0, 812.0],
                    within_m: 2.0,
                }),
                check(VehiclesClearOfProps),
                check(SoldiersClearOfProps),
                check(PropStays {
                    near: [895.0, 793.4],
                }),
            ],
        },
        Scenario {
            name: "v-works-by-the-buildings",
            caption: "defenders out of their building take the sandbags against a squad beyond the teeth",
            map: village([880.0, 725.0, 1000.0, 890.0]),
            units: json!([
                { "side": "blue", "kind": "rifle", "position": [870, 812] },
                { "side": "red", "kind": "rifle", "position": [953, 743] },
            ]),
            events: none.clone(),
            scripts: none.clone(),
            // Soldiers too tough to fall, so the fight lasts.
            rules: json!({ "catalog": { "soldiers": { "rifleman": { "hp": 1.0e6 } } } }),
            seconds: 30.0,
            seed: 1,
            checks: vec![
                check(InCover {
                    unit: 1,
                    threat: 0,
                    min: 5,
                }),
                check(SoldiersClearOfProps),
            ],
        },
        Scenario {
            name: "v-works-lean-by-a-corner",
            caption: "the works by the buildings with blue listed second: a defender's lean point sits off the north house's corner, and he walks to it without jamming",
            map: village([880.0, 725.0, 1000.0, 890.0]),
            units: json!([
                { "side": "red", "kind": "rifle", "position": [953, 743] },
                { "side": "blue", "kind": "rifle", "position": [870, 812] },
            ]),
            events: none.clone(),
            scripts: none.clone(),
            rules: json!({ "catalog": { "soldiers": { "rifleman": { "hp": 1.0e6 } } } }),
            seconds: 40.0,
            seed: 1,
            checks: vec![
                check(InCover {
                    unit: 0,
                    threat: 1,
                    min: 5,
                }),
                check(SoldiersClearOfProps),
            ],
        },
        Scenario {
            name: "v-square-sandbags",
            caption: "defenders on the village square take the sandbags facing the road",
            map: village([940.0, 768.0, 1070.0, 823.0]),
            units: json!([
                { "side": "red", "kind": "rifle", "position": [1018, 797] },
                { "side": "blue", "kind": "rifle", "position": [955, 788] },
            ]),
            events: none.clone(),
            scripts: none.clone(),
            rules: json!({ "catalog": { "soldiers": { "rifleman": { "hp": 1.0e6 } } } }),
            seconds: 30.0,
            seed: 1,
            checks: vec![
                check(InCover {
                    unit: 0,
                    threat: 1,
                    min: 5,
                }),
                check(SoldiersClearOfProps),
            ],
        },
        Scenario {
            name: "v-jeep-at-the-garden-fence",
            caption: "a jeep cannot shove the garden fence and drives round it or through its gate",
            map: village([925.0, 895.0, 990.0, 915.0]),
            units: json!([vehicle("blue", "jeep", [952.6, 925.0], -std::f64::consts::FRAC_PI_2)]),
            events: none.clone(),
            scripts: json!([go(0, [952.6, 890.0])]),
            rules: json!({}),
            seconds: 30.0,
            seed: 1,
            checks: vec![
                check(Arrive {
                    unit: 0,
                    at: [952.6, 890.0],
                    within_m: 1.5,
                }),
                check(VehiclesClearOfProps),
                check(PropStays {
                    near: [952.6, 905.0],
                }),
            ],
        },
        Scenario {
            name: "v-tank-shoves-garden-fence",
            caption: "a tank drives through the garden fence, shoving a panel aside",
            map: village([925.0, 895.0, 990.0, 915.0]),
            units: json!([vehicle("blue", "tank", [946.4, 927.0], -std::f64::consts::FRAC_PI_2)]),
            events: none.clone(),
            scripts: json!([go(0, [946.4, 889.0])]),
            rules: json!({}),
            seconds: 30.0,
            seed: 1,
            checks: vec![
                check(Arrive {
                    unit: 0,
                    at: [946.4, 889.0],
                    within_m: 1.5,
                }),
                check(PropMoved {
                    near: [946.4, 905.0],
                    min_m: 1.0,
                }),
            ],
        },
        Scenario {
            name: "t2-round-a-fence-end-by-a-road",
            caption: "a squad rounds the end of a fence that crosses a road bend: one man must not twitch at the end",
            map: {
                let mut map = village([0.0, 0.0, 0.0, 0.0]);
                map["props"] = (0..10)
                    .map(|k| prop("fence", [103.0 + 6.2 * k as f64, 768.0], 0.0, [3.0, 0.1, 0.6]))
                    .collect();
                map
            },
            units: json!([rifle("blue", [180.0, 790.0])]),
            events: none.clone(),
            scripts: json!([go(0, [130.0, 748.0])]),
            rules: json!({}),
            seconds: 40.0,
            seed: 1,
            checks: vec![
                check(Arrive {
                    unit: 0,
                    at: [130.0, 748.0],
                    within_m: 2.0,
                }),
                check(SoldiersClearOfProps),
            ],
        },
        Scenario {
            name: "v-blue-start",
            caption: "blue's start: a squad walks round the farm fence, the truck and the jeep drive off",
            map: village([60.0, 730.0, 240.0, 910.0]),
            units: json!([
                rifle("blue", [180.0, 790.0]),
                vehicle("blue", "supply", [100.0, 800.0], 0.0),
                vehicle("blue", "jeep", [125.0, 905.0], 0.0),
            ]),
            events: none.clone(),
            scripts: json!([go(0, [100.0, 748.0]), go(1, [230.0, 800.0]), go(2, [230.0, 880.0])]),
            rules: json!({}),
            seconds: 40.0,
            seed: 1,
            checks: vec![
                check(Arrive {
                    unit: 0,
                    at: [100.0, 748.0],
                    within_m: 2.0,
                }),
                check(Arrive {
                    unit: 1,
                    at: [230.0, 800.0],
                    within_m: 1.5,
                }),
                check(Arrive {
                    unit: 2,
                    at: [230.0, 880.0],
                    within_m: 1.5,
                }),
                check(SoldiersClearOfProps),
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
    let mut rules = sim::fixtures::village();
    let mut patch = s.rules.clone();
    if let Some(Value::Object(sections)) = patch.as_object_mut().and_then(|p| p.remove("catalog")) {
        for (section, entries) in sections {
            for (id, entry) in entries.as_object().unwrap() {
                sim::fixtures::patch_catalog(&mut rules, &section, id, entry.clone());
            }
        }
    }
    merge(&mut rules, &patch);
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
        .map(|(j, c)| j.verdict(c, &b))
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
    p.blocks(MoverClass::Infantry)
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

/// The cover tier the soldier at `p` has against a round from `threat`, by
/// the rule the spread reads (Q20): a tiered body within reach between
/// them, a live hull among them, or a crater under him.
pub fn cover_tier(b: &Battle, p: V2, threat: V2) -> Option<sim::cover::Tier> {
    let rules = &b.rules();
    let hulls = sim::cover::hull_bodies(&sim::lean::hulls(units(b), rules));
    sim::cover::at(b.world(), b.ground(), &hulls, rules, p, threat)
}

/// Per-check state gathered while the battle runs.
struct Judge {
    worst: f64,
    at: String,
    /// Distance to the goal when the unit first halted.
    halted_short: Option<f64>,
    prop: Option<(u32, V2)>,
    /// Every soldier's health at the start.
    health: Vec<f64>,
    waited: bool,
    /// The watched unit's start and last pose.
    start: Option<(V2, f64)>,
    last: Option<(V2, f64)>,
    /// The largest turn before the unit moved half a metre, in degrees.
    turned_deg: f64,
    reversed: bool,
    /// Soldiers seen leaning out (`Leans`).
    leaners: std::collections::BTreeSet<u32>,
    /// The tick each unit first stood idle after moving.
    arrived: [Option<u64>; 2],
    /// Trees standing at the start.
    trees: usize,
    /// Distance and seconds spent inside a rect, and the last position seen.
    travel: (f64, f64, Option<V2>),
    spotted: bool,
    /// Observer distance at each target's first identification.
    first_seen: [Option<f64>; 2],
    /// Each soldier's twitch watch, by (unit, member index).
    twitch: std::collections::BTreeMap<(u32, usize), Twitch>,
}

/// One soldier's steps while he has somewhere to go (`NoTwitch`).
struct Twitch {
    last: V2,
    /// His last step's direction, while active.
    dir: Option<V2>,
    reversals: u32,
    /// Where his current still stretch began, and when.
    anchor: V2,
    since: u64,
    /// His longest still stretch, in ticks.
    still: u64,
}

fn trees(b: &Battle) -> usize {
    b.world()
        .props()
        .filter(|p| b.world().types().id(p.kind) == "trunk")
        .count()
}

fn inside(rect: &[f64; 4], p: V2) -> bool {
    p.x >= rect[0] && p.x <= rect[2] && p.y >= rect[1] && p.y <= rect[3]
}

/// Whether `side` identifies `unit` this tick: its published ids are
/// opaque, so by kind and position.
fn identifies(b: &Battle, side: Side, unit: u32) -> bool {
    let u = b.unit(UnitId(unit)).unwrap();
    b.observe(side).identified.iter().any(|t| {
        t.kind == b.rules().catalog.id(u.kind)
            && (v2(t.position[0], t.position[1]) - u.position.xy()).length() < 1.0
    })
}

impl Judge {
    fn new(kind: &CheckKind, b: &Battle) -> Self {
        let prop = match kind {
            CheckKind::PropMoved { near, .. }
            | CheckKind::PropStays { near }
            | CheckKind::PropBecomes { near, .. } => {
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
            health: units(b)
                .flat_map(|u| u.members.iter().map(|s| s.hp))
                .collect(),
            waited: false,
            start: None,
            last: None,
            turned_deg: 0.0,
            reversed: false,
            leaners: Default::default(),
            arrived: [None, None],
            trees: trees(b),
            travel: (0.0, 0.0, None),
            spotted: false,
            first_seen: [None, None],
            twitch: Default::default(),
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
            CheckKind::NoTwitch { .. } => {
                let tick = b.tick();
                for u in units(b).filter(|u| !u.is_vehicle() && !u.garrisoned()) {
                    let waiting = matches!(
                        u.state,
                        MoveState::Waiting
                            | MoveState::RouteBlocked
                            | MoveState::Packing
                            | MoveState::Halted
                    );
                    for (i, s) in u.members.iter().enumerate() {
                        let here = s.position.xy();
                        let t = self.twitch.entry((u.id.0, i)).or_insert(Twitch {
                            last: here,
                            dir: None,
                            reversals: 0,
                            anchor: here,
                            since: tick,
                            still: 0,
                        });
                        let step = here - t.last;
                        t.last = here;
                        let goal = s.spot.or(s.post).filter(|g| (*g - here).length() > 0.5);
                        if !s.alive() || goal.is_none() || waiting || tick < s.start {
                            (t.dir, t.anchor, t.since) = (None, here, tick);
                            continue;
                        }
                        if step.length() > 1e-3 {
                            let d = step.normalized();
                            if t.dir.is_some_and(|p| p.dot(d) < -0.7) {
                                t.reversals += 1;
                            }
                            t.dir = Some(d);
                        }
                        if (here - t.anchor).length() > 1.0 {
                            (t.anchor, t.since) = (here, tick);
                        }
                        t.still = t.still.max(tick - t.since);
                    }
                }
            }
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
                        if q.blocks(contract::map::MoverClass::Vehicle)
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
            CheckKind::SoldiersClearOfHulls => {
                let hulls: Vec<_> = units(b)
                    .filter_map(|u| u.hull_box().filter(|_| u.alive()))
                    .collect();
                for u in units(b).filter(|u| !u.is_vehicle() && !u.garrisoned()) {
                    for p in u.member_positions() {
                        for h in &hulls {
                            let m = distance_to_box(h, p.xy()) - SOLDIER_RADIUS_M;
                            self.note(m, b, || format!("unit {} soldier in a hull", u.id.0));
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
            CheckKind::VehiclesNeverOverlap => {
                let hulls: Vec<_> = units(b)
                    .filter_map(|u| u.hull_box().filter(|_| u.alive()).map(|h| (u.id.0, h)))
                    .collect();
                for (i, (a, ha)) in hulls.iter().enumerate() {
                    for (c, hc) in &hulls[i + 1..] {
                        let depth = ha.separation(hc).map_or(0.0, |v| v.length());
                        self.note(-depth, b, || format!("units {a} and {c} overlap"));
                    }
                }
            }
            CheckKind::Waits { unit } => {
                if b.unit(UnitId(*unit)).unwrap().state == MoveState::Waiting {
                    self.waited = true;
                }
            }
            CheckKind::PivotsInPlace { unit, .. } => {
                let u = b.unit(UnitId(*unit)).unwrap();
                let (at, yaw) = *self.start.get_or_insert((u.position.xy(), u.yaw));
                if (u.position.xy() - at).length() < 0.5 {
                    let turned = sim::math::wrap_angle(u.yaw - yaw).abs().to_degrees();
                    self.turned_deg = self.turned_deg.max(turned);
                }
            }
            CheckKind::WithinRadius { unit } => {
                let u = b.unit(UnitId(*unit)).unwrap();
                let radius = u.mobility.drive.unwrap().radius_m;
                let now = (u.position.xy(), u.yaw);
                if let Some((at, yaw)) = self.last {
                    let rolled = (now.0 - at).length();
                    let turned = sim::math::wrap_angle(now.1 - yaw).abs();
                    self.note(rolled / radius * 1.001 + 1e-9 - turned, b, || {
                        format!("unit {unit} turned {turned:.4} rad over {rolled:.4} m")
                    });
                }
                self.last = Some(now);
            }
            CheckKind::Reverses { unit } | CheckKind::NeverReverses { unit } => {
                self.reversed |= b.unit(UnitId(*unit)).unwrap().reversing;
            }
            CheckKind::Leans { unit, .. } => {
                let u = b.unit(UnitId(*unit)).unwrap();
                for s in u.members.iter().filter(|s| s.alive()) {
                    let Some(l) = s.leaning(b.tick()) else {
                        continue;
                    };
                    self.leaners.insert(s.id);
                    let inside = b.world().props().any(|p| {
                        p.blocks(MoverClass::Infantry) && p.footprint().contains(l.at, 0.0)
                    });
                    if inside {
                        self.note(-1.0, b, || {
                            format!("soldier {} leans into a body at {:?}", s.id, l.at)
                        });
                    }
                }
            }
            CheckKind::FacingHeld { unit } => {
                let u = b.unit(UnitId(*unit)).unwrap();
                let (_, yaw) = *self.start.get_or_insert((u.position.xy(), u.yaw));
                let off = sim::math::wrap_angle(u.yaw - yaw).abs().to_degrees();
                self.note(1.0 - off, b, || format!("unit {unit} {off:.1} degrees off"));
            }
            CheckKind::ArrivesFirst { first, second } => {
                for (k, id) in [*first, *second].into_iter().enumerate() {
                    let u = b.unit(UnitId(id)).unwrap();
                    if b.tick() > 2 && u.state == MoveState::Idle && self.arrived[k].is_none() {
                        self.arrived[k] = Some(b.tick());
                    }
                }
            }
            CheckKind::TreesStand => {
                let now = trees(b);
                self.note(now as f64 - self.trees as f64, b, || {
                    format!("{now} trees stand")
                });
            }
            CheckKind::FastThrough { unit, rect, .. } => {
                let p = b.unit(UnitId(*unit)).unwrap().position.xy();
                if let Some(was) = self.travel.2 {
                    if inside(rect, was) && inside(rect, p) && (p - was).length() > 0.0 {
                        self.travel.0 += (p - was).length();
                        self.travel.1 += 1.0 / b.rules().tick_hz as f64;
                    }
                }
                self.travel.2 = Some(p);
            }
            CheckKind::SpottedIn { unit, side, rect } => {
                let p = b.unit(UnitId(*unit)).unwrap().position.xy();
                if inside(rect, p) && identifies(b, *side, *unit) {
                    self.spotted = true;
                }
            }
            CheckKind::SpottedFarther {
                first,
                then,
                observer,
                ..
            } => {
                let o = b.unit(UnitId(*observer)).unwrap();
                for (k, target) in [*first, *then].into_iter().enumerate() {
                    if self.first_seen[k].is_none() && identifies(b, o.side, target) {
                        let t = b.unit(UnitId(target)).unwrap().position.xy();
                        self.first_seen[k] = Some((t - o.position.xy()).length());
                    }
                }
            }
            _ => {}
        }
    }

    fn verdict(self, c: &Check, b: &Battle) -> Outcome {
        let (label, passed, detail) = match &c.kind {
            CheckKind::NoTwitch {
                max_reversals,
                max_still_s,
            } => {
                let hz = b.rules().tick_hz as f64;
                let most = self.twitch.iter().max_by_key(|(_, t)| t.reversals);
                let longest = self.twitch.iter().max_by_key(|(_, t)| t.still);
                let reversals = most.map_or(0, |(_, t)| t.reversals);
                let still_s = longest.map_or(0.0, |(_, t)| t.still as f64 / hz);
                (
                    format!("no twitch (≤ {max_reversals} reversals, ≤ {max_still_s} s still)"),
                    reversals <= *max_reversals && still_s <= *max_still_s,
                    format!(
                        "most reversals {reversals} (soldier {:?}), longest still {still_s:.1} s (soldier {:?})",
                        most.map(|(k, _)| k),
                        longest.map(|(k, _)| k)
                    ),
                )
            }
            CheckKind::EndsFacing { unit, deg } => {
                let u = b.unit(UnitId(*unit)).unwrap();
                let off = sim::math::wrap_angle(u.yaw - deg.to_radians())
                    .abs()
                    .to_degrees();
                (
                    format!("unit {unit} ends facing {deg}°"),
                    u.state == MoveState::Idle && off <= 2.0,
                    format!("{:?}, {off:.1} degrees off", u.state),
                )
            }
            CheckKind::Arrive { unit, at, within_m } => {
                let u = b.unit(UnitId(*unit)).unwrap();
                // A squad holds round its anchor, spread over its area (27d).
                let held = u.anchor.map_or(u.position.xy(), |a| a.at);
                let d = (held - v2(at[0], at[1])).length();
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
                    .filter(|p| cover_tier(b, p.xy(), t).is_some())
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
            CheckKind::ClearLines { unit, threat, min } => {
                let u = b.unit(UnitId(*unit)).unwrap();
                let lift = |z: f64| sim::math::v3(0.0, 0.0, z);
                let aims: Vec<_> = b
                    .unit(UnitId(*threat))
                    .unwrap()
                    .member_positions()
                    .map(|p| p + lift(b.rules().physics.infantry_aim_m))
                    .collect();
                let muzzle = lift(b.rules().physics.infantry_muzzle_m);
                // From where he stands, or out on his claimed lean (27d),
                // whose rounds pass the body he leans round.
                let n = u
                    .members
                    .iter()
                    .filter(|s| s.alive())
                    .filter(|s| {
                        let lean = s
                            .lean
                            .filter(|l| (l.from - s.position.xy()).length() <= 0.5);
                        let from = [(s.position, None)]
                            .into_iter()
                            .chain(lean.map(|l| (l.at.with_z(s.position.z), l.past())));
                        from.into_iter().any(|(p, past)| {
                            aims.iter()
                                .any(|a| b.world().segment_clear_except(p + muzzle, *a, past))
                        })
                    })
                    .count();
                (
                    format!("at least {min} of unit {unit} with a clear line"),
                    n >= *min,
                    format!("{n} clear"),
                )
            }
            CheckKind::PivotsInPlace { unit, min_deg } => (
                format!("unit {unit} turns {min_deg} degrees in place"),
                self.turned_deg >= *min_deg,
                format!("{:.0} degrees before moving 0.5 m", self.turned_deg),
            ),
            CheckKind::Leans { unit, min } => (
                format!("at least {min} of unit {unit} lean out to fire"),
                self.leaners.len() >= *min && self.worst >= 0.0,
                format!("{} leaned; {}", self.leaners.len(), self.at),
            ),
            CheckKind::Reverses { unit } => (
                format!("unit {unit} reverses"),
                self.reversed,
                format!("reversed: {}", self.reversed),
            ),
            CheckKind::NeverReverses { unit } => (
                format!("unit {unit} never reverses"),
                !self.reversed,
                format!("reversed: {}", self.reversed),
            ),
            CheckKind::ArrivesFirst { first, second } => (
                format!("unit {first} arrives before unit {second}"),
                matches!(self.arrived, [Some(a), Some(b)] if a < b),
                format!("arrival ticks {:?}", self.arrived),
            ),
            CheckKind::KnocksTrees { min } => {
                let fell = self.trees - trees(b);
                (
                    format!("at least {min} trees knocked down"),
                    fell >= *min,
                    format!("{fell} fell"),
                )
            }
            CheckKind::FastThrough { unit, min_mps, .. } => {
                let (m, s, _) = self.travel;
                let mps = if s > 0.0 { m / s } else { 0.0 };
                (
                    format!("unit {unit} crosses at {min_mps} m/s or more"),
                    s > 0.0 && mps >= *min_mps,
                    format!("{mps:.2} m/s over {m:.1} m"),
                )
            }
            CheckKind::SpottedIn { unit, side, .. } => (
                format!("{side:?} spots unit {unit} in the lane"),
                self.spotted,
                format!("spotted: {}", self.spotted),
            ),
            CheckKind::SpottedFarther {
                first, then, ratio, ..
            } => {
                let [a, c] = self.first_seen;
                (
                    format!("unit {first} spotted {ratio}× farther than unit {then}"),
                    a.zip(c).is_some_and(|(a, c)| a >= ratio * c),
                    format!("first seen at {a:.1?} m and {c:.1?} m"),
                )
            }
            CheckKind::Waits { unit } => (
                format!("unit {unit} waits for traffic"),
                self.waited,
                format!("waited: {}", self.waited),
            ),
            CheckKind::OneBodyEach { unit, threat, min } => {
                let u = b.unit(UnitId(*unit)).unwrap();
                let t = b.unit(UnitId(*threat)).unwrap().position.xy();
                let reach = b.rules().cover.reach_m;
                // Each soldier's nearest covering body.
                let bodies: Vec<Option<u32>> = u
                    .member_positions()
                    .map(|p| {
                        b.world()
                            .props_near(p.xy(), reach)
                            .into_iter()
                            .filter(|q| sim::cover::prop_tier(q).is_some())
                            .filter(|q| {
                                sim::cover::covers(
                                    &q.footprint(),
                                    p.xy(),
                                    t,
                                    reach,
                                    SOLDIER_RADIUS_M,
                                )
                            })
                            .min_by(|x, y| {
                                distance_to_box(&x.footprint(), p.xy())
                                    .total_cmp(&distance_to_box(&y.footprint(), p.xy()))
                            })
                            .map(|q| q.id)
                    })
                    .collect();
                let alone = bodies
                    .iter()
                    .filter(|id| id.is_some() && bodies.iter().filter(|o| o == id).count() == 1)
                    .count();
                (
                    format!("at least {min} of unit {unit} behind a body of their own"),
                    alone >= *min,
                    format!("{alone} alone behind one"),
                )
            }
            CheckKind::NobodyHurt => {
                let now: Vec<f64> = units(b)
                    .flat_map(|u| u.members.iter().map(|s| s.hp))
                    .collect();
                let hurt = now.iter().zip(&self.health).filter(|(n, h)| n < h).count();
                (
                    "nobody hurt".into(),
                    hurt == 0,
                    format!("{hurt} soldiers hurt"),
                )
            }
            CheckKind::PropStays { .. } => {
                let (id, start) = self.prop.expect("a prop near the point");
                let moved = b
                    .world()
                    .prop(id)
                    .map_or(f64::INFINITY, |p| (p.center - start).length());
                (
                    format!("prop {id} never moves"),
                    moved == 0.0,
                    format!("moved {moved:.2} m"),
                )
            }
            CheckKind::OpenGround { rect, min_share } => {
                let w = b.world();
                let (mut open, mut all) = (0usize, 0usize);
                let mut y = rect[1] + 0.5;
                while y < rect[3] {
                    let mut x = rect[0] + 0.5;
                    while x < rect[2] {
                        all += 1;
                        open += usize::from(!w.forest_ground(x, y) && w.foliage_at(x, y).is_open());
                        x += 1.0;
                    }
                    y += 1.0;
                }
                let share = open as f64 / all.max(1) as f64;
                (
                    format!(
                        "at least {:.0}% of {rect:?} is open ground",
                        min_share * 100.0
                    ),
                    share >= *min_share,
                    format!("{:.0}% open", share * 100.0),
                )
            }
            CheckKind::PropBecomes { into, .. } => {
                let (id, start) = self.prop.expect("a prop near the point");
                let remains = b
                    .world()
                    .props()
                    .find(|p| b.structures().replaced_by(p.id) == Some(id));
                (
                    format!("prop {id} is destroyed into {into}"),
                    b.world().prop(id).is_none()
                        && remains.is_some_and(|p| b.world().types().id(p.kind) == *into),
                    format!(
                        "{} at {start:?}",
                        remains.map_or("nothing", |p| b.world().types().id(p.kind))
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
                    CheckKind::SoldiersClearOfHulls => "no soldier inside a hull".into(),
                    CheckKind::VehiclesNeverOverlap => "no two hulls ever overlap".into(),
                    CheckKind::WithinRadius { unit } => {
                        format!("unit {unit} never turns tighter than its radius")
                    }
                    CheckKind::FacingHeld { unit } => format!("unit {unit} holds its facing"),
                    CheckKind::TreesStand => "no tree falls".into(),
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
fn t1_destroyed_cover_is_re_resolved() {
    assert_scenario("t1-cover-destroyed");
}

#[test]
fn t1_a_squad_re_covers_when_fire_grinds_its_cover_away() {
    assert_scenario("t1-cover-shot-away");
}

#[test]
fn t1_a_squad_takes_cover_behind_a_parked_tank() {
    assert_scenario("t1-behind-parked-tank");
}

#[test]
fn t1_blocked_soldiers_step_out_round_a_corner() {
    assert_scenario("t1-step-out-corner");
}

#[test]
fn t1_a_squad_in_a_wood_leans_out_from_its_trees() {
    assert_scenario("t1-wood-lean-out");
}

#[test]
fn t1_a_squad_leans_out_round_a_house_corner() {
    assert_scenario("t1-building-corner-lean-out");
}

#[test]
fn t1_a_squad_leans_out_past_its_parked_tank() {
    assert_scenario("t1-parked-tank-lean-out");
}

#[test]
fn t1_three_at_a_corner_all_end_able_to_engage() {
    assert_scenario("t1-corner-three");
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
fn t2_a_squad_threads_a_line_of_teeth() {
    assert_scenario("t2-line-of-teeth");
}

#[test]
fn t2_soldiers_walk_round_a_crate_stack() {
    assert_scenario("t2-around-crate-stack");
}

#[test]
fn t2_soldiers_replan_round_a_new_obstacle() {
    assert_scenario("t2-crate-dropped-in-lane");
}

#[test]
fn t2_soldiers_walk_round_a_parked_tank() {
    assert_scenario("t2-around-parked-tank");
}

#[test]
fn t3_a_resting_squad_yields_to_a_tank() {
    assert_scenario("t3-tank-through-resting-squad");
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
fn t3_a_truck_shoves_a_crate() {
    assert_scenario("t3-truck-pushes-crate");
}

#[test]
fn t3_infantry_cross_dragons_teeth_a_tank_goes_round() {
    assert_scenario("t3-infantry-cross-teeth-tank-goes-round");
}

#[test]
fn t1_a_squad_takes_one_tooth_each() {
    assert_scenario("t1-one-tooth-each");
}

#[test]
fn t3_enemy_tanks_meet_head_on_without_overlapping() {
    assert_scenario("t3-tanks-meet-head-on");
}

#[test]
fn t3_a_column_crosses_a_wreck_field() {
    assert_scenario("t3-column-through-wreck-field");
}

#[test]
fn t3_a_tank_carves_a_lane_a_jeep_follows() {
    assert_scenario("t3-tank-carves-lane");
}

#[test]
fn t3_a_jeep_threads_light_forest() {
    assert_scenario("t3-jeep-through-light-forest");
}

#[test]
fn t1_light_forest_hides_less_than_dense() {
    assert_scenario("t1-spotted-light-vs-dense");
}

#[test]
fn t3_a_barrage_clears_forest() {
    assert_scenario("t3-barrage-clears-forest");
}

#[test]
fn t3_a_tank_shells_a_house_through_the_works_on_its_line() {
    assert_scenario("t3-tank-shells-house-through-works");
}

#[test]
fn t1_sandbags_shot_to_rubble_are_re_covered() {
    assert_scenario("t1-sandbags-shot-to-rubble");
}

#[test]
fn t2_a_squad_crosses_a_medium_wood() {
    assert_scenario("t2-squad-crosses-medium-wood");
}

#[test]
fn v_the_village_road_block_turns_tanks_not_squads() {
    assert_scenario("v-teeth-roadblock");
}

#[test]
fn v_defenders_take_the_works_by_their_buildings() {
    assert_scenario("v-works-by-the-buildings");
}

#[test]
fn v_a_defender_walks_to_a_lean_point_off_a_building_corner_without_jamming() {
    assert_scenario("v-works-lean-by-a-corner");
}

#[test]
fn v_defenders_take_the_square_sandbags() {
    assert_scenario("v-square-sandbags");
}

#[test]
fn v_a_jeep_drives_round_the_garden_fence() {
    assert_scenario("v-jeep-at-the-garden-fence");
}

#[test]
fn v_a_tank_shoves_through_the_garden_fence() {
    assert_scenario("v-tank-shoves-garden-fence");
}

#[test]
fn t2_a_squad_rounds_a_fence_end_by_a_road() {
    assert_scenario("t2-round-a-fence-end-by-a-road");
}

#[test]
fn v_blue_leaves_its_start_round_the_farm_fence() {
    assert_scenario("v-blue-start");
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

#[test]
fn t4_a_tank_pivots_on_the_spot() {
    assert_scenario("t4-tank-pivots");
}

#[test]
fn t4_a_truck_u_turns_in_the_open() {
    assert_scenario("t4-truck-u-turn");
}

#[test]
fn t4_a_truck_turns_round_in_a_narrow_lane() {
    assert_scenario("t4-truck-three-point-turn");
}

#[test]
fn t4_a_tank_reverses_out_of_a_gap() {
    assert_scenario("t4-tank-reverses-out-of-gap");
}

#[test]
fn t5_a_right_drag_sets_the_final_facing() {
    assert_scenario("t5-right-drag-facing");
}

#[test]
fn t5_a_right_drag_sets_the_cover_side() {
    assert_scenario("t5-drag-sets-the-cover-side");
}

#[test]
fn t4_reverse_is_slower_than_forward() {
    assert_scenario("t4-reverse-is-slower");
}
