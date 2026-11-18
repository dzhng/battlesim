//! Standing suppliers, emergency purchases and the airlift lifecycle.
use super::purchases::seen;
use super::*;

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

fn emergency_match(units: Value) -> (Rules, ObservationFrame, EntrySite) {
    transport_tests::transport_match_on(supply_flyer_deck(), units)
}

fn active_credits(frame: &mut ObservationFrame, credits: f64) {
    me(frame).credits = credits;
    me(frame).cap = 100;
    me(frame).unit_count = frame.own.len() as u32;
}

#[test]
fn a_crippled_group_out_of_supplier_reach_buys_the_flyer() {
    let (rules, mut frame, entry) = emergency_match(json!([
        {"side":"red","kind":"test_tank","position":[500,700]}
    ]));
    let share = Tuning::shipped().emergency_health_share;
    frame.own[0].hp = rules.catalog.by_id("test_tank").hull().unwrap().hp * (share - 0.01);
    active_credits(&mut frame, 1_000.0);
    let orders = BalancedDoctrine::default().decide(
        DecisionInput {
            side: Side::Red,
            faction: Faction::Us,
            entry: &entry,
            assignment: None,
        },
        &frame,
        &rules,
        |_, _| Some(60.0),
    );
    assert!(orders.iter().any(|order| matches!(order,
        Order::ConfirmPurchase { variant, .. } if rules.catalog.get(rules.catalog.index(variant).unwrap()).mobility.flight().is_some()
    )), "emergency buys a flying supplier: {orders:?}");
}

#[test]
fn a_crippled_group_a_truck_can_reach_buys_no_flyer() {
    let (rules, mut frame, entry) = emergency_match(json!([
        {"side":"red","kind":"test_tank","position":[500,700]},
        {"side":"red","kind":"test_supply","position":[500,100]}
    ]));
    frame.own[0].hp = rules.catalog.by_id("test_tank").hull().unwrap().hp * 0.4;
    active_credits(&mut frame, 1000.0);
    let orders = BalancedDoctrine::default().decide(
        DecisionInput {
            side: Side::Red,
            faction: Faction::Us,
            entry: &entry,
            assignment: None,
        },
        &frame,
        &rules,
        |_, _| Some(10.0),
    );
    assert!(
        !orders
            .iter()
            .any(|order| matches!(order, Order::ConfirmPurchase { variant, .. } if super::super::purchase::flying_supplier(&rules, rules.catalog.index(variant).unwrap()))),
        "reachable truck suppresses emergency: {orders:?}"
    );
}

#[test]
fn a_healthy_group_buys_no_flyer() {
    let (rules, mut frame, entry) = emergency_match(json!([
        {"side":"red","kind":"test_tank","position":[500,700]}
    ]));
    active_credits(&mut frame, 1000.0);
    let orders = BalancedDoctrine::default().decide(
        DecisionInput {
            side: Side::Red,
            faction: Faction::Us,
            entry: &entry,
            assignment: None,
        },
        &frame,
        &rules,
        |_, _| Some(60.0),
    );
    assert!(
        !orders
            .iter()
            .any(|order| matches!(order, Order::ConfirmPurchase { variant, .. } if super::super::purchase::flying_supplier(&rules, rules.catalog.index(variant).unwrap()))),
        "healthy group has no airlift: {orders:?}"
    );
}

#[test]
fn unlimited_ammo_never_counts_as_running_low() {
    let (rules, mut frame, entry) = emergency_match(json!([
        {"side":"red","kind":"test_jeep","position":[500,700]}
    ]));
    active_credits(&mut frame, 1000.0);
    let orders = BalancedDoctrine::default().decide(
        DecisionInput {
            side: Side::Red,
            faction: Faction::Us,
            entry: &entry,
            assignment: None,
        },
        &frame,
        &rules,
        |_, _| Some(60.0),
    );
    assert!(
        !orders
            .iter()
            .any(|order| matches!(order, Order::ConfirmPurchase { variant, .. } if super::super::purchase::flying_supplier(&rules, rules.catalog.index(variant).unwrap()))),
        "unlimited HMG does not trigger: {orders:?}"
    );
}

