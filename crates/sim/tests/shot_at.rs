//! Shot at: each own unit's published flag says an enemy round aimed at it
//! passed near it, or struck it, on that tick. It is presentation evidence
//! (the deck's yellow blink), so it never reaches the battle's outcome.
use std::collections::BTreeMap;

use crate::common as test_common;
use contract::command::{Order, TargetRef};
use contract::ids::{Side, UnitId};
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::flight::{FlightEvent, ProjectileId, Struck};
use sim::weapons::VEHICLE_BODY_BASE;

const BLUE_TANK: UnitId = UnitId(0);
const AIMED_AT: UnitId = UnitId(1);
const BYSTANDER: UnitId = UnitId(2);

/// What one tick showed: red's published flags, and blue's rounds with what
/// each touched (passed near or struck) this tick.
struct Tick {
    shot_at: BTreeMap<UnitId, bool>,
    near: Vec<(UnitId, Option<UnitId>)>,
    struck: Vec<(UnitId, Option<UnitId>)>,
}

/// Blue's tank duels a red tank 320 m off; its rounds fly past a second red
/// tank 6 m beside the line of fire 160 m out. Every tank fires only back.
fn duel(edit: impl FnOnce(&mut Value)) -> Vec<Tick> {
    let mut rules = test_common::scenario_rules();
    edit(&mut rules);
    let map = json!({ "size": [1200.0, 600.0], "fog_cell_m": 8, "height_grid_m": 4,
        "slope_cutoff_deg": 35, "props": [] });
    let units = json!([
        { "side": "blue", "kind": "test_tank", "position": [100, 300], "engagement": "return_fire_only" },
        { "side": "red", "kind": "test_tank", "position": [420, 300], "engagement": "return_fire_only" },
        { "side": "red", "kind": "test_tank", "position": [260, 306], "engagement": "return_fire_only" },
    ]);
    let setup = serde_json::from_value(json!({
        "map": test_common::physical_map(&map.to_string()), "rules": rules,
        "units": units, "events": [], "scripts": [] }))
    .unwrap();
    let mut b = Battle::new(&setup, 4);
    let mut ordered = false;
    let mut ticks = Vec::new();
    for _ in 0..20 * b.rules().tick_hz {
        let aimed: BTreeMap<ProjectileId, Option<UnitId>> = b
            .rounds()
            .filter(|(_, r)| r.side == Side::Blue)
            .map(|(p, r)| (p.id, r.aimed_at))
            .collect();
        b.step();
        let mut tick = Tick {
            shot_at: b
                .observe(Side::Red)
                .own
                .iter()
                .map(|u| (u.id, u.shot_at))
                .collect(),
            near: Vec::new(),
            struck: Vec::new(),
        };
        for event in b.flight_events() {
            let Some(&aim) = aimed.get(&event.projectile()) else {
                continue;
            };
            let hull = |body: u32| UnitId(body.wrapping_sub(VEHICLE_BODY_BASE));
            match event {
                FlightEvent::NearMiss(n) => tick.near.push((n.unit, aim)),
                FlightEvent::Impact(i) => {
                    if let Struck::Body(body) = i.struck {
                        tick.struck.push((hull(body.0), aim));
                    }
                }
                FlightEvent::Ricochet(r) => tick.struck.push((hull(r.body.0), aim)),
                _ => {}
            }
        }
        ticks.push(tick);
        if !ordered {
            let seen = b.observe(Side::Blue).identified.iter().find(|u| {
                (u.position[0] - 420.0).abs() < 1.0 && (u.position[1] - 300.0).abs() < 1.0
            });
            if let Some(target) = seen {
                let order = Order::Attack {
                    units: vec![BLUE_TANK],
                    target: TargetRef::Identified { id: target.id },
                };
                test_common::order(&mut b, Side::Blue, 1, order);
                ordered = true;
            }
        }
    }
    ticks
}

impl Tick {
    /// A round aimed at `unit` passed near it or struck it.
    fn aimed_round_touched(&self, unit: UnitId) -> bool {
        let aimed = |&&(u, aim): &&(UnitId, Option<UnitId>)| u == unit && aim == Some(unit);
        self.near.iter().any(|e| aimed(&e)) || self.struck.iter().any(|e| aimed(&e))
    }
}

/// Whenever a red unit is flagged, a blue round aimed at it touched it.
fn flags_only_aimed_rounds(ticks: &[Tick]) {
    for (t, tick) in ticks.iter().enumerate() {
        for (&unit, &flag) in &tick.shot_at {
            assert!(
                !flag || tick.aimed_round_touched(unit),
                "tick {t}: {unit:?}"
            );
        }
    }
}

#[test]
fn a_round_aimed_at_a_unit_flags_it_and_not_the_unit_it_flies_past() {
    let ticks = duel(|_| {});
    flags_only_aimed_rounds(&ticks);
    let flagged = ticks
        .iter()
        .filter(|t| t.shot_at.get(&AIMED_AT) == Some(&true));
    assert!(flagged.count() > 0, "the aimed-at tank is shot at");
    // The tank's own machine gun takes the bystander on its own, so judge
    // only the ticks a round meant for the other tank flies past it alone.
    let passed_by_others: Vec<&Tick> = ticks
        .iter()
        .filter(|t| t.near.contains(&(BYSTANDER, Some(AIMED_AT))))
        .filter(|t| !t.aimed_round_touched(BYSTANDER))
        .collect();
    assert!(!passed_by_others.is_empty(), "rounds pass the bystander");
    for tick in passed_by_others {
        assert_eq!(tick.shot_at.get(&BYSTANDER), Some(&false));
    }

    // Rounds no one hears pass (no suppression) still flag the unit they
    // strike, glancing or not: being struck is being shot at.
    let ticks = duel(|rules| {
        for row in ["tank_ap", "tank_he", "hmg"] {
            rules["weapons"][row]["near_miss_suppression"] = json!(0.0);
        }
    });
    flags_only_aimed_rounds(&ticks);
    let mut struck = 0;
    for (t, tick) in ticks.iter().enumerate() {
        let Some(&flag) = tick.shot_at.get(&AIMED_AT) else {
            continue;
        };
        let hit = tick.struck.contains(&(AIMED_AT, Some(AIMED_AT)));
        struck += hit as usize;
        assert_eq!(flag, hit, "tick {t}");
    }
    assert!(struck > 0, "blue's rounds strike the tank");
}
