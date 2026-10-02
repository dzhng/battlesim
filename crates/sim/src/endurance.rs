//! The endurance load (validation.md): a synthetic, clearly labelled
//! stress battle for the scale verdict, not a play fixture. 100 units a side
//! (50 eight-soldier rifle squads, 50 vehicles and specialists) on a
//! 3 × 2 km field with woods and buildings, the catalogue's saved `endurance`
//! map. Seeded scripted orders keep the
//! fight turning over for an hour: waves of attack-moves to shifting points,
//! reserves arriving from the rear edge, supply trucks setting up behind the
//! lines. The optional late state adds 20,000 corpses and 2,000 wrecks as
//! stress input. Rules come from the one fixture owner (game.json).
use contract::command::{Order, TargetRef};
use contract::ids::{Side, UnitId};
use contract::map::MapDefinition;
use contract::scenario::{Rules, ScenarioDefinition, ScriptedOrder, UnitCondition, UnitSetup};
use serde_json::json;

use crate::rng::Rng;

/// Per side: (unit type, count). 100 units, 50 of them rifle squads.
const ROSTER: [(&str, usize); 5] = [
    ("rifle", 50),
    ("tank", 20),
    ("at", 12),
    ("recon", 10),
    ("supply", 8),
];
/// Every this many units of a side form one wave group.
const GROUP: usize = 10;
/// A wave every this many seconds, over this long; the wave at minute 30
/// is a burst of speculative fire instead of an advance.
const WAVE_S: u64 = 120;
const HOUR_S: u64 = 3600;
pub const BURST_WAVE: u64 = 15;
/// The late state: fallen squads and wrecks strewn over the field.
pub const LATE_CORPSES: usize = 20_000;
pub const LATE_WRECKS: usize = 2_000;

/// The endurance battle for `seed` on `field`, its saved map (the
/// catalogue's `endurance`); `late` adds the synthetic remains: the wrecks to
/// the map, as further authored props, and the fallen squads to the forces.
pub fn scenario(
    field: &MapDefinition,
    fixture: &serde_json::Value,
    seed: u64,
    late: bool,
) -> Result<ScenarioDefinition, String> {
    scenario_in_arena(field, fixture, seed, late, [0.0, 0.0, field.size[0]])
}

/// The same stress recipe concentrated in a central 3 × 2 km arena, with
/// the entire generated world loaded. This measures local contact at city
/// density, separately from `city_report`'s edge-to-edge transit workload.
pub fn city_scenario(
    field: &MapDefinition,
    fixture: &serde_json::Value,
    seed: u64,
    late: bool,
) -> Result<ScenarioDefinition, String> {
    if field.size[0] < 3000.0 || field.size[1] < 2000.0 {
        return Err("city stress needs a world at least 3 × 2 km".into());
    }
    let mut setup = scenario_in_arena(
        field,
        fixture,
        seed,
        late,
        [
            (field.size[0] - 3000.0) / 2.0,
            (field.size[1] - 2000.0) / 2.0,
            3000.0,
        ],
    )?;
    // The synthetic arena may cross generated buildings or water. Resolve
    // living starts with the production placement predicate, within 200 m.
    let world = crate::world::WorldGeometry::new(&setup.map, &setup.rules);
    let grid =
        crate::navigation::NavGrid::new(std::sync::Arc::new(crate::navigation::NavBase::build(
            &world,
            world.props(),
            setup.rules.physics.soldier_radius_m,
        )));
    let mut placed: Vec<crate::math::V2> = Vec::new();
    for unit in setup.units.iter_mut().filter(|u| u.condition.is_none()) {
        // Put the active fronts within mutual weapon reach; the unmodified
        // rear-edge recipe can spend minutes planning without firing a round.
        unit.position[0] += if unit.side == Side::Blue {
            850.0
        } else {
            -850.0
        };
        let wanted = crate::math::v2(unit.position[0], unit.position[1]);
        let mobility = crate::units::mobility(setup.rules.catalog.by_id(&unit.kind), &setup.rules);
        let mut found = None;
        'search: for ring in 0i32..=20 {
            for y in -ring..=ring {
                for x in -ring..=ring {
                    if x.abs().max(y.abs()) != ring || x * x + y * y > 20 * 20 {
                        continue;
                    }
                    let p = wanted + crate::math::v2(x as f64 * 10.0, y as f64 * 10.0);
                    if let Some(p) = grid
                        .placement_point(p, &mobility)
                        .filter(|p| placed.iter().all(|q| (*p - *q).length() >= 10.0))
                    {
                        found = Some(p);
                        break 'search;
                    }
                }
            }
        }
        let p = found.ok_or_else(|| {
            format!(
                "city stress cannot place {} near {:?}",
                unit.kind, unit.position
            )
        })?;
        unit.position = [p.x, p.y];
        placed.push(p);
    }
    Ok(setup)
}

