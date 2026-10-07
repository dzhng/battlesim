//! The encounter planner on small authored maps: each legality rule, each
//! ruled-out moment, and the battle a planned encounter starts.
//!
//! The town every test plans on: a road from the bottom edge to the top
//! through a crossroads, six buildings round it, and open ground beyond.
use std::collections::BTreeSet;

use contract::encounter::{
    EncounterDefinition, EncounterDiagnostic, EncounterDiagnosticCode as Code, EncounterRecipe,
    EncounterRecipes, EncounterSites, Post, Requirement,
};
use contract::identity::Seed;
use contract::ids::Side;
use contract::map::MapDefinition;
use contract::observation::GarrisonPhase;
use contract::scenario::Rules;
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::encounter::{plan_encounter, plan_encounter_json, PreparedMap};
use sim::math::v2;
use sim::world::{SurfaceKind, WorldGeometry};

use crate::common;

const WIDTH: f64 = 1200.0;
const DEPTH: f64 = 1600.0;
const ROAD_X: f64 = 600.0;
const BUILDING_HALF: [f64; 3] = [8.0, 6.0, 4.0];

/// Where the six buildings stand about the town's centre.
const BUILDINGS: [[f64; 2]; 6] = [
    [-40.0, -40.0],
    [40.0, -40.0],
    [-40.0, 40.0],
    [40.0, 40.0],
    [-90.0, 0.0],
    [90.0, 0.0],
];

/// The town with its centre `centre_y` up the road, and `extra` map fields
/// merged over it.
fn town_map(centre_y: f64, extra: Value) -> MapDefinition {
    let props: Vec<Value> = BUILDINGS
        .iter()
        .map(|[dx, dy]| {
            json!({ "kind": "building", "center": [ROAD_X + dx, centre_y + dy], "yaw": 0,
                "half_extents": BUILDING_HALF })
        })
        .collect();
    let mut map = json!({
        "size": [WIDTH, DEPTH], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
        "surfaces": [road(0.0, DEPTH)],
        "props": props,
    });
    for (key, value) in extra.as_object().unwrap() {
        if key == "props" {
            let all = map["props"].as_array_mut().unwrap();
            all.extend(value.as_array().unwrap().iter().cloned());
        } else {
            map[key] = value.clone();
        }
    }
    common::physical_map(serde_json::from_value(map).unwrap(), &common::rules())
}

fn road(from_y: f64, to_y: f64) -> Value {
    json!({ "kind": "country_road",
        "shape": { "kind": "stroke", "points": [[ROAD_X, from_y], [ROAD_X, to_y]], "width_m": 8 } })
}

/// The town's sites: one settlement about `centre_y` with one district, an
/// open approach due south and one due north.
fn town_sites(centre_y: f64) -> EncounterSites {
    sites_of(&[("town", centre_y)])
}

/// One settlement per name, each 300 m square about its place on the road,
/// with one district and an open approach due south and due north of it.
fn sites_of(towns: &[(&str, f64)]) -> EncounterSites {
    let south = -std::f64::consts::FRAC_PI_2;
    let mut settlements = Vec::new();
    let mut approaches = Vec::new();
    for (i, (id, centre_y)) in towns.iter().enumerate() {
        let ring = [
            [ROAD_X - 150.0, centre_y - 150.0],
            [ROAD_X + 150.0, centre_y - 150.0],
            [ROAD_X + 150.0, centre_y + 150.0],
            [ROAD_X - 150.0, centre_y + 150.0],
        ];
        settlements.push(
            json!({ "id": id, "center": [ROAD_X, centre_y], "outline": ring,
            "districts": [{ "id": format!("{id}/district-0"), "ring": ring, "area_m2": 90000 }] }),
        );
        for (half, bearing) in [("bottom", south), ("top", -south)] {
            approaches.push(json!({ "settlement": i, "half": half,
                "from_rad": bearing - 0.1, "to_rad": bearing + 0.1,
                "depth_m": 400, "front_m": 200 }));
        }
    }
    serde_json::from_value(json!({ "settlements": settlements, "approaches": approaches })).unwrap()
}

