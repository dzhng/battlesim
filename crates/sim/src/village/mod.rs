//! The village encounter: the authored scenario built from the one fixture,
//! and blue's comparison scripts played against the encounter's defender and
//! referee ([`crate::encounter::Defender`], [`crate::encounter::Referee`]).
use std::collections::BTreeMap;

use contract::command::{Engagement, Order};
use contract::ids::{Side, UnitId};
use contract::map::MapDefinition;
use contract::observation::EncounterResult;
use contract::scenario::{EncounterRules, Opponent, Rules, ScenarioDefinition, UnitSetup};
use serde::Deserialize;

pub mod scripts;

/// The village fixture's own sections (the rest is `Rules`).
#[derive(Deserialize)]
struct Fixture {
    map: MapDefinition,
    spawn: Spawns,
    variants: BTreeMap<String, Variant>,
    defender_policy: DefenderPolicy,
    encounter: EncounterFixture,
}

#[derive(Deserialize)]
struct Spawns {
    blue: Vec<SpawnRow>,
    red: Vec<SpawnRow>,
}

/// One spawn row: `[type id, x, y]`, or `[type id, x, y, engagement]` to
/// set the unit's starting fire policy over its side's default.
struct SpawnRow {
    kind: String,
    x: f64,
    y: f64,
    engagement: Option<Engagement>,
}

impl<'de> Deserialize<'de> for SpawnRow {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Row;
        impl<'de> serde::de::Visitor<'de> for Row {
            type Value = SpawnRow;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a spawn row [type, x, y] or [type, x, y, engagement]")
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut a: A,
            ) -> Result<SpawnRow, A::Error> {
                use serde::de::Error;
                let short = |n| A::Error::invalid_length(n, &self);
                let row = SpawnRow {
                    kind: a.next_element()?.ok_or_else(|| short(0))?,
                    x: a.next_element()?.ok_or_else(|| short(1))?,
                    y: a.next_element()?.ok_or_else(|| short(2))?,
                    engagement: a.next_element()?,
                };
                if a.next_element::<serde::de::IgnoredAny>()?.is_some() {
                    return Err(A::Error::invalid_length(5, &self));
                }
                Ok(row)
            }
        }
        d.deserialize_seq(Row)
    }
}

#[derive(Deserialize)]
struct Variant {
    disabled_red_spawn_indices: Vec<usize>,
}

#[derive(Deserialize)]
struct DefenderPolicy {
    /// (red spawn index, building index among the map's buildings).
    initial_garrisons: Vec<(usize, usize)>,
    at_attack_range_m: f64,
    tank_retreat_hp_fraction: f64,
    tank_fallback: [f64; 2],
    infantry_retreat_survivor_fraction: f64,
    infantry_fallback: [f64; 2],
}

#[derive(Deserialize)]
struct EncounterFixture {
    success_zone_center: [f64; 2],
    success_zone_radius_m: f64,
    hold_s: f64,
    max_assessment_s: f64,
}

/// The authored encounter for `variant` ("ordinary" or "prepared_crossfire"):
/// blue attacks from the west, red defends the village with the policy.
pub fn scenario(fixture: &serde_json::Value, variant: &str) -> Result<ScenarioDefinition, String> {
    let f: Fixture = serde_json::from_value(fixture.clone()).map_err(|e| e.to_string())?;
    let rules: Rules = serde_json::from_value(fixture.clone()).map_err(|e| e.to_string())?;
    let v = f
        .variants
        .get(variant)
        .ok_or_else(|| format!("no variant {variant}"))?;
    let setup = |side, kind: &String, x, y, yaw, engagement| UnitSetup {
        side,
        kind: kind.clone(),
        position: [x, y],
        yaw,
        engagement,
        condition: None,
        stock: None,
    };
    let mut units: Vec<UnitSetup> = f
        .spawn
        .blue
        .iter()
        .map(|r| setup(Side::Blue, &r.kind, r.x, r.y, 0.0, r.engagement))
        .collect();
    // Red spawn index → unit id, skipping the variant's disabled spawns.
    let mut red_ids = BTreeMap::new();
    for (i, r) in f.spawn.red.iter().enumerate() {
        if v.disabled_red_spawn_indices.contains(&i) {
            continue;
        }
        red_ids.insert(i, units.len() as u32);
        // AT teams start holding fire unless their row says otherwise; the
        // rest fire at will (encounter.md).
        let at = rules.catalog.by_id(&r.kind).has_role("at");
        let engagement = r.engagement.or(at.then_some(Engagement::ReturnFireOnly));
        units.push(setup(
            Side::Red,
            &r.kind,
            r.x,
            r.y,
            std::f64::consts::PI,
            engagement,
        ));
    }
    // Building index among the map's garrisonable owners → stable prop id.
    let props = rules.catalog.props();
    let buildings: Vec<u32> = f
        .map
        .buildings
        .iter()
        .filter(|b| props.by_id(&b.kind).body.garrison)
        .map(|b| b.owner)
        .collect();
    let p = &f.defender_policy;
    let garrisons = p
        .initial_garrisons
        .iter()
        .filter_map(|&(spawn, b)| Some([*red_ids.get(&spawn)?, *buildings.get(b)?]))
        .collect();
    Ok(ScenarioDefinition {
        skirmish: None,
        map: f.map,
        rules,
        units,
        events: Vec::new(),
        scripts: Vec::new(),
        opponent: Some(Opponent {
            side: Side::Red,
            garrisons,
            at_attack_range_m: p.at_attack_range_m,
            tank_retreat_hp_fraction: p.tank_retreat_hp_fraction,
            tank_fallback: p.tank_fallback,
            infantry_retreat_survivor_fraction: p.infantry_retreat_survivor_fraction,
            infantry_fallback: p.infantry_fallback,
        }),
        encounter: Some(EncounterRules {
            attacker: Side::Blue,
            success_zone_center: f.encounter.success_zone_center,
            success_zone_radius_m: f.encounter.success_zone_radius_m,
            hold_s: f.encounter.hold_s,
            max_assessment_s: f.encounter.max_assessment_s,
        }),
    })
}

