//! The skirmish AI's pull toward its own lane, with the shipped tuning, on
//! generated maps: at the battle's start every AI plans on the road its slot
//! holds (`sim::entries::homes`), whoever sits in the other slots.
use contract::ids::Side;
use contract::observation::ObservationFrame;
use contract::players::{ControllerKind, Player, PlayerIndex};
use sim::skirmish::PlayerState;
use sim::skirmish_ai::team::{plan, TeamMemory, TeamTuning};

const PRESETS: &str = include_str!("../../../fixtures/map-presets.json");
const TEMPLATES: &str = include_str!("../../../fixtures/prototype-building-templates.json");
const PHYSICAL_RULES: &str =
    include_str!("../../../fixtures/parity/map-layout/physical-rules.json");

/// `per_side` a side, all AI but Blue's `human` slot (from 1; 0 for none).
fn players(per_side: usize, human: usize) -> Vec<Player> {
    Side::COMBATANTS
        .into_iter()
        .flat_map(|side| {
            (1..=per_side).map(move |slot| {
                let you = side == Side::Blue && slot == human;
                let name = if side == Side::Blue { "blue" } else { "red" };
                serde_json::from_value(serde_json::json!({
                    "id": format!("{name}-{slot}"), "side": name,
                    "faction": if side == Side::Blue { "us" } else { "eastern" },
                    "controller": if you { "human" } else { "active_ai" },
                    "human_capable": you,
                }))
                .unwrap()
            })
        })
        .collect()
}

/// The first `size` mixed skirmish map with `roads` a side that generates
/// and that the simulation admits for `seated`, and its admitted sites.
/// Both may refuse a seed, so this draws seeds until both accept one.
fn admitted_sites(
    size: &str,
    roads: usize,
    seated: &[Player],
    rules: &contract::scenario::Rules,
) -> contract::encounter::SkirmishSites {
    let catalogue =
        contract::templates::TemplateGeometryCatalog::new(serde_json::from_str(TEMPLATES).unwrap())
            .unwrap();
    let presets = mapgen::layout::PresetDefinitions::from_json(PRESETS).unwrap();
    for seed in 1..30u64 {
        let request = serde_json::json!({
            "generator_version": mapgen::layout::GENERATOR_VERSION,
            "preset_revision": presets.revision, "seed": seed.to_string(),
            "template_catalog_hash": catalogue.hash(), "type": "mixed", "size": size,
            "profile": "skirmish", "entries_per_side": roads,
            "limits": { "max_authored_parts": 1000000,
                "max_bay_positions": 100000000, "max_ground_points": 10000000 }
        });
        let mapgen::CompileOutcome::Ok { result } =
            mapgen::generate_map(&request.to_string(), PRESETS, TEMPLATES, PHYSICAL_RULES)
        else {
            continue;
        };
        let prepared = sim::encounter::PreparedMap::new(&result.map, rules);
        if let Ok(admission) = sim::map_analysis::admit_skirmish(
            &prepared.queries(&result.map, &result.sites),
            rules,
            seated,
        ) {
            return admission.sites.skirmish.unwrap();
        }
    }
    panic!("no {size} map was admitted in 29 seeds");
}

#[test]
fn at_the_start_every_ai_plans_on_its_own_lane() {
    let rules: contract::scenario::Rules =
        serde_json::from_value(sim::fixtures::test_rules()).unwrap();
    for (size, per_side) in [("medium", 2), ("large", 3), ("xl", 3)] {
        let sites = admitted_sites(size, per_side, &players(per_side, 0), &rules);
        for human in 0..=per_side {
            let seated = players(per_side, human);
            let states: Vec<_> = seated.iter().map(|p| PlayerState::new(p, 0)).collect();
            let sides: Vec<_> = seated.iter().map(|p| p.side).collect();
            let homes = sim::entries::homes(&sites, &sides);
            for side in Side::COMBATANTS {
                let mut memory = TeamMemory::default();
                let frame = ObservationFrame::default();
                let plan = plan(
                    side,
                    &rules,
                    &frame,
                    &states,
                    &sites,
                    &mut memory,
                    TeamTuning::shipped(),
                );
                for (index, player) in seated.iter().enumerate() {
                    if player.side != side || player.controller != ControllerKind::ActiveAi {
                        continue;
                    }
                    let lane = plan
                        .players
                        .iter()
                        .find(|p| p.player == PlayerIndex(index as u16))
                        .map(|p| p.lane);
                    assert_eq!(
                        lane, homes[index],
                        "{size}, human in Blue {human}: {}",
                        player.id
                    );
                }
            }
        }
    }
}
