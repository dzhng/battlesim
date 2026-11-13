//! Fog inputs and side knowledge through the canonical presentation owner.
use contract::ids::Side;
use presentation::presentation::{FogSetup, Presentation};
use presentation::static_world::files;
use sim::battle::Battle;
use sim::publication::{self, Publisher};
use std::path::Path;

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

#[test]
fn fog_reads_the_simulations_height_grid_and_foliage() {
    let root = Path::new(ROOT);
    let rules = files::rules(root, "sensors").unwrap();
    let ground = files::ground(root, "sensors", &rules).unwrap();
    let (nx, ny, spacing) = (
        ground.source.grid.nx,
        ground.source.grid.ny,
        ground.source.grid.spacing,
    );
    let foliage = ground.source.foliage[3..].to_vec();
    assert!(foliage.as_chunks::<4>().0.iter().any(|cell| cell[2] > 0.0));
    let mut sensors = ground.sensors;
    let mut p = Presentation::new(None, 30.0).unwrap();
    p.set_ground(ground.source, ground.biome, ground.tuning)
        .unwrap();
    p.set_fog(Some(FogSetup {
        sensors: Some(sensors),
        ..Default::default()
    }))
    .unwrap();
    let upload = p.prepare_fog(None, 64, 64, 0).unwrap().unwrap();
    assert_eq!(upload.params.height_nx as usize, nx);
    assert_eq!(upload.params.height_ny as usize, ny);
    assert_eq!(f64::from(upload.params.height_spacing), spacing);
    assert_eq!(upload.params.foliage_cell_m, 8.0);
    assert_eq!(upload.foliage.unwrap(), foliage);
    sensors.foliage_full_block = 2.0;
    let error = p
        .set_fog(Some(FogSetup {
            sensors: Some(sensors),
            ..Default::default()
        }))
        .unwrap_err();
    assert!(error.contains("foliage_full_block"), "{error}");
}

fn blue_fog(
    wall: Option<[f64; 2]>,
) -> (
    Vec<presentation::fog::sight::Eye>,
    Vec<presentation::fog::sight::Occluder>,
) {
    let root = Path::new(ROOT);
    let rules = files::rules(root, "sensors").unwrap();
    let ground = files::ground(root, "sensors", &rules).unwrap();
    let mut scenario = sim::maps::scenario("sensors", "sensors", rules).unwrap();
    scenario.units = serde_json::from_value(serde_json::json!([
        {"side":"blue","kind":"test_recon","position":[300,120]},
        {"side":"blue","kind":"test_tank","position":[300,70]},
        {"side":"red","kind":"test_rifle","position":[1100,110],"yaw":std::f64::consts::PI}
    ]))
    .unwrap();
    scenario.scripts.clear();
    scenario.events = wall.map_or(vec![], |center| {
        serde_json::from_value(serde_json::json!([
            {"tick":5,"add_prop":{"kind":"wall","center":center,"yaw":0,"half_extents":[0.5,6,3]}}
        ]))
        .unwrap()
    });
    let mut battle = Battle::new(&scenario, 3);
    for _ in 0..40 {
        battle.step();
    }
    let mut p = Presentation::new(Some(&publication::layout_json(&battle)), 30.0).unwrap();
    p.set_ground(ground.source, ground.biome, ground.tuning)
        .unwrap();
    let mut publisher = Publisher::new();
    let words: Vec<u32> = publisher
        .publish(&battle, Side::Blue)
        .unwrap()
        .iter()
        .map(|f| f.to_bits())
        .collect();
    p.push_publication(&words, 0.0).unwrap();
    p.set_fog(Some(FogSetup {
        sensors: Some(ground.sensors),
        ..Default::default()
    }))
    .unwrap();
    p.prepare_fog(None, 64, 64, 0).unwrap();
    (p.fog_eyes(), p.fog_occluders())
}

