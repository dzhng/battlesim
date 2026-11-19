use super::*;
use crate::fixtures::{self, test_rules};
use serde_json::{json, Value};

mod observed;
use observed::identified;

/// Test units as a U.S. deck, one card per category.
fn me(frame: &mut ObservationFrame) -> &mut contract::skirmish::PlayerView {
    &mut frame.skirmish.as_mut().unwrap().players[0]
}

fn deck() -> serde_json::Value {
    let mut rules = test_rules();
    for (id, category) in [
        ("test_rifle", "inf"),
        ("test_at", "inf"),
        ("test_recon", "rec"),
        ("test_jeep", "rec"),
        ("test_tank", "veh"),
        ("test_supply", "sup"),
    ] {
        fixtures::patch_catalog(
            &mut rules,
            "units",
            id,
            json!({ "roster": { "factions": ["us"], "category": category,
                "family_name": "Test", "variant": "Test" } }),
        );
    }
    rules
}

fn empty_match() -> (Rules, ObservationFrame, EntrySite) {
    let setup: contract::scenario::ScenarioDefinition = serde_json::from_value(json!({
        "map":{"size":[1000,1000],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35},
        "rules":deck(),"units":[{"side":"blue","kind":"test_rifle","position":[900,950]}],"events":[],"scripts":[]
    })).unwrap();
    let battle = crate::battle::Battle::new(&setup, 1);
    let mut frame = battle.observe(Side::Red).clone();
    frame.skirmish = Some(contract::skirmish::SkirmishView {
        phase: contract::skirmish::Phase::Preparation,
        players: vec![contract::skirmish::PlayerView {
            index: contract::players::PlayerIndex(0),
            id: "red-1".into(),
            controller: contract::players::ControllerKind::ActiveAi,
            ready: false,
            credits: 1000.0,
            income_per_minute: 0.0,
            unit_count: 0,
            cap: 30,
            pending: vec![],
            offered: None,
        }],
        preparation_remaining_s: 45.0,
        objectives: vec![contract::skirmish::ObjectiveView {
            id: "field".into(),
            center: [500.0, 500.0],
            radius_m: 50.0,
            owner: None,
            capturing: None,
            capture_progress: 0.0,
            contested: false,
        }],
        scores: [0.0; 2],
        result: None,
        report: None,
    });
    (
        setup.rules,
        frame,
        EntrySite {
            side: Side::Red,
            center: [500.0, 10.0],
            yaw: 1.57,
        },
    )
}

#[test]
fn missing_opening_role_uses_admitted_category_fallback() {
    let (_, frame, entry) = empty_match();
    // The deck's one infantry-role card loses the role.
    let mut authored = deck();
    fixtures::patch_catalog(
        &mut authored,
        "units",
        "test_rifle",
        json!({"roles":["at"]}),
    );
    let rules: Rules = serde_json::from_value(authored).unwrap();
    let orders = BalancedDoctrine::default().decide(
        DecisionInput {
            side: Side::Red,
            faction: Faction::Us,
            entry: &entry,
            assignment: None,
        },
        &frame,
        &rules,
        |_, _| Some(1.0),
    );
    let Order::ConfirmPurchase { variant, .. } = &orders[0] else {
        panic!("category fallback must purchase")
    };
    let selected = rules.catalog.by_id(variant);
    assert!(!selected.has_role("infantry"));
    assert_eq!(selected.roster.as_ref().unwrap().category, Category::Inf);
}

#[test]
fn opening_is_bought_and_readied_in_the_first_decision_within_budget_pending_and_cap() {
    let (rules, mut frame, entry) = empty_match();
    let mut ai = BalancedDoctrine::default();
    let mut ready = false;
    for iteration in 0..10 {
        frame.tick = iteration * 5 * u64::from(rules.tick_hz);
        for order in ai.decide(
            DecisionInput {
                side: Side::Red,
                faction: Faction::Us,
                entry: &entry,
                assignment: None,
            },
            &frame,
            &rules,
            |_, _| Some(1.0),
        ) {
            match order {
                Order::ConfirmPurchase { .. } => {
                    confirm_purchases(&rules, &mut frame, &[order]);
                }
                Order::Ready => {
                    ready = true;
                    me(&mut frame).ready = true;
                }
                _ => panic!("preparation must only buy or ready"),
            }
        }
        if ready {
            break;
        }
    }
    assert!(ready && frame.tick == 0, "the first decision readies");
    let view = frame.skirmish.as_ref().unwrap();
    assert!(view.players[0].pending.iter().any(|p| rules
        .catalog
        .get(p.kind)
        .has_role("logistics")
        && rules.catalog.get(p.kind).mobility.locomotion().is_some()));
    let mut capped = frame.clone();
    let view = capped.skirmish.as_mut().unwrap();
    view.phase = Phase::Active;
    view.players[0].unit_count = view.players[0].cap;
    assert!(
        !view.players[0].pending.is_empty(),
        "the cap test includes reservations"
    );
    capped.tick += 5 * u64::from(rules.tick_hz);
    assert!(!ai
        .decide(
            DecisionInput {
                side: Side::Red,
                faction: Faction::Us,
                entry: &entry,
                assignment: None
            },
            &capped,
            &rules,
            |_, _| Some(1.0)
        )
        .iter()
        .any(|o| matches!(o, Order::ConfirmPurchase { .. })));
}

