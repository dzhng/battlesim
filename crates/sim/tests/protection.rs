use crate::common::*;
use contract::ids::UnitId;
use sim::flight::{
    self, Body, BodyId, FlightEvent, ImpactContext, ImpactDecision, ImpactResolver, Interception,
    Launch, Pose, ProjectileId, Projectiles, Shape, Struck,
};
use sim::math::v3;

struct Shield {
    intercepted: Vec<Interception>,
}
impl ImpactResolver for Shield {
    fn resolve(&mut self, _: &ImpactContext) -> ImpactDecision {
        ImpactDecision::Stop
    }
    fn max_standoff(&self, _: ProjectileId) -> f64 {
        8.0
    }
    fn standoff(&self, _: ProjectileId, _: &Body) -> Option<f64> {
        Some(8.0)
    }
    fn intercept(&mut self, event: &Interception) -> bool {
        self.intercepted.push(Interception { ..*event });
        true
    }
}
fn shot(x: f64, speed: f64) -> Launch {
    Launch {
        origin: v3(x, 200.0, 1.0),
        velocity: v3(speed, 0.0, 0.0),
        gravity_scale: 0.0,
        lifetime_s: profile("rifle").lifetime_s,
        suppression_radius_m: 2.0,
        shooter: None,
        guidance: None,
        motor: None,
        fall: None,
    }
}
fn hull() -> Body {
    Body {
        id: BodyId(1),
        unit: UnitId(1),
        shape: Shape::Box {
            half: v3(2.0, 2.0, 2.0),
        },
        from: Pose {
            base: v3(130.0, 200.0, 0.0),
            yaw: 0.0,
        },
        to: Pose {
            base: v3(130.0, 200.2, 0.0),
            yaw: 0.0,
        },
    }
}
#[test]
fn swept_fast_moving_hull_threat_bursts_at_surface_standoff_without_direct_hit() {
    let world = flat([1000.0, 400.0], "");
    let mut store = Projectiles::new(config());
    let id = store.launch(shot(100.0, 1200.0));
    let mut events = Vec::new();
    let mut shield = Shield {
        intercepted: Vec::new(),
    };
    flight::advance_projectiles(&mut store, &world, &[hull()], &mut events, &mut shield);
    assert_eq!(shield.intercepted.len(), 1);
    assert!((shield.intercepted[0].point.x - 120.0).abs() < 0.001);
    assert!(events.iter().any(|e|matches!(e,FlightEvent::Impact(i) if i.projectile==id && i.detonated && i.struck==Struck::Terrain && (i.point.x-120.0).abs()<0.001)));
    assert!(!events
        .iter()
        .any(|e| matches!(e,FlightEvent::Impact(i) if matches!(i.struck,Struck::Body(_)))));
}

fn capability() -> contract::catalog::ActiveProtection {
    contract::catalog::ActiveProtection {
        name: "Trophy".into(),
        icon: "trophy".into(),
        capacity: 4,
        cooldown_s: 3.0,
        standoff_m: 8.0,
        service_s: 10.0,
        stock_per_charge: 20,
    }
}
#[test]
fn finite_protection_obeys_fractional_exact_cooldown_and_exhaustion() {
    let cap = capability();
    let mut state = sim::protection::State::new(&cap);
    assert!(state.intercept(&cap, 10.25, 30));
    assert_eq!(state.charges, 3);
    assert_eq!(state.ready_at_tick, 100.25);
    assert!(!state.intercept(&cap, 100.25 - 0.000001, 30));
    assert_eq!(state.charges, 3);
    for time in [100.25, 190.25, 280.25] {
        assert!(state.intercept(&cap, time, 30));
    }
    assert_eq!(state.charges, 0);
    assert!(!state.intercept(&cap, 1000.0, 30));
    assert_eq!(state.ready_at_tick, 370.25);
}

#[test]
fn near_pass_and_cover_before_the_hull_do_not_request_interception() {
    for covered in [false, true] {
        let world = flat(
            [1000.0, 400.0],
            if covered {
                r#", "props":[{"kind":"wall","center":[125,200],"yaw":0,"half_extents":[0.5,8,2]}]"#
            } else {
                ""
            },
        );
        let mut store = Projectiles::new(config());
        let mut launch = shot(100.0, 1200.0);
        if !covered {
            launch.origin.y = 205.0;
        }
        store.launch(launch);
        let mut events = Vec::new();
        let mut shield = Shield {
            intercepted: Vec::new(),
        };
        flight::advance_projectiles(&mut store, &world, &[hull()], &mut events, &mut shield);
        assert!(shield.intercepted.is_empty());
        if covered {
            assert!(events
                .iter()
                .any(|e| matches!(e,FlightEvent::Impact(i) if i.struck==Struck::Prop(0))));
        } else {
            assert_eq!(store.active().len(), 1);
        }
    }
}