#[test]
fn hidden_props_never_reach_fog_but_learned_props_do() {
    let (eyes, base) = blue_fog(None);
    assert_eq!(eyes.len(), 2);
    let (hidden_eyes, hidden) = blue_fog(Some([840.0, 470.0]));
    assert_eq!(
        serde_json::to_value(&hidden_eyes).unwrap(),
        serde_json::to_value(&eyes).unwrap()
    );
    assert_eq!(hidden, base);
    let (seen_eyes, seen) = blue_fog(Some([420.0, 110.0]));
    assert_eq!(
        serde_json::to_value(&seen_eyes).unwrap(),
        serde_json::to_value(&eyes).unwrap()
    );
    assert_eq!(seen.len(), base.len() + 1);
    let wall = seen.last().unwrap();
    assert_eq!([wall.x, wall.y, wall.hx, wall.hy], [420.0, 110.0, 0.5, 6.0]);
}

#[test]
fn building_fog_keeps_each_sides_known_height_after_shelling() {
    let root = Path::new(ROOT);
    let rules = files::rules(root, "camera-lab").unwrap();
    let mut battle = Battle::new(
        &sim::maps::scenario("camera-lab", "shelling", rules).unwrap(),
        7,
    );
    for _ in 0..600 {
        battle.step();
    }
    let mut by_side = vec![];
    for side in [Side::Blue, Side::Red] {
        let ground = files::ground(root, "camera-lab", battle.rules()).unwrap();
        let mut p = Presentation::new(Some(&publication::layout_json(&battle)), 30.0).unwrap();
        p.set_ground(ground.source, ground.biome, ground.tuning)
            .unwrap();
        p.set_fog(Some(FogSetup {
            sensors: Some(ground.sensors),
            eyes: Some(vec![]),
            ..Default::default()
        }))
        .unwrap();
        p.prepare_fog(None, 64, 64, 0).unwrap();
        let authored = p.fog_occluders();
        let mut publisher = Publisher::new();
        let words: Vec<u32> = publisher
            .publish(&battle, side)
            .unwrap()
            .iter()
            .map(|f| f.to_bits())
            .collect();
        p.push_publication(&words, 0.0).unwrap();
        p.set_fog(Some(FogSetup {
            sensors: Some(ground.sensors),
            ..Default::default()
        }))
        .unwrap();
        p.prepare_fog(None, 64, 64, 0).unwrap();
        by_side.push((authored, p.fog_occluders()));
    }
    let (authored, blue) = &by_side[0];
    let (_, red) = &by_side[1];
    assert_eq!(red, authored, "red still knows the intact map");
    let changed: Vec<_> = authored
        .iter()
        .map(|a| {
            (
                a,
                blue.iter()
                    .find(|b| a.x == b.x && a.y == b.y)
                    .expect("every authored box remains"),
            )
        })
        .filter(|(a, b)| a.top != b.top)
        .collect();
    assert_eq!(changed.len(), 3, "all three compound parts become ruins");
    for (a, b) in changed {
        assert!(b.top < a.top && b.top > b.base);
    }
    let tower = authored
        .iter()
        .max_by(|a, b| a.top.total_cmp(&b.top))
        .unwrap();
    let drawn = blue
        .iter()
        .find(|b| b.x == tower.x && b.y == tower.y)
        .unwrap();
    assert_eq!(
        drawn.top - drawn.base,
        tower.top - tower.base,
        "the gutted tower keeps its height"
    );
}

/// The keys of the eyes a prepared fog frame rebuilt.
fn rebuilt(p: &mut Presentation) -> (presentation::fog::sight::FogUpload, Vec<String>) {
    let up = p.prepare_fog(None, 64, 64, 0).unwrap().unwrap();
    let eyes = p.fog_eyes();
    let keys = (0..up.params.rebuild_count as usize)
        .map(|r| eyes[up.rebuild[r * 3] as usize].key.clone())
        .collect();
    (up, keys)
}

