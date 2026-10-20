//! Which card the doctrine buys (D56, D2): value per credit within a role,
//! weapon-made answers to aircraft, and armour in the recipe.

use super::*;

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
    }
}

fn us_cards(rules: &Rules) -> Vec<TypeIndex> {
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
