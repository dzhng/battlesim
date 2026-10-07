//! Leaning out round tall cover: a soldier behind a body taller
//! than his muzzle fires from past its edge, never through it, whatever the
//! body's kind; with no edge giving a line, he does not lean.
use contract::ids::{Side, UnitId};
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::flight::{FlightEvent, Struck};
use sim::math::{v2, Obb2, V2};

use crate::common;

/// Hull cover must retain only living vehicles, in their original unit order;
/// infantry membership and casualties cannot turn a squad into a hull.
#[test]
fn hull_cover_keeps_live_vehicle_geometry_and_order_among_fallen_squads() {
    let rules: contract::scenario::Rules =
        serde_json::from_value(common::scenario_rules()).unwrap();
    let fallen = rules.catalog.by_id("test_rifle").squad_size();
    let setup = common::scenario_with(
        r#"{"size":[340,160],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35}"#,
        json!([
            {"side":"blue","kind":"test_rifle","position":[20,20],"condition":{"casualties":fallen}},
            {"side":"red","kind":"test_jeep","position":[40,120]},
            {"side":"blue","kind":"test_tank","position":[100,60],"yaw":0.37},
            {"side":"red","kind":"test_rifle","position":[140,120],"condition":{"casualties":1}},
            {"side":"red","kind":"test_jeep","position":[180,60],"yaw":-0.82},
            {"side":"red","kind":"test_rifle","position":[240,120],"condition":{"casualties":fallen}},
            {"side":"blue","kind":"test_jeep","position":[260,60],"yaw":1.1}
        ]),
        json!([]),
        json!([]),
    );
    let battle = Battle::new(&setup, 1);
    let mut units: Vec<_> = (0..7)
        .map(|id| battle.unit(UnitId(id)).unwrap().clone())
        .collect();
    // Authored health is clamped above zero; the query also sees vehicles
    // destroyed during a battle.
    units[1].hp = 0.0;
    assert!(!units[0].alive() && !units[1].alive() && !units[5].alive());
    assert!(units[3].alive() && units[3].members.iter().any(|s| !s.alive()));
    let hulls = sim::lean::hulls(units.iter(), &setup.rules);
    assert_eq!(
        hulls.iter().map(|h| h.unit).collect::<Vec<_>>(),
        vec![UnitId(2), UnitId(4), UnitId(6)]
    );
    for (hull, (id, side, at, yaw)) in hulls.iter().zip([
        (2, Side::Blue, v2(100.0, 60.0), 0.37),
        (4, Side::Red, v2(180.0, 60.0), -0.82),
        (6, Side::Blue, v2(260.0, 60.0), 1.1),
    ]) {
        let unit = battle.unit(UnitId(id)).unwrap();
        assert_eq!(hull.side, side);
        assert_eq!(hull.rect.center, at);
        assert_eq!(hull.rect.yaw, yaw);
        assert_eq!(hull.base, unit.position.z);
        assert_eq!(hull.top, unit.position.z + 2.0 * unit.hull.unwrap().z);
        assert_eq!(hull.tier, sim::cover::vehicle_tier(unit, &setup.rules));
    }
}

const RADIUS: f64 = 0.3;

fn prop(kind: &str, center: [f64; 2], half: [f64; 3]) -> Value {
    json!({ "kind": kind, "center": center, "yaw": 0, "half_extents": half })
}

/// What blue's fire from cover did.
#[derive(Debug, Default)]
struct Fight {
    /// Blue rounds launched away from where their soldier stood: leaned.
    leaned: usize,
    /// The nearest any leaned round's origin came to the cover's footprint.
    closest: f64,
    /// Blue rounds that struck the cover.
    struck_cover: usize,
    red_hurt: bool,
    /// Ticks on which some blue soldier was leaning out.
    leaning_ticks: usize,
}