#[test]
fn a_finite_weapon_below_its_tuning_share_triggers_the_airlift() {
    let (rules, mut frame, entry) = emergency_match(json!([
        {"side":"red","kind":"test_tank","position":[500,700]}
    ]));
    let tank = rules.catalog.index("test_tank").unwrap();
    let ammo_cap = match rules.weapons.get("tank_ap").unwrap().ammo {
        contract::weapons::AmmoCapacity::Rounds(cap) => cap,
        contract::weapons::AmmoCapacity::Unlimited(_) => unreachable!(),
    };
    let share = Tuning::shipped().emergency_ammo_share;
    frame.own[0].mounts[0].ammo[0] = Some((ammo_cap as f64 * (share - 0.01)) as u32);
    assert_eq!(frame.own[0].kind, tank);
    active_credits(&mut frame, 1_000.0);
    let orders = BalancedDoctrine::default().decide(
        DecisionInput {
            side: Side::Red,
            faction: Faction::Us,
            entry: &entry,
            assignment: None,
        },
        &frame,
        &rules,
        |_, _| Some(60.0),
    );
    assert!(orders.iter().any(
        |order| matches!(order, Order::ConfirmPurchase { variant, .. } if variant == "test_heli")
    ));
}

#[test]
fn a_reserve_flyer_is_sent_rather_than_a_second_bought() {
    let (rules, mut frame, entry) = emergency_match(json!([
        {"side":"red","kind":"test_tank","position":[500,700]},
        {"side":"red","kind":"test_heli","position":[500,100]}
    ]));
    frame.own[0].hp = rules.catalog.by_id("test_tank").hull().unwrap().hp * 0.4;
    active_credits(&mut frame, 1_000.0);
    let flyer = frame.own[1].id;
    let orders = BalancedDoctrine::default().decide(
        DecisionInput {
            side: Side::Red,
            faction: Faction::Us,
            entry: &entry,
            assignment: None,
        },
        &frame,
        &rules,
        |_, _| Some(60.0),
    );
    assert!(
        !orders
            .iter()
            .any(|order| matches!(order, Order::ConfirmPurchase { .. })),
        "reserve flyer avoids a second purchase: {orders:?}"
    );
    assert!(
        orders
            .iter()
            .any(|order| matches!(order, Order::Move { units, .. } if units.contains(&flyer))),
        "reserve flyer is sent: {orders:?}"
    );
}

/// The reserve flyer is what answers an emergency, not what rules one out:
/// a group out of every truck's reach calls it even when it could fly there
/// in time.
#[test]
fn a_reserve_flyer_in_reach_still_answers_the_emergency() {
    let (rules, mut frame, entry) = emergency_match(json!([
        {"side":"red","kind":"test_tank","position":[500,700]},
        {"side":"red","kind":"test_heli","position":[500,100]}
    ]));
    frame.own[0].hp = rules.catalog.by_id("test_tank").hull().unwrap().hp * 0.4;
    active_credits(&mut frame, 1_000.0);
    let flyer = frame.own[1].id;
    let reach = Tuning::shipped().emergency_reach_s;
    let orders = BalancedDoctrine::default().decide(
        DecisionInput {
            side: Side::Red,
            faction: Faction::Us,
            entry: &entry,
            assignment: None,
        },
        &frame,
        &rules,
        |unit, _| {
            Some(if unit == flyer {
                reach / 3.0
            } else {
                reach * 2.0
            })
        },
    );
    assert!(
        orders
            .iter()
            .any(|order| matches!(order, Order::Move { units, .. } if units.contains(&flyer))),
        "the reserve flyer is sent: {orders:?}"
    );
}

