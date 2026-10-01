//! The endurance load (validation.md): a synthetic, clearly labelled
//! stress battle for the scale verdict, not a play fixture. 100 units a side
//! (50 eight-soldier rifle squads, 50 vehicles and specialists) on a
//! 3 × 2 km field with woods and buildings. Seeded scripted orders keep the
//! fight turning over for an hour: waves of attack-moves to shifting points,
//! reserves arriving from the rear edge, supply trucks setting up behind the
//! lines. The optional late state adds 20,000 corpses and 2,000 wrecks as
//! stress input. Rules come from the one fixture owner (village.json).
use contract::command::{Order, TargetRef};
use contract::ids::{Side, UnitId};
use contract::scenario::{Rules, ScenarioDefinition, ScriptedOrder, UnitCondition, UnitSetup};
use serde_json::json;

use crate::rng::Rng;

pub const FIELD: [f64; 2] = [3000.0, 2000.0];
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

/// The endurance battle for `seed`; `late` adds the synthetic remains.
pub fn scenario(
    fixture: &serde_json::Value,
    seed: u64,
    late: bool,
) -> Result<ScenarioDefinition, String> {
    let rules: Rules = serde_json::from_value(fixture.clone()).map_err(|e| e.to_string())?;
    // Separate streams, so the late state's remains never move the waves.
    let mut rng = Rng::new(seed);
    let mut remains = Rng::new(seed ^ 0x005e_ed0f_4e3a_1175);
    let mut props = Vec::new();
    let template = contract::templates::BuildingTemplateDescriptor::solid_box(
        "api-box-12-10-4".into(),
        contract::templates::BuildingCategory::Farmstead,
        "api_fixture".into(),
        [12.0, 10.0, 4.0],
    );
    let catalogue = contract::templates::TemplateGeometryCatalog::new(vec![template.clone()])?;
    let mut buildings = Vec::new();
    // A village of buildings in each third of the field's middle band.
    for cx in [1100.0, 1500.0, 1900.0] {
        for k in 0..4 {
            let (dx, dy) = ((k % 2) as f64 * 60.0 - 30.0, (k / 2) as f64 * 60.0 - 30.0);
            let owner = buildings.len() as u32;
            buildings.push(contract::map::BuildingDefinition::materialize(
                &template,
                contract::templates::PlacementFrame {
                    translation: [cx + dx, 1000.0 + dy, 0.0],
                    yaw: 0.0,
                },
                "building".into(),
                owner,
                vec![contract::map::BuildingPartReference {
                    part: "body".into(),
                    prop: owner,
                }],
            )?);
        }
    }
    if late {
        for _ in 0..LATE_WRECKS {
            let (x, y) = (
                400.0 + remains.unit() * 2200.0,
                100.0 + remains.unit() * 1800.0,
            );
            props.push(
                json!({ "id": buildings.len()+props.len(), "kind": "heavy_wreck", "center": [x, y], "yaw": remains.unit() * std::f64::consts::TAU,
                "half_extents": [3.5, 1.8, 1.2] }),
            );
        }
    }
    let map = json!({
        "size": FIELD, "fog_cell_m": 8, "height_grid_m": 8, "slope_cutoff_deg": 35,
        "relief": [{ "kind": "ridge", "center": [1500, 500], "peak_m": 15, "radius_m": 250 }],
        "forests": [
            { "shape":{"kind":"polygon","ring":[[1300.0,1400.0],[1550.0,1400.0],[1550.0,1600.0],[1300.0,1600.0]]} },
            { "shape":{"kind":"polygon","ring":[[1500.0,250.0],[1700.0,250.0],[1700.0,500.0],[1500.0,500.0]]} }
        ],
        "props": props,"buildings":buildings,"template_catalog_hash":catalogue.hash(),
    });
    let map = serde_json::from_value(map).map_err(|e| e.to_string())?;
    let mut units = Vec::new();
    let mut scripts = Vec::new();
    for side in Side::ALL {
        let east = side == Side::Red;
        let x_of = |depth: f64| if east { FIELD[0] - depth } else { depth };
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
                let y = 150.0 + (n as f64 * 17.0) % 1700.0;
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
                    goal: [x_of(150.0), 200.0 + k as f64 * 220.0],
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
                    150.0 + rng.unit() * 1700.0,
                ];
                gesture += 1;
                let order = if wave == BURST_WAVE {
                    // Everyone fires at the ground across the middle.
                    let point = [FIELD[0] / 2.0, goal[1], 0.0];
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
                400.0 + remains.unit() * 2200.0,
                100.0 + remains.unit() * 1800.0,
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