/// Blue's rifle squad at rest at `blue`, red's in the open at `red`, both
/// firing at will, soldiers too tough to fall, for `seconds`; the cover is
/// the footprint `cover` (struck through `struck`).
fn fight(
    props: Value,
    mut units: Vec<Value>,
    blue: [f64; 2],
    red: [f64; 2],
    cover: Obb2,
    struck: impl Fn(Struck) -> bool,
    seconds: u64,
) -> Fight {
    let mut rules = common::game();
    sim::fixtures::patch_catalog(
        &mut rules,
        "soldiers",
        "test_rifleman",
        json!({ "hp": 1.0e6 }),
    );
    units.insert(
        0,
        json!({ "side": "red", "kind": "test_rifle", "position": red }),
    );
    units.insert(
        0,
        json!({ "side": "blue", "kind": "test_rifle", "position": blue }),
    );
    let mut setup: contract::scenario::ScenarioDefinition = serde_json::from_value(json!({
        "map": { "size": [160, 110], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
                 "props": props, "forests": [] },
        "rules": rules,
        "units": units,
        "events": [],
        "scripts": [],
    }))
    .unwrap();
    setup.map = common::physical_map(setup.map, &setup.rules);
    let mut b = Battle::new(&setup, 1);
    let mut seen = std::collections::BTreeSet::new();
    let mut blue_rounds = std::collections::BTreeSet::new();
    let mut out = Fight {
        closest: f64::INFINITY,
        ..Fight::default()
    };
    for _ in 0..seconds * 30 {
        b.step();
        let tick = b.tick();
        let squad = b.unit(UnitId(0)).unwrap();
        if squad.members.iter().any(|s| s.leaning(tick).is_some()) {
            out.leaning_ticks += 1;
        }
        for e in b.flight_events() {
            let FlightEvent::Impact(i) = e else { continue };
            if blue_rounds.contains(&i.projectile) && struck(i.struck) {
                out.struck_cover += 1;
            }
        }
        for (p, r) in b.rounds() {
            if !seen.insert(p.id) || r.side != Side::Blue || r.unit != UnitId(0) {
                continue;
            }
            let Some(shooter) = p.shooter else { continue };
            let Some(s) = squad.members.iter().find(|s| s.id == shooter.body.0) else {
                continue;
            };
            // Track rounds actually fired past this claimed lean body. A standing
            // soldier receiving a cover tier can still scatter into a nearby hull.
            if shooter.cover.is_some_and(&struck) {
                blue_rounds.insert(p.id);
            }
            let origin: V2 = (p.position - p.velocity * p.age_s).xy();
            if s.leaning(tick)
                .is_some_and(|l| (origin - l.at).length() < 0.2)
            {
                out.leaned += 1;
                out.closest = out.closest.min(cover.distance(origin));
            }
        }
    }
    out.red_hurt = b
        .unit(UnitId(1))
        .unwrap()
        .members
        .iter()
        .any(|s| s.hp < 1.0e6);
    out
}

/// Props are numbered in map order, so a lone cover body is prop 0.
fn prop_struck(_: [f64; 2]) -> impl Fn(Struck) -> bool {
    |s| matches!(s, Struck::Prop(0))
}

fn assert_leans_clear(f: &Fight) {
    assert!(f.leaned > 0, "someone leaned out to fire: {f:?}");
    assert!(f.leaning_ticks > 0, "the lean is sim state: {f:?}");
    assert!(
        f.closest >= RADIUS,
        "every lean point is clear of the cover by his radius: {f:?}"
    );
    assert_eq!(f.struck_cover, 0, "no round of his hit his cover: {f:?}");
    assert!(f.red_hurt, "his fire reached the enemy: {f:?}");
}

fn rect(center: [f64; 2], half: [f64; 2]) -> Obb2 {
    Obb2 {
        center: v2(center[0], center[1]),
        yaw: 0.0,
        half: v2(half[0], half[1]),
    }
}

#[test]
fn a_soldier_leans_out_round_a_trunk_to_fire() {
    let f = fight(
        json!([prop("trunk", [64.0, 45.0], [0.35, 0.35, 6.0])]),
        vec![],
        [61.0, 45.0],
        [110.0, 45.0],
        rect([64.0, 45.0], [0.35, 0.35]),
        prop_struck([64.0, 45.0]),
        30,
    );
    assert_leans_clear(&f);
}

#[test]
fn a_soldier_leans_out_round_a_building_corner_to_fire() {
    // The house's north-east corner at (70, 50); red north-east of it.
    let f = fight(
        json!([prop("building", [64.0, 44.0], [6.0, 6.0, 4.0])]),
        vec![],
        [60.0, 53.0],
        [110.0, 62.0],
        rect([64.0, 44.0], [6.0, 6.0]),
        prop_struck([64.0, 44.0]),
        30,
    );
    assert_leans_clear(&f);
}

