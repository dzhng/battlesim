//! Which target a mount picks, whether its rounds can reach it, and when a
//! seeker may acquire one.

use super::choice::{return_fire_threat, select};
use super::*;
use contract::command::Engagement;
use serde_json::json;

#[test]
fn seeker_acquisition_wraps_at_north_and_honours_forward_cone() {
    let north = std::f64::consts::FRAC_PI_2;
    assert!(seeker_in_fov(north, north + 0.5_f64.to_radians(), 70.0));
    assert!(seeker_in_fov(north, north - 34.0_f64.to_radians(), 70.0));
    assert!(!seeker_in_fov(north, north + 36.0_f64.to_radians(), 70.0));
    assert!(seeker_in_fov(0.0, std::f64::consts::TAU - 0.1, 70.0));
}

#[test]
fn a_claimed_lean_preserves_the_launchers_full_bore_offset() {
    let setup: contract::scenario::ScenarioDefinition = serde_json::from_value(json!({
        "map":{"size":[500,500],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,
            "props":[{"kind":"wall","center":[101,100],"yaw":0,"half_extents":[0.3,2,3]}]},
        "rules":crate::fixtures::test_rules(),"units":[{"side":"blue","kind":"test_at","position":[100,100]}],
        "events":[],"scripts":[]
    })).unwrap();
    let battle = crate::battle::Battle::new(&setup, 5);
    let knowledge = [
        SideKnowledge::new(1, battle.ground()),
        SideKnowledge::new(2, battle.ground()),
        SideKnowledge::new(3, battle.ground()),
    ];
    let ctx = FireContext {
        world: battle.world(),
        windows: battle.windows(),
        structures: battle.structures(),
        ground: battle.ground(),
        arsenal: battle.arsenal(),
        rules: &setup.rules,
        tick: 0,
        seed: 5,
        knowledge: &knowledge,
        rounds: &[],
    };
    let mut soldier = battle.unit(UnitId(0)).unwrap().members[0].clone();
    soldier.position = v3(100.0, 100.0, 0.0);
    soldier.lean = Some(lean::Lean {
        from: v2(100.0, 100.0),
        at: v2(100.0, 103.0),
        side: lean::LeanSide::Left,
        body: lean::Round::Prop(0),
    });
    let from = fire_from(
        &ctx,
        &[],
        &soldier,
        v3(300.0, 100.0, 1.0),
        v3(0.8, -0.1, 0.75),
    )
    .expect("lean clears the wall");
    assert!(from.leaning, "the standing bore is behind the wall");
    assert!(
        (from.origin - v3(100.8, 102.9, 0.75)).length() < 1e-9,
        "{:?}",
        from.origin
    );
}

#[test]
fn known_infantry_bore_turns_toward_us_from_the_observed_center() {
    let mut game = crate::fixtures::test_rules();
    game["weapons"]["atgm"]["min_range_m"] = json!(200.0);
    game["catalog"].as_array_mut().unwrap().push(json!({
        "soldiers":{"grounded":{"extends":"test_atgm_gunner", "mounts":[{
            "id":"ATGM launcher","pivot_m":[20,0,0.75],"muzzle_m":[0,0,0]
        }]}},
        "units":{"grounded":{"extends":"test_at","body":{"squad":{"slots":["grounded"]}}}}
    }));
    let setup: contract::scenario::ScenarioDefinition = serde_json::from_value(json!({
        "map":{"size":[1000,600],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35},
        "rules":game,"units":[{"side":"blue","kind":"test_tank","position":[100,300]},
            {"side":"red","kind":"grounded","position":[800,300]}],"events":[],"scripts":[]
    }))
    .unwrap();
    let battle = crate::battle::Battle::new(&setup, 5);
    let knowledge = [
        SideKnowledge::new(1, battle.ground()),
        SideKnowledge::new(2, battle.ground()),
        SideKnowledge::new(3, battle.ground()),
    ];
    let ctx = FireContext {
        world: battle.world(),
        windows: battle.windows(),
        structures: battle.structures(),
        ground: battle.ground(),
        arsenal: battle.arsenal(),
        rules: &setup.rules,
        tick: 0,
        seed: 5,
        knowledge: &knowledge,
        rounds: &[],
    };
    let track = crate::knowledge::Track {
        id: contract::observation::ObservedTargetId(7),
        last_seen: 0,
        position: v3(300.0, 300.0, 0.0),
        yaw: 0.0,
        velocity: v2(0.0, 0.0),
        members: vec![],
        layer: contract::catalog::AltitudeLayer::Ground,
        class: contract::weapons::TargetClass::Vehicle,
    };
    // The declared bore points west: 180m to us is inside its minimum.
    // Using its observed carrier yaw would point east (220m), and using
    // the hidden physical squad at800m would also misclassify the threat.
    assert!(!return_fire_threat(
        &ctx,
        battle.unit(UnitId(0)).unwrap(),
        battle.unit(UnitId(1)).unwrap().kind,
        &track
    ));
}

