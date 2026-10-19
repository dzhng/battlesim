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

#[test]
fn a_claim_party_gets_a_direct_goal_after_the_main_group_is_ready() {
    let mut setup = two_entry_setup();
    setup.skirmish.as_mut().unwrap().rules.preparation_s = 0;
    setup.players[0].controller = contract::players::ControllerKind::ActiveAi;
    setup.players[2].controller = contract::players::ControllerKind::ActiveAi;
    let mut battle = Battle::new(&setup, 19);
    let tank = battle.rules().catalog.index("test_tank").unwrap();
    battle.script_purchases("blue-1", vec![tank; 6], None);
    battle.script_purchases("blue-2", vec![tank; 6], None);
    let hz = battle.rules().tick_hz;
    let mut claim_seen = None;
    for _ in 0..(900 * hz) {
        battle.step();
        let claims = battle.team_plans()[Side::Blue.index()].is_some_and(|plan| {
            plan.parties
                .iter()
                .any(|party| party.kind == sim::skirmish_ai::team::PartyKind::Claim)
        });
        if claims && claim_seen.is_none() {
            claim_seen = Some(battle.tick());
        }
        // A few decisions after it is raised, every unit of every claim
        // drives for the claim's own objective, not the group's staging.
        if claim_seen.is_some_and(|tick| battle.tick() >= tick + 20 * u64::from(hz)) {
            break;
        }
    }
    assert!(
        claim_seen.is_some(),
        "the ready force never formed a claim party"
    );
    let plan = battle.team_plans()[Side::Blue.index()].expect("blue plan");
    let own = &battle.observe(Side::Blue).own;
    let claims: Vec<_> = plan
        .parties
        .iter()
        .filter(|party| party.kind == sim::skirmish_ai::team::PartyKind::Claim)
        .collect();
    assert!(!claims.is_empty(), "the claim was dropped: {plan:?}");
    for party in claims {
        for id in &party.units {
            let unit = own
                .iter()
                .find(|unit| unit.id == *id)
                .expect("claim unit visible");
            let near = |at: [f64; 2]| {
                (at[0] - party.objective.center[0]).hypot(at[1] - party.objective.center[1])
                    <= party.objective.radius_m * 2.0
            };
            assert!(
                near([unit.position[0], unit.position[1]]) || unit.goal.is_some_and(near),
                "claim unit is not going to {}: {unit:?}",
                party.objective.id
            );
        }
    }
}