#[test]
fn lane_purchases_clear_the_entry_before_the_staging_point() {
    let (rules, frame, entry) = empty_match();
    let assignment = Assignment {
        main_objective: [500.0, 500.0],
        staging_point: [500.0, 320.0],
        force_advance: false,
        parties: vec![],
    };
    let orders = BalancedDoctrine::default().decide(
        DecisionInput {
            side: Side::Red,
            faction: Faction::Us,
            entry: &entry,
            assignment: Some(&assignment),
        },
        &frame,
        &rules,
        |_, _| Some(1.0),
    );
    let destinations: Vec<_> = orders
        .iter()
        .filter_map(|order| match order {
            Order::ConfirmPurchase { destination, .. } => Some(*destination),
            _ => None,
        })
        .collect();
    assert!(destinations.len() >= 3, "opening did not buy a group");
    for destination in destinations {
        let from_entry = distance(destination, entry.center);
        let before_staging = distance(destination, assignment.staging_point)
            < distance(entry.center, assignment.staging_point);
        assert!(
            from_entry > 10.0,
            "arrival still occupies the entry: {destination:?}"
        );
        assert!(
            before_staging,
            "arrival passed the staging point: {destination:?}"
        );
        assert!((destination[0] - 500.0).abs() < 0.1);
        assert!((destination[1] - 34.0).abs() < 0.1);
    }
}

#[test]
fn objective_orders_use_known_route_cost_and_stable_sites() {
    let (rules, mut frame, entry) = empty_match();
    let setup: contract::scenario::ScenarioDefinition = serde_json::from_value(json!({
        "map":{"size":[1000,1000],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35},
        "rules":deck(),"units":[{"side":"red","kind":"test_rifle","position":[500,100]}],
        "events":[],"scripts":[]
    }))
    .unwrap();
    let battle = crate::battle::Battle::new(&setup, 1);
    frame.own = battle.observe(Side::Red).own.clone();
    let view = frame.skirmish.as_mut().unwrap();
    view.phase = Phase::Active;
    view.players[0].credits = 0.0;
    let mut distant = view.objectives[0].clone();
    distant.id = "a-distant".into();
    distant.center = [800.0, 800.0];
    view.objectives.push(distant);
    for (far, near, wanted) in [
        (1.0, 10.0, [800.0, 800.0]),
        (10.0, 1.0, [500.0, 500.0]),
        (1.0, 1.0, [800.0, 800.0]),
    ] {
        let orders = BalancedDoctrine::default().decide(
            DecisionInput {
                side: Side::Red,
                faction: Faction::Us,
                entry: &entry,
                assignment: None,
            },
            &frame,
            &rules,
            |_, goal| Some(if goal == [800.0, 800.0] { far } else { near }),
        );
        assert!(orders
            .iter()
            .any(|o| matches!(o,Order::AttackMove{units,goal,..}
            if units==&vec![UnitId(0)] && *goal==wanted)));
    }
    assert!(BalancedDoctrine::default()
        .decide(
            DecisionInput {
                side: Side::Red,
                faction: Faction::Us,
                entry: &entry,
                assignment: None
            },
            &frame,
            &rules,
            |_, _| None
        )
        .is_empty());
}

#[test]
fn zero_unit_opening_uses_an_affordable_admitted_faction_purchase() {
    let (rules, frame, entry) = empty_match();
    let orders = BalancedDoctrine::default().decide(
        DecisionInput {
            side: Side::Red,
            faction: Faction::Us,
            entry: &entry,
            assignment: None,
        },
        &frame,
        &rules,
        |_, _| Some(1.0),
    );
    let Order::ConfirmPurchase {
        variant,
        destination,
        ..
    } = orders.first().expect("buy an opening unit")
    else {
        panic!("first command must purchase")
    };
    let card = rules.catalog.card(variant).unwrap();
    assert!(
        card.disabled_reason.is_none()
            && card.cost as f64 <= frame.skirmish.as_ref().unwrap().players[0].credits
    );
    assert!(card.roster.factions.contains(&Faction::Us));
    assert_eq!(*destination, [500.0, 500.0]);
}

