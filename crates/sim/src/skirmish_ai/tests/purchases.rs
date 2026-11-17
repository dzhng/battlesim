//! Which card the doctrine buys (D56, D2): value per credit within a role,
//! weapon-made answers to aircraft, and armour in the recipe.

use super::*;
use crate::skirmish_ai::purchase::pick;

fn seen(rules: &Rules, kind: &str) -> contract::observation::IdentifiedUnit {
    let kind = rules.catalog.index(kind).unwrap();
    contract::observation::IdentifiedUnit {
        id: ObservedTargetId(1),
        kind,
        cost: rules.catalog.get(kind).cost,
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
        building: None,
    }
}

pub(super) fn us_cards(rules: &Rules) -> Vec<TypeIndex> {
    rules
        .catalog
        .indices()
        .filter(|&k| {
            rules
                .catalog
                .get(k)
                .roster
                .as_ref()
                .is_some_and(|r| r.factions.contains(&Faction::Us))
        })
        .collect()
}

/// The AT team as a second infantry card.
fn two_infantry_cards(authored: &mut Value) {
    crate::fixtures::patch_catalog(
        authored,
        "units",
        "test_at",
        json!({ "roles": ["infantry"] }),
    );
}

/// At equal cost the armed flyer wins on value, or on its earlier id when
/// there is no known threat. Only its mobility should rule it out.
fn supply_flyer_deck() -> Value {
    let mut authored = deck();
    let truck = authored["catalog"]
        .as_array()
        .unwrap()
        .iter()
        .find_map(|doc| doc["units"].get("test_supply"))
        .unwrap()
        .clone();
    let mut capabilities = truck["capabilities"].clone();
    capabilities["deploy"]["drop_seconds"] = json!(0.1);
    fixtures::patch_catalog(
        &mut authored,
        "units",
        "test_heli",
        json!({
            "roles": ["logistics"],
            "cost": truck["cost"],
            "mobility": { "air": { "rotor_radius_m": 6.0 } },
            "capabilities": capabilities,
            "roster": { "factions": ["us"], "category": "sup",
                "family_name": "Test", "variant": "Test" }
        }),
    );
    authored
}