#[test]
fn an_airlift_lands_in_service_range_of_the_needy_member_of_a_spread_group() {
    let (rules, mut frame, entry) = emergency_match(json!([
        {"side":"red","kind":"test_tank","position":[500,700]},
        {"side":"red","kind":"test_tank","position":[800,700]},
        {"side":"red","kind":"test_heli","position":[500,100]}
    ]));
    frame.own[1].hp = 1.0;
    active_credits(&mut frame, 1_000.0);
    let heli = frame.own[2].id;
    let needy = frame.own[1].id;
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
            |_, _| Some(60.0),
        )
    };
    let first = decide(&mut ai, &frame);
    let goal = first
        .iter()
        .find_map(|o| match o {
            Order::Move { units, goal, .. } if units.contains(&heli) => Some(*goal),
            _ => None,
        })
        .expect("the emergency sends the flyer");
    let recipient = frame.own.iter().find(|u| u.id == needy).unwrap();
    let from_needy = distance(goal, [recipient.position[0], recipient.position[1]]);
    assert!(
        (from_needy - rules.service.radius_m * 0.5).abs() < 1e-9,
        "landing goal {goal:?} is not half a service radius from the needy unit"
    );

    let flyer = frame.own.iter_mut().find(|u| u.id == heli).unwrap();
    flyer.position = [goal[0], goal[1], 0.0];
    flyer.goal = Some(goal);
    flyer.grounded = true;
    flyer.deployment.as_mut().unwrap().progress = 1.0;
    frame.tick += 10 * u64::from(rules.tick_hz);
    let serving = decide(&mut ai, &frame);
    assert!(matches!(ai.airlift, airlift::Mission::Serving { .. }));
    assert!(
        !serving
            .iter()
            .any(|o| matches!(o, Order::Move { units, .. } if units.contains(&heli))),
        "serving at the needy member reorders: {serving:?}"
    );
    let steady = decide(&mut ai, &frame);
    assert!(
        !steady
            .iter()
            .any(|o| matches!(o, Order::Move { units, .. } if units.contains(&heli))),
        "a serving airlift keeps reordering: {steady:?}"
    );
}

#[test]
fn a_refused_or_unroutable_inbound_airlift_returns_home_without_a_goal() {
    for refused in [true, false] {
        let (rules, mut frame, entry) = emergency_match(json!([
            {"side":"red","kind":"test_tank","position":[500,700]},
            {"side":"red","kind":"test_heli","position":[500,100]}
        ]));
        frame.own[0].hp = 1.0;
        let heli = frame.own[1].id;
        active_credits(&mut frame, 1_000.0);
        let mut ai = BalancedDoctrine::default();
        let decide =
            |ai: &mut BalancedDoctrine, frame: &ObservationFrame, blocked: Option<[f64; 2]>| {
                ai.decide(
                    DecisionInput {
                        side: Side::Red,
                        faction: Faction::Us,
                        entry: &entry,
                        assignment: None,
                    },
                    frame,
                    &rules,
                    |_, point| (Some(point) != blocked).then_some(60.0),
                )
            };
        let first = decide(&mut ai, &frame, None);
        let far_side = first
            .iter()
            .find_map(|o| match o {
                Order::Move { units, goal, .. } if units.contains(&heli) => Some(*goal),
                _ => None,
            })
            .expect("the emergency sends the flyer");
        let flyer = frame.own.iter_mut().find(|u| u.id == heli).unwrap();
        flyer.position[0] = far_side[0];
        flyer.position[1] = far_side[1];
        flyer.goal = None;
        flyer.state = if refused {
            contract::observation::MoveState::NoLandingSpot
        } else {
            contract::observation::MoveState::Idle
        };
        frame.tick += 10 * u64::from(rules.tick_hz);
        let ended = decide(&mut ai, &frame, (!refused).then_some(far_side));
        let airlift::Mission::Returning { post, .. } = ai.airlift else {
            panic!(
                "a refused or unroutable far side must return: {:?}",
                ai.airlift
            );
        };
        assert!(
        ended
            .iter()
            .any(|o| matches!(o, Order::Move { units, goal, .. } if units.contains(&heli) && *goal == post)),
        "a refused or unroutable far side returns home: {ended:?}"
    );
    }
}

