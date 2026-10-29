//! A candidate catalog cannot change the currently displayed presentation.
use presentation::presentation::{lab::Asset, Presentation, Ui};
use presentation::view::Camera;
use presentation::world::{Detail, WorldInput};
use serde_json::json;

fn asset() -> Asset {
    let head = serde_json::to_vec(&json!({"content":{"kind":"prop","states":[{
        "name":"default","bounds":{"min":[-1,-1,0],"max":[1,1,2]},"tiers":[{}]
    }]},"views":[]}))
    .unwrap();
    let mut bytes = Vec::new();
    for word in [0x5441_4742u32, 4, head.len() as u32, 0] {
        bytes.extend(word.to_le_bytes());
    }
    bytes.extend(head);
    Asset::from_bundle(&bytes, None).unwrap()
}
fn detail() -> Detail {
    Detail {
        lod_px: [80.0, 40.0, 20.0],
        impostor_px: 4.0,
        shadow_caster_px: 0.0,
    }
}
fn model(name: &str, x: f64) -> Vec<presentation::presentation::lab::ModelInput> {
    serde_json::from_value(json!([{"appearance":name,"x":x,"y":0,"z":0,"yaw":0,
        "tier":0,"pose":{"kind":"static","state":"default"}}]))
    .unwrap()
}
fn camera() -> Camera {
    Camera {
        eye: [0.0, -20.0, 15.0],
        target: [0.0; 3],
        up: [0.0, 0.0, 1.0],
        fov_y: 1.0,
        near: 0.1,
        size: [640, 480],
    }
}
fn names(p: &Presentation) -> Vec<String> {
    let w = p.world();
    w.appearances
        .iter()
        .map(|r| w.strings[r.name as usize..(r.name + r.name_len) as usize].to_owned())
        .collect()
}

#[test]
fn pending_catalog_preserves_live_rows_and_models_and_commits_latest_models() {
    let mut p = Presentation::new(None, 30.0).unwrap();
    p.install_lab_assets([("committed".into(), asset())].into(), detail());
    p.set_lab_models(model("committed", 1.0)).unwrap();
    let candidate = p
        .stage_catalog(
            [("candidate".into(), asset())].into(),
            detail(),
            Default::default(),
        )
        .unwrap();
    assert!(candidate
        .view(&p)
        .unwrap()
        .world()
        .strings
        .contains("candidate"));
    assert!(names(&p).iter().all(|n| n == "committed"));
    assert_eq!(
        p.frame(0.0, &camera(), &Ui::default(), 0)
            .unwrap()
            .instances[0]
            .placement[0],
        1.0
    );
    // Ordinary frames and latest editing do not invalidate candidate admission.
    p.set_lab_models(model("committed", 2.0)).unwrap();
    assert_eq!(
        p.frame(1.0, &camera(), &Ui::default(), 0)
            .unwrap()
            .instances[0]
            .placement[0],
        2.0
    );
    assert!(p
        .commit_catalog(candidate, model("candidate", 7.0))
        .unwrap());
    assert!(names(&p).iter().all(|n| n == "candidate"));
    assert_eq!(
        p.frame(1.0, &camera(), &Ui::default(), 0)
            .unwrap()
            .instances[0]
            .placement[0],
        7.0
    );
}