struct LimitedShield {
    state: sim::protection::State,
    seen: Vec<ProjectileId>,
}
impl ImpactResolver for LimitedShield {
    fn resolve(&mut self, _: &ImpactContext) -> ImpactDecision {
        ImpactDecision::Stop
    }
    fn max_standoff(&self, _: ProjectileId) -> f64 {
        8.0
    }
    fn standoff(&self, _: ProjectileId, _: &Body) -> Option<f64> {
        Some(8.0)
    }
    fn intercept(&mut self, event: &Interception) -> bool {
        self.seen.push(event.projectile);
        self.state.intercept(&capability(), event.time, 30)
    }
}
#[test]
fn simultaneous_threats_resolve_by_event_time_then_id_and_saturate_cooldown() {
    for tied in [false, true] {
        let world = flat([1000.0, 400.0], "");
        let mut store = Projectiles::new(config());
        let first = store.launch(shot(100.0, 1200.0));
        let second = store.launch(shot(if tied { 100.0 } else { 110.0 }, 1200.0));
        let mut events = Vec::new();
        let mut shield = LimitedShield {
            state: sim::protection::State::new(&capability()),
            seen: Vec::new(),
        };
        flight::advance_projectiles(&mut store, &world, &[hull()], &mut events, &mut shield);
        let (early, late) = if tied {
            (first, second)
        } else {
            (second, first)
        };
        assert_eq!(shield.seen, vec![early, late]);
        assert_eq!(shield.state.charges, 3);
        assert!(events.iter().any(|e|matches!(e,FlightEvent::Impact(i) if i.projectile==early && i.detonated && i.struck==Struck::Terrain)));
        assert!(events.iter().any(|e|matches!(e,FlightEvent::Impact(i) if i.projectile==late && i.struck==Struck::Body(BodyId(1)))));
    }
}

#[test]
fn production_resolver_prevents_hull_damage_but_keeps_ordinary_collateral_and_source() {
    use contract::ids::Side;
    use contract::random::Rng;
    use serde_json::json;
    use sim::battle::{Battle, Round};
    use sim::damage::{self, DamageContext, HullResolver};
    use std::collections::BTreeMap;
    let mut rules = sim::fixtures::test_game();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_tank",
        json!({"capabilities":{"active_protection":capability()}}),
    );
    rules["weapons"]["atgm"]["interceptable"] = json!(true);
    let setup=serde_json::from_value(json!({"rules":rules,"map":{"size":[1000,400],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35},"units":[
        {"side":"blue","kind":"test_tank","position":[50,200]},
        {"side":"red","kind":"test_tank","position":[130,200]},
        {"side":"red","kind":"test_jeep","position":[118,202.5]}
    ]})).unwrap();
    let battle = Battle::new(&setup, 7);
    let mut units: Vec<_> = (0..3)
        .map(|i| battle.unit(UnitId(i)).unwrap().clone())
        .collect();
    units[1].hp = 1.0;
    units[2].hp = 0.01;
    let half = units[1].hull.unwrap();
    let body = Body {
        id: BodyId(sim::weapons::VEHICLE_BODY_BASE + 1),
        unit: UnitId(1),
        shape: Shape::Box { half },
        from: Pose {
            base: units[1].position,
            yaw: 0.0,
        },
        to: Pose {
            base: units[1].position,
            yaw: 0.0,
        },
    };
    let mut store = Projectiles::new(config());
    let id = store.launch(shot(100.0, 1200.0));
    let weapon = battle
        .arsenal()
        .weapons
        .iter()
        .position(|w| w.id == "atgm")
        .unwrap();
    let rounds = BTreeMap::from([(
        id,
        Round {
            weapon,
            unit: UnitId(0),
            side: Side::Blue,
        },
    )]);
    let mut events = Vec::new();
    let mut ricochet = Rng::new(7);
    let mut resolver = HullResolver {
        rules: battle.rules(),
        arsenal: battle.arsenal(),
        rounds: &rounds,
        units: &mut units,
        tick: 1,
        rng: &mut ricochet,
    };
    flight::advance_projectiles(
        &mut store,
        battle.world(),
        &[body],
        &mut events,
        &mut resolver,
    );
    assert_eq!(units[1].protection.unwrap().charges, 3);
    let outcome = damage::resolve(
        &DamageContext {
            world: battle.world(),
            ground: battle.ground(),
            arsenal: battle.arsenal(),
            rules: battle.rules(),
            tick: 1,
        },
        &events,
        &rounds,
        &mut BTreeMap::new(),
        &mut units,
        &mut Rng::new(7),
    );
    assert_eq!(
        units[1].hp, 1.0,
        "no prevented direct hit or duplicate blast on protected hull"
    );
    assert_eq!(
        outcome.destroyed,
        vec![damage::UnitDeath {
            victim: UnitId(2),
            source: Some(damage::LethalSource {
                unit: UnitId(0),
                side: Side::Blue
            })
        }]
    );
}