fn scenario_in_arena(
    field: &MapDefinition,
    fixture: &serde_json::Value,
    seed: u64,
    late: bool,
    arena: [f64; 3],
) -> Result<ScenarioDefinition, String> {
    let [left, bottom, width] = arena;
    let rules: Rules = serde_json::from_value(fixture.clone()).map_err(|e| e.to_string())?;
    // Separate streams, so the late state's remains never move the waves.
    let mut rng = Rng::new(seed);
    let mut remains = Rng::new(seed ^ 0x005e_ed0f_4e3a_1175);
    let mut map = field.clone();
    if late {
        let first_wreck = map.authored_props()?.len();
        for wreck_index in 0..LATE_WRECKS {
            let (x, y) = (
                left + 400.0 + remains.unit() * 2200.0,
                bottom + 100.0 + remains.unit() * 1800.0,
            );
            let wreck = json!({ "id": first_wreck + wreck_index, "kind": "heavy_wreck", "center": [x, y], "yaw": remains.unit() * std::f64::consts::TAU,
                "half_extents": [3.5, 1.8, 1.2] });
            map.props
                .push(serde_json::from_value(wreck).map_err(|e| e.to_string())?);
        }
    }
    let mut units = Vec::new();
    let mut scripts = Vec::new();
    for side in Side::ALL {
        let east = side == Side::Red;
        let x_of = |depth: f64| left + if east { width - depth } else { depth };
        let yaw = if east { std::f64::consts::PI } else { 0.0 };
        let first = units.len() as u32;
        let mut n = 0usize;
        for (kind, count) in ROSTER {
            for _ in 0..count {
                // A fifth of each side waits at the rear edge as reserves.
                let reserve = n % 5 == 4;
                let depth = if reserve {
                    60.0
                } else {
                    250.0 + (n % 4) as f64 * 60.0
                };
                let y = bottom + 150.0 + (n as f64 * 17.0) % 1700.0;
                units.push(UnitSetup {
                    side,
                    kind: kind.to_string(),
                    position: [x_of(depth), y],
                    yaw,
                    engagement: None,
                    condition: None,
                    stock: None,
                });
                n += 1;
            }
        }
        let ids: Vec<UnitId> = (first..first + n as u32).map(UnitId).collect();
        let supplies = |id: &&UnitId| {
            let kind = units_kind(&ROSTER, (id.0 - first) as usize);
            rules.catalog.by_id(kind).has_role("logistics")
        };
        let supply: Vec<UnitId> = ids.iter().filter(supplies).copied().collect();
        let fighters: Vec<UnitId> = ids.iter().filter(|id| !supplies(id)).copied().collect();
        // The trucks set up at the rear, behind the start line.
        for (k, t) in supply.iter().enumerate() {
            scripts.push(ScriptedOrder {
                tick: 1,
                side,
                order: Order::Move {
                    units: vec![*t],
                    gesture: 1 + k as u64,
                    goal: [x_of(150.0), bottom + 200.0 + k as f64 * 220.0],
                    route: contract::command::RoutePolicy::Shortest,
                    direction: contract::command::MoveDirection::Forward,
                    facing: None,
                },
                queued: false,
            });
            scripts.push(ScriptedOrder {
                tick: 60 * rules.tick_hz as u64,
                side,
                order: Order::SetDeployment {
                    units: vec![*t],
                    deployed: true,
                },
                queued: false,
            });
        }
        // Waves: every group attack-moves to a seeded point in the middle
        // band; reserves join from the second wave on.
        let mut gesture = 1000;
        for wave in 0..HOUR_S / WAVE_S {
            let tick = (wave * WAVE_S + 5) * rules.tick_hz as u64;
            for group in fighters.chunks(GROUP) {
                let group: Vec<UnitId> = group
                    .iter()
                    .copied()
                    .filter(|id| wave > 0 || (id.0 - first) as usize % 5 != 4)
                    .collect();
                if group.is_empty() {
                    continue;
                }
                let goal = [
                    x_of(1100.0 + rng.unit() * 500.0),
                    bottom + 150.0 + rng.unit() * 1700.0,
                ];
                gesture += 1;
                let order = if wave == BURST_WAVE {
                    // Everyone fires at the ground across the middle.
                    let point = [left + width / 2.0, goal[1], 0.0];
                    Order::Attack {
                        units: group,
                        target: TargetRef::Ground { point },
                    }
                } else {
                    Order::AttackMove {
                        units: group,
                        gesture,
                        goal,
                    }
                };
                scripts.push(ScriptedOrder {
                    tick,
                    side,
                    order,
                    queued: false,
                });
            }
        }
    }
    if late {
        // Whole squads already fallen, their soldiers lying where they stood.
        let squad = rules.catalog.by_id("rifle").squad_size();
        for k in 0..LATE_CORPSES.div_ceil(squad) {
            let (x, y) = (
                left + 400.0 + remains.unit() * 2200.0,
                bottom + 100.0 + remains.unit() * 1800.0,
            );
            units.push(UnitSetup {
                side: if k % 2 == 0 { Side::Blue } else { Side::Red },
                kind: "rifle".to_string(),
                position: [x, y],
                yaw: remains.unit() * std::f64::consts::TAU,
                engagement: None,
                condition: Some(UnitCondition {
                    casualties: squad as u32,
                    ..Default::default()
                }),
                stock: None,
            });
        }
    }
    Ok(ScenarioDefinition {
        map,
        rules,
        units,
        events: Vec::new(),
        scripts,
        opponent: None,
        encounter: None,
    })
}

/// The unit type of a side's `n`-th unit in roster order.
fn units_kind(roster: &[(&'static str, usize)], mut n: usize) -> &'static str {
    for &(kind, count) in roster {
        if n < count {
            return kind;
        }
        n -= count;
    }
    unreachable!("index beyond the roster")
}
