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
