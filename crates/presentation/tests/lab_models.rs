//! Tooling models use the same frame records as battle and static populations.
use presentation::presentation::{lab::Asset, Presentation, Ui};
use presentation::records::{FOG_FACES, FOG_PAINTED_FACES, GHOST_PASS, NONE};
use presentation::view::Camera;
use presentation::world::Detail;
use serde_json::json;

fn static_asset() -> Asset {
    let head = serde_json::to_vec(&json!({
        "content": {"kind":"prop", "states":[{
            "name":"default", "bounds":{"min":[-1,-1,0], "max":[1,1,2]},
            "tiers":[{}, {}, {}, {}]
        }]}, "views":[]
    }))
    .unwrap();
    let mut bytes = Vec::new();
    for word in [0x5441_4742u32, 4, head.len() as u32, 0] {
        bytes.extend(word.to_le_bytes());
    }
    bytes.extend(head);
    Asset::from_bundle(&bytes, None).unwrap()
}

fn camera() -> Camera {
    Camera {
        eye: [0.0, -20.0, 15.0],
        target: [0.0, 0.0, 0.0],
        up: [0.0, 0.0, 1.0],
        fov_y: 1.0,
        near: 0.1,
        size: [640, 480],
    }
}

#[test]
fn tooling_detail_uses_its_admitted_thresholds_when_gameplay_quality_changes() {
    let mut p = Presentation::new(None, 30.0).unwrap();
    p.install_lab_assets(
        [("study".into(), static_asset())].into(),
        Detail {
            lod_px: [80.0, 40.0, 20.0],
            impostor_px: 0.0,
            shadow_caster_px: 0.0,
        },
    );
    p.set_lab_models(
        serde_json::from_value(json!([{
            "appearance":"study", "x":0, "y":0, "z":0, "yaw":0,
            "pose":{"kind":"static", "state":"default"}
        }]))
        .unwrap(),
    )
    .unwrap();
    let tier = p.frame(0.0, &camera(), &Ui::default(), 0).unwrap().batches[0].tier;
    p.set_detail_scale(8.0, 8.0);
    let changed = p.frame(0.0, &camera(), &Ui::default(), 0).unwrap();
    assert_eq!(
        changed.batches[0].tier, tier,
        "tooling owns explicit detail thresholds independently of gameplay quality"
    );
}

#[test]
fn lab_models_compose_named_states_motion_and_ghosts_in_the_canonical_frame() {
    let mut p = Presentation::new(None, 30.0).unwrap();
    p.install_lab_assets(
        [("tool-model".into(), static_asset())].into(),
        Detail {
            lod_px: [80.0, 40.0, 20.0],
            impostor_px: 4.0,
            shadow_caster_px: 0.0,
        },
    );
    p.set_lab_models(
        serde_json::from_value(json!([
            {"appearance":"tool-model","x":1,"y":2,"z":3,"yaw":0.5,
             "scale":[2,3,4],"tint":[0.2,0.4,0.6],"tier":0,
             "pose":{"kind":"static","state":"default",
               "motion":[1,0,0,0,0,1,0,0,0,0,1,0,4,5,6,1]},
             "xray":[1,0,0,0.9]},
            {"appearance":"tool-model","x":2,"y":2,"z":3,"yaw":0,
             "tier":0,"pose":{"kind":"static","state":"default","ground":true}},
            {"appearance":"tool-model","x":3,"y":2,"z":3,"yaw":0,
             "tier":4,"pose":{"kind":"static","state":"default"},
             "ghost":[0.2,0.4,0.6,0.5],"xray":[1,0,0,1]}
        ]))
        .unwrap(),
    )
    .unwrap();
    let f = p.frame(7.0, &camera(), &Ui::default(), 0).unwrap();
    let ordinary = f
        .batches
        .iter()
        .find(|b| b.fog == FOG_FACES && b.pass != GHOST_PASS)
        .unwrap();
    let ground = f
        .batches
        .iter()
        .find(|b| b.fog == FOG_PAINTED_FACES)
        .unwrap();
    assert_ne!(
        ordinary.key, ground.key,
        "ground paint has its own batch identity"
    );
    let instance = &f.instances[ordinary.first as usize];
    assert_eq!(instance.placement, [1.0, 2.0, 3.0, 0.5]);
    assert_eq!(instance.tint, [0.2, 0.4, 0.6, -1.0]);
    assert_eq!(
        instance.scale,
        [2.0, 3.0, 4.0, 0.0],
        "a static face never receives a unit x-ray"
    );
    let joint = &f.palettes[instance.data[0] as usize];
    assert_eq!(joint.x, [1.0, 0.0, 0.0, 4.0]);
    assert_eq!(joint.y, [0.0, 1.0, 0.0, 5.0]);
    assert_eq!(joint.z, [0.0, 0.0, 1.0, 6.0]);
    let ghost = f.batches.iter().find(|b| b.pass == GHOST_PASS).unwrap();
    assert_eq!(ghost.tier, 3, "ghosts use mesh tiers only");
    assert_eq!(ghost.caster, NONE);
    assert_eq!(ghost.cascade_mask, 0);
    assert_eq!(f.instances[ghost.first as usize].scale[3], 0.5);
    let index = f.index;
    p.set_lab_models(Vec::new()).unwrap();
    let empty = p.frame(7.0, &camera(), &Ui::default(), 0).unwrap();
    assert!(empty.instances.is_empty());
    assert!(
        empty.index > index,
        "paused editing invalidates preparation"
    );
}