#[test]
fn stale_or_invalid_admission_never_replaces_the_active_generation() {
    let mut p = Presentation::new(None, 30.0).unwrap();
    p.install_lab_assets([("committed".into(), asset())].into(), detail());
    let stale = p
        .stage_catalog(
            [("stale".into(), asset())].into(),
            detail(),
            Default::default(),
        )
        .unwrap();
    let mut another = Presentation::new(None, 30.0).unwrap();
    assert!(
        !stale.is_current(&another),
        "catalog identity includes its Presentation owner"
    );
    assert!(stale.view(&another).is_err());
    assert!(!another.commit_catalog(stale, Vec::new()).unwrap());
    let stale = p
        .stage_catalog(
            [("stale".into(), asset())].into(),
            detail(),
            Default::default(),
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
    assert!(!p.commit_catalog(stale, Vec::new()).unwrap());
    let draft = p
        .stage_catalog(
            [("invalid".into(), asset())].into(),
            detail(),
            Default::default(),
        )
        .unwrap();
    let bad = serde_json::from_value(json!([{"appearance":"invalid","x":0,"y":0,"z":0,"yaw":0,
        "pose":{"kind":"posed","palette":[1,2,3]}}]))
    .unwrap();
    assert!(p.commit_catalog(draft, bad).is_err());
    assert!(names(&p).iter().all(|n| n == "committed"));
}

#[test]
fn battle_art_replacement_changes_live_joint_facts_without_restarting_the_observer() {
    use contract::ids::Side;
    use presentation::{
        fielded,
        world::{Appearance, BattleInput, SoldierBody, Xray},
    };
    use sim::{
        battle::Battle,
        publication::{self, Publisher},
    };
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let game =
        serde_json::from_slice(&std::fs::read(root.join("fixtures/game.json")).unwrap()).unwrap();
    let battle = Battle::new(
        &serde_json::from_value(json!({
            "map":{"size":[600,400],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35},
            "rules":sim::fixtures::test_game(),
            "units":[{"side":"blue","kind":"test_tank","position":[0,0]}],"events":[],"scripts":[]
        }))
        .unwrap(),
        1,
    );
    let fielded = fielded::fielded(&root, &game, "air", &["test_tank".into()]).unwrap();
    let (name, mut replacement) = fielded
        .units
        .appearances
        .iter()
        .find(|(_, a)| matches!(a, Appearance::Articulated { .. }))
        .map(|(n, a)| (n.clone(), a.clone()))
        .unwrap();
    if let Appearance::Articulated { nodes, .. } = &mut replacement {
        nodes[0].bind.t[0] += 4.0;
    }
    let detail = fielded.units.detail;
    let input = WorldInput {
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
    p.load_world(input, fielded.tables, None).unwrap();
    let words: Vec<u32> = Publisher::new()
        .publish(&battle, Side::Blue)
        .unwrap()
        .iter()
        .map(|v| v.to_bits())
        .collect();
    p.push_publication(&words, 0.0).unwrap();
    let frame = p.frame(0.0, &camera(), &Ui::default(), 0).unwrap();
    let instance = frame.instances[0];
    let root_x = frame.palettes[instance.data[0] as usize].x[3];
    assert!(p
        .stage_catalog([(name.clone(), asset())].into(), detail, Default::default())
        .is_err());
    assert_eq!(p.last_frame().instances[0].placement, instance.placement);
    let draft = p
        .stage_catalog(
            [(
                name,
                Asset::Animated {
                    appearance: Box::new(replacement),
                    table: Vec::new(),
                },
            )]
            .into(),
            detail,
            Default::default(),
        )
        .unwrap();
    assert!(p.commit_catalog(draft, Vec::new()).unwrap());
    let frame = p.frame(0.0, &camera(), &Ui::default(), 0).unwrap();
    assert_eq!(
        frame.instances[0].placement, instance.placement,
        "art replacement retains the observed unit"
    );
    assert_eq!(
        frame.palettes[frame.instances[0].data[0] as usize].x[3],
        root_x + 4.0,
        "battle posing must read the new facts, not only tooling metadata"
    );
    assert!(
        p.observation().is_some(),
        "the battle stream remains attached"
    );
}

#[test]
fn battle_compatibility_requires_only_retained_clips_and_blends() {
    use presentation::{
        units::driver::{FeedFrame, FeedSoldier, FeedUnit, PoseDriver},
        world::{Appearance, UnitsSection},
    };
    let units: UnitsSection = serde_json::from_slice(
        &std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/paris-corner-units.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let mut driver = PoseDriver::new(&units).unwrap();
    let feed = |time, x| FeedFrame {
        time,
        units: vec![FeedUnit {
            id: 1.0,
            kind: "capture_rifle".into(),
            side: "blue".into(),
            soldiers: vec![FeedSoldier {
                id: 1.0,
                position: [x, 0.0, 0.0],
                active_mount: Some(0),
                ..Default::default()
            }],
            ..Default::default()
        }],
        ..Default::default()
    };
    driver.update(&feed(0.0, 0.0), &units, None);
    driver.update(&feed(0.1, 0.4), &units, None);
    let pose = &driver.poses().soldiers[0];
    let current = pose.clip.to_string();
    let previous = pose
        .blend
        .as_ref()
        .expect("movement crosses from idle to running")
        .clip
        .to_string();
    assert_ne!(current, previous);
    let remove = |clip: &str| {
        let mut candidate = units.clone();
        for a in candidate.appearances.values_mut() {
            if let Appearance::Skinned { clips, .. } = a {
                clips.remove(clip);
            }
        }
        candidate
    };
    assert!(driver.validate_catalog(&units).is_ok());
    assert!(
        driver.validate_catalog(&remove(&current)).is_err(),
        "current clip is required"
    );
    assert!(
        driver.validate_catalog(&remove(&previous)).is_err(),
        "crossfade source remains live"
    );
    // An unused clip can be renamed or retired without freezing the old catalog.
    let unused = units
        .appearances
        .values()
        .find_map(|a| match a {
            Appearance::Skinned { clips, .. } => clips
                .keys()
                .find(|c| **c != current && **c != previous)
                .cloned(),
            _ => None,
        })
        .unwrap();
    assert!(driver.validate_catalog(&remove(&unused)).is_ok());
}