/// The shipped assault recipe, scaled to the test town.
fn recipe() -> EncounterRecipe {
    let text = std::fs::read_to_string(sim::fixtures::dir().join("encounters.json")).unwrap();
    let mut recipe = EncounterRecipes::from_json(&text)
        .unwrap()
        .recipes
        .remove("assault")
        .unwrap();
    recipe.deployment.edge_inset_m = 40.0;
    recipe.deployment.max_advance_m = 400.0;
    recipe.garrison.reach_m = 150.0;
    recipe.garrison.apart_m = 30.0;
    recipe.overwatch.sight_m = 300.0;
    recipe
}

type Planned = Result<EncounterDefinition, Vec<EncounterDiagnostic>>;

fn plan_with(
    map: &MapDefinition,
    sites: &EncounterSites,
    rules: &Rules,
    recipe: &EncounterRecipe,
    seed: u64,
) -> Planned {
    let prepared = PreparedMap::new(map, rules);
    plan_encounter(
        &prepared.queries(map, sites),
        rules,
        recipe,
        Seed::from(seed),
    )
}

fn plan(map: &MapDefinition, sites: &EncounterSites, recipe: &EncounterRecipe) -> Planned {
    plan_with(map, sites, &common::rules(), recipe, 1)
}

fn codes(planned: &Planned) -> Vec<Code> {
    planned
        .as_ref()
        .expect_err("the placement should be refused")
        .iter()
        .map(|d| d.code)
        .collect()
}

fn river(y: f64, width: f64) -> Value {
    json!({ "points": [
        { "xy": [0, y], "width_m": width, "depth_m": 1.5 },
        { "xy": [WIDTH, y], "width_m": width, "depth_m": 1.5 }],
        "surface_z": -0.5 })
}

fn column(encounter: &EncounterDefinition, side: Side) -> &contract::encounter::Deployment {
    encounter
        .placement
        .deployments
        .iter()
        .find(|d| d.side == side)
        .expect("the side has a column")
}

/// Every unit stands on ground a mover may stand on, outside every
/// building, with room between it and the next unit.
fn assert_stands_clear(map: &MapDefinition, encounter: &EncounterDefinition) {
    let rules = common::rules();
    let world = WorldGeometry::new(map, &rules);
    let units = &encounter.setup.units;
    for (i, unit) in units.iter().enumerate() {
        let [x, y] = unit.position;
        assert!(
            world.traversable_at(x, y),
            "unit {i} ({}) stands on water or a slope at {x}, {y}",
            unit.kind
        );
        for prop in world.props() {
            assert!(
                !prop.footprint().contains(v2(x, y), 0.9),
                "unit {i} ({}) at {x}, {y} stands in the body at {:?}",
                unit.kind,
                prop.center
            );
        }
        for (j, other) in units.iter().enumerate().skip(i + 1) {
            let gap = (x - other.position[0]).hypot(y - other.position[1]);
            assert!(gap > 8.0, "units {i} and {j} stand {gap} m apart");
        }
    }
}