#[test]
fn the_opening_buys_the_supplier_that_drives_even_when_a_flyer_ties() {
    let rules: Rules = serde_json::from_value(supply_flyer_deck()).unwrap();
    for doctrine in [
        Doctrine::BalancedOpening,
        Doctrine::TankSwarm,
        Doctrine::AirDenial,
    ] {
        for enemy in [None, Some("test_rifle")] {
            let (_, mut frame, entry) = empty_match();
            me(&mut frame).credits = 100_000.0;
            me(&mut frame).cap = 1_000;
            frame.identified = enemy.map(|id| seen(&rules, id)).into_iter().collect();
            let mut ai = BalancedDoctrine {
                doctrine,
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
            let suppliers: Vec<_> = confirm_purchases(&rules, &mut frame, &orders)
                .into_iter()
                .filter(|&kind| rules.catalog.get(kind).has_role("logistics"))
                .collect();
            assert!(
                !suppliers.is_empty(),
                "{doctrine:?}, {enemy:?}: buys a supplier"
            );
            for kind in suppliers {
                assert!(
                    rules.catalog.get(kind).mobility.locomotion().is_some(),
                    "{doctrine:?}, {enemy:?}: bought a flying standing supplier: {}",
                    rules.catalog.id(kind)
                );
            }
        }
    }
}

#[test]
fn a_lost_truck_is_replaced_by_one_that_drives() {
    for flyer in ["absent", "owned", "pending"] {
        let (rules, mut frame, entry) = transport_tests::transport_match_on(
            supply_flyer_deck(),
            json!([
                {"side":"red","kind":"test_supply","position":[500,10]},
                {"side":"red","kind":"test_heli","position":[540,30]}
            ]),
        );
        // Lose the truck, with an emergency flyer absent, on the field, or
        // still on its way. None can stand in for the lost driver.
        frame
            .own
            .retain(|u| flyer == "owned" && rules.catalog.get(u.kind).mobility.flight().is_some());
        me(&mut frame).unit_count = frame.own.len() as u32;
        me(&mut frame).credits = 100_000.0;
        if flyer == "pending" {
            me(&mut frame).pending.push(pending(
                1,
                rules.catalog.index("test_heli").unwrap(),
                entry.center,
                0,
            ));
            me(&mut frame).unit_count += 1;
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
        let bought = confirm_purchases(&rules, &mut frame, &orders);
        let kind = *bought.first().expect("replace the lost supplier");
        let supplier = rules.catalog.get(kind);
        assert!(
            supplier.has_role("logistics") && supplier.mobility.locomotion().is_some(),
            "flyer {flyer}: replacement must drive, bought {}",
            rules.catalog.id(kind)
        );
    }
}

#[test]
fn the_support_loop_never_moves_a_flyer() {
    let (rules, mut frame, entry) = transport_tests::transport_match_on(
        supply_flyer_deck(),
        json!([
            {"side":"red","kind":"test_rifle","position":[500,600]},
            {"side":"red","kind":"test_supply","position":[500,10]},
            {"side":"red","kind":"test_heli","position":[540,30]}
        ]),
    );
    let (truck, flyer) = (frame.own[1].id, frame.own[2].id);
    let mut ai = BalancedDoctrine::default();
    let mut support = None;
    for parked in [false, true] {
        if parked {
            let [x, y] = support.expect("the truck was sent to its support spot");
            for unit in &mut frame.own[1..] {
                unit.position[0] = x;
                unit.position[1] = y;
                unit.goal = None;
                unit.deployment.as_mut().unwrap().target = contract::observation::Posture::Packed;
            }
            frame.tick += 5 * u64::from(rules.tick_hz);
        }
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
        if parked {
            assert!(
                orders.iter().any(|order| matches!(order,
                Order::SetDeployment { units, deployed: true } if units.contains(&truck))),
                "the standing truck deploys: {orders:?}"
            );
        } else {
            support = orders.iter().find_map(|order| match order {
                Order::Move { units, goal, .. } if units.contains(&truck) => Some(*goal),
                _ => None,
            });
            assert!(support.is_some(), "the standing truck moves: {orders:?}");
        }
        for order in &orders {
            if let Order::Move { units, .. }
            | Order::AttackMove { units, .. }
            | Order::SetDeployment { units, .. } = order
            {
                assert!(
                    !units.contains(&flyer),
                    "parked={parked}: the support loop ordered the flyer: {order:?}"
                );
                for id in units {
                    let kind = frame.own.iter().find(|u| u.id == *id).unwrap().kind;
                    assert!(
                        rules.catalog.get(kind).mobility.locomotion().is_some(),
                        "support orders go only to ground movers: {order:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn within_a_role_a_card_that_can_engage_the_enemy_seen_beats_a_cheaper_one_that_cannot() {
    let mut authored = deck();
    two_infantry_cards(&mut authored);
    for weapon in ["rifle", "grenade"] {
        authored["weapons"][weapon]["targets"]["ground"]["classes"] = json!(["infantry"]);
    }
    let rules: Rules = serde_json::from_value(authored).unwrap();
    let infantry = (Some("infantry"), Category::Inf, 0);
    let choose = |enemy: &[&str]| {
        let kinds = enemy.iter().map(|id| rules.catalog.index(id).unwrap());
        let threat = (!enemy.is_empty()).then(|| team::Threat::of(&rules, kinds));
        let card = pick(&rules, &us_cards(&rules), threat.as_ref(), infantry).unwrap();
        rules.catalog.id(card).to_string()
    };
    assert_eq!(choose(&[]), "test_rifle", "no matchup known: the cheaper");
    assert_eq!(choose(&["test_tank"]), "test_at");
    assert_eq!(choose(&["test_rifle"]), "test_rifle");
}

#[test]
fn raising_a_cards_cost_hands_its_role_to_an_equally_capable_card() {
    let mut authored = deck();
    two_infantry_cards(&mut authored);
    let rifles = authored["catalog"]
        .as_array()
        .unwrap()
        .iter()
        .find_map(|doc| doc["units"].get("test_rifle"))
        .unwrap()["body"]
        .clone();
    crate::fixtures::patch_catalog(&mut authored, "units", "test_at", json!({ "body": rifles }));
    let choose = |rifle_cost: u32| {
        let mut authored = authored.clone();
        crate::fixtures::patch_catalog(
            &mut authored,
            "units",
            "test_rifle",
            json!({ "cost": rifle_cost }),
        );
        let rules: Rules = serde_json::from_value(authored).unwrap();
        let threat = team::Threat::of(&rules, [rules.catalog.index("test_rifle").unwrap()]);
        let card = pick(
            &rules,
            &us_cards(&rules),
            Some(&threat),
            (Some("infantry"), Category::Inf, 0),
        );
        rules.catalog.id(card.unwrap()).to_string()
    };
    let at_cost = deck_cost("test_at");
    assert_eq!(choose(at_cost - 10), "test_rifle");
    assert_eq!(choose(at_cost + 10), "test_at");
}

fn deck_cost(id: &str) -> u32 {
    let rules: Rules = serde_json::from_value(deck()).unwrap();
    rules.catalog.get(rules.catalog.index(id).unwrap()).cost
}

/// The doctrine over `decisions` active decisions with steady income,
/// after its opening; returns the opening and the reinforcements bought.
fn purchases(
    rules: &Rules,
    frame: &mut ObservationFrame,
    entry: &EntrySite,
    decisions: u64,
) -> (Vec<TypeIndex>, Vec<TypeIndex>) {
    let mut ai = BalancedDoctrine::new(1);
    let mut decide = |frame: &ObservationFrame| {
        ai.decide(
            DecisionInput {
                side: Side::Red,
                faction: Faction::Us,
                entry,
                assignment: None,
            },
            frame,
            rules,
            |_, _| Some(1.0),
        )
    };
    me(frame).cap = 1_000;
    let orders = decide(frame);
    let opening = confirm_purchases(rules, frame, &orders);
    frame.skirmish.as_mut().unwrap().phase = Phase::Active;
    let mut bought = Vec::new();
    for decision in 1..=decisions {
        frame.tick = decision * 5 * u64::from(rules.tick_hz);
        me(frame).credits += 100.0;
        let orders = decide(frame);
        bought.extend(confirm_purchases(rules, frame, &orders));
    }
    (opening, bought)
}

#[test]
fn an_enemy_aircraft_raises_air_defence_purchases() {
    let mut authored = deck();
    crate::fixtures::patch_catalog(
        &mut authored,
        "units",
        "test_aa",
        json!({ "roster": { "factions": ["us"], "category": "inf",
            "family_name": "Test", "variant": "Test" } }),
    );
    let rules: Rules = serde_json::from_value(authored).unwrap();
    let aa = rules.catalog.index("test_aa").unwrap();
    let count = |enemy: Option<&str>| {
        let (_, mut frame, entry) = empty_match();
        frame.identified = enemy.map(|id| seen(&rules, id)).into_iter().collect();
        let (opening, bought) = purchases(&rules, &mut frame, &entry, 20);
        opening.iter().chain(&bought).filter(|&&k| k == aa).count()
    };
    let calm = count(None);
    let threatened = count(Some("test_heli"));
    assert!(
        threatened > calm,
        "a helicopter seen buys more AA: {threatened} vs {calm}"
    );
}

#[test]
fn affordable_armour_is_bought_in_the_opening_and_the_reinforcements() {
    let (_, mut frame, entry) = empty_match();
    // The tank costs a tenth of the opening's credits: armour the opening
    // can afford beside the rest of its recipe.
    let tank_cost = (me(&mut frame).credits / 10.0) as u32;
    let mut authored = deck();
    crate::fixtures::patch_catalog(
        &mut authored,
        "units",
        "test_tank",
        json!({ "cost": tank_cost }),
    );
    let rules: Rules = serde_json::from_value(authored).unwrap();
    let (opening, bought) = purchases(&rules, &mut frame, &entry, 40);
    let armour = |kinds: &[TypeIndex]| kinds.iter().any(|&k| rules.catalog.get(k).has_role("mbt"));
    assert!(armour(&opening), "the opening fields armour");
    assert!(armour(&bought), "reinforcements field armour");
}

#[test]
fn seeded_reinforcements_keep_the_wasm_choice_on_native() {
    let (rules, mut frame, entry) = empty_match();
    frame.skirmish.as_mut().unwrap().phase = Phase::Active;
    me(&mut frame).unit_count = 1;
    me(&mut frame).credits = 10_000.0;
    // Existing logistics avoids the urgent supply purchase; no squad or
    // carrier demand hides the seeded reinforcement choice.
    me(&mut frame)
        .pending
        .push(contract::skirmish::PendingPurchase {
            id: contract::skirmish::PurchaseId(1),
            kind: rules.catalog.index("test_supply").unwrap(),
            entry: 0,
            carrier: None,
            destination: entry.center,
            confirmed_tick: 0,
            blocked: false,
            arrival_tick: None,
        });
    // These policy streams start with armour on wasm32. Casting the full
    // hash to a platform-sized index changes them to infantry or AT on 64-bit hosts.
    for seed in [12_225_630_750_127_142_817, 18_190_410_158_482_095_592] {
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
        let bought: Vec<_> = orders
            .iter()
            .filter_map(|order| match order {
                Order::ConfirmPurchase { variant, .. } => Some(variant.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(bought, ["test_tank"], "policy seed {seed}");
    }
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
    me.pending.push(pending(
        1,
        rules.catalog.index("test_supply").unwrap(),
        entry.center,
        0,
    ));
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
            cost: 300,
            ..identified(
                7,
                rules.catalog.index("test_jet").unwrap(),
                [500.0, 500.0, 250.0],
            )
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
