//! Value-weighted strength (D56): units weigh their cost against what their
//! weapons can engage.

use super::super::influence::build_influence;
use super::super::value::{unit_value, Threat};
use super::*;
use contract::observation::IdentifiedUnit;

/// The team cards, with rifle sections whose weapons can only engage
/// soldiers: squads that cannot hurt a tank.
fn rules_with_squads_that_cannot_hurt_armour() -> Rules {
    let mut fixture = serde_json::to_value(team_rules()).unwrap();
    for weapon in ["rifle", "grenade"] {
        fixture["weapons"][weapon]["targets"]["ground"]["classes"] =
            serde_json::json!(["infantry"]);
    }
    serde_json::from_value(fixture).unwrap()
}

fn seen(rules: &Rules, kind: &str, at: [f64; 2], index: u32) -> IdentifiedUnit {
    let kind = rules.catalog.index(kind).unwrap();
    IdentifiedUnit {
        id: contract::observation::ObservedTargetId(index),
        kind,
        cost: rules.catalog.get(kind).cost,
        position: [at[0], at[1], 0.0],
        yaw: 0.0,
        velocity: [0.0, 0.0],
        members: vec![],
        member_ids: vec![],
        member_slots: vec![],
        member_active_mounts: vec![],
        weapon_poses: vec![],
        reversing: false,
        smoking: false,
        member_leans: vec![],
    }
}

/// `units` (card, position, owner) fielded by Blue, observed by Blue.
fn fielded(rules: &Rules, units: &[(&str, [f64; 2], u16)]) -> ObservationFrame {
    let setup: contract::scenario::ScenarioDefinition = serde_json::from_value(serde_json::json!({
        "map":{"size":[1000,1000],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35},
        "rules":serde_json::to_value(rules).unwrap(),
        "units":units.iter().map(|(kind, at, _)| serde_json::json!(
            {"side":"blue","kind":kind,"position":at}
        )).collect::<Vec<_>>(),
        "events":[],"scripts":[]
    }))
    .unwrap();
    let mut frame = crate::battle::Battle::new(&setup, 1)
        .observe(Side::Blue)
        .clone();
    for (unit, (_, _, owner)) in frame.own.iter_mut().zip(units) {
        unit.owner = Some(PlayerIndex(*owner));
    }
    frame
}

#[test]
fn a_tank_holds_a_lane_against_squads_that_cannot_hurt_it() {
    let rules = rules_with_squads_that_cannot_hurt_armour();
    let lane = [300.0, 500.0];
    let mut frame = fielded(&rules, &[("test_tank", lane, 0)]);
    frame.identified = (1..=3)
        .map(|index| seen(&rules, "test_rifle", lane, index))
        .collect();
    let grid = build_influence(&rules, &frame, &sites(), None, TeamTuning::shipped());
    assert!(
        grid.at(lane) > 0.0,
        "three squads that cannot hurt the tank do not outweigh it: {}",
        grid.at(lane)
    );
}

#[test]
fn an_at_team_counts_against_armour_and_an_aa_unit_against_aircraft() {
    // The teams' riflemen fight soldiers only: each team's reach beyond
    // that is its launcher's.
    let mut fixture = serde_json::to_value(team_rules()).unwrap();
    fixture["weapons"]["rifle"]["targets"] = serde_json::json!({ "ground": { "classes": ["infantry"], "range_m": 300, "scatter_mrad": 8 } });
    let rules: Rules = serde_json::from_value(fixture).unwrap();
    let card = |id: &str| rules.catalog.index(id).unwrap();
    let against = |id: &str| Threat::of(&rules, [card(id)]);
    let value = |unit: &str, target: &str| unit_value(&rules, card(unit), &against(target));
    assert!(value("test_at", "test_tank") > 0.0);
    assert_eq!(value("test_at", "test_heli"), 0.0);
    assert!(value("test_aa", "test_heli") > 0.0);
    assert_eq!(value("test_aa", "test_tank"), 0.0);
}