#[test]
fn known_return_fire_threat_requires_a_clear_physical_trajectory() {
    for blocked in [false, true] {
        let props = if blocked {
            json!([{"kind":"wall","center":[200,300],"yaw":0,
            "half_extents":[1,50,5]}])
        } else {
            json!([])
        };
        let setup: contract::scenario::ScenarioDefinition = serde_json::from_value(json!({
            "map":{"size":[1000,600],"fog_cell_m":8,"height_grid_m":4,
                "slope_cutoff_deg":35,"props":props},"rules":crate::fixtures::test_rules(),
            "units":[{"side":"blue","kind":"test_tank","position":[100,300]},
                {"side":"red","kind":"test_tank","position":[300,300]}],"events":[],"scripts":[]
        }))
        .unwrap();
        let battle = crate::battle::Battle::new(&setup, 5);
        let knowledge = [
            SideKnowledge::new(1, battle.ground()),
            SideKnowledge::new(2, battle.ground()),
            SideKnowledge::new(3, battle.ground()),
        ];
        let ctx = FireContext {
            world: battle.world(),
            windows: battle.windows(),
            structures: battle.structures(),
            ground: battle.ground(),
            arsenal: battle.arsenal(),
            rules: &setup.rules,
            tick: 0,
            seed: 5,
            knowledge: &knowledge,
            rounds: &[],
        };
        let enemy = battle.unit(UnitId(1)).unwrap();
        let track = crate::knowledge::Track {
            id: contract::observation::ObservedTargetId(7),
            last_seen: 0,
            position: enemy.position,
            yaw: enemy.yaw,
            velocity: v2(0.0, 0.0),
            members: vec![],
            layer: contract::catalog::AltitudeLayer::Ground,
            class: contract::weapons::TargetClass::Vehicle,
        };
        assert_eq!(
            return_fire_threat(&ctx, battle.unit(UnitId(0)).unwrap(), enemy.kind, &track),
            !blocked
        );
    }
}

#[test]
fn frozen_observation_keeps_priority_with_hidden_ammo_reload_orders_and_pose_changes() {
    let mut authored = crate::fixtures::test_rules();
    crate::fixtures::patch_catalog(&mut authored, "units", "test_rifle", json!({"cost":1000}));
    crate::fixtures::patch_catalog(&mut authored, "units", "test_tank", json!({"cost":100}));
    let setup: contract::scenario::ScenarioDefinition = serde_json::from_value(json!({
        "map":{"size":[1000,600],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35},
        "rules":authored,"units":[
            {"side":"blue","kind":"test_tank","position":[100,300]},
            {"side":"red","kind":"test_rifle","position":[300,260]},
            {"side":"red","kind":"test_tank","position":[300,340]}],
        "events":[],"scripts":[]
    }))
    .unwrap();
    let battle = crate::battle::Battle::new(&setup, 5);
    let mut units: Vec<_> = (0..3)
        .map(|id| battle.unit(UnitId(id)).unwrap().clone())
        .collect();
    let mut knowledge = [
        SideKnowledge::new(1, battle.ground()),
        SideKnowledge::new(2, battle.ground()),
        SideKnowledge::new(3, battle.ground()),
    ];
    let sightings: Vec<_> = (1..3)
        .map(|id| crate::sensing::Sighting {
            observer: UnitId(0),
            target: UnitId(id),
            members: (0..units[id as usize].members.len()).collect(),
        })
        .collect();
    knowledge[0].update(0, &sightings, &units, &setup.rules);
    let ctx = FireContext {
        world: battle.world(),
        windows: battle.windows(),
        structures: battle.structures(),
        ground: battle.ground(),
        arsenal: battle.arsenal(),
        rules: &setup.rules,
        tick: 0,
        seed: 5,
        knowledge: &knowledge,
        rounds: &[],
    };
    let choose = |units: &[Unit]| {
        select(
            &ctx,
            &units[0],
            units,
            &units[0].mounts[0],
            &ctx.arsenal.specs(units[0].kind)[0],
            &mut Assessed::default(),
        )
        .0
    };
    assert_eq!(choose(&units), Some(Target::Unit(UnitId(2))));
    let enemy = &mut units[2];
    enemy.position = v3(950.0, 550.0, 0.0);
    enemy.yaw = 2.0;
    enemy.engagement = Engagement::ReturnFireOnly;
    enemy.orders.push_back(crate::units::UnitOrder::Exit);
    for mount in &mut enemy.mounts {
        mount.ammo.fill(Some(0));
        mount.bearing = 2.0;
        for cycle in &mut mount.cycles {
            cycle.loaded = None;
            cycle.reload = Some((0, 0.0));
        }
    }
    assert_eq!(choose(&units), Some(Target::Unit(UnitId(2))));
}

