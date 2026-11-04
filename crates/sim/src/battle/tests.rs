use super::*;

/// Two idle aircraft in high air, one flown by wings and one, against
/// its row, by rotors: the wings keep it flying, the rotors hold it in a
/// hover. The flight model picks the law, never the layer.
#[test]
fn the_flight_model_not_the_layer_picks_the_flight_law() {
    let setup: ScenarioDefinition = serde_json::from_value(serde_json::json!({
        "map": { "size": [3000, 3000], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": crate::fixtures::test_rules(),
        "units": [
            { "side": "blue", "kind": "test_jet", "position": [1000, 1000] },
            { "side": "blue", "kind": "test_jet", "position": [2000, 2000] }
        ],
        "events": [], "scripts": []
    }))
    .unwrap();
    let mut battle = Battle::new(&setup, 3);
    let units::Motion::Air(flight) = &mut battle.units[1].motion else {
        unreachable!("a jet flies")
    };
    flight.model = contract::catalog::FlightModel::Rotor;
    assert_eq!(battle.units[1].layer(), AltitudeLayer::HighAir);
    // Path flown, not displacement: an idle jet circles where it is.
    let mut flown = [0.0; 2];
    for _ in 0..5 * battle.rules.tick_hz {
        let before = [battle.units[0].position.xy(), battle.units[1].position.xy()];
        battle.step();
        for (i, at) in before.into_iter().enumerate() {
            flown[i] += (battle.units[i].position.xy() - at).length();
        }
    }
    let units::Motion::Air(flight) = battle.units[0].motion else {
        unreachable!("a jet flies")
    };
    assert!(
        flown[0] > 4.0 * flight.cruise_mps,
        "the winged jet keeps flying: {:.1} m",
        flown[0]
    );
    assert!(flown[1] < 1.0, "the rotor jet hovers: {:.1} m", flown[1]);
}

/// A helicopter's settling goal steers it every tick, so it is part of
/// the battle's state: two battles alike but for it digest apart.
#[test]
fn a_helicopters_settling_goal_is_in_the_digest() {
    let setup: ScenarioDefinition = serde_json::from_value(serde_json::json!({
        "map": { "size": [2000, 2000], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": crate::fixtures::test_rules(),
        "units": [{ "side": "blue", "kind": "test_heli", "position": [1000, 1000] }],
        "events": [], "scripts": []
    }))
    .unwrap();
    let settling = |goal: V2| {
        let mut battle = Battle::new(&setup, 1);
        battle.units[0].air.as_mut().unwrap().settle = Some(crate::units::Settle {
            goal,
            aim: v2(1100.0, 1000.0),
            back: false,
        });
        battle.digest()
    };
    assert_ne!(settling(v2(1050.0, 1000.0)), settling(v2(1060.0, 1000.0)));
}

/// No order leaves a jet in a run: changing the route of its attack-move
/// ends its run, release cue and dogfight, as every other order does.
#[test]
fn upgrading_a_jets_move_breaks_off_its_run() {
    let setup: ScenarioDefinition = serde_json::from_value(serde_json::json!({
        "map": { "size": [4000, 4000], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": crate::fixtures::test_rules(),
        "units": [
            { "side": "blue", "kind": "test_jet", "position": [1000, 2000], "yaw": 0.0 },
            { "side": "red", "kind": "test_tank", "position": [2000, 2000], "engagement": "return_fire_only" }
        ],
        "events": [], "scripts": []
    }))
    .unwrap();
    let mut battle = Battle::new(&setup, 1);
    let envelope = |seq, order| CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq,
        queued: false,
        order,
    };
    let ack = battle.accept(envelope(
        1,
        Order::AttackMove {
            units: vec![UnitId(0)],
            gesture: 1,
            goal: [3000.0, 2000.0],
        },
    ));
    assert_eq!(ack.error, None);
    battle.step();
    let jet = &mut battle.units[0];
    assert!(
        jet.orders[0].movement_mut().unwrap().policy != RoutePolicy::Fastest,
        "the upgrade changes the route"
    );
    // Mid-run, about to release, in a fight.
    let air = jet.air.as_mut().unwrap();
    air.pass = Some(crate::air::pass::PassPlan {
        target: Target::Unit(UnitId(1)),
        aim: v3(2000.0, 2000.0, 0.0),
        phase: crate::air::pass::PassPhase::Dive,
        weapon: crate::air::pass::PassWeapon::Gun,
        agl: 120.0,
        heading: 0.0,
    });
    air.pass_release = true;
    air.dogfight_target = Some(UnitId(1));
    let upgrade = envelope(
        2,
        Order::UpgradeMove {
            gesture: 1,
            route: RoutePolicy::Fastest,
        },
    );
    let prepared = battle.prepare(upgrade, battle.tick).unwrap();
    battle.apply(prepared);
    let air = battle.units[0].air.unwrap();
    assert_eq!(air.pass, None, "the run survived the order");
    assert!(!air.pass_release, "the release cue survived the order");
    assert_eq!(air.dogfight_target, None, "the dogfight survived the order");
}

/// Contact ids are each side's own: when blue and red each hold a
/// contact numbered alike, in different places, each side's seeker
/// flies at the contact its own side holds.
#[test]
fn a_seeker_flies_at_its_own_sides_contact() {
    let setup: ScenarioDefinition = serde_json::from_value(serde_json::json!({
        "map": { "size": [6000, 6000], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": crate::fixtures::test_rules(),
        "units": [
            { "side": "blue", "kind": "test_tank", "position": [500, 500], "engagement": "return_fire_only" },
            { "side": "red", "kind": "test_tank", "position": [5500, 5500], "engagement": "return_fire_only" }
        ],
        "events": [], "scripts": []
    }))
    .unwrap();
    let mut battle = Battle::new(&setup, 3);
    // Each side hears the other fire from far apart: each holds one
    // contact, numbered alike.
    let blue_heard = v3(4000.0, 1000.0, 0.0);
    let red_heard = v3(1000.0, 4000.0, 0.0);
    battle.knowledge[Side::Blue.index()].note_fire(UnitId(1), blue_heard, AltitudeLayer::Ground, 0);
    battle.knowledge[Side::Red.index()].note_fire(UnitId(0), red_heard, AltitudeLayer::Ground, 0);
    battle.step();
    let contact = |side: Side| {
        let contacts: Vec<_> = battle.knowledge[side.index()].all_contacts().collect();
        assert_eq!(contacts.len(), 1, "{side:?} holds one contact");
        (contacts[0].id, contacts[0].center)
    };
    let (blue_id, blue_at) = contact(Side::Blue);
    let (red_id, red_at) = contact(Side::Red);
    assert_eq!(blue_id, red_id, "both sides number their contact alike");
    let weapon = battle
        .arsenal
        .weapons
        .iter()
        .position(|w| w.def.seeker.is_some())
        .expect("a seeker row");
    let mut launch = |side: Side, unit: UnitId| {
        let projectile = battle.projectiles.launch(crate::flight::Launch {
            origin: v3(3000.0, 3000.0, 100.0),
            carrier: v3(0.0, 0.0, 0.0),
            velocity: v3(0.0, 0.0, 100.0),
            gravity_scale: 0.0,
            lifetime_s: 30.0,
            suppression_radius_m: 0.0,
            shooter: None,
            guidance: Some(crate::flight::Guidance {
                point: v3(3000.0, 3000.0, 1000.0),
                turn_rad_s: 1.0,
                supported: false,
                loft: None,
            }),
            motor: None,
            fall: None,
        });
        battle.rounds.insert(
            projectile,
            Round {
                weapon,
                unit,
                side,
                seeker_target: Some(Target::Contact(blue_id)),
                fused_target: None,
                layer: AltitudeLayer::Ground,
            },
        );
        projectile
    };
    let blue_round = launch(Side::Blue, UnitId(0));
    let red_round = launch(Side::Red, UnitId(1));
    battle.step();
    let steered = |round| {
        battle
            .projectiles
            .get(round)
            .and_then(|p| p.guidance)
            .expect("the seeker still flies")
            .point
    };
    let near = |a: V3, b: V3| (a.xy() - b.xy()).length() < 1.0;
    assert!(
        near(steered(blue_round), blue_at),
        "blue's seeker flies at blue's contact"
    );
    assert!(
        near(steered(red_round), red_at),
        "red's seeker flies at red's contact"
    );
}

/// A battle that changes what each side knows in every way a battle
/// can: a tank shoves a crate and knocks a lane through a wood, shells
/// fell trees, break a wall into rubble and a wreck into a lighter
/// wreck, and a wreck appears; one side watches from close by, another
/// learns of each only as it comes into view, and the neutral side looks
/// on from where it stands, past the wall. Whatever a side
/// believes at any tick, its planning grid, kept up body by body, is
/// the grid built whole from those beliefs.
#[test]
fn each_sides_grid_stays_the_whole_build_of_what_it_believes() {
    let burst = |tick: u64, at: [f64; 2]| serde_json::json!({ "tick": tick, "burst": { "point": at, "weapon": "tank_he" } });
    let mut events = vec![serde_json::json!({ "tick": 60, "add_prop": {
        "kind": "light_wreck", "center": [150, 30], "yaw": 0.4, "half_extents": [2.2, 1.0, 0.9],
        "wreck_of": "test_jeep" } })];
    for round in 0..12 {
        let tick = 20 + 25 * round;
        events.push(burst(tick, [150.0 + 4.0 * round as f64, 80.0]));
        events.push(burst(tick + 3, [232.0, 60.0]));
        if round < 5 {
            events.push(burst(tick + 6, [236.0, 140.0]));
        }
    }
    // A row of crates too close together for a tank to pass between.
    let mut props: Vec<serde_json::Value> = (0..13)
        .map(|k| {
            serde_json::json!({ "kind": "crate", "center": [70.0, 79.0 + 3.5 * k as f64],
                "yaw": 0, "half_extents": [0.8, 0.8, 0.6] })
        })
        .collect();
    props.push(
        serde_json::json!({ "kind": "wall", "center": [232, 60], "yaw": 0,
        "half_extents": [0.4, 10, 1] }),
    );
    props.push(
        serde_json::json!({ "kind": "heavy_wreck", "center": [236, 140], "yaw": 0.3,
        "half_extents": [3, 1.5, 1], "wreck_of": "test_tank" }),
    );
    let (wall, wreck) = (13, 14);
    let drive = |side: &str, unit: u32, goal: [f64; 2]| {
        serde_json::json!({ "tick": 1, "side": side, "order": { "kind": "move",
            "units": [unit], "gesture": unit + 1, "goal": goal, "route": "shortest" } })
    };
    let setup: ScenarioDefinition = serde_json::from_value(serde_json::json!({
        "map": { "size": [900, 200], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
            "forests": [{ "shape": { "kind": "polygon",
                "ring": [[100, 50], [200, 50], [200, 150], [100, 150]] } }],
            "props": props },
        "rules": crate::fixtures::test_rules(),
        "units": [
            { "side": "blue", "kind": "test_tank", "position": [40, 100], "yaw": 0.0,
              "engagement": "return_fire_only" },
            { "side": "blue", "kind": "test_jeep", "position": [40, 30], "yaw": 0.0,
              "engagement": "return_fire_only" },
            { "side": "red", "kind": "test_rifle", "position": [860, 100],
              "engagement": "return_fire_only" },
            { "side": "red", "kind": "test_jeep", "position": [880, 180], "yaw": 3.1,
              "engagement": "return_fire_only" },
            { "side": "neutral", "kind": "test_rifle", "position": [320, 130] }
        ],
        "events": events,
        "scripts": [
            drive("blue", 0, [290.0, 100.0]),
            drive("blue", 1, [300.0, 30.0]),
            drive("red", 3, [30.0, 180.0])
        ]
    }))
    .expect("a scenario");
    let mut battle = Battle::new(&setup, 3);
    let radius = battle.rules.physics.soldier_radius_m;
    for tick in 1..=1200 {
        battle.step();
        if tick % 20 != 0 {
            continue;
        }
        for side in Side::ALL {
            let (kept, whole) = battle.sides[side.index()].grid_beside_whole_build(
                &battle.world,
                battle.authored_props,
                radius,
            );
            kept.assert_same(&format!("tick {tick}, {side:?}"), &whole);
        }
    }
    // The battle did change what the sides know, in each of the ways.
    let world = &battle.world;
    assert!(world.cleared_cells() > 0, "a lane was cleared");
    assert!(world.moved().next().is_some(), "a crate was shoved");
    assert!(world.prop(wall).is_none(), "the wall fell");
    assert!(world.prop(wreck).is_none(), "the wreck broke up");
    assert!(
        world.props().any(|p| p.known_to_all),
        "remains both sides plan with"
    );
    for side in Side::COMBATANTS {
        let side = &battle.sides[side.index()];
        assert!(side.revision > 0);
        assert!(!side.seen.is_empty());
    }
    assert!(battle.load().grid_cells_relaid > 0);
}

#[test]
fn an_enemy_round_is_clipped_to_seen_ground_leg_by_leg_keeping_its_ricochets() {
    // 8 m cells, row 0 seen except column 2 (x 16–24).
    let seen: u32 = (0..10).filter(|&i| i != 2).map(|i| 1 << i).sum();
    let fog = VisibilityField {
        cell_m: 8.0,
        nx: 10,
        ny: 2,
        bits: vec![seen],
    };
    let normal = v3(-1.0, 0.0, 0.0);
    let round = Flown {
        side: Side::Blue,
        from: v3(2.0, 4.0, 1.0),
        ricochets: vec![(v3(40.0, 4.0, 1.0), normal)],
        to: v3(60.0, 4.0, 1.0),
        weapon: 0,
        shooter: None,
        hit: Some((SegmentHit::Ground, v3(0.0, 0.0, 1.0))),
    };
    let pieces = clip_to_seen(&fog, &round);
    let paths: Vec<Vec<[f64; 3]>> = pieces.iter().map(|p| p.path.clone()).collect();
    // The first leg samples every 4.75 m: seen to 11.5, unseen at 16.25
    // and 21, seen again from 25.75 through the ricochet to the end.
    assert_eq!(
        paths,
        [
            vec![[2.0, 4.0, 1.0], [11.5, 4.0, 1.0]],
            vec![[25.75, 4.0, 1.0], [40.0, 4.0, 1.0], [60.0, 4.0, 1.0]],
        ]
    );
    assert!(pieces[0].ricochets.is_empty());
    assert_eq!(pieces[0].hit, SegmentHit::None);
    assert_eq!(
        pieces[1].ricochets,
        [SegmentRicochet {
            point: 1,
            normal: [-1.0, 0.0, 0.0]
        }]
    );
    assert_eq!(pieces[1].hit, SegmentHit::Ground, "its end is seen");
    assert!(pieces.iter().all(|p| !p.own && p.kind == 0));
}
#[test]
fn seeing_one_remain_clears_far_historical_building_snapshots() {
    let mut descriptor: contract::templates::BuildingTemplateDescriptor = serde_json::from_str(
        include_str!("../../../../fixtures/parity/templates/asymmetric.json"),
    )
    .unwrap();
    descriptor.parts[0].half_extents[0] = 50.0;
    descriptor.parts[1].center[0] = 52.0;
    for edge in &mut descriptor.edges {
        if edge.id == "main-north" || edge.id == "main-south" {
            edge.span_m = [-50.0, 50.0];
        }
    }
    let catalogue =
        contract::templates::TemplateGeometryCatalog::new(vec![descriptor.clone()]).unwrap();
    let geometry = descriptor
        .materialize(contract::templates::PlacementFrame {
            translation: [400.0, 300.0, 0.0],
            yaw: 0.0,
        })
        .unwrap();
    let mut rules = crate::fixtures::test_rules();
    crate::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_rifle",
        serde_json::json!({"sensors":{"ground_m":30}}),
    );
    let setup = serde_json::from_value(serde_json::json!({
        "map":{"size":[800,600],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,
            "template_catalog_hash":catalogue.hash(),"regional_family":descriptor.regional_family,
            "buildings":[{"owner":0,"kind":"building","category":descriptor.category,"regional_family":descriptor.regional_family,"parts":[{"part":"main","prop":0},{"part":"wing","prop":1}],"geometry":geometry}]},
        "rules":rules,"units":[{"side":"blue","kind":"test_rifle","position":[250,100],"engagement":"return_fire_only"}],"events":[],"scripts":[]
    })).unwrap();
    let mut battle = Battle::new(&setup, 11);
    battle.destroy_prop(0, v2(1.0, 0.0));
    assert!(
        battle.sides[0].standing().contains_key(&1),
        "unseen wing remains remembered"
    );
    let delta = crate::math::v3(78.0, 200.0, 0.0);
    battle.units[0].position = battle.units[0].position + delta;
    for member in &mut battle.units[0].members {
        member.position = member.position + delta;
    }
    battle.sweep_fog(Side::Blue);
    assert!(
        !battle.fog[0].visible(452.0, 301.0),
        "far wing stays fogged"
    );
    assert!(!battle.sides[0].standing().contains_key(&0));
    assert!(
        !battle.sides[0].standing().contains_key(&1),
        "whole aggregate's historical snapshots are cleared"
    );
}
#[test]
fn corpse_support_loss_is_physical_but_enemy_memory_waits_for_sight() {
    for whole_squad in [false, true] {
        let descriptor: contract::templates::BuildingTemplateDescriptor = serde_json::from_str(
            include_str!("../../../../fixtures/parity/templates/asymmetric.json"),
        )
        .unwrap();
        let catalogue =
            contract::templates::TemplateGeometryCatalog::new(vec![descriptor.clone()]).unwrap();
        let mut geometry = descriptor
            .materialize(contract::templates::PlacementFrame {
                translation: [400.0, 300.0, 0.0],
                yaw: 0.0,
            })
            .unwrap();
        geometry.floor_z = vec![0.0, 3.0, 6.0];
        let mut rules = crate::fixtures::test_rules();
        crate::fixtures::patch_catalog(
            &mut rules,
            "units",
            "test_rifle",
            serde_json::json!({"sensors":{"ground_m":60}}),
        );
        rules["weapons"]["rifle"]["damage"] = serde_json::json!(1e6);
        rules["bail_out"]["building"]["death"] = serde_json::json!(1.0);
        let setup = serde_json::from_value(serde_json::json!({
            "map":{"size":[800,600],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,
                "template_catalog_hash":catalogue.hash(),"regional_family":descriptor.regional_family,
                "buildings":[{"owner":0,"kind":"building","category":descriptor.category,"regional_family":descriptor.regional_family,
                    "parts":[{"part":"main","prop":0},{"part":"wing","prop":1}],"geometry":geometry}]},
            "rules":rules,"units":[
                {"side":"red","kind":"test_rifle","position":[380,300],"engagement":"return_fire_only"},
                {"side":"blue","kind":"test_rifle","position":[350,300],"engagement":"return_fire_only"}
            ],"events":[],"scripts":[]
        })).unwrap();
        let mut battle = Battle::new(&setup, 11);
        let ack = battle.accept(CommandEnvelope {
            side: Side::Red,
            player: None,
            seq: 1,
            queued: false,
            order: Order::Garrison {
                units: vec![UnitId(0)],
                building: 0,
            },
        });
        assert_eq!(ack.error, None);
        for _ in 0..300 {
            battle.step();
        }
        assert!(battle.units[0].garrisoned());
        let upper = battle.units[0]
            .members
            .iter()
            .find(|s| s.position.z == 6.0 && battle.fog[0].visible(s.position.x, s.position.y))
            .unwrap()
            .clone();
        let targets: Vec<_> = battle.units[0]
            .members
            .iter()
            .filter(|s| whole_squad || s.id == upper.id)
            .map(|s| (s.id, s.position))
            .collect();
        let weapon = battle
            .arsenal
            .weapons
            .iter()
            .position(|w| w.id == "rifle")
            .unwrap();
        // Feed physical hit events at damage's boundary: stochastic aiming
        // is not the support or knowledge contract under test.
        let mut rounds = BTreeMap::new();
        let events: Vec<_> = targets
            .iter()
            .enumerate()
            .map(|(i, (id, at))| {
                let projectile = ProjectileId(i as u64);
                rounds.insert(
                    projectile,
                    Round {
                        weapon,
                        unit: UnitId(1),
                        side: Side::Blue,
                        seeker_target: None,
                        fused_target: None,
                        layer: AltitudeLayer::Ground,
                    },
                );
                FlightEvent::Impact(flight::Impact {
                    projectile,
                    struck: flight::Struck::Body(flight::BodyId(*id)),
                    point: *at,
                    normal: v3(1.0, 0.0, 0.0),
                    velocity: v3(100.0, 0.0, 0.0),
                    time: 0.0,
                    bounces: 0,
                    pose: None,
                    detonated: false,
                })
            })
            .collect();
        damage::resolve(
            &DamageContext {
                world: &battle.world,
                ground: &battle.ground,
                arsenal: &battle.arsenal,
                rules: &battle.rules,
                tick: battle.tick,
            },
            &events,
            &rounds,
            &mut battle.suppressed,
            &mut battle.units,
            &mut battle.damage_rng,
        );
        assert_eq!(battle.units[0].alive(), !whole_squad);
        assert_eq!(battle.units[0].hold.is_none(), whole_squad);
        battle.sweep_fog(Side::Blue);
        battle.observe_all();
        let remembered = battle
            .observe(Side::Blue)
            .corpses
            .iter()
            .find(|f| f.soldier == upper.id)
            .unwrap()
            .clone();
        assert_eq!(remembered.position[2], 6.0);
        let delta = v3(-300.0, -250.0, 0.0);
        battle.units[1].position = battle.units[1].position + delta;
        for member in &mut battle.units[1].members {
            member.position = member.position + delta;
        }
        battle.sweep_fog(Side::Blue);
        assert!(!battle.fog[0].visible(remembered.position[0], remembered.position[1]));
        let damage_before_collapse = battle.damage_rng.clone();
        battle.destroy_prop(0, v2(1.0, 0.0));
        assert_eq!(
            battle.damage_rng, damage_before_collapse,
            "collapse never draws from damage_rng"
        );
        battle.observe_all();
        let physical = battle
            .observe(Side::Red)
            .corpses
            .iter()
            .find(|f| f.soldier == upper.id)
            .unwrap();
        let z = battle
            .world
            .surface_at(remembered.position[0], remembered.position[1])
            .unwrap()
            .z;
        assert_eq!(
            physical.position,
            [remembered.position[0], remembered.position[1], z]
        );
        assert_eq!(
            battle
                .observe(Side::Blue)
                .corpses
                .iter()
                .find(|f| f.soldier == upper.id)
                .unwrap()
                .position,
            remembered.position,
            "an unseen collapse cannot move a remembered enemy corpse"
        );
        battle.destroy_prop(0, v2(1.0, 0.0));
        battle.units[1].position = battle.units[1].position - delta;
        for member in &mut battle.units[1].members {
            member.position = member.position - delta;
        }
        battle.sweep_fog(Side::Blue);
        assert!(battle.fog[0].visible(remembered.position[0], remembered.position[1]));
        battle.observe_all();
        assert_eq!(
            battle
                .observe(Side::Blue)
                .corpses
                .iter()
                .find(|f| f.soldier == upper.id)
                .unwrap()
                .position,
            [remembered.position[0], remembered.position[1], z]
        );
    }
}
