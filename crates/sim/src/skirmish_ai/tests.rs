use super::*;
use serde_json::json;

/// The test units as a U.S. deck: a card for each, in its category.
fn deck() -> serde_json::Value {
    let mut rules = crate::fixtures::test_game();
    for (id, category) in [
        ("test_rifle", "inf"),
        ("test_at", "inf"),
        ("test_recon", "rec"),
        ("test_jeep", "rec"),
        ("test_tank", "veh"),
        ("test_supply", "sup"),
    ] {
        crate::fixtures::patch_catalog(
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
        ready: [false; 2],
        preparation_remaining_s: 45.0,
        credits: 1000.0,
        occupied_slots: 0,
        max_units: 30,
        pending: vec![],
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
    crate::fixtures::patch_catalog(
        &mut authored,
        "units",
        "test_rifle",
        json!({"roles":["at"]}),
    );
    let rules: Rules = serde_json::from_value(authored).unwrap();
    let orders = BalancedDoctrine::default().decide(
        Side::Red,
        Faction::Us,
        &entry,
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
    view.credits = 0.0;
    let mut ai = BalancedDoctrine::default();
    let orders = ai.decide(Side::Red, Faction::Us, &entry, &frame, &rules, |_, _| {
        Some(1.0)
    });
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
    let orders = ai.decide(Side::Red, Faction::Us, &entry, &frame, &rules, |_, _| {
        panic!("standing deployment needs no new route")
    });
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
    let orders = ai.decide(Side::Red, Faction::Us, &entry, &frame, &rules, |_, _| {
        Some(1.0)
    });
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
    frame
        .skirmish
        .as_mut()
        .unwrap()
        .pending
        .push(contract::skirmish::PendingPurchase {
            id: contract::skirmish::PurchaseId(1),
            kind,
            destination: *destination,
            confirmed_tick: 0,
            blocked: false,
        });
    assert!(ai
        .decide(Side::Red, Faction::Us, &entry, &frame, &rules, |_, _| Some(
            1.0
        ))
        .is_empty());
    frame.tick = 5 * u64::from(rules.tick_hz);
    let orders = ai.decide(Side::Red, Faction::Us, &entry, &frame, &rules, |_, _| {
        Some(1.0)
    });
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
        for order in ai.decide(Side::Red, Faction::Us, &entry, &frame, &rules, |_, _| {
            Some(1.0)
        }) {
            let view = frame.skirmish.as_mut().unwrap();
            match order {
                Order::ConfirmPurchase {
                    variant,
                    destination,
                } => {
                    let kind = rules.catalog.index(&variant).unwrap();
                    let price = f64::from(rules.catalog.get(kind).cost);
                    assert!(price <= view.credits && view.occupied_slots < view.max_units);
                    view.credits -= price;
                    view.occupied_slots += 1;
                    view.pending.push(contract::skirmish::PendingPurchase {
                        id: contract::skirmish::PurchaseId(view.pending.len() as u32),
                        kind,
                        destination,
                        confirmed_tick: frame.tick,
                        blocked: false,
                    });
                }
                Order::Ready => {
                    ready = true;
                    view.ready[1] = true;
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
    assert!(view
        .pending
        .iter()
        .any(|p| rules.catalog.get(p.kind).has_role("logistics")));
    let mut capped = frame.clone();
    let view = capped.skirmish.as_mut().unwrap();
    view.phase = Phase::Active;
    view.occupied_slots = view.max_units;
    capped.tick += 5 * u64::from(rules.tick_hz);
    assert!(!ai
        .decide(
            Side::Red,
            Faction::Us,
            &entry,
            &capped,
            &rules,
            |_, _| Some(1.0)
        )
        .iter()
        .any(|o| matches!(o, Order::ConfirmPurchase { .. })));
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
    view.credits = 0.0;
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
            Side::Red,
            Faction::Us,
            &entry,
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
        .decide(Side::Red, Faction::Us, &entry, &frame, &rules, |_, _| None)
        .is_empty());
}

#[test]
fn zero_unit_opening_uses_an_affordable_admitted_faction_purchase() {
    let (rules, frame, entry) = empty_match();
    let orders = BalancedDoctrine::default().decide(
        Side::Red,
        Faction::Us,
        &entry,
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
            && card.cost as f64 <= frame.skirmish.as_ref().unwrap().credits
    );
    assert!(card.roster.factions.contains(&Faction::Us));
    assert_eq!(*destination, [500.0, 500.0]);
}

#[test]
fn seeded_policy_instances_repeat_the_same_opening_commands() {
    let (rules, frame, entry) = empty_match();
    let mut left = BalancedDoctrine::new(17);
    let mut right = BalancedDoctrine::new(17);
    let left_orders = left.decide(Side::Red, Faction::Us, &entry, &frame, &rules, |_, _| {
        Some(1.0)
    });
    let right_orders = right.decide(Side::Red, Faction::Us, &entry, &frame, &rules, |_, _| {
        Some(1.0)
    });
    assert_eq!(left_orders, right_orders);
}

#[test]
fn different_seeds_can_change_a_reinforcement_choice_without_changing_legality() {
    let (rules, mut frame, entry) = empty_match();
    frame.skirmish.as_mut().unwrap().phase = Phase::Active;
    frame.skirmish.as_mut().unwrap().occupied_slots = 1;
    frame.skirmish.as_mut().unwrap().credits = 500.0;
    frame
        .skirmish
        .as_mut()
        .unwrap()
        .pending
        .push(contract::skirmish::PendingPurchase {
            id: contract::skirmish::PurchaseId(1),
            kind: rules.catalog.index("test_supply").unwrap(),
            destination: entry.center,
            confirmed_tick: 0,
            blocked: false,
        });
    let mut variants = std::collections::BTreeSet::new();
    for seed in 0..32 {
        let orders = BalancedDoctrine::new(seed).decide(
            Side::Red,
            Faction::Us,
            &entry,
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
    crate::fixtures::patch_catalog(
        &mut authored,
        "units",
        "test_rifle",
        json!({
            "body": { "squad": { "slots": ["test_atgm_gunner"] } }
        }),
    );
    crate::fixtures::patch_catalog(
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
    frame.skirmish.as_mut().unwrap().credits = 1_000.0;
    frame.skirmish.as_mut().unwrap().occupied_slots = 1;
    let supply = rules.catalog.index("test_supply").unwrap();
    frame
        .skirmish
        .as_mut()
        .unwrap()
        .pending
        .push(contract::skirmish::PendingPurchase {
            id: contract::skirmish::PurchaseId(1),
            kind: supply,
            destination: entry.center,
            confirmed_tick: 0,
            blocked: false,
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
    let orders = ai.decide(Side::Red, Faction::Us, &entry, &frame, &rules, |_, _| {
        Some(1.0)
    });
    let Order::ConfirmPurchase { variant, .. } = orders
        .iter()
        .find(|order| matches!(order, Order::ConfirmPurchase { .. }))
        .expect("an active reinforcement purchase")
    else {
        unreachable!()
    };
    let kind = rules.catalog.index(variant).unwrap();
    assert!(unit_can_target_layer(
        &rules,
        kind,
        contract::catalog::AltitudeLayer::LowAir
    ));
}

#[test]
fn doctrine_pivots_after_maximum_dwell_without_visible_progress() {
    let (rules, mut frame, entry) = empty_match();
    frame.skirmish.as_mut().unwrap().phase = Phase::Active;
    frame.skirmish.as_mut().unwrap().credits = 0.0;
    let mut ai = BalancedDoctrine::new(7);
    ai.decide(Side::Red, Faction::Us, &entry, &frame, &rules, |_, _| {
        Some(1.0)
    });
    assert_eq!(ai.doctrine(), Doctrine::BalancedOpening);
    frame.tick = 181 * u64::from(rules.tick_hz);
    ai.decide(Side::Red, Faction::Us, &entry, &frame, &rules, |_, _| {
        Some(1.0)
    });
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
    frame.skirmish.as_mut().unwrap().credits = 0.0;
    let mut ai = BalancedDoctrine::default();
    let orders = ai.decide(Side::Red, Faction::Us, &entry, &frame, &rules, |_, _| {
        Some(1.0)
    });
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
fn opening_procures_an_authored_artillery_role() {
    let mut authored = deck();
    crate::fixtures::patch_catalog(
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
        Side::Red,
        Faction::Us,
        &entry,
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
    view.credits = 0.0;
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
        Side::Red,
        Faction::Us,
        &entry,
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
    crate::fixtures::patch_catalog(
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
    view.credits = 0.0;
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
        });
    let orders = BalancedDoctrine::default().decide(
        Side::Red,
        Faction::Us,
        &entry_from([100.0, 100.0]),
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
    crate::fixtures::patch_catalog(
        &mut authored,
        "units",
        "test_jeep",
        json!({"roles":["artillery"]}),
    );
    authored["weapons"]["hmg"]["trajectory"] = json!("indirect");
    authored["weapons"]["hmg"]["min_range_m"] = json!(50);
    authored["weapons"]["hmg"]["range_m"] = json!(500);
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
    view.credits = 0.0;
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
            Side::Red,
            Faction::Us,
            &entry,
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
        Side::Red,
        Faction::Us,
        &entry,
        &frame,
        &rules,
        |_, _| Some(1.0),
    ))
    .is_empty());
    frame.tick = 2 * u64::from(rules.tick_hz);
    frame.skirmish.as_mut().unwrap().phase = Phase::Active;
    frame.own[0].engagement = contract::command::Engagement::FireAtWill;
    frame.identified.clear();
    let orders = ai.decide(Side::Red, Faction::Us, &entry, &frame, &rules, |_, _| {
        Some(1.0)
    });
    let points = ground_orders(&orders);
    assert_eq!(points, vec![[240.0, 220.0, 0.0]], "orders: {orders:?}");

    frame.tick = 7 * u64::from(rules.tick_hz);
    assert!(ground_orders(&ai.decide(
        Side::Red,
        Faction::Us,
        &entry,
        &frame,
        &rules,
        |_, _| Some(1.0),
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
        Side::Red,
        Faction::Us,
        &entry,
        &frame,
        &rules,
        |_, _| Some(1.0),
    ))
    .is_empty());
    frame.skirmish.as_mut().unwrap().objectives[0].center = [300.0, 300.0];
    let points = ground_orders(&mut BalancedDoctrine::default().decide(
        Side::Red,
        Faction::Us,
        &entry,
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
        Side::Red,
        Faction::Us,
        &entry,
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
        Side::Red,
        Faction::Us,
        &entry,
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
        Side::Red,
        Faction::Us,
        &entry,
        &frame,
        &rules,
        |_, _| Some(1.0),
    );
    assert!(
        ground_orders(&orders).is_empty(),
        "policy must not override hold-fire: {orders:?}"
    );
}