#[test]
fn reach_is_judged_where_the_round_meets_its_target_before_the_line_of_fire() {
    // A tank in the open, a tall wall 100 m east of it, and a cannon whose
    // reach the test sets.
    let reach = 300.0;
    let mut game = crate::fixtures::test_rules();
    game["weapons"]["tank_ap"]["targets"]["ground"]["range_m"] = json!(reach);
    let setup: contract::scenario::ScenarioDefinition = serde_json::from_value(json!({
        "map":{"size":[1000,600],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,
            "props":[{"kind":"wall","center":[200,300],"yaw":0,"half_extents":[1,50,5]}]},
        "rules":game,"units":[{"side":"blue","kind":"test_tank","position":[100,300]}],
        "events":[],"scripts":[]
    }))
    .unwrap();
    let battle = crate::battle::Battle::new(&setup, 5);
    let knowledge = [
        SideKnowledge::new(1, battle.ground()),
        SideKnowledge::new(2, battle.ground()),
        SideKnowledge::new(3, battle.ground()),
    ];
    let ctx = FireContext {
        world: battle.world(),
        windows: battle.windows(),
        structures: battle.structures(),
        ground: battle.ground(),
        arsenal: battle.arsenal(),
        rules: &setup.rules,
        tick: 0,
        seed: 5,
        knowledge: &knowledge,
        rounds: &[],
    };
    let units = [battle.unit(UnitId(0)).unwrap().clone()];
    let tank = &units[0];
    let spec = &ctx.arsenal.specs(tank.kind)[0];
    let k = (0..spec.kinds.len())
        .find(|&k| ctx.arsenal.weapons[spec.kinds[k]].id == "tank_ap")
        .unwrap();
    let speed = ctx.arsenal.weapons[spec.kinds[k]].profile.speed_mps;
    let shoot = |x: f64, y: f64, velocity: V3| {
        let point = v3(x, y, 1.0);
        engage(
            &ctx,
            tank,
            &units,
            &tank.mounts[0],
            spec,
            k,
            Target::Ground(point),
            &Resolved {
                point,
                velocity,
                current: true,
                armor: Some(None),
                layer: AltitudeLayer::Ground,
                class: Some(TargetClass::Vehicle),
                emits: false,
            },
        )
    };
    let still = v3(0.0, 0.0, 0.0);
    // Behind the wall: within reach the wall holds the gun; past reach it is
    // out of range, whatever stands in the way.
    assert_eq!(
        shoot(300.0, 300.0, still).err(),
        Some(ActionReason::BlockedTrajectory)
    );
    assert_eq!(
        shoot(100.0 + reach + 100.0, 300.0, still).err(),
        Some(ActionReason::OutOfRange)
    );
    // Clear of the wall: a target within reach is engaged.
    assert!(shoot(300.0, 480.0, still).is_ok());
    // Clear of the wall and past reach now, but closing fast enough to be met
    // inside it: the shot meets it where the reach is judged, so it is engaged.
    let toward = v3(-1.0, -1.0, 0.0).normalized();
    let start = v3(100.0, 300.0, 0.0) - toward * (reach + 40.0);
    let closing = shoot(start.x, start.y, toward * (speed * 0.5)).expect("met inside reach");
    assert!((closing.intercept - tank.position).length() <= reach);
}