#[test]
fn help_comes_from_an_expensive_tank_not_from_squads_that_cannot_fight_the_threat() {
    let mut fixture = serde_json::to_value(rules_with_squads_that_cannot_hurt_armour()).unwrap();
    crate::fixtures::patch_catalog(
        &mut fixture,
        "units",
        "test_tank",
        serde_json::json!({ "cost": 1000 }),
    );
    let rules: Rules = serde_json::from_value(fixture).unwrap();
    // Blue 2 fields one expensive tank, Blue 3 three cheap squads; Blue 1's
    // lane is pressed by an enemy tank.
    let mut frame = fielded(
        &rules,
        &[
            ("test_tank", [300.0, 300.0], 1),
            ("test_rifle", [500.0, 300.0], 2),
            ("test_rifle", [510.0, 300.0], 2),
            ("test_rifle", [520.0, 300.0], 2),
        ],
    );
    frame.skirmish = Some(contract::skirmish::SkirmishView {
        phase: contract::skirmish::Phase::Active,
        players: (0..3)
            .map(|index| contract::skirmish::PlayerView {
                index: PlayerIndex(index),
                id: format!("blue-{}", index + 1),
                controller: ControllerKind::ActiveAi,
                ready: true,
                credits: 0.0,
                unit_count: [0, 1, 3][usize::from(index)],
                cap: 30,
                pending: vec![],
            })
            .collect(),
        preparation_remaining_s: 0.0,
        objectives: vec![],
        scores: [0.0; 2],
        result: None,
        report: None,
    });
    let site = sites().objectives;
    frame.identified = vec![seen(&rules, "test_tank", site[0].center, 1)];
    let lanes: Vec<_> = (0..3)
        .map(|index| {
            let objective = site[index].clone();
            let staging = [objective.center[0], 400.0];
            (
                PlayerIndex(index as u16),
                EntryIndex(index),
                objective,
                staging,
            )
        })
        .collect();
    // The pressed lane needs one unit of strength; the helpers' lanes are
    // quiet.
    let mut memory = TeamMemory {
        influence: Some(InfluenceGrid {
            resolution: 3,
            min: [0.0, 0.0],
            max: [600.0, 1_000.0],
            values: vec![0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        }),
        previous_influence: [(site[0].id.clone(), 0.0)].into_iter().collect(),
        ..TeamMemory::default()
    };
    let tuning = TeamTuning {
        help_trend_threshold: 0.5,
        help_force_margin: 1.0,
        ..TeamTuning::shipped()
    };
    let requests = update_help(
        Side::Blue,
        &frame,
        &players(),
        &lanes,
        &mut memory,
        tuning,
        &rules,
    );
    let request = requests
        .iter()
        .find(|request| request.requester == PlayerIndex(0))
        .expect("the pressed lane asks for help");
    assert_eq!(request.answered_by, Some(PlayerIndex(1)), "{requests:?}");
    let key = PartyKey::Help(PlayerIndex(0), request.objective.id.clone());
    assert_eq!(memory.parties[&key].units, vec![frame.own[0].id]);
}

#[test]
fn an_enemy_on_an_objective_counts_wherever_the_grid_cells_fall() {
    // Cells 100 m a side; the objective sits 5 m short of a cell's edge, and
    // the enemy on it stands just across that edge.
    let rules = team_rules();
    let objective = ObjectiveSite {
        id: "held".into(),
        center: [195.0, 150.0],
        radius_m: 30.0,
        kind: contract::encounter::ObjectiveSiteKind::Field,
        counterpart: None,
    };
    let mut frame = fielded(&rules, &[]);
    frame.identified = vec![seen(&rules, "test_rifle", [210.0, 150.0], 1)];
    let mut sites = sites();
    sites.entries.iter_mut().for_each(|e| e.center = [0.0, 0.0]);
    sites.objectives = vec![
        objective.clone(),
        ObjectiveSite {
            center: [800.0, 800.0],
            ..objective.clone()
        },
    ];
    let grid = build_influence(
        &rules,
        &frame,
        &sites,
        None,
        TeamTuning {
            grid_resolution: 8,
            ..TeamTuning::shipped()
        },
    );
    assert!(
        grid.on(&objective) < 0.0,
        "the enemy standing on the objective weighs on it: {}",
        grid.on(&objective)
    );
}