#[test]
fn a_published_cloud_closes_the_fog_at_its_step_and_rebuilds_only_the_eyes_it_reaches() {
    let root = Path::new(ROOT);
    let rules = files::rules(root, "sensors").unwrap();
    let ground = files::ground(root, "sensors", &rules).unwrap();
    let mut scenario = sim::maps::scenario("sensors", "sensors", rules).unwrap();
    // A smoke tank, and a tank far past any reach of its cloud.
    scenario.units = serde_json::from_value(serde_json::json!([
        {"side":"blue","kind":"test_smoke_tank","position":[300,120],"engagement":"return_fire_only"},
        {"side":"blue","kind":"test_tank","position":[1100,400],"engagement":"return_fire_only"},
        {"side":"red","kind":"test_rifle","position":[1300,500],"yaw":std::f64::consts::PI,"engagement":"return_fire_only"}
    ]))
    .unwrap();
    scenario.scripts.clear();
    scenario.events.clear();
    let mut battle = Battle::new(&scenario, 3);
    for _ in 0..5 {
        battle.step();
    }
    let tick_hz = battle.rules().tick_hz;
    let mut p =
        Presentation::new(Some(&publication::layout_json(&battle)), f64::from(tick_hz)).unwrap();
    p.set_ground(ground.source, ground.biome, ground.tuning)
        .unwrap();
    p.set_fog(Some(FogSetup {
        sensors: Some(ground.sensors),
        ..Default::default()
    }))
    .unwrap();
    let mut publisher = Publisher::new();
    let mut publish = |battle: &Battle, p: &mut Presentation| {
        let words: Vec<u32> = publisher
            .publish(battle, Side::Blue)
            .unwrap()
            .iter()
            .map(|f| f.to_bits())
            .collect();
        assert!(p.push_publication(&words, 0.0).unwrap());
    };
    publish(&battle, &mut p);
    let (first, _) = rebuilt(&mut p);
    assert_eq!(first.params.smoke_count, 0);
    let ack = battle.accept(contract::command::CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 1,
        order: contract::command::Order::PopSmoke {
            units: vec![contract::ids::UnitId(0)],
        },
        queued: false,
    });
    assert_eq!(ack.error, None);
    let near = |keys: &[String]| keys.iter().all(|k| k.starts_with("0:")) && !keys.is_empty();
    let mut changes = 0;
    let mut last = None;
    loop {
        battle.step();
        let cloud = battle
            .world()
            .smoke()
            .live()
            .next()
            .map(|(c, _)| c.cloud.clone());
        publish(&battle, &mut p);
        let (up, keys) = rebuilt(&mut p);
        let state = cloud
            .as_ref()
            .and_then(|c| contract::smoke::state(c, battle.tick(), tick_hz));
        assert_eq!(up.params.smoke_count, u32::from(state.is_some()));
        if let Some(sent) = &up.smoke {
            match &state {
                Some(s) => {
                    assert_eq!(sent.len(), 1);
                    assert_eq!(sent[0].center, s.centre.map(|v| v as f32));
                    assert_eq!(sent[0].radius, s.radius_m as f32);
                    assert_eq!(sent[0].depth_per_m, s.depth_per_m as f32);
                }
                None => assert!(sent.is_empty()),
            }
        }
        let phase = state.as_ref().map(|s| s.phase);
        if phase != last {
            changes += 1;
            assert!(up.smoke.is_some(), "a new step is sent at once");
            assert!(near(&keys), "a step rebuilds the eyes it reaches: {keys:?}");
        } else {
            assert!(up.smoke.is_none());
            assert!(
                keys.is_empty(),
                "nothing changed, nothing rebuilt: {keys:?}"
            );
        }
        last = phase;
        if state.is_none() {
            break;
        }
    }
    let smoke = scenario
        .rules
        .catalog
        .by_id("test_smoke_tank")
        .capabilities
        .smoke
        .clone();
    let cloud = smoke.expect("the smoke tank carries smoke").cloud;
    // Every bloom step, the full cloud, every fade step, and gone.
    assert_eq!(
        changes,
        usize::from(cloud.bloom_steps) + 1 + usize::from(cloud.fade_steps) + 1
    );
}
