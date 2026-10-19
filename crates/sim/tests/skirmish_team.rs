use contract::ids::Side;
use sim::battle::Battle;
use sim::skirmish_ai::team::lane_advancing;

fn two_entry_setup() -> contract::scenario::ScenarioDefinition {
    let mut setup = crate::skirmish::setup();
    let mut blue_two = setup.players[0].clone();
    blue_two.id = "blue-2".into();
    let mut red_two = setup.players[1].clone();
    red_two.id = "red-2".into();
    setup.players.extend([blue_two, red_two]);
    let sites = &mut setup.skirmish.as_mut().unwrap().sites;
    sites.entries[0].center = [100.0, 10.0];
    sites.entries[1].center = [700.0, 590.0];
    sites.entries.extend([
        contract::encounter::EntrySite {
            side: Side::Blue,
            center: [700.0, 10.0],
            yaw: std::f64::consts::FRAC_PI_2,
        },
        contract::encounter::EntrySite {
            side: Side::Red,
            center: [100.0, 590.0],
            yaw: -std::f64::consts::FRAC_PI_2,
        },
    ]);
    setup
}

#[test]
fn a_battle_push_orders_each_sent_lane_toward_its_planned_objective() {
    let mut setup = two_entry_setup();
    setup.skirmish.as_mut().unwrap().rules.preparation_s = 0;
    setup.players[0].controller = contract::players::ControllerKind::ActiveAi;
    setup.players[2].controller = contract::players::ControllerKind::ActiveAi;
    let mut battle = Battle::new(&setup, 11);
    let tank = battle.rules().catalog.index("test_tank").unwrap();
    battle.script_purchases("blue-1", vec![tank], None);
    battle.script_purchases("blue-2", vec![tank], None);
    let mut observed_push = false;
    for _ in 0..(120 * battle.rules().tick_hz) {
        battle.step();
        let Some(plan) = battle.team_plans()[Side::Blue.index()] else {
            continue;
        };
        if plan.push_started_tick.is_none() {
            continue;
        }
        observed_push = true;
        for player in plan
            .players
            .iter()
            .filter(|player| !plan.outgunned.contains(&player.player))
        {
            let units: Vec<_> = battle
                .observe(Side::Blue)
                .own
                .iter()
                .filter(|unit| unit.owner == Some(player.player))
                .collect();
            assert!(
                lane_advancing(battle.rules(), &units, player),
                "push left player {} without an advance toward {}: {:?}",
                player.player.0,
                player.main_objective.id,
                units
            );
        }
        break;
    }
    assert!(observed_push, "the planner never started a push");
}
