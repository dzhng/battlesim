use super::*;
use crate::fixtures::{self, test_rules};
use serde_json::{json, Value};

/// Test units as a U.S. deck: a card for each, in its category.
/// The frame's one player row, the deciding player's (`own_player`).
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
            unit_count: 0,
            cap: 30,
            pending: vec![],
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
fn a_scripted_purchase_plan_replaces_the_doctrines_own_shopping() {
    let (rules, mut frame, entry) = empty_match();
    let plan = vec![
        rules.catalog.index("test_tank").unwrap(),
        rules.catalog.index("test_rifle").unwrap(),
    ];
    let expected = plan
        .iter()
        .map(|&kind| rules.catalog.id(kind).to_owned())
        .collect::<Vec<_>>();
    let mut ai = BalancedDoctrine::default();
    ai.set_purchase_plan(plan);
    let first = ai
        .decide(
            DecisionInput {
                side: Side::Red,
                faction: Faction::Us,
                entry: &entry,
                assignment: None,
            },
            &frame,
            &rules,
            |_, _| Some(1.0),
        )
        .into_iter()
        .filter_map(|order| match order {
            Order::ConfirmPurchase { variant, .. } => Some(variant),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        first, expected,
        "an affordable scripted opening is one batch"
    );
    for _ in 0..expected.len() {
        ai.purchase_result(true);
    }
    frame.tick += 5 * u64::from(rules.tick_hz);
    let second = ai
        .decide(
            DecisionInput {
                side: Side::Red,
                faction: Faction::Us,
                entry: &entry,
                assignment: None,
            },
            &frame,
            &rules,
            |_, _| Some(1.0),
        )
        .into_iter()
        .filter_map(|order| match order {
            Order::ConfirmPurchase { variant, .. } => Some(variant),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(second.is_empty(), "the batch exhausted before readiness");
    assert_eq!(
        first.into_iter().chain(second).collect::<Vec<_>>(),
        expected
    );
}

#[test]
fn a_refused_scripted_purchase_is_retried_in_order() {
    let (rules, mut frame, entry) = empty_match();
    me(&mut frame).credits = 1_000.0;
    let plan = vec![
        rules.catalog.index("test_tank").unwrap(),
        rules.catalog.index("test_rifle").unwrap(),
    ];
    let mut ai = BalancedDoctrine::default();
    ai.set_purchase_plan(plan.clone());
    let purchase = |ai: &mut BalancedDoctrine, frame: &mut ObservationFrame| {
        frame.tick += 5 * u64::from(rules.tick_hz);
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
        .into_iter()
        .find_map(|order| match order {
            Order::ConfirmPurchase { variant, .. } => Some(variant),
            _ => None,
        })
        .unwrap()
    };
    let first = purchase(&mut ai, &mut frame);
    ai.purchase_result(false);
    assert_eq!(purchase(&mut ai, &mut frame), first);
    ai.purchase_result(true);
    assert_eq!(
        purchase(&mut ai, &mut frame),
        rules.catalog.id(plan[1]).to_owned()
    );
}

#[test]
fn support_truck_moves_behind_observed_troops_and_deploys_after_arrival() {
    let (rules, mut frame, entry) = empty_match();
    let setup: contract::scenario::ScenarioDefinition = serde_json::from_value(json!({
        "map":{"size":[1000,1000],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35},
        "rules":deck(),"units":[
            {"side":"red","kind":"test_rifle","position":[500,600]},
            {"side":"red","kind":"test_supply","position":[500,10]}],
        "events":[],"scripts":[]
    }))
    .unwrap();
    let battle = crate::battle::Battle::new(&setup, 1);
    frame.own = battle.observe(Side::Red).own.clone();
    let view = frame.skirmish.as_mut().unwrap();
    view.phase = Phase::Active;
    view.players[0].credits = 0.0;
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
    let support = orders
        .iter()
        .find_map(|o| match o {
            Order::Move { units, goal, .. } if units == &vec![UnitId(1)] => Some(*goal),
            _ => None,
        })
        .unwrap();
    assert!(support[1] > entry.center[1] && support[1] < frame.own[0].position[1]);
    frame.own[0].goal = Some([500.0, 500.0]);
    frame.own[1].position = [support[0], support[1], 0.0];
    frame.own[1].goal = None;
    // Explicitly packed own posture needs a deploy command; ordinary stopped
    // trucks already head to deployed through the deployment owner.
    frame.own[1].deployment.as_mut().unwrap().target = contract::observation::Posture::Packed;
    frame.tick += 5 * u64::from(rules.tick_hz);
    let orders = ai.decide(
        DecisionInput {
            side: Side::Red,
            faction: Faction::Us,
            entry: &entry,
            assignment: None,
        },
        &frame,
        &rules,
        |_, _| panic!("standing deployment needs no new route"),
    );
    assert!(orders
        .iter()
        .any(|o| matches!(o,Order::SetDeployment{units,deployed:true} if units==&vec![UnitId(1)])));
}

#[test]
fn reinforcement_replaces_a_lost_truck_before_more_combat_units() {
    let (rules, mut frame, entry) = empty_match();
    let view = frame.skirmish.as_mut().unwrap();
    view.phase = Phase::Active;
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
    let Order::ConfirmPurchase {
        variant,
        destination,
    } = &orders[0]
    else {
        panic!("replace truck")
    };
    let kind = rules.catalog.index(variant).unwrap();
    assert!(rules.catalog.get(kind).has_role("logistics"));
    assert_eq!(*destination, entry.center);
    me(&mut frame)
        .pending
        .push(contract::skirmish::PendingPurchase {
            id: contract::skirmish::PurchaseId(1),
            kind,
            entry: 0,
            destination: *destination,
            confirmed_tick: 0,
            blocked: false,
            arrival_tick: None,
        });
    assert!(ai
        .decide(
            DecisionInput {
                side: Side::Red,
                faction: Faction::Us,
                entry: &entry,
                assignment: None
            },
            &frame,
            &rules,
            |_, _| Some(1.0)
        )
        .is_empty());
    frame.tick = 5 * u64::from(rules.tick_hz);
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
    let Order::ConfirmPurchase { variant, .. } = &orders[0] else {
        panic!("reinforce combat")
    };
    assert!(!rules.catalog.by_id(variant).has_role("logistics"));
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
            let view = frame.skirmish.as_mut().unwrap();
            match order {
                Order::ConfirmPurchase {
                    variant,
                    destination,
                } => {
                    let kind = rules.catalog.index(&variant).unwrap();
                    let price = f64::from(rules.catalog.get(kind).cost);
                    let me = &mut view.players[0];
                    assert!(price <= me.credits && me.unit_count < me.cap);
                    me.credits -= price;
                    me.unit_count += 1;
                    me.pending.push(contract::skirmish::PendingPurchase {
                        id: contract::skirmish::PurchaseId(me.pending.len() as u32),
                        kind,
                        entry: 0,
                        destination,
                        confirmed_tick: frame.tick,
                        blocked: false,
                        arrival_tick: None,
                    });
                }
                Order::Ready => {
                    ready = true;
                    view.players[0].ready = true;
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
    assert!(view.players[0]
        .pending
        .iter()
        .any(|p| rules.catalog.get(p.kind).has_role("logistics")));
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
    me(&mut frame)
        .pending
        .push(contract::skirmish::PendingPurchase {
            id: contract::skirmish::PurchaseId(1),
            kind: rules.catalog.index("test_supply").unwrap(),
            entry: 0,
            destination: entry.center,
            confirmed_tick: 0,
            blocked: false,
            arrival_tick: None,
        });
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
        .push(contract::skirmish::PendingPurchase {
            id: contract::skirmish::PurchaseId(1),
            kind: supply,
            entry: 0,
            destination: entry.center,
            confirmed_tick: 0,
            blocked: false,
            arrival_tick: None,
        });
    frame.own.clear();
    frame
        .identified
        .push(contract::observation::IdentifiedUnit {
            id: contract::observation::ObservedTargetId(7),
            kind: heli,
            cost: rules.catalog.get(heli).cost,
            position: [500.0, 500.0, 20.0],
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
                id: contract::observation::ObservedTargetId(id),
                kind,
                cost: 1,
                position,
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
fn the_ai_buys_more_high_air_defence_when_it_sees_enemy_jets() {
    let mut authored = deck();
    fixtures::patch_catalog(
        &mut authored,
        "units",
        "test_jeep",
        json!({
            "mounts": [{ "id": "HMG", "weapons": ["aa_long"] }]
        }),
    );
    fixtures::patch_catalog(
        &mut authored,
        "units",
        "test_jet",
        json!({
            "roster": { "factions": ["us"], "category": "air", "family_name": "Test jet", "variant": "Test" }
        }),
    );
    let rules: Rules = serde_json::from_value(authored).unwrap();
    let (_, mut frame, entry) = empty_match();
    let view = frame.skirmish.as_mut().unwrap();
    view.phase = Phase::Active;
    let me = &mut view.players[0];
    me.credits = 500.0;
    me.unit_count = 1;
    me.pending.push(contract::skirmish::PendingPurchase {
        id: contract::skirmish::PurchaseId(1),
        kind: rules.catalog.index("test_supply").unwrap(),
        entry: 0,
        destination: entry.center,
        confirmed_tick: 0,
        blocked: false,
        arrival_tick: None,
    });
    let ordinary = BalancedDoctrine::default().decide(
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
    frame
        .identified
        .push(contract::observation::IdentifiedUnit {
            id: contract::observation::ObservedTargetId(7),
            kind: rules.catalog.index("test_jet").unwrap(),
            cost: 300,
            position: [500.0, 500.0, 250.0],
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
        });
    let threatened = BalancedDoctrine::default().decide(
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
    let purchased = |orders: &[Order]| {
        orders.iter().find_map(|order| match order {
            Order::ConfirmPurchase { variant, .. } => Some(rules.catalog.index(variant).unwrap()),
            _ => None,
        })
    };
    let ordinary_kind = purchased(&ordinary).expect("ordinary reinforcement");
    assert!(!unit_engages_aircraft_in(
        &rules,
        ordinary_kind,
        AltitudeLayer::HighAir
    ));
    let defended = purchased(&threatened).expect("jet sighting buys a defender");
    assert!(
        unit_engages_aircraft_in(&rules, defended, AltitudeLayer::HighAir),
        "threat response bought {}",
        rules.catalog.id(defended)
    );
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
    me.pending.push(contract::skirmish::PendingPurchase {
        id: contract::skirmish::PurchaseId(1),
        kind: rules.catalog.index("test_supply").unwrap(),
        entry: 0,
        destination: entry.center,
        confirmed_tick: 0,
        blocked: false,
        arrival_tick: None,
    });
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

/// Credit a wallet's purchases to the frame the way the match does: price
/// out, slot taken, card pending. Returns the kinds bought.
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
            destination,
        } = order
        else {
            continue;
        };
        let kind = rules.catalog.index(variant).unwrap();
        let price = f64::from(rules.catalog.get(kind).cost);
        let me = &mut view.players[0];
        assert!(price <= me.credits && me.unit_count < me.cap);
        me.credits -= price;
        me.unit_count += 1;
        me.pending.push(contract::skirmish::PendingPurchase {
            id: contract::skirmish::PurchaseId(me.pending.len() as u32),
            kind,
            entry: 0,
            destination: *destination,
            confirmed_tick: tick,
            blocked: false,
            arrival_tick: None,
        });
        bought.push(kind);
    }
    bought
}

#[test]
fn with_steady_income_the_ai_saves_for_an_expensive_jet_and_keeps_its_aa() {
    // Prices are the test's own: a jet costs as much as a dozen cheap cards,
    // far more than one decision's income, and the AA card fits the opening.
    let mut authored = deck();
    for (id, cost) in [
        ("test_rifle", 50),
        ("test_at", 50),
        ("test_recon", 50),
        ("test_tank", 50),
        ("test_supply", 50),
        ("test_jeep", 150),
    ] {
        fixtures::patch_catalog(&mut authored, "units", id, json!({ "cost": cost }));
    }
    fixtures::patch_catalog(
        &mut authored,
        "units",
        "test_jeep",
        json!({ "mounts": [{ "id": "HMG", "weapons": ["aa_long"] }] }),
    );
    fixtures::patch_catalog(
        &mut authored,
        "units",
        "test_jet",
        json!({
            "cost": 600,
            "roster": { "factions": ["us"], "category": "air", "family_name": "Test jet", "variant": "Test" }
        }),
    );
    let rules: Rules = serde_json::from_value(authored).unwrap();
    let jet = rules.catalog.index("test_jet").unwrap();
    let aa = rules.catalog.index("test_jeep").unwrap();
    let (_, mut frame, entry) = empty_match();
    me(&mut frame).credits = 200.0;
    me(&mut frame).cap = 1_000;
    let mut ai = BalancedDoctrine::new(3);
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
    let orders = decide(&mut ai, &frame);
    let opening = confirm_purchases(&rules, &mut frame, &orders);
    assert!(opening.contains(&aa), "the opening carries its AA card");

    // Twenty credits a decision: a jet takes thirty decisions of saving.
    frame.skirmish.as_mut().unwrap().phase = Phase::Active;
    let mut reinforcements = Vec::new();
    for decision in 1..=150 {
        frame.tick = decision * 5 * u64::from(rules.tick_hz);
        me(&mut frame).credits += 20.0;
        let orders = decide(&mut ai, &frame);
        reinforcements.extend(confirm_purchases(&rules, &mut frame, &orders));
    }
    let names: Vec<_> = reinforcements
        .iter()
        .map(|k| rules.catalog.id(*k))
        .collect();
    let first_jet = reinforcements
        .iter()
        .position(|&k| k == jet)
        .unwrap_or_else(|| panic!("steady income never bought the jet: {names:?}"));
    assert!(
        reinforcements[first_jet + 1..]
            .iter()
            .any(|&k| k != jet && !rules.catalog.get(k).has_role("logistics")),
        "after the jet the AI buys ground units again: {names:?}"
    );
}

#[test]
fn active_policy_uses_an_ordinary_attack_for_a_known_target() {
    let (rules, mut frame, entry) = empty_match();
    let setup: contract::scenario::ScenarioDefinition = serde_json::from_value(json!({
        "map":{"size":[1000,1000],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35},
        "rules":deck(),"units":[{"side":"red","kind":"test_jeep","position":[500,100]}],
        "events":[],"scripts":[]
    }))
    .unwrap();
    let battle = crate::battle::Battle::new(&setup, 1);
    frame.own = battle.observe(Side::Red).own.clone();
    let view = frame.skirmish.as_mut().unwrap();
    view.phase = Phase::Active;
    view.players[0].credits = 0.0;
    frame
        .identified
        .push(contract::observation::IdentifiedUnit {
            id: contract::observation::ObservedTargetId(9),
            kind: rules.catalog.index("test_tank").unwrap(),
            cost: 100,
            position: [500.0, 500.0, 0.0],
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
        });
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
        Order::Attack { units, target: contract::command::TargetRef::Identified { id } }
            if units == &vec![UnitId(0)] && *id == contract::observation::ObservedTargetId(9)
    )));
}

#[test]
fn artillery_fires_a_bounded_ground_guess_from_a_contact() {
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
    authored["weapons"]["hmg"]["trajectory"] = json!("indirect");
    let rules: Rules = serde_json::from_value(authored).unwrap();
    let setup: contract::scenario::ScenarioDefinition = serde_json::from_value(json!({
        "map":{"size":[1000,1000],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35},
        "rules":deck(),"units":[{"side":"red","kind":"test_jeep","position":[100,100]}],
        "events":[],"scripts":[]
    }))
    .unwrap();
    let battle = crate::battle::Battle::new(&setup, 1);
    let (_, mut frame, _) = empty_match();
    frame.own = battle.observe(Side::Red).own.clone();
    frame.own[0].kind = rules.catalog.index("test_jeep").unwrap();
    let view = frame.skirmish.as_mut().unwrap();
    view.phase = Phase::Active;
    view.players[0].credits = 0.0;
    frame
        .contacts
        .push(contract::observation::ApproximateContact {
            id: contract::observation::ContactId(4),
            source: contract::observation::ContactSource::Firing,
            center: [220.0, 220.0, 0.0],
            layer: contract::catalog::AltitudeLayer::Ground,
            aloft: false,
            radius: 30.0,
            evidence_tick: 0,
            expires_tick: 120,
            kind: None,
            heard: 0,
            bearing_deg: None,
            range_m: None,
        });
    let orders = BalancedDoctrine::default().decide(
        DecisionInput {
            side: Side::Red,
            faction: Faction::Us,
            entry: &entry_from([100.0, 100.0]),
            assignment: None,
        },
        &frame,
        &rules,
        |_, _| Some(1.0),
    );
    assert!(orders.iter().any(|order| matches!(
        order,
        Order::Attack { units, target: contract::command::TargetRef::Ground { point } }
            if units == &vec![UnitId(0)] && point[0] != 220.0 && point[1] != 220.0
    )));
}

fn entry_from(center: [f64; 2]) -> EntrySite {
    EntrySite {
        side: Side::Red,
        center,
        yaw: 0.0,
    }
}

fn artillery_frame() -> (Rules, ObservationFrame, EntrySite) {
    let mut authored = deck();
    fixtures::patch_catalog(
        &mut authored,
        "units",
        "test_jeep",
        json!({"roles":["artillery"]}),
    );
    authored["weapons"]["hmg"]["trajectory"] = json!("indirect");
    authored["weapons"]["hmg"]["min_range_m"] = json!(50);
    authored["weapons"]["hmg"]["targets"]["ground"]["range_m"] = json!(500);
    let setup: contract::scenario::ScenarioDefinition = serde_json::from_value(json!({
        "map":{"size":[1000,1000],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35},
        "rules":authored,"units":[{"side":"red","kind":"test_jeep","position":[100,100]}],
        "events":[],"scripts":[]
    }))
    .unwrap();
    let battle = crate::battle::Battle::new(&setup, 1);
    let (_, mut frame, entry) = empty_match();
    frame.own = battle.observe(Side::Red).own.clone();
    let view = frame.skirmish.as_mut().unwrap();
    view.phase = Phase::Active;
    view.players[0].credits = 0.0;
    (setup.rules, frame, entry)
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

fn ground_orders(orders: &[Order]) -> Vec<[f64; 3]> {
    orders
        .iter()
        .filter_map(|order| match order {
            Order::Attack {
                target: TargetRef::Ground { point },
                ..
            } => Some(*point),
            _ => None,
        })
        .collect()
}

#[test]
fn artillery_refuses_stale_expired_and_unreachable_reports() {
    let (rules, mut frame, entry) = artillery_frame();
    frame.tick = 20 * u64::from(rules.tick_hz);
    for contact in [
        firing_contact([220.0, 100.0, 0.0], 0, frame.tick + 1000),
        firing_contact([220.0, 100.0, 0.0], frame.tick, frame.tick - 1),
        firing_contact([900.0, 100.0, 0.0], frame.tick, frame.tick + 1000),
    ] {
        frame.contacts = vec![contact];
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
        assert!(
            ground_orders(&orders).is_empty(),
            "stale or unreachable evidence must not fire: {orders:?}"
        );
    }
}

#[test]
fn artillery_prediction_uses_only_a_matching_sighting_and_stays_bounded() {
    let (rules, mut frame, entry) = artillery_frame();
    let target_kind = rules.catalog.index("test_tank").unwrap();
    let target = contract::observation::IdentifiedUnit {
        id: contract::observation::ObservedTargetId(4),
        kind: target_kind,
        cost: 100,
        position: [220.0, 220.0, 0.0],
        yaw: 0.0,
        velocity: [10.0, 0.0],
        members: vec![],
        member_ids: vec![],
        member_slots: vec![],
        member_active_mounts: vec![],
        weapon_poses: vec![],
        reversing: false,
        smoking: false,
        member_leans: vec![],
    };
    frame.identified.push(target);
    // A second same-kind sighting is far from the firing report. The policy
    // must retain each observation by ID rather than borrowing its velocity.
    frame
        .identified
        .push(contract::observation::IdentifiedUnit {
            id: contract::observation::ObservedTargetId(5),
            kind: target_kind,
            cost: 100,
            position: [700.0, 700.0, 0.0],
            yaw: 0.0,
            velocity: [1000.0, 0.0],
            members: vec![],
            member_ids: vec![],
            member_slots: vec![],
            member_active_mounts: vec![],
            weapon_poses: vec![],
            reversing: false,
            smoking: false,
            member_leans: vec![],
        });
    // Hold fire while recording the first sighting, then release it for the
    // next decision so the prediction is the only possible salvo.
    frame.own[0].engagement = contract::command::Engagement::ReturnFireOnly;
    frame.skirmish.as_mut().unwrap().phase = Phase::Finished;
    let mut ai = BalancedDoctrine::default();
    assert!(ground_orders(&ai.decide(
        DecisionInput {
            side: Side::Red,
            faction: Faction::Us,
            entry: &entry,
            assignment: None
        },
        &frame,
        &rules,
        |_, _| Some(1.0)
    ))
    .is_empty());
    frame.tick = 2 * u64::from(rules.tick_hz);
    frame.skirmish.as_mut().unwrap().phase = Phase::Active;
    frame.own[0].engagement = contract::command::Engagement::FireAtWill;
    frame.identified.clear();
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
    let points = ground_orders(&orders);
    assert_eq!(points, vec![[240.0, 220.0, 0.0]], "orders: {orders:?}");

    frame.tick = 7 * u64::from(rules.tick_hz);
    assert!(ground_orders(&ai.decide(
        DecisionInput {
            side: Side::Red,
            faction: Faction::Us,
            entry: &entry,
            assignment: None
        },
        &frame,
        &rules,
        |_, _| Some(1.0)
    ))
    .is_empty());
}

#[test]
fn artillery_uses_a_range_checked_public_objective_when_contacts_are_stale() {
    let (rules, mut frame, entry) = artillery_frame();
    frame.tick = 0;
    frame.contacts = vec![firing_contact([900.0, 900.0, 0.0], 0, 0)];
    frame.skirmish.as_mut().unwrap().objectives[0].owner = Some(Side::Blue);
    frame.skirmish.as_mut().unwrap().objectives[0].center = [900.0, 900.0];
    assert!(ground_orders(&BalancedDoctrine::default().decide(
        DecisionInput {
            side: Side::Red,
            faction: Faction::Us,
            entry: &entry,
            assignment: None
        },
        &frame,
        &rules,
        |_, _| Some(1.0)
    ))
    .is_empty());
    frame.skirmish.as_mut().unwrap().objectives[0].center = [300.0, 300.0];
    let points = ground_orders(&BalancedDoctrine::default().decide(
        DecisionInput {
            side: Side::Red,
            faction: Faction::Us,
            entry: &entry,
            assignment: None,
        },
        &frame,
        &rules,
        |_, _| Some(1.0),
    ));
    assert_eq!(points.len(), 1);
    let point = points[0];
    assert_ne!(
        point,
        [300.0, 300.0, 0.0],
        "objective aim keeps a fallible offset"
    );
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
    assert_eq!(ground_orders(&orders), vec![point]);
}

#[test]
fn artillery_final_aim_is_in_one_weapon_envelope_and_return_fire_holds() {
    let (rules, mut frame, entry) = artillery_frame();
    frame.contacts = vec![firing_contact([599.0, 100.0, 0.0], 0, 100)];
    let points = ground_orders(&BalancedDoctrine::default().decide(
        DecisionInput {
            side: Side::Red,
            faction: Faction::Us,
            entry: &entry,
            assignment: None,
        },
        &frame,
        &rules,
        |_, _| Some(1.0),
    ));
    assert_eq!(points.len(), 1);
    assert!(
        distance([100.0, 100.0], [points[0][0], points[0][1]]) <= 500.0,
        "final offset left weapon range: {points:?}"
    );
    frame.own[0].engagement = contract::command::Engagement::ReturnFireOnly;
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
    assert!(
        ground_orders(&orders).is_empty(),
        "policy must not override hold-fire: {orders:?}"
    );
}

fn identified(
    id: u32,
    kind: TypeIndex,
    position: [f64; 3],
) -> contract::observation::IdentifiedUnit {
    contract::observation::IdentifiedUnit {
        id: ObservedTargetId(id),
        kind,
        cost: 100,
        position,
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
/// `relocate_m` off, clear of the wall, and from there it fires. The
#[test]
fn the_ai_drives_a_launcher_its_cover_holds_to_open_ground_and_fires() {
    let mut rules = deck();
    rules["weapons"]["test_launcher"] = json!({
        "extends": "artillery_mortar", "speed_mps": 80, "range_m": 650,
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

#[test]
fn a_row_that_shoots_down_missiles_alone_is_no_defence_against_aircraft() {
    let defends = |classes: Value| {
        let mut fixture = test_rules();
        fixture["weapons"]["hmg"]["targets"]["low_air"]["classes"] = classes;
        fixture["weapons"]["hmg"]["proximity_fuse_m"] = serde_json::json!(2);
        let rules: Rules = serde_json::from_value(fixture).unwrap();
        let jeep = rules.catalog.index("test_jeep").unwrap();
        let heli = rules.catalog.index("test_heli").unwrap();
        (
            unit_engages_aircraft_in(&rules, jeep, AltitudeLayer::LowAir),
            unit_can_engage_target(&rules, jeep, heli),
        )
    };
    assert_eq!(defends(json!(["air_rotor"])), (true, true));
    assert_eq!(defends(json!(["missile"])), (false, false));
}

mod backline;
mod groups;
mod parties;
mod purchases;
mod transport_tests;