#[test]
fn seeded_policy_instances_repeat_the_same_opening_commands() {
    let (rules, frame, entry) = empty_match();
    let mut left = BalancedDoctrine::new(17);
    let mut right = BalancedDoctrine::new(17);
    let left_orders = left.decide(
        DecisionInput {
            side: Side::Red,
            faction: Faction::Us,
            entry: &entry,
            assignment: None,
        },
        &frame,
        &rules,
        |_, _| Some(1.0),
    );
    let right_orders = right.decide(
        DecisionInput {
            side: Side::Red,
            faction: Faction::Us,
            entry: &entry,
            assignment: None,
        },
        &frame,
        &rules,
        |_, _| Some(1.0),
    );
    assert_eq!(left_orders, right_orders);
}

#[test]
fn different_seeds_can_change_a_reinforcement_choice_without_changing_legality() {
    let (rules, mut frame, entry) = empty_match();
    frame.skirmish.as_mut().unwrap().phase = Phase::Active;
    me(&mut frame).unit_count = 1;
    me(&mut frame).credits = 500.0;
    me(&mut frame).pending.push(pending(
        1,
        rules.catalog.index("test_supply").unwrap(),
        entry.center,
        0,
    ));
    let mut variants = std::collections::BTreeSet::new();
    for seed in 0..32 {
        let orders = BalancedDoctrine::new(seed).decide(
            DecisionInput {
                side: Side::Red,
                faction: Faction::Us,
                entry: &entry,
                assignment: None,
            },
            &frame,
            &rules,
            |_, _| Some(1.0),
        );
        if let Some(Order::ConfirmPurchase { variant, .. }) = orders
            .iter()
            .find(|order| matches!(order, Order::ConfirmPurchase { .. }))
        {
            variants.insert(variant.clone());
        }
    }
    assert!(
        variants.len() > 1,
        "bounded seed stream should vary role choices"
    );
}

#[test]
fn identified_helicopter_selects_a_catalog_unit_that_can_engage_low_air() {
    let mut authored = deck();
    fixtures::patch_catalog(
        &mut authored,
        "units",
        "test_rifle",
        json!({
            "body": { "squad": { "slots": ["test_atgm_gunner"] } }
        }),
    );
    fixtures::patch_catalog(
        &mut authored,
        "units",
        "test_jeep",
        json!({ "roster": { "factions": ["us"], "category": "veh", "family_name": "Test", "variant": "Air guard" } }),
    );
    let rules: Rules = serde_json::from_value(authored).unwrap();
    let (mut frame, entry) = {
        let (rules0, frame, entry) = empty_match();
        assert_eq!(
            rules0
                .catalog
                .id(rules.catalog.index("test_rifle").unwrap()),
            "test_rifle"
        );
        (frame, entry)
    };
    let heli = rules.catalog.index("test_heli").unwrap();
    frame.skirmish.as_mut().unwrap().phase = Phase::Active;
    me(&mut frame).credits = 1_000.0;
    me(&mut frame).unit_count = 1;
    let supply = rules.catalog.index("test_supply").unwrap();
    me(&mut frame)
        .pending
        .push(pending(1, supply, entry.center, 0));
    frame.own.clear();
    frame
        .identified
        .push(contract::observation::IdentifiedUnit {
            cost: rules.catalog.get(heli).cost,
            ..identified(7, heli, [500.0, 500.0, 20.0])
        });
    let mut ai = BalancedDoctrine::new(0);
    let orders = ai.decide(
        DecisionInput {
            side: Side::Red,
            faction: Faction::Us,
            entry: &entry,
            assignment: None,
        },
        &frame,
        &rules,
        |_, _| Some(1.0),
    );
    let Order::ConfirmPurchase { variant, .. } = orders
        .iter()
        .find(|order| matches!(order, Order::ConfirmPurchase { .. }))
        .expect("an active reinforcement purchase")
    else {
        unreachable!()
    };
    let kind = rules.catalog.index(variant).unwrap();
    assert!(unit_engages_aircraft_in(
        &rules,
        kind,
        contract::catalog::AltitudeLayer::LowAir
    ));
}