#[test]
fn the_assault_recipe_puts_a_column_at_each_edge_and_the_garrison_round_the_objective() {
    let (map, sites, recipe) = (town_map(800.0, json!({})), town_sites(800.0), recipe());
    let encounter = plan(&map, &sites, &recipe).unwrap();
    let units = &encounter.setup.units;

    // Blue's rows, then red's, in the recipe's order.
    let kinds = |side: Side| -> Vec<&str> {
        units
            .iter()
            .filter(|u| u.side == side)
            .map(|u| u.kind.as_str())
            .collect()
    };
    let rows = |side: Side| -> Vec<&str> {
        recipe
            .forces
            .side(side)
            .iter()
            .map(|r| r.kind.as_str())
            .collect()
    };
    assert_eq!(kinds(Side::Blue), rows(Side::Blue));
    assert_eq!(kinds(Side::Red), rows(Side::Red));
    assert_stands_clear(&map, &encounter);

    // Blue's column stands on the road at the bottom edge, heading up it,
    // its leader nearest the town and each unit a spacing behind the last.
    let world = WorldGeometry::new(&map, &common::rules());
    let blue = column(&encounter, Side::Blue);
    assert_eq!(blue.units, (0..9).collect::<Vec<u32>>());
    assert_eq!(blue.advance_m, 0.0);
    for (k, &unit) in blue.units.iter().enumerate() {
        let u = &units[unit as usize];
        assert_eq!(u.position[0], ROAD_X);
        assert_eq!(u.position[1], 40.0 + (8 - k) as f64 * 30.0);
        assert!((u.yaw - std::f64::consts::FRAC_PI_2).abs() < 1e-6);
        assert_eq!(
            world.surface_at(u.position[0], u.position[1]).unwrap().kind,
            SurfaceKind::Road
        );
    }
    // Red's column stands at the top edge, heading down the road.
    let red = column(&encounter, Side::Red);
    assert_eq!(red.units, vec![13, 14]);
    assert_eq!(units[14].position, [ROAD_X, DEPTH - 40.0]);
    assert_eq!(units[13].position, [ROAD_X, DEPTH - 70.0]);
    assert!((units[13].yaw + std::f64::consts::FRAC_PI_2).abs() < 1e-6);

    // The objective is the town's crossroads; both drives end in its zone
    // and differ by no more than the recipe allows.
    let rules = &encounter.setup.encounter;
    assert_eq!(rules.attacker, Side::Blue);
    assert_eq!(rules.success_zone_center, [ROAD_X, 800.0]);
    assert_eq!(rules.success_zone_radius_m, recipe.objective.zone_radius_m);
    for d in [blue, red] {
        let end = d.route.last().unwrap();
        assert!((end[0] - ROAD_X).hypot(end[1] - 800.0) <= recipe.objective.zone_radius_m);
        assert!(d.route_s > 0.0 && d.route_m > 400.0);
    }
    assert!((blue.route_s - red.route_s).abs() <= recipe.deployment.max_route_difference_s);

    // Each garrison row has a building of its own within reach of the
    // centre, a seat for every soldier, and stands at its wall.
    let garrisons = &encounter.placement.garrisons;
    assert_eq!(
        garrisons.iter().map(|g| g.unit).collect::<Vec<_>>(),
        vec![9, 10, 11]
    );
    let held: BTreeSet<u32> = garrisons.iter().map(|g| g.building).collect();
    assert_eq!(held.len(), 3);
    for g in garrisons {
        let building = map
            .buildings
            .iter()
            .find(|b| b.owner == g.building)
            .expect("a garrison names one of the map's buildings");
        let [bx, by, _] = building.geometry.frame.translation;
        assert!((bx - ROAD_X).hypot(by - 800.0) <= recipe.garrison.reach_m);
        // Blue comes from the south: the garrison holds the near side.
        assert!(
            by <= 800.0,
            "the building at {bx}, {by} is behind the centre"
        );
        assert!(g.seats >= g.soldiers && g.soldiers == 8);
        assert_eq!(g.district, "town/district-0");
        let squad = units[g.unit as usize].position;
        let wall = (squad[0] - bx).abs() - BUILDING_HALF[0];
        let side = (squad[1] - by).abs() - BUILDING_HALF[1];
        assert!(
            (wall.max(side) - recipe.garrison.door_standoff_m).abs() < 0.02,
            "the squad stands {} m from its building's wall",
            wall.max(side)
        );
    }
    assert_eq!(
        encounter.setup.opponent.garrisons,
        garrisons
            .iter()
            .map(|g| [g.unit, g.building])
            .collect::<Vec<_>>()
    );
    assert_eq!(encounter.setup.opponent.side, Side::Red);

    // The AT team watches the road blue drives in by, from the town's
    // southern edge, holding its fire.
    let [post] = encounter.placement.overwatch.as_slice() else {
        panic!("one overwatch post")
    };
    assert_eq!(post.unit, 12);
    assert_eq!(post.approach, None);
    assert!((post.yaw + std::f64::consts::FRAC_PI_2).abs() < 1e-6);
    assert!(post.at[1] < 650.0 && post.at[1] > 630.0, "{:?}", post.at);
    assert!(
        (post.at[0] - ROAD_X).abs() > 4.0,
        "beside the road, not on it"
    );
    assert_eq!(
        units[12].engagement,
        Some(contract::command::Engagement::ReturnFireOnly)
    );

    // Red's column is ordered to the objective as the battle starts.
    let [script] = encounter.setup.scripts.as_slice() else {
        panic!("one scripted order")
    };
    assert_eq!((script.tick, script.side), (0, Side::Red));
    match &script.order {
        contract::command::Order::Move { units, goal, .. } => {
            assert_eq!(units.iter().map(|u| u.0).collect::<Vec<_>>(), vec![13, 14]);
            assert_eq!(*goal, [ROAD_X, 800.0]);
        }
        other => panic!("the column's order is a move, not {other:?}"),
    }
}