fn container(content: serde_json::Value, views: serde_json::Value, body: &[u8]) -> Vec<u8> {
    let head = serde_json::to_vec(&json!({"content":content,"views":views})).unwrap();
    let mut bytes = Vec::new();
    for word in [0x5441_4742u32, 4, head.len() as u32, body.len() as u32] {
        bytes.extend(word.to_le_bytes());
    }
    bytes.extend(head);
    bytes.extend(body);
    bytes
}

fn skinned_asset() -> Asset {
    let translations = bytemuck::cast_slice::<f32, u8>(&[0.0, 0.0, 0.0, 0.0, 0.0, 2.0]).to_vec();
    let mut body = vec![0, 2];
    body.extend(&translations);
    let skeleton = container(
        json!({
            "sample_hz":1,"clips":[{"name":"rise","frames":2,"duration":1,"loop":false,
                "rotation_modes":{"$view":0},"translation_modes":{"$view":1},
                "rotations":{"$view":2},"translations":{"$view":3}}]
        }),
        json!([["u8", 0, 1], ["u8", 1, 1], ["i16", 2, 0], ["f32", 2, 6]]),
        &body,
    );
    let positions =
        bytemuck::cast_slice::<f32, u8>(&[1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]).to_vec();
    let mut body = positions;
    body.extend([0u8; 12]);
    body.extend(bytemuck::cast_slice::<u16, u8>(&[
        65535, 0, 0, 0, 65535, 0, 0, 0, 65535, 0, 0, 0,
    ]));
    let tier = json!({"positions":{"$view":0},"joints":{"$view":1},"weights":{"$view":2}});
    let bundle = container(
        json!({
            "kind":"skinned","skeleton":"test","joints":[{"name":"root","parent":-1,
                "bind":{"t":[0,0,0],"r":[0,0,0,1],"s":[1,1,1]},
                "inverse_bind":[1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1]}],
            "tiers":[tier.clone(),tier.clone(),tier.clone(),tier],
            "far_pose":{"clip":"rise","phase":0},"corpse_poses":[{"clip":"rise","phase":1}],"sockets":[]
        }),
        json!([["f32", 0, 9], ["u8", 36, 12], ["u16", 48, 12]]),
        &body,
    );
    Asset::from_bundle(&bundle, Some(&skeleton)).unwrap()
}

#[test]
fn skinned_lab_bodies_and_corpses_select_cards_and_ghosts_keep_meshes() {
    let mut p = Presentation::new(None, 30.0).unwrap();
    p.install_lab_assets(
        [("body".into(), skinned_asset())].into(),
        Detail {
            lod_px: [80.0, 40.0, 20.0],
            impostor_px: 1000.0,
            shadow_caster_px: 0.0,
        },
    );
    p.set_lab_models(
        serde_json::from_value(json!([
            {"appearance":"body","x":0,"y":0,"z":0,"yaw":0,
             "pose":{"kind":"rest"}},
            {"appearance":"body","x":1,"y":0,"z":0,"yaw":0,"tier":4,
             "pose":{"kind":"corpse","lies":0}},
            {"appearance":"body","x":2,"y":0,"z":0,"yaw":0,"tier":4,
             "pose":{"kind":"rest"},"ghost":[0.1,0.5,1,0.4]},
            {"appearance":"body","x":3,"y":0,"z":0,"yaw":0,"tier":4,
             "pose":{"kind":"rest"},"highlight":true}
        ]))
        .unwrap(),
    )
    .unwrap();
    let frame = p.frame(0.0, &camera(), &Ui::default(), 0).unwrap();
    let at = |x| {
        frame
            .batches
            .iter()
            .find(|batch| {
                frame.instances[batch.first as usize..(batch.first + batch.count) as usize]
                    .iter()
                    .any(|instance| instance.placement[0] == x)
            })
            .unwrap()
    };
    for x in [0.0, 1.0] {
        assert_eq!(at(x).tier, 4, "admitted authored pose must use its card");
        assert_eq!(at(x).caster, NONE);
    }
    assert_eq!(at(2.0).tier, 3, "a ghost always keeps actual geometry");
    assert_eq!(at(2.0).pass, GHOST_PASS);
    assert_eq!(
        at(3.0).tier,
        3,
        "highlight keeps mesh geometry rather than losing its cue on a card"
    );
    assert_eq!(frame.instances[at(3.0).first as usize].tint[3], -2.0);
}