#[test]
fn doctrine_pivots_after_maximum_dwell_without_visible_progress() {
    let (rules, mut frame, entry) = empty_match();
    frame.skirmish.as_mut().unwrap().phase = Phase::Active;
    me(&mut frame).credits = 0.0;
    // Its tuning, not a number in the code, says how long it dwells: 60 s.
    let mut ai = BalancedDoctrine::new(7);
    ai.tuning.max_dwell_s = 60.0;
    ai.decide(
        DecisionInput {
            side: Side::Red,
            faction: Faction::Us,
            entry: &entry,
            assignment: None,
        },
        &frame,
        &rules,
        |_, _| Some(1.0),
    );
    assert_eq!(ai.doctrine(), Doctrine::BalancedOpening);
    frame.tick = 55 * u64::from(rules.tick_hz);
    ai.decide(
        DecisionInput {
            side: Side::Red,
            faction: Faction::Us,
            entry: &entry,
            assignment: None,
        },
        &frame,
        &rules,
        |_, _| Some(1.0),
    );
    assert_eq!(ai.doctrine(), Doctrine::BalancedOpening);
    frame.tick = 65 * u64::from(rules.tick_hz);
    ai.decide(
        DecisionInput {
            side: Side::Red,
            faction: Faction::Us,
            entry: &entry,
            assignment: None,
        },
        &frame,
        &rules,
        |_, _| Some(1.0),
    );
    assert_eq!(ai.doctrine(), Doctrine::ReconScreen);
    assert_eq!(ai.pivot_count(), 1);
}

#[test]
fn recon_moves_to_a_sensor_standoff_before_objective() {
    let (rules, mut frame, entry) = empty_match();
    let setup: contract::scenario::ScenarioDefinition = serde_json::from_value(json!({
        "map":{"size":[1000,1000],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35},
        "rules":deck(),"units":[{"side":"red","kind":"test_recon","position":[500,100]}],
        "events":[],"scripts":[]
    }))
    .unwrap();
    let battle = crate::battle::Battle::new(&setup, 1);
    frame.own = battle.observe(Side::Red).own.clone();
    frame.skirmish.as_mut().unwrap().phase = Phase::Active;
    me(&mut frame).credits = 0.0;
    let mut ai = BalancedDoctrine::default();
    let orders = ai.decide(
        DecisionInput {
            side: Side::Red,
            faction: Faction::Us,
            entry: &entry,
            assignment: None,
        },
        &frame,
        &rules,
        |_, _| Some(1.0),
    );
    let goal = orders
        .iter()
        .find_map(|order| match order {
            Order::Move { units, goal, .. } if units == &vec![UnitId(0)] => Some(*goal),
            _ => None,
        })
        .expect("recon gets a movement order");
    assert!(goal[1] < 500.0);
    assert!(distance(goal, [500.0, 500.0]) > 100.0);
}

#[test]
fn a_route_blocked_unit_gets_a_new_non_plan_order_next_decision() {
    let (rules, mut frame, entry) = empty_match();
    let setup: contract::scenario::ScenarioDefinition = serde_json::from_value(json!({
        "map":{"size":[1000,1000],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35},
        "rules":deck(),"units":[{"side":"red","kind":"test_jeep","position":[500,100]}],
        "events":[],"scripts":[]
    }))
    .unwrap();
    let battle = crate::battle::Battle::new(&setup, 1);
    frame.own = battle.observe(Side::Red).own.clone();
    frame.skirmish.as_mut().unwrap().phase = Phase::Active;
    me(&mut frame).credits = 0.0;
    me(&mut frame).unit_count = 1;
    frame.own[0].state = contract::observation::MoveState::RouteBlocked;
    frame.own[0].goal = Some([500.0, 500.0]);
    let orders = BalancedDoctrine::default().decide(
        DecisionInput {
            side: Side::Red,
            faction: Faction::Us,
            entry: &entry,
            assignment: None,
        },
        &frame,
        &rules,
        |_, _| Some(1.0),
    );
    assert!(orders.iter().any(|order| matches!(
        order,
        Order::AttackMove { units, .. } if units == &vec![frame.own[0].id]
    )));
    let assignment = Assignment {
        main_objective: [500.0, 500.0],
        staging_point: [500.0, 400.0],
        force_advance: false,
        parties: vec![],
    };
    let orders = BalancedDoctrine::default().decide(
        DecisionInput {
            side: Side::Red,
            faction: Faction::Us,
            entry: &entry,
            assignment: Some(&assignment),
        },
        &frame,
        &rules,
        |_, _| Some(1.0),
    );
    assert!(orders.iter().any(|order| matches!(
        order,
        Order::AttackMove { units, .. } if units == &vec![frame.own[0].id]
    )));
}