#[test]
fn a_planned_encounter_starts_a_battle_whose_garrisons_enter_their_buildings() {
    let (map, sites, recipe) = (town_map(800.0, json!({})), town_sites(800.0), recipe());
    let encounter = plan(&map, &sites, &recipe).unwrap();
    let garrisons = encounter.placement.garrisons.clone();
    let column = column(&encounter, Side::Red).units.clone();
    let start: Vec<[f64; 2]> = encounter.setup.units.iter().map(|u| u.position).collect();
    let scenario = encounter.setup.scenario(map, common::rules());
    let mut battle = Battle::new(&scenario, 1);
    let hz = battle.rules().tick_hz as u64;
    for _ in 0..20 * hz {
        battle.step();
    }
    let red = battle.observe(Side::Red);
    for g in &garrisons {
        let squad = red.own.iter().find(|u| u.id.0 == g.unit).unwrap();
        let held = squad.garrison.expect("the squad holds a building");
        assert_eq!(
            (held.building, held.phase),
            (g.building, GarrisonPhase::Inside)
        );
    }
    // The relief column has set off down the road toward the objective.
    for unit in column {
        let now = red.own.iter().find(|u| u.id.0 == unit).unwrap().position;
        assert!(
            start[unit as usize][1] - now[1] > 50.0,
            "unit {unit} has moved {} m toward the town",
            start[unit as usize][1] - now[1]
        );
    }
}

#[test]
fn a_column_whose_road_starts_under_water_stands_on_the_far_bank() {
    // Water from 25 m to 175 m up the road, and no bridge over it.
    let map = town_map(800.0, json!({ "rivers": [river(100.0, 150.0)] }));
    let encounter = plan(&map, &town_sites(800.0), &recipe()).unwrap();
    assert_stands_clear(&map, &encounter);
    let blue = column(&encounter, Side::Blue);
    assert_eq!(blue.advance_m, 150.0);
    for &unit in &blue.units {
        let y = encounter.setup.units[unit as usize].position[1];
        assert!(y > 175.0, "unit {unit} stands at {y}, in the river");
    }
}

#[test]
fn a_column_with_no_dry_road_within_its_advance_is_refused() {
    let map = town_map(800.0, json!({ "rivers": [river(100.0, 150.0)] }));
    let mut recipe = recipe();
    recipe.deployment.max_advance_m = 100.0;
    let planned = plan(&map, &town_sites(800.0), &recipe);
    assert_eq!(codes(&planned), vec![Code::NoObjective, Code::NoDeployment]);
    let why = &planned.unwrap_err()[1];
    assert!(why.message.contains("water"), "{}", why.message);
    assert!(
        why.location.starts_with("$.recipe.forces.blue["),
        "{}",
        why.location
    );
}

#[test]
fn a_column_stands_clear_of_a_building_on_its_road() {
    // A building across the road where the third unit from the tail would stand.
    let on_road = json!({ "props": [{ "kind": "building", "center": [ROAD_X, 100.0], "yaw": 0,
        "half_extents": [6, 6, 4] }] });
    let map = town_map(800.0, on_road);
    let encounter = plan(&map, &town_sites(800.0), &recipe()).unwrap();
    assert_stands_clear(&map, &encounter);
    assert!(column(&encounter, Side::Blue).advance_m > 0.0);
}