#[test]
fn an_unanswerable_emergency_keeps_ordinary_shopping_available() {
    for unavailable in ["no card", "no credits", "at cap"] {
        let mut authored = supply_flyer_deck();
        let patch = match unavailable {
            "no card" => json!({"roster":{"factions":["eastern"]}}),
            "no credits" => json!({"cost":2000}),
            "at cap" => json!({}),
            _ => unreachable!(),
        };
        fixtures::patch_catalog(&mut authored, "units", "test_heli", patch);
        let (rules, mut frame, entry) = transport_tests::transport_match_on(
            authored,
            json!([
                {"side":"red","kind":"test_tank","position":[500,700]}
            ]),
        );
        frame.own[0].hp = rules.catalog.by_id("test_tank").hull().unwrap().hp * 0.4;
        active_credits(&mut frame, 1000.0);
        let mut ai = BalancedDoctrine::default();
        let mut decide = |frame: &ObservationFrame| {
            ai.decide(
                DecisionInput {
                    side: Side::Red,
                    faction: Faction::Us,
                    entry: &entry,
                    assignment: None,
                },
                frame,
                &rules,
                |_, _| Some(60.0),
            )
        };
        if unavailable == "at cap" {
            me(&mut frame).cap = me(&mut frame).unit_count;
            assert!(!decide(&frame)
                .iter()
                .any(|o| matches!(o, Order::ConfirmPurchase { .. })));
            frame.tick += 10 * u64::from(rules.tick_hz);
            me(&mut frame).cap += 1;
        }
        let orders = decide(&frame);
        assert!(
            orders.iter().any(|o| matches!(o, Order::ConfirmPurchase { variant, .. }
                if unavailable == "at cap" || rules.catalog.by_id(variant).mobility.locomotion().is_some())),
            "{unavailable}: ordinary shopping must still buy: {orders:?}"
        );
    }
}

#[test]
fn an_inbound_airlift_returns_when_the_need_vanishes() {
    for vanished in ["recovered", "gone"] {
        let (rules, mut frame, entry) = emergency_match(json!([
            {"side":"red","kind":"test_tank","position":[500,700]},
            {"side":"red","kind":"test_heli","position":[500,100]}
        ]));
        frame.own[0].hp = 1.0;
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
                |_, _| Some(60.0),
            )
        };
        decide(&mut ai, &frame);
        assert!(matches!(ai.airlift, airlift::Mission::Inbound { .. }));
        if vanished == "gone" {
            frame.own.remove(0);
        } else {
            frame.own[0].hp = rules.catalog.by_id("test_tank").hull().unwrap().hp;
        }
        let flyer = frame.own.last_mut().unwrap();
        let flyer_id = flyer.id;
        flyer.goal = Some([500.0, 660.0]);
        frame.tick += 10 * u64::from(rules.tick_hz);
        let returning = decide(&mut ai, &frame);
        let airlift::Mission::Returning { post, .. } = ai.airlift else {
            panic!("{vanished}: {:?}", ai.airlift);
        };
        assert!(returning.iter().any(|o| matches!(o, Order::Move {units, goal, ..} if units.contains(&flyer_id) && *goal == post)), "it replaces the outbound flight: {returning:?}");
    }
}

#[test]
fn a_refused_flyer_purchase_does_not_block_the_next_emergency() {
    let (rules, mut frame, entry) = emergency_match(json!([
        {"side":"red","kind":"test_tank","position":[500,700]}
    ]));
    frame.own[0].hp = 1.0;
    active_credits(&mut frame, 1000.0);
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
            |_, _| Some(60.0),
        )
    };
    let first = decide(&mut ai, &frame);
    assert!(first
        .iter()
        .any(|o| matches!(o, Order::ConfirmPurchase {variant, ..} if variant == "test_heli")));
    ai.purchase_result(false, 1);
    // No flyer and no reservation appear in the next authoritative observation.
    frame.tick += 10 * u64::from(rules.tick_hz);
    decide(&mut ai, &frame);
    frame.tick += 10 * u64::from(rules.tick_hz);
    let retry = decide(&mut ai, &frame);
    assert!(
        retry
            .iter()
            .any(|o| matches!(o, Order::ConfirmPurchase {variant, ..} if variant == "test_heli")),
        "later emergency is answered: {retry:?}, {:?}",
        ai.airlift
    );
}