#[test]
fn nearest_air_target_uses_body_distance_in_three_dimensions() {
    let mut authored = deck();
    fixtures::patch_catalog(
        &mut authored,
        "units",
        "test_jeep",
        json!({ "mounts": [{ "id": "AA", "name": "AA", "weapons": ["aa_long"], "turret": true, "pivot_m": [0, 0, 1.68], "muzzle_m": [1.43, 0, 0.32] }] }),
    );
    authored["weapons"]["aa_long"]["targets"]["low_air"] =
        json!({ "classes": ["air_rotor", "air_fixed_wing"], "range_m": 5200, "scatter_mrad": 0.5 });
    let rules: Rules = serde_json::from_value(authored.clone()).unwrap();
    let setup: contract::scenario::ScenarioDefinition = serde_json::from_value(json!({
        "map":{"size":[1000,1000],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35},
        "rules":authored,"units":[{"side":"red","kind":"test_jeep","position":[500,500]}],
        "events":[],"scripts":[]
    }))
    .unwrap();
    let battle = crate::battle::Battle::new(&setup, 1);
    let (mut frame, entry) = {
        let (_, mut frame, entry) = empty_match();
        frame.own = battle.observe(Side::Red).own.clone();
        (frame, entry)
    };
    frame.skirmish.as_mut().unwrap().phase = Phase::Active;
    me(&mut frame).credits = 0.0;
    me(&mut frame).unit_count = 1;
    let heli = rules.catalog.index("test_heli").unwrap();
    let jet = rules.catalog.index("test_jet").unwrap();
    for (id, kind, position) in [
        (1, heli, [550.0, 500.0, 20.0]),
        (2, jet, [450.0, 500.0, 250.0]),
    ] {
        frame
            .identified
            .push(contract::observation::IdentifiedUnit {
                cost: 1,
                ..identified(id, kind, position)
            });
    }
    let orders = BalancedDoctrine::default().decide(
        DecisionInput {
            side: Side::Red,
            faction: Faction::Us,
            entry: &entry,
            assignment: None,
        },
        &frame,
        &rules,
        |_, _| Some(1.0),
    );
    assert!(orders.iter().any(|order| matches!(
        order,
        Order::Attack { units, target: TargetRef::Identified { id } }
            if units == &vec![frame.own[0].id] && *id == contract::observation::ObservedTargetId(1)
    )));
}

#[test]
fn opening_procures_an_authored_artillery_role() {
    let mut authored = deck();
    fixtures::patch_catalog(
        &mut authored,
        "units",
        "test_jeep",
        json!({
            "roles": ["artillery"],
            "roster": { "factions": ["us"], "category": "sup", "family_name": "Test", "variant": "Test artillery" }
        }),
    );
    let rules: Rules = serde_json::from_value(authored).unwrap();
    let (_, frame, entry) = empty_match();
    let orders = BalancedDoctrine::default().decide(
        DecisionInput {
            side: Side::Red,
            faction: Faction::Us,
            entry: &entry,
            assignment: None,
        },
        &frame,
        &rules,
        |_, _| Some(1.0),
    );
    assert!(orders.iter().any(|order| matches!(
        order,
        Order::ConfirmPurchase { variant, .. }
            if rules.catalog.by_id(variant).has_role("artillery")
    )));
}

#[test]
fn every_ai_opening_includes_a_high_air_defender() {
    let rules: Rules = serde_json::from_value(fixtures::rules()).unwrap();
    let (_, frame, entry) = empty_match();
    for faction in [Faction::Us, Faction::Europe, Faction::Eastern] {
        let orders = BalancedDoctrine::default().decide(
            DecisionInput {
                side: Side::Red,
                faction,
                entry: &entry,
                assignment: None,
            },
            &frame,
            &rules,
            |_, _| Some(1.0),
        );
        assert!(
            orders.iter().any(|order| matches!(
                order,
                Order::ConfirmPurchase { variant, .. }
                    if unit_engages_aircraft_in(
                        &rules,
                        rules.catalog.index(variant).unwrap(),
                        AltitudeLayer::HighAir
                    )
            )),
            "{faction:?} opening has no HighAir defender: {orders:?}"
        );
    }

    // The pick reads weapon rows, not a list: left as Europe's only air
    // defender, the Gepard is the one the opening buys. Every other
    // high-air-capable row leaves Europe's roster, and one Europe alone
    // fielded leaves the catalog.
    let gepard = "europe_gepard_1a2";
    let defends = |id: &str| {
        id != gepard
            && rules
                .catalog
                .index(id)
                .is_some_and(|k| unit_engages_aircraft_in(&rules, k, AltitudeLayer::HighAir))
    };
    let mut authored = fixtures::rules();
    for doc in authored["catalog"].as_array_mut().unwrap() {
        let Some(units) = doc.get_mut("units").and_then(Value::as_object_mut) else {
            continue;
        };
        units.retain(|id, unit| {
            let Some(factions) = unit
                .pointer_mut("/roster/factions")
                .and_then(Value::as_array_mut)
            else {
                return true;
            };
            if defends(id) {
                factions.retain(|f| f != "europe");
            }
            !factions.is_empty()
        });
    }
    let only_gepard: Rules = serde_json::from_value(authored).unwrap();
    let orders = BalancedDoctrine::default().decide(
        DecisionInput {
            side: Side::Red,
            faction: Faction::Europe,
            entry: &entry,
            assignment: None,
        },
        &frame,
        &only_gepard,
        |_, _| Some(1.0),
    );
    assert!(
        orders.iter().any(|order| matches!(
            order,
            Order::ConfirmPurchase { variant, .. } if variant == gepard
        )),
        "Europe's opening skipped its only air defender: {orders:?}"
    );
}