#[test]
fn a_soldier_leans_out_round_a_wreck_to_fire() {
    let f = fight(
        json!([prop("heavy_wreck", [64.0, 45.0], [1.8, 3.5, 1.2])]),
        vec![],
        [60.0, 45.0],
        [110.0, 45.0],
        rect([64.0, 45.0], [1.8, 3.5]),
        prop_struck([64.0, 45.0]),
        30,
    );
    assert_leans_clear(&f);
}

#[test]
fn a_soldier_leans_out_round_a_parked_tanks_hull_to_fire() {
    let tank = json!({ "side": "blue", "kind": "test_tank", "position": [64, 45],
        "yaw": std::f64::consts::FRAC_PI_2, "engagement": "return_fire_only" });
    let f = fight(
        json!([]),
        vec![tank],
        [60.0, 45.0],
        [110.0, 45.0],
        rect([64.0, 45.0], [1.8, 3.5]),
        |s| matches!(s, Struck::Body(b) if b.0 == sim::weapons::VEHICLE_BODY_BASE + 2),
        30,
    );
    assert_leans_clear(&f);
}

#[test]
fn nobody_leans_round_a_trunk_when_neither_edge_gives_a_line() {
    // A tall wall a step east of the trunk shadows both its edges from red:
    // a man behind the trunk has no line either way, so no one leans round
    // it, and nobody stays behind it (the cover search rejects the spot).
    let mut rules = common::game();
    sim::fixtures::patch_catalog(
        &mut rules,
        "soldiers",
        "test_rifleman",
        json!({ "hp": 1.0e6 }),
    );
    let setup = serde_json::from_value(json!({
        "map": { "size": [160, 110], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
                 "props": [
                     prop("trunk", [64.0, 38.0], [0.35, 0.35, 6.0]),
                     prop("wall", [66.0, 38.0], [0.2, 2.0, 1.5]),
                 ], "forests": [] },
        "rules": rules,
        "units": [
            { "side": "blue", "kind": "test_rifle", "position": [60, 45] },
            { "side": "red", "kind": "test_rifle", "position": [110, 45] },
        ],
        "events": [], "scripts": [],
    }))
    .unwrap();
    let mut b = Battle::new(&setup, 1);
    let trunk = rect([64.0, 38.0], [0.35, 0.35]);
    let mut resolved = std::collections::BTreeSet::new();
    for _ in 0..30 * 30 {
        b.step();
        let squad = b.unit(UnitId(0)).unwrap();
        resolved.insert(squad.cover.resolved_at);
        for s in squad.members.iter().filter(|s| s.alive()) {
            let round_trunk = s.lean.is_some_and(|l| l.body == sim::lean::Round::Prop(0));
            assert!(!round_trunk, "a lean round the trunk: {:?}", s.lean);
        }
    }
    assert!(resolved.len() > 2, "the squad resolved cover: {resolved:?}");
    let squad = b.unit(UnitId(0)).unwrap();
    for s in squad.members.iter().filter(|s| s.alive()) {
        assert!(
            trunk.distance(s.position.xy()) > 1.0,
            "nobody sits behind the trunk he can't fight from: {:?}",
            s.position
        );
    }
}