/// Blue commanded by a comparison script, as a player would: before each
/// step it reads blue's fresh observation and sends the script's orders
/// through `Battle::accept`, so they are recorded and replay like input. It
/// is blue's only commander: its sequence numbers start at 1.
#[derive(Clone, Debug)]
pub struct ScriptedBlue {
    script: scripts::Script,
    rules: Rules,
    seq: u64,
    /// Orders the battle refused (a script bug when not zero).
    pub rejected: u32,
    /// Replenished units whose return to the fight the battle admitted.
    pub rejoined: u32,
}

impl ScriptedBlue {
    pub fn new(plan: scripts::Plan, setup: &ScenarioDefinition) -> Self {
        ScriptedBlue {
            script: scripts::Script::new(plan, setup),
            rules: setup.rules.clone(),
            seq: 0,
            rejected: 0,
            rejoined: 0,
        }
    }

    /// Issue this tick's orders; call it before `Battle::step`.
    pub fn command(&mut self, battle: &mut crate::battle::Battle) {
        let orders = self.script.orders(battle.observe(Side::Blue), &self.rules);
        for order in orders {
            self.accept(battle, order);
        }
        for (unit, goals) in self.script.rejoins(battle.observe(Side::Blue), &self.rules) {
            let goal = goals.into_iter().find(|&goal| {
                battle
                    .preview_move(
                        Side::Blue,
                        &contract::command::MovePreviewRequest {
                            units: vec![unit],
                            goal,
                            ..Default::default()
                        },
                    )
                    .is_ok_and(|places| places.iter().any(|p| p.placed))
            });
            let accepted = goal
                .and_then(|g| self.script.attack_move(vec![unit], g))
                .is_some_and(|order| self.accept(battle, order).error.is_none());
            self.script.rejoin_checked(unit, battle.tick(), accepted);
            self.rejoined += u32::from(accepted);
        }
    }

    fn accept(
        &mut self,
        battle: &mut crate::battle::Battle,
        order: Order,
    ) -> contract::command::CommandAck {
        self.seq += 1;
        let ack = battle.accept(contract::command::CommandEnvelope {
            side: Side::Blue,
            seq: self.seq,
            order,
            queued: false,
        });
        self.rejected += ack.error.is_some() as u32;
        ack
    }
}

/// One scripted trial's outcome, measured by the referee and the ledger.
#[derive(Clone, Debug, PartialEq)]
pub struct Trial {
    pub result: EncounterResult,
    /// Seconds until the village was held, if it was.
    pub captured_s: Option<f64>,
    /// Blue losses in cost points: a destroyed vehicle's cost, and each
    /// fallen soldier's share of its squad's cost.
    pub blue_cost_lost: f64,
    pub tanks_lost: u32,
    /// Replenished units whose return order the battle admitted.
    pub rejoined: u32,
    /// Script orders the battle refused (a script bug when not zero).
    pub rejected: u32,
    /// Blue's tanks at the start.
    pub tanks: u32,
    pub digest: u64,
}

/// Run `plan` for blue against the defender in `variant` with `seed`, until a
/// verdict or `max_s` seconds. Blue's orders go through `Battle::accept`.
pub fn trial(
    fixture: &serde_json::Value,
    variant: &str,
    plan: scripts::Plan,
    seed: u64,
    max_s: f64,
) -> Trial {
    use crate::battle::Battle;
    let setup = scenario(fixture, variant).expect("the village fixture builds");
    let rules = setup.rules.clone();
    let mut battle = Battle::new(&setup, seed);
    let mut blue = ScriptedBlue::new(plan, &setup);
    let max_ticks = (max_s * rules.tick_hz as f64) as u64;
    let mut captured_s = None;
    while battle.tick() < max_ticks {
        blue.command(&mut battle);
        battle.step();
        match battle.encounter().map(|e| e.result) {
            Some(EncounterResult::Captured) => {
                captured_s = Some(battle.tick() as f64 / rules.tick_hz as f64);
                break;
            }
            Some(EncounterResult::Defeated) => break,
            _ => {}
        }
    }
    let mut blue_cost_lost = 0.0;
    let mut tanks_lost = 0;
    for i in 0..setup.units.len() {
        let Some(unit) = battle.unit(UnitId(i as u32)) else {
            continue;
        };
        if unit.side != Side::Blue {
            continue;
        }
        let t = unit.unit_type(&rules);
        let cost = t.cost as f64;
        if unit.hull.is_some() {
            if !unit.alive() {
                blue_cost_lost += cost;
                tanks_lost += t.has_role("mbt") as u32;
            }
        } else {
            let fallen = unit.members.iter().filter(|s| !s.alive()).count() as f64;
            blue_cost_lost += fallen * cost / t.squad_size() as f64;
        }
    }
    Trial {
        result: battle
            .encounter()
            .map_or(EncounterResult::Running, |e| e.result),
        captured_s,
        blue_cost_lost,
        tanks_lost,
        rejoined: blue.rejoined,
        rejected: blue.rejected,
        tanks: setup
            .units
            .iter()
            .filter(|u| u.side == Side::Blue && rules.catalog.by_id(&u.kind).has_role("mbt"))
            .count() as u32,
        digest: battle.digest(),
    }
}