/// The AA card is the opening's first claim on its wallet, not a hold on
/// it: when it costs more than the whole wallet, the rest of the opening is
/// still bought.
#[test]
fn an_opening_that_cannot_afford_its_aa_card_buys_the_rest() {
    let mut authored = deck();
    for id in [
        "test_rifle",
        "test_at",
        "test_recon",
        "test_tank",
        "test_supply",
    ] {
        fixtures::patch_catalog(&mut authored, "units", id, json!({ "cost": 50 }));
    }
    fixtures::patch_catalog(
        &mut authored,
        "units",
        "test_jeep",
        json!({ "cost": 900, "mounts": [{ "id": "HMG", "weapons": ["aa_long"] }] }),
    );
    let rules: Rules = serde_json::from_value(authored).unwrap();
    let (_, mut frame, entry) = empty_match();
    me(&mut frame).credits = 400.0;
    let orders = BalancedDoctrine::default().decide(
        DecisionInput {
            side: Side::Red,
            faction: Faction::Us,
            entry: &entry,
            assignment: None,
        },
        &frame,
        &rules,
        |_, _| Some(1.0),
    );
    let bought = confirm_purchases(&rules, &mut frame, &orders);
    let aa = rules.catalog.index("test_jeep").unwrap();
    assert!(!bought.is_empty(), "the opening bought nothing: {orders:?}");
    assert!(!bought.contains(&aa));
}

#[test]
fn the_ai_sends_bought_jets_on_attack_move() {
    let mut authored = deck();
    fixtures::patch_catalog(
        &mut authored,
        "units",
        "test_jet",
        json!({
            "roster": { "factions": ["us"], "category": "air", "family_name": "Test jet", "variant": "Test" }
        }),
    );
    let rules: Rules = serde_json::from_value(authored.clone()).unwrap();
    let setup: contract::scenario::ScenarioDefinition = serde_json::from_value(json!({
        "map":{"size":[1000,1000],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35},
        "rules":authored,"units":[{"side":"red","kind":"test_jet","position":[500,100]}],
        "events":[],"scripts":[]
    }))
    .unwrap();
    let battle = crate::battle::Battle::new(&setup, 1);
    let (_, mut frame, entry) = empty_match();
    frame.own = battle.observe(Side::Red).own.clone();
    let view = frame.skirmish.as_mut().unwrap();
    view.phase = Phase::Active;
    let me = &mut view.players[0];
    me.credits = 500.0;
    me.unit_count = 1;
    me.pending.push(pending(
        1,
        rules.catalog.index("test_supply").unwrap(),
        entry.center,
        0,
    ));
    let jet = rules.catalog.index("test_jet").unwrap();
    assert_eq!(frame.own[0].kind, jet);
    let mut ai = BalancedDoctrine {
        reinforcement: 6,
        ..BalancedDoctrine::default()
    };
    let orders = ai.decide(
        DecisionInput {
            side: Side::Red,
            faction: Faction::Us,
            entry: &entry,
            assignment: None,
        },
        &frame,
        &rules,
        |_, _| Some(1.0),
    );
    assert!(orders.iter().any(|order| matches!(
        order,
        Order::ConfirmPurchase { variant, .. } if variant == "test_jet"
    )));
    assert!(orders.iter().any(|order| matches!(
        order,
        Order::AttackMove { units, .. } if units == &vec![frame.own[0].id]
    )));
}