#[test]
fn a_squad_holding_through_a_long_firefight_keeps_its_anchor_and_area() {
    // Blue at rest among trunks and sandbags; red's squad and a jeep's HMG
    // fire at will; shells land round blue and a wall is removed, so the
    // squad re-resolves again and again. Nothing it does moves its anchor,
    // and no soldier leaves the area round it (no drift).
    let mut rules = common::game();
    sim::fixtures::patch_catalog(
        &mut rules,
        "soldiers",
        "test_rifleman",
        json!({ "hp": 1.0e6 }),
    );
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jeep",
        json!({ "body": { "hull": { "hp": 1.0e6 } } }),
    );
    let mut props: Vec<Value> = (0..6)
        .map(|k| {
            prop(
                "trunk",
                [66.0 + 1.5 * (k % 2) as f64, 36.0 + 3.5 * k as f64],
                [0.35, 0.35, 6.0],
            )
        })
        .collect();
    props.push(prop("sandbags", [63.0, 45.0], [0.4, 3.0, 0.5]));
    props.push(prop("wall", [58.0, 52.0], [2.0, 0.3, 1.5]));
    // A lure: a line of low field walls running north, heavier cover than
    // anything round the squad. A squad whose area followed its soldiers
    // would chain up the line, re-resolve by re-resolve.
    props.extend((1..12).map(|k| prop("wall", [64.0, 52.0 + 5.0 * k as f64], [0.4, 1.5, 0.5])));
    let mut events: Vec<Value> = (0..60)
        .map(|k| {
            let (x, y) = (
                52.0 + 3.0 * (k % 7) as f64,
                30.0 + 5.0 * ((k / 7) % 6) as f64,
            );
            json!({ "tick": 60 + 90 * k, "burst": { "point": [x, y], "weapon": "tank_he" } })
        })
        .collect();
    // Three ATGM bursts on the wall's middle bring it down to rubble.
    events
        .extend((0..3).map(
            |_| json!({ "tick": 1800, "burst": { "point": [58.0, 52.0], "weapon": "atgm" } }),
        ));
    let setup = serde_json::from_value(json!({
        "map": { "size": [160, 110], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
                 "props": props, "forests": [] },
        "rules": rules,
        "units": [
            { "side": "blue", "kind": "test_rifle", "position": [60, 45] },
            { "side": "red", "kind": "test_rifle", "position": [112, 40] },
            { "side": "red", "kind": "test_jeep", "position": [110, 58], "yaw": std::f64::consts::PI },
        ],
        "events": events, "scripts": [],
    }))
    .unwrap();
    let mut b = Battle::new(&setup, 1);
    let anchor = b
        .unit(UnitId(0))
        .unwrap()
        .anchor
        .expect("a squad has an anchor");
    let radius = sim::cover::area_radius(b.rules(), b.unit(UnitId(0)).unwrap());
    let mut resolved = std::collections::BTreeSet::new();
    let mut farthest: f64 = 0.0;
    for _ in 0..240 * 30 {
        b.step();
        let squad = b.unit(UnitId(0)).unwrap();
        assert_eq!(squad.anchor, Some(anchor), "tick {}", b.tick());
        resolved.insert(squad.cover.resolved_at);
        for s in squad.members.iter().filter(|s| s.alive()) {
            farthest = farthest.max((s.position.xy() - anchor.at).length());
        }
    }
    assert!(resolved.len() >= 20, "many re-resolves: {}", resolved.len());
    // Inside the area; a body's breadth of slack for shoves between soldiers.
    assert!(
        farthest <= radius + 0.5,
        "farthest {farthest:.2} m, area {radius:.2} m"
    );
}

#[test]
fn a_fight_from_leaning_positions_replays_to_the_same_digest() {
    let mut rules = common::game();
    sim::fixtures::patch_catalog(
        &mut rules,
        "soldiers",
        "test_rifleman",
        json!({ "hp": 1.0e6 }),
    );
    let setup: contract::scenario::ScenarioDefinition = serde_json::from_value(json!({
        "map": { "size": [160, 110], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
                 "props": [prop("heavy_wreck", [64.0, 45.0], [1.8, 3.5, 1.2])], "forests": [] },
        "rules": rules,
        "units": [
            { "side": "blue", "kind": "test_rifle", "position": [60, 45] },
            { "side": "red", "kind": "test_rifle", "position": [110, 45] },
        ],
        "events": [], "scripts": [],
    }))
    .unwrap();
    let mut live = Battle::new(&setup, 4);
    let mut digests = Vec::new();
    let mut leaned = false;
    for _ in 0..600 {
        live.step();
        let tick = live.tick();
        leaned |= live
            .unit(UnitId(0))
            .unwrap()
            .members
            .iter()
            .any(|s| s.leaning(tick).is_some());
        digests.push(live.digest());
    }
    assert!(leaned, "the fight has someone leaning out");
    let mut replayed = Battle::from_replay(&setup, &live.replay()).unwrap();
    for (i, want) in digests.iter().enumerate() {
        replayed.step();
        assert_eq!(replayed.digest(), *want, "tick {}", i + 1);
    }
}

