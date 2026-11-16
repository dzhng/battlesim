//! Growth shares resident clip data, while its poses match a cold catalog.
use contract::identity::{bytes_hash, ContentId};
use presentation::presentation::{lab::Asset, Presentation, Ui};
use presentation::view::Camera;
use presentation::world::{Appearance, BattleInput, SoldierBody, WorldInput, Xray};
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::Arc;

use presentation::units::drawn::ClipTables;
type Skin = (
    Arc<Appearance>,
    Arc<Vec<f32>>,
    Arc<Vec<presentation::units::skin::Mat4>>,
);

fn content(name: &str) -> ContentId {
    ContentId::bundle(bytes_hash(name.as_bytes())).unwrap()
}

#[test]
fn growing_skinned_catalog_shares_retained_tables_and_equals_cold_staging() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let rules = serde_json::from_value(sim::fixtures::test_rules()).unwrap();
    let fielded = presentation::fielded::fielded(&root, &rules, &["test_rifle".into()]).unwrap();
    let (record, samples) = fielded
        .units
        .appearances
        .values()
        .find_map(|a| {
            let Appearance::Skinned { table, .. } = a.as_ref() else {
                return None;
            };
            Some((a.as_ref().clone(), fielded.tables[table].as_ref().clone()))
        })
        .unwrap();
    // Distinct tables catch accidental sharing across unequal content. Each
    // table still plays the fixture's real clips and rig, including rest poses.
    let skins: BTreeMap<String, Skin> = ["a", "x", "y"]
        .into_iter()
        .enumerate()
        .map(|(i, name)| {
            let mut samples = samples.clone();
            for sample in samples.as_chunks_mut::<8>().0 {
                sample[5] += i as f32 * 0.01;
            }
            let Asset::Animated {
                appearance,
                table,
                inverse_binds,
            } = Asset::animated(record.clone(), samples)
            else {
                unreachable!()
            };
            (name.into(), (appearance, table, inverse_binds))
        })
        .collect();
    let input = |names: &[&str]| {
        let mut units = fielded.units.clone();
        units.types.clear();
        units.appearances = names
            .iter()
            .map(|&n| (n.into(), skins[n].0.clone()))
            .collect();
        WorldInput {
            battle: Some(BattleInput {
                units,
                effects: fielded.effects.clone(),
                side: "blue".into(),
                tick_hz: 30.0,
                soldier: SoldierBody {
                    radius_m: 0.2,
                    height_m: 1.8,
                },
                xray: Xray {
                    own: [0.0; 4],
                    selected: [0.0; 4],
                    ally: [0.0; 4],
                },
                blocked: [0.0; 4],
                stand_in: None,
            }),
            sun_azimuth: 0.0,
            sun_elevation: 0.7,
            buildings: None,
        }
    };
    let tables = |names: &[&str]| -> ClipTables {
        names
            .iter()
            .map(|&n| {
                let Appearance::Skinned { table, .. } = skins[n].0.as_ref() else {
                    unreachable!()
                };
                (table.clone(), skins[n].1.clone())
            })
            .collect()
    };
    let wanted = |names: &[&str]| names.iter().map(|&n| (n.into(), content(n))).collect();
    let assets = |names: &[&str]| {
        names
            .iter()
            .map(|&n| {
                (
                    content(n),
                    Asset::Animated {
                        appearance: skins[n].0.clone(),
                        table: skins[n].1.clone(),
                        inverse_binds: skins[n].2.clone(),
                    },
                )
            })
            .collect()
    };
    let detail = fielded.units.detail;
    let mut grown = Presentation::new(None, 30.0).unwrap();
    grown
        .load_world(input(&["x", "y"]), tables(&["x", "y"]), None)
        .unwrap();
    let initial: Vec<_> = ["x", "y"]
        .into_iter()
        .map(|n| (n, Arc::strong_count(&skins[n].1)))
        .collect();
    let first = grown
        .stage_catalog(
            wanted(&["x", "y"]),
            assets(&["x", "y"]),
            detail,
            Default::default(),
        )
        .unwrap();
    assert!(grown.commit_catalog(first, Vec::new()).unwrap());
    for (name, count) in initial {
        assert!(
            Arc::strong_count(&skins[name].1) > count,
            "{name}: the first catalog must hold its asset's clip allocation"
        );
    }
    assert!(grown
        .stage_battle(input(&["a", "x", "y"]), tables(&["a", "x", "y"]))
        .unwrap());
    let held: Vec<_> = ["x", "y"]
        .into_iter()
        .map(|n| (n, Arc::strong_count(&skins[n].1)))
        .collect();
    let delta = grown
        .stage_catalog(
            wanted(&["a", "x", "y"]),
            assets(&["a"]),
            detail,
            Default::default(),
        )
        .unwrap();
    for (name, count) in held {
        assert!(Arc::strong_count(&skins[name].1) > count,
            "{name}: the staged catalog must hold the previous catalog's clip allocation, not copy it");
    }
    let mut cold = Presentation::new(None, 30.0).unwrap();
    cold.load_world(input(&["a", "x", "y"]), tables(&["a", "x", "y"]), None)
        .unwrap();
    let full = cold
        .stage_catalog(
            wanted(&["a", "x", "y"]),
            assets(&["a", "x", "y"]),
            detail,
            Default::default(),
        )
        .unwrap();
    let (delta_view, full_view) = (delta.view(&grown).unwrap(), full.view(&cold).unwrap());
    assert_eq!(delta_view.world(), full_view.world());
    for row in 0..delta_view.world().appearances.len() as u32 {
        let expected = full_view
            .rest_palette(row)
            .expect("each skinned row has a rest palette");
        assert_eq!(
            delta_view.rest_palette(row).as_deref(),
            Some(expected.as_slice())
        );
    }
    let models = || {
        serde_json::from_value(
            json!(["a", "x", "y"].into_iter().enumerate().map(|(i, n)| json!({
        "appearance":n,"x":i as f64 * 3.0,"y":0,"z":0,"yaw":0,"tier":0,"pose":{"kind":"rest"}
    })).collect::<Vec<_>>()),
        )
        .unwrap()
    };
    assert!(grown.commit_catalog(delta, models()).unwrap());
    assert!(cold.commit_catalog(full, models()).unwrap());
    let camera = Camera {
        eye: [0.0, -20.0, 15.0],
        target: [0.0; 3],
        up: [0.0, 0.0, 1.0],
        fov_y: 1.0,
        near: 0.1,
        size: [640, 480],
    };
    let (a, b) = (
        grown.frame(0.0, &camera, &Ui::default(), 0).unwrap(),
        cold.frame(0.0, &camera, &Ui::default(), 0).unwrap(),
    );
    assert_eq!(a.batches, b.batches);
    assert_eq!(a.instances, b.instances);
    assert_eq!(a.palettes, b.palettes);
    assert!(
        !a.instances.is_empty(),
        "the shared tables produce drawn bodies"
    );
    // A native/tooling caller can replace fielded facts without supplying a lab
    // asset. Sharing retained inverses must not keep the replaced rig's binds.
    let mut replacement = input(&["a", "x", "y"]);
    let x = replacement
        .battle
        .as_mut()
        .unwrap()
        .units
        .appearances
        .get_mut("x")
        .unwrap();
    let Appearance::Skinned {
        joints,
        inverse_binds,
        ..
    } = Arc::make_mut(x)
    else {
        unreachable!()
    };
    *inverse_binds =
        vec![
            [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0];
            joints.len()
        ];
    inverse_binds[0][12] = 4.0;
    let mut replaced_cold = Presentation::new(None, 30.0).unwrap();
    replaced_cold
        .load_world(replacement.clone(), tables(&["a", "x", "y"]), None)
        .unwrap();
    assert!(grown
        .stage_battle(replacement, tables(&["a", "x", "y"]))
        .unwrap());
    let replacement = grown
        .stage_catalog(
            Default::default(),
            Default::default(),
            detail,
            Default::default(),
        )
        .unwrap();
    assert!(grown.commit_catalog(replacement, Vec::new()).unwrap());
    assert_eq!(grown.world(), replaced_cold.world());
    for row in 0..grown.world().appearances.len() as u32 {
        assert_eq!(
            grown.catalog().rest_palette(row),
            replaced_cold.catalog().rest_palette(row),
            "replaced fielding facts use their own inverse binds"
        );
    }
    // Different appearance contents can contain the same clips. Their asset
    // facts must release the duplicate payload too, not just collapse map keys.
    let (left, right) = (
        Asset::animated(record.clone(), samples.clone()),
        Asset::animated(record, samples),
    );
    let Asset::Animated { table, .. } = &right else {
        unreachable!()
    };
    let duplicate = Arc::downgrade(table);
    let mut lab = Presentation::new(None, 30.0).unwrap();
    lab.install_lab_assets(
        [
            ("left".into(), (content("left"), left)),
            ("right".into(), (content("right"), right)),
        ]
        .into(),
        detail,
    );
    assert!(
        duplicate.upgrade().is_none(),
        "equal clip content has one resident allocation across asset facts"
    );
    let rest = |name| {
        let row = lab
            .world()
            .appearances
            .iter()
            .position(|r| lab.world().name(r) == name && r.lies == presentation::records::NONE)
            .unwrap();
        lab.catalog()
            .rest_palette(row as u32)
            .expect("the admitted name keeps its authored rest palette")
    };
    assert_eq!(rest("left"), rest("right"));
}