#[test]
fn supply_restores_one_charge_per_ten_seconds_with_finite_stock_and_unchanged_cooldown() {
    use serde_json::json;
    use sim::battle::Battle;
    use std::collections::BTreeSet;
    let mut rules = sim::fixtures::test_game();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_tank",
        json!({"capabilities":{"active_protection":capability()}}),
    );
    let setup=serde_json::from_value(json!({"rules":rules,"map":{"size":[1000,400],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35},"units":[
        {"side":"blue","kind":"test_supply","position":[100,200],"stock":40},
        {"side":"blue","kind":"test_tank","position":[120,200]}
    ]})).unwrap();
    let battle = Battle::new(&setup, 7);
    let mut units: Vec<_> = (0..2)
        .map(|i| battle.unit(UnitId(i)).unwrap().clone())
        .collect();
    let deployment = units[0].deployment.as_mut().unwrap();
    deployment.current = deployment.duration;
    units[1].protection = Some(sim::protection::State {
        charges: 0,
        ready_at_tick: 9999.25,
    });
    let mut next_soldier = 0;
    let service = |units: &mut [sim::units::Unit], next_soldier: &mut u32| {
        sim::supply::service(
            battle.world(),
            units,
            battle.arsenal(),
            battle.rules(),
            &[false, false],
            &BTreeSet::new(),
            next_soldier,
        )
    };
    for _ in 0..299 {
        service(&mut units, &mut next_soldier);
    }
    assert_eq!(units[1].protection.unwrap().charges, 0);
    assert_eq!(units[0].stock, Some(40));
    service(&mut units, &mut next_soldier);
    assert_eq!(units[1].protection.unwrap().charges, 1);
    assert_eq!(units[0].stock, Some(20));
    for _ in 0..600 {
        service(&mut units, &mut next_soldier);
    }
    assert_eq!(units[1].protection.unwrap().charges, 2);
    assert_eq!(units[0].stock, Some(0));
    assert_eq!(units[1].protection.unwrap().ready_at_tick, 9999.25);
    units[0].stock = Some(60);
    for _ in 0..900 {
        service(&mut units, &mut next_soldier);
    }
    assert_eq!(units[1].protection.unwrap().charges, 4);
    assert_eq!(units[0].stock, Some(20));
}

#[test]
fn active_protection_intercepts_a_top_attack_missile_in_its_dive() {
    // Top attack beats thin roof armour, not active protection: a Trophy
    // hull meets a missile diving onto it as it meets a flat one, at its
    // standoff above the roof.
    use contract::ids::Side;
    use serde_json::json;
    use sim::battle::Battle;
    let mut rules = sim::fixtures::test_game();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_tank",
        json!({"capabilities":{"active_protection":capability()}}),
    );
    let dive_deg = 40.0;
    rules["weapons"]["atgm"]["turn_deg_s"] = json!(360);
    rules["weapons"]["atgm"]["top_attack"] = json!({ "loft_m": 60, "dive_deg": dive_deg });
    let setup = serde_json::from_value(json!({"rules":rules,"map":{"size":[1200,600],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35},"units":[
        {"side":"blue","kind":"test_at","position":[40,300]},
        {"side":"red","kind":"test_tank","position":[600,300],"engagement":"return_fire_only"}
    ]}))
    .unwrap();
    let mut b = Battle::new(&setup, 7);
    let mut path: Vec<[f64; 3]> = Vec::new();
    let mut burst = None;
    for _ in 0..900 {
        b.step();
        let view = b.observe(Side::Blue);
        match view.guided.first() {
            Some(m) => path.push(m.position),
            None if !path.is_empty() => {
                burst = view.blasts.first().map(|x| x.point);
                break;
            }
            None => {}
        }
    }
    let burst = burst.expect("the missile burst");
    let tank = b.unit(UnitId(1)).unwrap();
    assert_eq!(tank.protection.unwrap().charges, capability().capacity - 1);
    assert_eq!(
        tank.hp,
        crate::common::hull("test_tank").hp,
        "intercepted short of the hull"
    );
    let [.., a, c] = path[..] else {
        panic!("it flew")
    };
    let (run, drop) = ((c[0] - a[0]).hypot(c[1] - a[1]), a[2] - c[2]);
    assert!(
        drop.atan2(run).to_degrees() >= dive_deg,
        "diving when met: {a:?} → {c:?}"
    );
    let roof = 2.0 * crate::common::hull("test_tank").half_extents_m[2];
    assert!(burst[2] > roof + 1.0, "met above the roof: {burst:?}");
}

#[test]
fn ordinary_tank_shell_metadata_bypasses_active_protection() {
    let rules = sim::fixtures::test_game();
    for id in [
        "rifle",
        "tank_ap",
        "tank_he",
        "advanced_tank_ap",
        "advanced_tank_he",
    ] {
        assert!(
            !rules["weapons"][id]["interceptable"]
                .as_bool()
                .unwrap_or(false),
            "{id} is ordinary fire"
        );
    }
    for id in [
        "atgm",
        "tow",
        "kornet",
        "spike",
        "bastion",
        "rpg_light",
        "rpg_heavy",
    ] {
        assert!(
            rules["weapons"][id]["interceptable"]
                .as_bool()
                .unwrap_or(false),
            "{id} is an eligible threat"
        );
    }
}