/// Credit a wallet's purchases to the frame the way the match does: prices
/// out, a place per unit taken, purchase pending. Returns every kind bought,
/// carriers included.
fn confirm_purchases(
    rules: &Rules,
    frame: &mut ObservationFrame,
    orders: &[Order],
) -> Vec<TypeIndex> {
    let tick = frame.tick;
    let view = frame.skirmish.as_mut().unwrap();
    let mut bought = Vec::new();
    for order in orders {
        let Order::ConfirmPurchase {
            variant,
            carrier,
            destination,
        } = order
        else {
            continue;
        };
        let mut row = pending(0, rules.catalog.index(variant).unwrap(), *destination, tick);
        row.carrier = carrier.as_deref().and_then(|id| rules.catalog.index(id));
        let price: f64 = row
            .kinds()
            .map(|k| f64::from(rules.catalog.get(k).cost))
            .sum();
        let me = &mut view.players[0];
        assert!(price <= me.credits && me.unit_count + row.units() <= me.cap);
        row.id = contract::skirmish::PurchaseId(me.pending.len() as u32);
        me.credits -= price;
        me.unit_count += row.units();
        bought.extend(row.kinds());
        me.pending.push(row);
    }
    bought
}

fn entry_from(center: [f64; 2]) -> EntrySite {
    EntrySite {
        side: Side::Red,
        center,
        yaw: 0.0,
    }
}

fn firing_contact(
    center: [f64; 3],
    evidence_tick: Tick,
    expires_tick: Tick,
) -> contract::observation::ApproximateContact {
    contract::observation::ApproximateContact {
        id: contract::observation::ContactId(0),
        source: contract::observation::ContactSource::Firing,
        center,
        layer: AltitudeLayer::Ground,
        aloft: false,
        radius: 30.0,
        evidence_tick,
        expires_tick,
        kind: None,
        heard: 0,
        bearing_deg: None,
        range_m: None,
    }
}

fn attack_targets(orders: &[Order], unit: UnitId) -> Vec<ObservedTargetId> {
    orders
        .iter()
        .filter_map(|order| match order {
            Order::Attack {
                units,
                target: TargetRef::Identified { id },
            } if units == &vec![unit] => Some(*id),
            _ => None,
        })
        .collect()
}

/// A player who sends a jet at a tank lets it keep making passes at that
/// tank while it is still there, rather than re-aiming it at whatever is
/// nearest each time they look. A jet's target changes only once the side
/// can no longer see it (destroyed or lost).
#[test]
fn a_jet_keeps_its_target_while_the_side_still_sees_it() {
    let (rules, mut frame, entry) = empty_match();
    let setup: contract::scenario::ScenarioDefinition = serde_json::from_value(json!({
        "map":{"size":[1000,1000],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35},
        "rules":deck(),"units":[{"side":"red","kind":"test_jet","position":[500,100]}],
        "events":[],"scripts":[]
    }))
    .unwrap();
    let battle = crate::battle::Battle::new(&setup, 1);
    frame.own = battle.observe(Side::Red).own.clone();
    let jet = frame.own[0].id;
    let view = frame.skirmish.as_mut().unwrap();
    view.phase = Phase::Active;
    view.players[0].credits = 0.0;
    let tank = rules.catalog.index("test_tank").unwrap();
    frame.identified = vec![identified(1, tank, [500.0, 600.0, 0.0])];
    let mut ai = BalancedDoctrine::default();
    let decide = |ai: &mut BalancedDoctrine, frame: &ObservationFrame| {
        ai.decide(
            DecisionInput {
                side: Side::Red,
                faction: Faction::Us,
                entry: &entry,
                assignment: None,
            },
            frame,
            &rules,
            |_, _| Some(1.0),
        )
    };
    assert_eq!(
        attack_targets(&decide(&mut ai, &frame), jet),
        vec![ObservedTargetId(1)]
    );

    // A second tank turns up right under the jet, at the next decision.
    frame.tick += 10 * u64::from(rules.tick_hz);
    frame
        .identified
        .push(identified(2, tank, [500.0, 120.0, 0.0]));
    let kept = attack_targets(&decide(&mut ai, &frame), jet);
    assert!(
        kept.iter().all(|&id| id == ObservedTargetId(1)),
        "the jet was re-aimed off a target it still sees: {kept:?}"
    );

    // The first tank is gone from the side's picture: now the jet re-aims.
    frame.tick += 10 * u64::from(rules.tick_hz);
    frame.identified.retain(|t| t.id != ObservedTargetId(1));
    assert_eq!(
        attack_targets(&decide(&mut ai, &frame), jet),
        vec![ObservedTargetId(2)]
    );
}