#[test]
fn authored_clips_rest_corpses_and_supplied_palettes_share_the_frame_pose_contract() {
    use presentation::records::{FOG_GROUND, FOG_UNITS};
    use presentation::world::WorldInput;
    let mut p = Presentation::new(None, 30.0).unwrap();
    p.install_lab_assets(
        [("body".into(), skinned_asset())].into(),
        Detail {
            lod_px: [80.0, 40.0, 20.0],
            impostor_px: 4.0,
            shadow_caster_px: 0.0,
        },
    );
    // A world reload must preserve the admitted tooling catalog and requests.
    p.set_lab_models(
        serde_json::from_value(json!([
            {"appearance":"body","x":0,"y":0,"z":0,"yaw":0,"tier":0,
             "pose":{"kind":"clip","clip":"rise","phase":0.5},"xray":[1,0,0,0.7]},
            {"appearance":"body","x":3,"y":0,"z":0,"yaw":0,"tier":0,
             "pose":{"kind":"corpse","lies":0},"xray":[1,0,0,1]},
            {"appearance":"body","x":6,"y":0,"z":0,"yaw":0,"tier":0,
             "pose":{"kind":"posed","palette":[1,0,0,2,0,1,0,3,0,0,1,4],"scroll":[-0.25,1.25]}}
        ]))
        .unwrap(),
    )
    .unwrap();
    p.load_world(
        WorldInput {
            battle: None,
            sun_azimuth: 0.0,
            sun_elevation: 0.7,
            buildings: None,
        },
        Default::default(),
        None,
    )
    .unwrap();
    let corpse_row = p
        .world()
        .appearances
        .iter()
        .position(|r| r.lies == 0)
        .unwrap() as u32;
    let body_row = p
        .world()
        .appearances
        .iter()
        .position(|r| r.lies == NONE)
        .unwrap() as u32;
    assert_eq!(
        p.rest_palette(corpse_row).unwrap()[0].z,
        [0.0, 0.0, 1.0, 2.0]
    );
    assert_eq!(p.rest_palette(body_row).unwrap()[0].z, [0.0, 0.0, 1.0, 0.0]);
    let f = p.frame(0.0, &camera(), &Ui::default(), 0).unwrap();
    let at = |x| f.instances.iter().find(|i| i.placement[0] == x).unwrap();
    let posed = at(0.0);
    assert_eq!(f.palettes[posed.data[0] as usize].z, [0.0, 0.0, 1.0, 1.0]);
    assert_eq!(posed.scale[3], 0.7);
    assert_eq!(at(3.0).data[0], 0.0, "corpse uses its baked resting mesh");
    assert_eq!(
        at(3.0).scale[3],
        0.0,
        "corpse is ground-fogged and has no x-ray"
    );
    let supplied = at(6.0);
    assert_eq!(&supplied.data[1..3], &[0.75, 0.25]);
    assert_eq!(
        f.palettes[supplied.data[0] as usize].z,
        [0.0, 0.0, 1.0, 4.0]
    );
    assert!(f
        .batches
        .iter()
        .any(|b| b.fog == FOG_UNITS && b.appearance == body_row));
    assert!(f
        .batches
        .iter()
        .any(|b| b.fog == FOG_GROUND && b.appearance == corpse_row));
}