#[test]
fn an_objective_the_column_cannot_reach_is_refused() {
    // A river from side to side between blue's edge and the town, unbridged.
    let map = town_map(800.0, json!({ "rivers": [river(450.0, 30.0)] }));
    let planned = plan(&map, &town_sites(800.0), &recipe());
    assert_eq!(
        codes(&planned),
        vec![Code::NoObjective, Code::UnreachableObjective]
    );
    assert_eq!(
        planned.unwrap_err()[1].feature.as_deref(),
        Some("forces.blue")
    );
}

#[test]
fn a_squad_is_garrisoned_only_where_every_soldier_has_a_seat() {
    let (map, sites, recipe) = (town_map(800.0, json!({})), town_sites(800.0), recipe());
    // Seven seats to a building: one too few for a squad of eight.
    let mut fixture = common::game();
    fixture["buildings"]["capacity_soldiers"] = json!(7);
    let cramped: Rules = serde_json::from_value(fixture.clone()).unwrap();
    let planned = plan_with(&map, &sites, &cramped, &recipe, 1);
    assert_eq!(
        codes(&planned),
        vec![Code::NoObjective, Code::NoGarrisonBuilding]
    );
    let why = &planned.unwrap_err()[1];
    assert_eq!(why.feature.as_deref(), Some("forces.red[0]"));
    assert!(
        why.message.contains("6 with too few seats"),
        "{}",
        why.message
    );
    // Eight seats take the squad whole.
    fixture["buildings"]["capacity_soldiers"] = json!(8);
    let snug: Rules = serde_json::from_value(fixture).unwrap();
    let encounter = plan_with(&map, &sites, &snug, &recipe, 1).unwrap();
    for g in &encounter.placement.garrisons {
        assert_eq!((g.soldiers, g.seats), (8, 8));
    }
}

#[test]
fn more_garrison_rows_than_buildings_in_reach_is_refused() {
    let (map, sites, mut recipe) = (town_map(800.0, json!({})), town_sites(800.0), recipe());
    // Only the four buildings by the crossroads stand within 60 m of it.
    recipe.garrison.reach_m = 60.0;
    assert_eq!(
        plan(&map, &sites, &recipe)
            .unwrap()
            .placement
            .garrisons
            .len(),
        3
    );
    for _ in 0..2 {
        recipe.forces.red.push(contract::encounter::RosterRow {
            kind: "test_rifle".into(),
            post: Post::Garrison,
            engagement: None,
        });
    }
    let planned = plan(&map, &sites, &recipe);
    assert_eq!(
        codes(&planned),
        vec![Code::NoObjective, Code::NoGarrisonBuilding]
    );
    assert_eq!(
        planned.unwrap_err()[1].feature.as_deref(),
        Some("forces.red[7]")
    );
}

#[test]
fn a_side_whose_edge_no_road_meets_is_refused() {
    // The road stops at the town: nothing meets the top edge.
    let map = town_map(800.0, json!({ "surfaces": [road(0.0, 1000.0)] }));
    let planned = plan(&map, &town_sites(800.0), &recipe());
    assert_eq!(codes(&planned), vec![Code::NoObjective, Code::NoEdgeRoad]);
    assert_eq!(
        planned.unwrap_err()[1].feature.as_deref(),
        Some("forces.red")
    );
}

#[test]
fn a_map_without_a_settlement_or_its_open_approach_has_no_objective() {
    let (map, recipe) = (town_map(800.0, json!({})), recipe());
    let none = EncounterSites::default();
    assert_eq!(codes(&plan(&map, &none, &recipe)), vec![Code::NoObjective]);

    // The recipe requires an open approach on the attacker's side: one
    // whose bearing points toward the attacker's edge.
    let mut closed = town_sites(800.0);
    closed.approaches.remove(0);
    assert_eq!(
        codes(&plan(&map, &closed, &recipe)),
        vec![Code::NoObjective, Code::NoOpenApproach]
    );
    // Preferred only: the encounter stands, its post on the attacker's road.
    let mut relaxed = recipe.clone();
    relaxed.objective.open_approach = Requirement::Preferred;
    let encounter = plan(&map, &closed, &relaxed).unwrap();
    assert_eq!(encounter.placement.overwatch[0].approach, None);
}