/// A launcher its own cover holds is driven out by the AI that owns it, as
/// a player would drive it out: a red mortar jeep standing behind a high
/// wall is sent by the AI to shell the approach to an objective blue holds,
/// over the wall, and holds for a clear launch path (`BlockedTrajectory`);
/// a ground point is no target to close on, so nothing else moves it.
/// After its tuning's `relocate_after_s` of that, the AI orders it about
/// `relocate_m` off, clear of the wall, where it fires.
#[test]
fn the_ai_drives_a_launcher_its_cover_holds_to_open_ground_and_fires() {
    let mut rules = deck();
    rules["weapons"]["test_launcher"] = json!({
        "extends": "artillery_mortar", "arc_height_m": null, "speed_mps": 80,
        "targets": { "ground": { "range_m": 650, "scatter_mrad": 26 } },
        "requires_clear_launch_path": true,
    });
    rules["weapons"]["test_launcher"]["round_cost"] = json!(1);
    fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jeep",
        json!({
            "roles": ["artillery"],
            "mounts": [{ "id": "HMG", "weapons": ["test_launcher"] }],
        }),
    );
    let setup: contract::scenario::ScenarioDefinition = serde_json::from_value(json!({
        "map": { "size": [800, 600], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
            "props": [{ "kind": "wall", "center": [400, 446], "yaw": 0,
                "half_extents": [5.0, 0.5, 7.0] }] },
        "rules": rules,
        "units": [{ "side": "red", "kind": "test_jeep", "position": [400, 450], "yaw": -std::f64::consts::FRAC_PI_2 }],
        "events": [], "scripts": []
    }))
    .unwrap();
    let mut battle = crate::battle::Battle::new(&setup, 3);
    let (_, empty, _) = empty_match();
    let mut view = empty.skirmish.unwrap();
    view.phase = Phase::Active;
    view.players[0].credits = 0.0;
    view.objectives[0].center = [400.0, 150.0];
    view.objectives[0].owner = Some(Side::Blue);
    let entry = entry_from([400.0, 590.0]);
    let mut ai = BalancedDoctrine::new(3);
    ai.tuning.relocate_after_s = 10.0;
    ai.tuning.relocate_m = 90.0;
    let mut seq = 0;
    let jeep = UnitId(0);
    let start = battle.unit(jeep).unwrap().position;
    let mut held = false;
    let mut fired_from = None;
    for _ in 0..120 * battle.rules().tick_hz {
        let mut frame = battle.observe(Side::Red).clone();
        frame.skirmish = Some(view.clone());
        let rules = battle.rules().clone();
        let orders = ai.decide(
            DecisionInput {
                side: Side::Red,
                faction: Faction::Us,
                entry: &entry,
                assignment: None,
            },
            &frame,
            &rules,
            |id, goal| battle.route_time(Side::Red, id, goal),
        );
        for order in orders {
            seq += 1;
            let ack = battle.accept(contract::command::CommandEnvelope {
                side: Side::Red,
                player: None,
                seq,
                order,
                queued: false,
            });
            assert_eq!(ack.error, None, "the AI's order {seq} was refused");
        }
        battle.step();
        let shown = battle.observe(Side::Red).own[0].mounts[0].reason;
        held |= shown == contract::observation::ActionReason::BlockedTrajectory;
        if battle.rounds().any(|(_, r)| r.unit == jeep) {
            fired_from = Some(battle.unit(jeep).unwrap().position);
            break;
        }
    }
    assert!(held, "the launcher was never held by the wall");
    let at = fired_from.expect("the launcher never fired");
    let moved = (at - start).xy().length();
    assert!(
        (70.0..=110.0).contains(&moved),
        "it fired {moved:.0} m from where it was held"
    );
}

mod airlift_tests;
mod artillery;
mod backline;
mod groups;
mod parties;
mod purchase_plan;
mod purchases;
mod ride_purchases;
mod smoke;
mod support;
mod targeting;
mod transport_tests;

/// A purchase of `kind` alone, confirmed at `tick` and not yet entering.
fn pending(
    id: u32,
    kind: TypeIndex,
    destination: [f64; 2],
    tick: Tick,
) -> contract::skirmish::PendingPurchase {
    contract::skirmish::PendingPurchase {
        id: contract::skirmish::PurchaseId(id),
        kind,
        carrier: None,
        entry: 0,
        destination,
        confirmed_tick: tick,
        blocked: false,
        arrival_tick: None,
        condition: None,
    }
}