#[test]
fn a_lean_is_published_for_its_own_side_and_for_an_enemy_that_sees_him() {
    // Own: every leaning soldier, with his side and lean point. Enemy: the
    // same lean for each seen soldier, beside his tucked-in position.
    let mut rules = common::game();
    sim::fixtures::patch_catalog(
        &mut rules,
        "soldiers",
        "test_rifleman",
        json!({ "hp": 1.0e6 }),
    );
    let setup = serde_json::from_value(json!({
        "map": { "size": [160, 110], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
                 "props": [prop("heavy_wreck", [64.0, 45.0], [1.8, 3.5, 1.2])], "forests": [] },
        "rules": rules,
        "units": [
            { "side": "blue", "kind": "test_rifle", "position": [60, 45] },
            { "side": "red", "kind": "test_rifle", "position": [110, 45] },
        ],
        "events": [], "scripts": [],
    }))
    .unwrap();
    let mut b = Battle::new(&setup, 1);
    let (mut own, mut enemy) = (0, 0);
    for _ in 0..20 * 30 {
        b.step();
        let tick = b.tick();
        let squad = b.unit(UnitId(0)).unwrap();
        let living: Vec<_> = squad.members.iter().filter(|s| s.alive()).collect();
        let blue = b
            .observe(Side::Blue)
            .own
            .iter()
            .find(|u| u.members.len() == living.len());
        let blue = blue.expect("blue's squad is published");
        assert_eq!(blue.member_leans.len(), living.len());
        // Its markers show the anchor it was placed at, not its soldiers' middle.
        assert_eq!(blue.area.map(|a| a.anchor), Some([60.0, 45.0]));
        for (s, published) in living.iter().zip(&blue.member_leans) {
            let want = s.leaning(tick).map(|l| ([l.at.x, l.at.y], l.side));
            assert_eq!(published.map(|p| (p.at, p.side)), want);
            own += published.is_some() as usize;
        }
        for e in &b.observe(Side::Red).identified {
            assert_eq!(e.member_leans.len(), e.members.len());
            for (id, published) in e.member_ids.iter().zip(&e.member_leans) {
                let s = squad.members.iter().find(|s| s.id == *id).unwrap();
                assert_eq!(
                    published.map(|p| p.at),
                    s.leaning(tick).map(|l| [l.at.x, l.at.y])
                );
                enemy += published.is_some() as usize;
            }
        }
    }
    assert!(
        own > 0 && enemy > 0,
        "leans published: own {own}, seen {enemy}"
    );
}

#[test]
fn a_soldier_leans_out_for_a_burst_then_tucks_back_in() {
    // The film's picture: out past the tree, a burst, back behind it,
    // and out again. No stretch out lasts longer than a burst, and he stays
    // tucked in for the spell between.
    let mut rules = common::game();
    sim::fixtures::patch_catalog(
        &mut rules,
        "soldiers",
        "test_rifleman",
        json!({ "hp": 1.0e6 }),
    );
    let burst = rules["cover"]["lean_burst_s"].as_f64().unwrap();
    let tuck = rules["cover"]["lean_tuck_s"].as_f64().unwrap();
    let setup = serde_json::from_value(json!({
        "map": { "size": [160, 110], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
                 "props": [prop("trunk", [64.0, 45.0], [0.35, 0.35, 6.0])], "forests": [] },
        "rules": rules,
        "units": [
            { "side": "blue", "kind": "test_rifle", "position": [61, 45] },
            { "side": "red", "kind": "test_rifle", "position": [110, 45] },
        ],
        "events": [], "scripts": [],
    }))
    .unwrap();
    let mut b = Battle::new(&setup, 1);
    let hz = b.rules().tick_hz as f64;
    // Per soldier: the stretches out and in, in seconds, in order.
    let mut stretches: std::collections::BTreeMap<u32, Vec<(bool, f64)>> = Default::default();
    for _ in 0..40 * 30 {
        b.step();
        let tick = b.tick();
        for s in b.unit(UnitId(0)).unwrap().members.iter() {
            let out = s.leaning(tick).is_some();
            let runs = stretches.entry(s.id).or_default();
            match runs.last_mut() {
                Some((was, secs)) if *was == out => *secs += 1.0 / hz,
                _ => runs.push((out, 1.0 / hz)),
            }
        }
    }
    let leaners: Vec<_> = stretches
        .values()
        .filter(|r| r.iter().any(|s| s.0))
        .collect();
    assert!(!leaners.is_empty(), "someone leaned out");
    let mut cycles = 0;
    for runs in leaners {
        for (k, &(out, secs)) in runs.iter().enumerate() {
            if out {
                assert!(
                    secs <= burst + 0.1,
                    "out {secs:.2} s, a burst is {burst} s: {runs:?}"
                );
            } else if k > 0 && k + 1 < runs.len() {
                // Between two stretches out: tucked in for the spell at least.
                assert!(
                    secs >= tuck - 0.1,
                    "in {secs:.2} s between bursts: {runs:?}"
                );
                cycles += 1;
            }
        }
    }
    assert!(cycles > 0, "out, in and out again: {stretches:?}");
}