#[test]
fn a_second_overwatch_post_covers_the_open_approach_on_the_attackers_side() {
    let (map, sites, mut recipe) = (town_map(800.0, json!({})), town_sites(800.0), recipe());
    recipe.forces.red.push(contract::encounter::RosterRow {
        kind: "test_at".into(),
        post: Post::Overwatch,
        engagement: None,
    });
    let encounter = plan(&map, &sites, &recipe).unwrap();
    assert_stands_clear(&map, &encounter);
    let posts = &encounter.placement.overwatch;
    assert_eq!(
        posts.iter().map(|p| p.approach).collect::<Vec<_>>(),
        vec![None, Some(0)]
    );
    // On the approach's own line, just beyond the town's southern edge.
    assert_eq!(posts[1].at[0], ROAD_X);
    assert!(posts[1].at[1] < 650.0 && posts[1].at[1] > 630.0);
}

#[test]
fn the_objective_goes_on_the_settlement_the_recipe_prefers() {
    // The main town in the bottom half, and a second one in the top half.
    let north: Vec<Value> = BUILDINGS[..4]
        .iter()
        .map(|[dx, dy]| {
            json!({ "kind": "building", "center": [ROAD_X + dx, 1150.0 + dy], "yaw": 0,
                "half_extents": BUILDING_HALF })
        })
        .collect();
    let map = town_map(700.0, json!({ "props": north }));
    let sites = sites_of(&[("town", 700.0), ("north", 1150.0)]);
    let objective = |preference: contract::encounter::SettlementPreference| {
        let mut recipe = recipe();
        recipe.objective.settlement = preference;
        recipe.deployment.max_advance_m = 600.0;
        let encounter = plan(&map, &sites, &recipe).unwrap();
        assert_stands_clear(&map, &encounter);
        // The garrison holds buildings of the objective's own settlement.
        for g in &encounter.placement.garrisons {
            assert!(g
                .district
                .starts_with(&encounter.placement.objective.settlement));
        }
        encounter.placement.objective.settlement
    };
    use contract::encounter::SettlementPreference::{AttackerHalf, DefenderHalf, Main};
    assert_eq!(objective(Main), "town");
    assert_eq!(objective(AttackerHalf), "town");
    assert_eq!(objective(DefenderHalf), "north");
}

#[test]
fn the_farther_column_starts_up_its_road_until_the_two_drives_match() {
    // The town stands 200 m nearer the bottom edge than the top.
    let (map, sites, mut recipe) = (town_map(700.0, json!({})), town_sites(700.0), recipe());
    recipe.deployment.max_route_difference_s = 1.0;
    let encounter = plan(&map, &sites, &recipe).unwrap();
    assert_stands_clear(&map, &encounter);
    let (blue, red) = (
        column(&encounter, Side::Blue),
        column(&encounter, Side::Red),
    );
    assert_eq!((blue.advance_m, red.advance_m), (0.0, 400.0));
    assert!((blue.route_s - red.route_s).abs() <= 1.0);
    // Red's leader now stands as far from the centre as blue's does.
    let head = |d: &contract::encounter::Deployment| (d.head[1] - 700.0).abs();
    assert!((head(blue) - head(red)).abs() < 50.0);

    // With no room to move up, the start cannot be made fair.
    recipe.deployment.max_advance_m = 200.0;
    let planned = plan(&map, &sites, &recipe);
    assert_eq!(
        codes(&planned),
        vec![Code::NoObjective, Code::UnfairDeployment]
    );
}

#[test]
fn the_same_inputs_plan_the_same_bytes_and_the_seed_chooses_the_garrisons() {
    let (map, sites, recipe) = (town_map(800.0, json!({})), town_sites(800.0), recipe());
    let rules = common::rules();
    let bytes = |seed: u64| {
        serde_json::to_string(&plan_with(&map, &sites, &rules, &recipe, seed).unwrap()).unwrap()
    };
    assert_eq!(bytes(1), bytes(1));

    let mut assignments = BTreeSet::new();
    for seed in 1..=8 {
        let encounter = plan_with(&map, &sites, &rules, &recipe, seed).unwrap();
        assert_stands_clear(&map, &encounter);
        // Only the garrison is the seed's: the columns stand where they did.
        assert_eq!(column(&encounter, Side::Blue).head, [ROAD_X, 280.0]);
        assignments.insert(
            encounter
                .placement
                .garrisons
                .iter()
                .map(|g| g.building)
                .collect::<Vec<_>>(),
        );
    }
    assert!(
        assignments.len() > 1,
        "eight seeds all chose {assignments:?}"
    );
}