#[test]
fn a_returning_airlift_finishes_at_its_post_and_flies_on_past_a_refused_landing() {
    for blocked in [false, true] {
        let (rules, mut frame, entry) = emergency_match(json!([
            {"side":"red","kind":"test_heli","position":[500,100]}
        ]));
        let heli = frame.own[0].id;
        if blocked {
            frame.own[0].state = contract::observation::MoveState::NoLandingSpot;
        }
        let post = if blocked {
            [800.0, 800.0]
        } else {
            [500.0, 100.0]
        };
        let mut ai = BalancedDoctrine {
            airlift: airlift::Mission::Returning { heli, post },
            ..Default::default()
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
            |_, _| Some(60.0),
        );
        if blocked {
            assert!(
                matches!(ai.airlift, airlift::Mission::Returning { .. }),
                "a refusal away from the post keeps the mission returning: {:?}",
                ai.airlift
            );
            assert!(
                orders.iter().any(|order| matches!(order,
                    Order::Move { units, goal, .. } if units.contains(&heli) && *goal == post)),
                "it keeps flying home: {orders:?}"
            );
        } else {
            assert!(
                matches!(ai.airlift, airlift::Mission::Idle),
                "{:#?}",
                ai.airlift
            );
        }
    }
}

#[test]
fn an_inbound_airlift_returns_if_its_original_recipient_is_gone() {
    let (rules, mut frame, entry) = emergency_match(json!([
        {"side":"red","kind":"test_tank","position":[500,700]},
        {"side":"red","kind":"test_tank","position":[520,700]},
        {"side":"red","kind":"test_heli","position":[500,100]}
    ]));
    frame.own[0].hp = 1.0;
    frame.own[1].hp = 1.0;
    let original = frame.own[0].id;
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
            |_, _| Some(60.0),
        )
    };
    decide(&mut ai, &frame);
    assert!(matches!(ai.airlift, airlift::Mission::Inbound { .. }));
    frame.own.retain(|u| u.id != original);
    frame.tick += 10 * u64::from(rules.tick_hz);
    decide(&mut ai, &frame);
    assert!(
        matches!(ai.airlift, airlift::Mission::Returning { .. }),
        "the original emergency ends even if another member is hurt: {:?}",
        ai.airlift
    );
}

#[test]
fn an_airlift_follows_a_group_that_moves_before_or_after_landing() {
    for (landed, displacement) in [(true, 45.0), (false, 400.0), (true, 400.0)] {
        let (mut rules, mut frame, entry) = emergency_match(json!([
            {"side":"red","kind":"test_tank","position":[500,700]},
            {"side":"red","kind":"test_heli","position":[500,100]}
        ]));
        rules.service.radius_m = 80.0;
        rules.air.rotor.landing_reach_m = 40.0;
        frame.own[0].hp = 1.0;
        let heli = frame.own[1].id;
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
                |_, _| Some(60.0),
            )
        };
        let first = decide(&mut ai, &frame);
        let goal = first
            .iter()
            .find_map(|o| match o {
                Order::Move { units, goal, .. } if units.contains(&heli) => Some(*goal),
                _ => None,
            })
            .expect("the emergency sends the flyer");
        if landed {
            frame.own[1].position = [goal[0], goal[1], 0.0];
            frame.own[1].grounded = true;
            frame.own[1].deployment.as_mut().unwrap().progress = 1.0;
            frame.tick += 10 * u64::from(rules.tick_hz);
            decide(&mut ai, &frame);
        } else {
            frame.own[1].goal = Some(goal);
        }
        let away = [
            frame.own[0].position[0] - goal[0],
            frame.own[0].position[1] - goal[1],
        ];
        let length = libm::hypot(away[0], away[1]);
        frame.own[0].position[0] += displacement * away[0] / length;
        frame.own[0].position[1] += displacement * away[1] / length;
        frame.tick += 10 * u64::from(rules.tick_hz);
        let following = decide(&mut ai, &frame);
        assert!(
            following.iter().any(|o| matches!(o,
                Order::Move { units, goal: next, .. }
                    if units.contains(&heli) && distance(*next, goal) > displacement * 0.5
            )),
            "landed={landed}: the flyer follows the moving group: {following:?}"
        );
    }
}