#[test]
fn battle_and_lab_instances_of_one_appearance_share_one_batch_without_replacing_the_battle() {
    use contract::ids::Side;
    use presentation::fielded;
    use presentation::world::{BattleInput, SoldierBody, WorldInput, Xray};
    use sim::{
        battle::Battle,
        publication::{self, Publisher},
    };
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let battle = Battle::new(
        &serde_json::from_value(json!({
            "map":{"size":[600,400],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35},
            "rules":sim::fixtures::test_rules(),
            "units":[{"side":"blue","kind":"test_tank","position":[0,0]}],"events":[],"scripts":[]
        }))
        .unwrap(),
        1,
    );
    let fielded = fielded::fielded(&root, battle.rules(), &["test_tank".into()]).unwrap();
    let (name, appearance) = fielded
        .units
        .appearances
        .iter()
        .find(|(_, a)| matches!(a, presentation::world::Appearance::Articulated { .. }))
        .unwrap();
    let (name, appearance) = (name.clone(), appearance.clone());
    let detail = fielded.units.detail;
    let world = WorldInput {
        battle: Some(BattleInput {
            units: fielded.units,
            effects: fielded.effects,
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
        }),
        sun_azimuth: 0.0,
        sun_elevation: 0.7,
        buildings: None,
    };
    let mut p = Presentation::new(Some(&publication::layout_json(&battle)), 30.0).unwrap();
    p.load_world(world, fielded.tables, None).unwrap();
    let words: Vec<u32> = Publisher::new()
        .publish(&battle, Side::Blue)
        .unwrap()
        .iter()
        .map(|v| v.to_bits())
        .collect();
    p.push_publication(&words, 0.0).unwrap();
    let first = p.frame(0.0, &camera(), &Ui::default(), 0).unwrap();
    let baseline = first.instances[0];
    let batch = first.batches.iter().find(|b| b.pass == 0).unwrap();
    let (key, tier, count) = (batch.key, batch.tier, batch.count);
    p.install_lab_assets(
        [(
            name.clone(),
            Asset::Animated {
                appearance: Box::new(appearance),
                table: Vec::new(),
            },
        )]
        .into(),
        detail,
    );
    p.set_lab_models(
        serde_json::from_value(json!([{
            "appearance":name,"x":5,"y":0,"z":0,"yaw":0,"tier":tier,"pose":{"kind":"rest"}
        }]))
        .unwrap(),
    )
    .unwrap();
    let together = p.frame(0.0, &camera(), &Ui::default(), 0).unwrap();
    let matching: Vec<_> = together.batches.iter().filter(|b| b.key == key).collect();
    assert_eq!(
        matching.len(),
        1,
        "one stable batch key per appearance/tier/pass"
    );
    assert_eq!(matching[0].count, count + 1);
    let members = &together.instances
        [matching[0].first as usize..(matching[0].first + matching[0].count) as usize];
    assert_eq!(
        members[0].placement, baseline.placement,
        "lab admission preserves the battle model"
    );
    assert_eq!(members[1].placement, [5.0, 0.0, 0.0, 0.0]);
    let palette = &together.palettes[members[1].data[0] as usize];
    assert!(
        palette.x[0].is_finite(),
        "the shared model reads its authored rest joints"
    );
    p.install_lab_assets(Default::default(), detail);
    let battle_only = p.frame(0.0, &camera(), &Ui::default(), 0).unwrap();
    let retained = battle_only.batches.iter().find(|b| b.key == key).unwrap();
    assert_eq!(
        retained.count, count,
        "removing tooling preserves the observed battle"
    );
    assert_eq!(
        battle_only.instances[retained.first as usize].placement,
        baseline.placement
    );
}

#[test]
fn replacing_a_tooling_catalog_removes_rows_the_new_gpu_library_cannot_load() {
    let mut p = Presentation::new(None, 30.0).unwrap();
    let detail = Detail {
        lod_px: [80.0, 40.0, 20.0],
        impostor_px: 4.0,
        shadow_caster_px: 0.0,
    };
    p.install_lab_assets([("retired".into(), static_asset())].into(), detail);
    p.install_lab_assets([("current".into(), static_asset())].into(), detail);
    let world = p.world();
    for row in &world.appearances {
        let name = &world.strings[row.name as usize..(row.name + row.name_len) as usize];
        assert_eq!(
            name, "current",
            "GPU installation must not request an asset from a retired catalog"
        );
    }
    p.set_lab_models(
        serde_json::from_value(json!([{
            "appearance":"current","x":8,"y":0,"z":0,"yaw":0,"tier":0,
            "pose":{"kind":"static","state":"default"}
        }]))
        .unwrap(),
    )
    .unwrap();
    let f = p.frame(0.0, &camera(), &Ui::default(), 0).unwrap();
    assert_eq!(f.instances[0].placement, [8.0, 0.0, 0.0, 0.0]);
    assert_eq!(
        f.batches[0].appearance, 0,
        "new generation reuses the retired row frontier"
    );
}