#[test]
fn a_recipe_is_refused_for_what_it_asks_before_any_map_is_read() {
    let (map, sites) = (town_map(800.0, json!({})), town_sites(800.0));
    let refused = |change: &dyn Fn(&mut EncounterRecipe)| {
        let mut recipe = recipe();
        change(&mut recipe);
        plan(&map, &sites, &recipe).unwrap_err()
    };
    // The attacker comes from its edge: it holds nothing at the start.
    let why = refused(&|r| r.forces.blue[4].post = Post::Garrison);
    assert_eq!(why[0].code, Code::InvalidRecipe);
    assert!(
        why[0].message.contains("forces.blue[4].post"),
        "{}",
        why[0].message
    );
    // A vehicle garrisons nothing.
    let why = refused(&|r| r.forces.red[0].kind = "test_tank".into());
    assert_eq!(
        (why[0].code, why[0].feature.as_deref()),
        (Code::InvalidRecipe, Some("forces.red[0]"))
    );
    // A unit type the catalog lacks.
    let why = refused(&|r| r.forces.blue[0].kind = "zeppelin".into());
    assert_eq!(
        (why[0].code, why[0].feature.as_deref()),
        (Code::InvalidRecipe, Some("forces.blue[0]"))
    );
    let why = refused(&|r| r.deployment.spacing_m = 0.0);
    assert!(
        why[0].message.contains("deployment.spacing_m"),
        "{}",
        why[0].message
    );
}

#[test]
fn the_recipes_file_names_the_recipe_and_field_it_refuses() {
    let text = std::fs::read_to_string(sim::fixtures::dir().join("encounters.json")).unwrap();
    let mut file: Value = serde_json::from_str(&text).unwrap();
    file["recipes"]["assault"]["objective"]["zone_radius_m"] = json!(-1);
    assert_eq!(
        EncounterRecipes::from_json(&file.to_string()).unwrap_err(),
        "recipes.assault.objective.zone_radius_m must be finite and positive"
    );
    file["recipes"]["assault"]["objective"]["zone_radius_m"] = json!(150);
    file["recipes"]["assault"]["surprise"] = json!(true);
    assert!(EncounterRecipes::from_json(&file.to_string())
        .unwrap_err()
        .contains("unknown field `surprise`"));
}

#[test]
fn the_json_boundary_answers_with_the_planned_encounter_or_named_diagnostics() {
    let (map, sites, recipe) = (town_map(800.0, json!({})), town_sites(800.0), recipe());
    let ask = |seed: &str, sites: &EncounterSites| -> Value {
        serde_json::from_str(&plan_encounter_json(
            &serde_json::to_string(&map).unwrap(),
            &serde_json::to_string(sites).unwrap(),
            &common::game().to_string(),
            &serde_json::to_string(&recipe).unwrap(),
            seed,
        ))
        .unwrap()
    };
    let ok = ask("1", &sites);
    assert_eq!(ok["status"], "ok");
    assert_eq!(ok["encounter"]["encounter_seed"], "1");
    let direct = plan(&map, &sites, &recipe).unwrap();
    assert_eq!(ok["encounter"], serde_json::to_value(&direct).unwrap());

    // A seed is canonical decimal text.
    let bad = ask("01", &sites);
    assert_eq!(bad["status"], "error");
    assert_eq!(bad["diagnostics"][0]["code"], "invalid_request");
    assert_eq!(bad["diagnostics"][0]["location"], "$.encounter_seed");

    let none = ask("1", &EncounterSites::default());
    assert_eq!(none["diagnostics"][0]["code"], "no_objective");
}
