use super::*;
use contract::catalog::Faction;
use contract::players::{ControllerKind, Player};
use contract::scenario::{ScenarioDefinition, UnitCondition, UnitSetup};
use serde_json::json;

fn setup() -> ScenarioDefinition {
    let mut authored = crate::fixtures::test_rules();
    for (id, category) in [
        ("test_tank", "veh"),
        ("test_supply", "sup"),
        ("test_supply_heli", "sup"),
        ("test_rifle", "inf"),
    ] {
        crate::fixtures::patch_catalog(
            &mut authored,
            "units",
            id,
            json!({"roster":{"factions":["us","eastern"],"category":category,"family_name":"Test","variant":"Test"}}),
        );
    }
    let players = vec![
        Player {
            id: "blue-1".into(),
            side: Side::Blue,
            faction: Some(Faction::Us),
            controller: ControllerKind::ActiveAi,
            human_capable: false,
        },
        Player {
            id: "red-1".into(),
            side: Side::Red,
            faction: Some(Faction::Eastern),
            controller: ControllerKind::PassiveAi,
            human_capable: false,
        },
    ];
    let mut sites = crate::skirmish::synthetic_sites([2400.0, 1600.0], 1);
    sites.entries[0].center = [100.0, 100.0];
    sites.entries[1].center = [2300.0, 1500.0];
    for objective in &mut sites.objectives {
        objective.center = [1800.0, 800.0];
    }
    let mut skirmish =
        crate::skirmish::setup_for(&players, sites, crate::skirmish::starter_rules());
    skirmish.rules.preparation_s = 0;
    skirmish.rules.credits_per_minute = 100;
    skirmish.rules.starting_minutes = 20;
    skirmish.rules.max_units = 4;
    skirmish.rules.majority_s = 1_000_000;
    skirmish.rules.victory_score = 1_000_000_000_000.0;
    skirmish.rules.capture_s = 1_000_000;
    serde_json::from_value(json!({
        "map":{"size":[2400,1600],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35},
        "rules":authored,
        "units":[],"events":[],"scripts":[],
        "players":players,
        "skirmish":skirmish
    }))
    .unwrap()
}

fn authored_units(battle: &mut Battle) {
    let setups = [
        UnitSetup {
            side: Side::Blue,
            owner: Some("blue-1".into()),
            kind: "test_tank".into(),
            position: [1800.0, 800.0],
            yaw: 0.0,
            engagement: Some(contract::command::Engagement::ReturnFireOnly),
            condition: Some(UnitCondition {
                hp: Some(40.0),
                ..Default::default()
            }),
            stock: None,
        },
        UnitSetup {
            side: Side::Blue,
            owner: Some("blue-1".into()),
            kind: "test_supply".into(),
            position: [100.0, 100.0],
            yaw: 0.0,
            engagement: Some(contract::command::Engagement::ReturnFireOnly),
            condition: None,
            stock: None,
        },
        UnitSetup {
            side: Side::Red,
            owner: Some("red-1".into()),
            kind: "test_rifle".into(),
            position: [1920.0, 800.0],
            yaw: std::f64::consts::PI,
            engagement: Some(contract::command::Engagement::ReturnFireOnly),
            condition: None,
            stock: None,
        },
    ];
    let mut soldier_ids = battle.last_soldier;
    for (index, setup) in setups.iter().enumerate() {
        let mut unit = spawn_unit(
            index,
            setup,
            &battle.world,
            &battle.rules,
            &battle.arsenal,
            &mut soldier_ids,
            battle.seed,
        );
        if let Some(condition) = &setup.condition {
            spawn::wear(&mut unit, condition, &battle.arsenal, &battle.rules);
        }
        unit.settle();
        let player = battle
            .skirmish
            .as_ref()
            .unwrap()
            .player_index(setup.owner.as_deref().unwrap())
            .unwrap();
        battle.skirmish.as_mut().unwrap().entered.insert(
            UnitId(index as u32),
            crate::skirmish::EnteredUnit {
                price: 100,
                entered_tick: 0,
                player,
            },
        );
        battle.units.push(unit);
        battle.skirmish.as_mut().unwrap().players[player].wallet =
            2_000 * crate::skirmish::CREDIT_SCALE;
    }
    battle.last_soldier = soldier_ids;
    battle.skirmish.as_mut().unwrap().phase = contract::skirmish::Phase::Active;
    for player in &mut battle.skirmish.as_mut().unwrap().players {
        player.ready = true;
    }
    battle.observe_all();
}

#[test]
fn emergency_airlift_buys_lands_serves_and_returns_to_support() {
    let mut battle = Battle::new(&setup(), 901);
    authored_units(&mut battle);
    let hz = battle.rules.tick_hz;
    let heli_kind = battle.rules.catalog.index("test_supply_heli").unwrap();
    let flying_suppliers: std::collections::BTreeSet<_> = battle
        .rules
        .catalog
        .indices()
        .filter(|&kind| {
            let unit = battle.rules.catalog.get(kind);
            unit.has_role("logistics")
                && unit.capabilities.supply.is_some()
                && unit.mobility.flight().is_some()
                && unit.mobility.locomotion().is_none()
        })
        .map(|kind| battle.rules.catalog.id(kind).to_owned())
        .collect();
    let mut bought = false;
    let mut landed = false;
    let mut improved = false;
    let mut returning = false;
    let mut landing_bearing = None;
    for _ in 0..360 * hz {
        battle.step();
        let observation = battle.observe(Side::Blue);
        let tank = observation
            .own
            .iter()
            .find(|u| u.kind == battle.rules.catalog.index("test_tank").unwrap());
        if let Some(heli) = observation.own.iter().find(|u| u.kind == heli_kind) {
            landed |= heli.grounded;
            landing_bearing = landing_bearing.or(heli
                .grounded
                .then_some([heli.position[0] - 1800.0, heli.position[1] - 800.0]));
            returning |= landed && heli.goal.is_some_and(|goal| goal[0] < 900.0);
        }
        improved |= tank.is_some_and(|u| {
            u.hp > 40.0 || u.service == contract::observation::ServiceStatus::Serving
        });
        bought |= battle.replay().accepted.iter().any(|(_, command)| {
            command.side == Side::Blue
                && matches!(&command.order, Order::ConfirmPurchase { variant, .. } if flying_suppliers.contains(variant))
        });
        if bought && landed && improved && returning {
            break;
        }
    }
    let enemy_direction = [120.0, 0.0];
    let bearing = landing_bearing.expect("the heli touched down");
    assert!(
        bought,
        "the purchased supplier is the flying card: {:?}",
        battle.replay().accepted
    );
    assert!(landed || returning, "the supplier lands before returning");
    assert!(improved, "the tank's health or service state improves");
    assert!(
        bearing[0] * enemy_direction[0] + bearing[1] * enemy_direction[1] < 0.0,
        "landing bearing is away from the seen enemy: {bearing:?}"
    );
    assert!(returning, "the supplier heads back toward the support post");
}
