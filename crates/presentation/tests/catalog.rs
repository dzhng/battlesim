//! A candidate catalog cannot change the currently displayed presentation.
use contract::identity::ContentId;
use presentation::presentation::{lab::Asset, Presentation, Ui};
use presentation::view::Camera;
use presentation::world::{Detail, WorldInput};
use serde_json::json;
#[path = "common/ground.rs"]
mod ground;

fn asset() -> Asset {
    asset_with_height(2)
}
fn asset_with_height(height: u32) -> Asset {
    asset_with_state("default", height)
}
fn asset_with_state(state: &str, height: u32) -> Asset {
    let head = serde_json::to_vec(&json!({"content":{"kind":"prop","states":[{
        "name":state,"bounds":{"min":[-1,-1,0],"max":[1,1,height]},"tiers":[{}]
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

fn content(name: &str) -> ContentId {
    ContentId::bundle(contract::identity::bytes_hash(name.as_bytes())).unwrap()
}

fn installed(name: &str, asset: Asset) -> (String, (ContentId, Asset)) {
    (name.to_owned(), (content(name), asset))
}

fn staged(
    name: &str,
    asset: Asset,
) -> (
    std::collections::BTreeMap<String, ContentId>,
    std::collections::BTreeMap<ContentId, Asset>,
) {
    let id = content(name);
    ([(name.to_owned(), id.clone())].into(), [(id, asset)].into())
}
/// What each static batch draws, by appearance rather than row number:
/// name, state, draw kind, tier, pass and count.
fn static_draws(
    p: &Presentation,
    batches: &[presentation::records::Batch],
) -> Vec<(String, String, u32, u32, u32, u32)> {
    let w = p.world();
    let text = |at: u32, len: u32| w.strings[at as usize..(at + len) as usize].to_owned();
    let mut draws: Vec<_> = batches
        .iter()
        .map(|b| (b, &w.appearances[b.appearance as usize]))
        .filter(|(_, r)| r.placed != 0)
        .map(|(b, r)| {
            let (name, state) = (text(r.name, r.name_len), text(r.state, r.state_len));
            (name, state, r.draws, b.tier, b.pass, b.count)
        })
        .collect();
    draws.sort();
    draws
}
/// How many unit models the batches draw: those of rows not placed.
fn units_drawn(p: &Presentation, batches: &[presentation::records::Batch]) -> u32 {
    let rows = &p.world().appearances;
    batches
        .iter()
        .filter(|b| rows[b.appearance as usize].placed == 0)
        .map(|b| b.count)
        .sum()
}

#[test]
fn catalog_delta_reuses_facts_retained_by_content_identity() {
    let fixtures = [("a", "short", 2), ("x", "tall", 3), ("y", "tower", 4)];
    let initial = || {
        let mut p = Presentation::new(None, 30.0).unwrap();
        p.install_lab_assets(
            fixtures[1..]
                .iter()
                .map(|(name, state, height)| {
                    (
                        (*name).to_owned(),
                        (content(name), asset_with_state(state, *height)),
                    )
                })
                .collect(),
            detail(),
        );
        p
    };
    let wanted = || {
        fixtures
            .iter()
            .map(|(name, _, _)| ((*name).to_owned(), content(name)))
            .collect()
    };
    let facts = || {
        fixtures
            .iter()
            .map(|(name, state, height)| (content(name), asset_with_state(state, *height)))
            .collect()
    };
    let models = || {
        fixtures
            .iter()
            .enumerate()
            .map(|(i, (name, state, _))| {
                serde_json::from_value(
                    json!({"appearance":name,"x":i as f64 * 2.0,"y":0,"z":0,"yaw":0,
            "pose":{"kind":"static","state":state}}),
                )
                .unwrap()
            })
            .collect()
    };
    let (mut delta_p, mut full_p) = (initial(), initial());
    let delta = delta_p
        .stage_catalog(
            wanted(),
            [(content("a"), asset_with_state("short", 2))].into(),
            detail(),
            Default::default(),
        )
        .unwrap();
    let full = full_p
        .stage_catalog(wanted(), facts(), detail(), Default::default())
        .unwrap();
    // World equality includes the row lookup index as well as every public row
    // and the string arena. Distinct states catch facts routed to the wrong id.
    assert_eq!(
        delta.view(&delta_p).unwrap().world(),
        full.view(&full_p).unwrap().world()
    );
    assert!(delta_p.commit_catalog(delta, models()).unwrap());
    assert!(full_p.commit_catalog(full, models()).unwrap());
    assert_eq!(delta_p.world(), full_p.world());
    let delta_frame = delta_p.frame(0.0, &camera(), &Ui::default(), 0).unwrap();
    let full_frame = full_p.frame(0.0, &camera(), &Ui::default(), 0).unwrap();
    assert_eq!(delta_frame.instances.len(), fixtures.len());
    assert_eq!(delta_frame.batches, full_frame.batches);
    assert_eq!(delta_frame.instances, full_frame.instances);
    assert_eq!(delta_frame.palettes, full_frame.palettes);
}

#[test]
fn a_held_draft_retired_before_preparation_is_rejected_without_touching_live_art() {
    let mut p = Presentation::new(None, 30.0).unwrap();
    p.install_lab_assets([installed("held", asset())].into(), detail());
    let held_id = content("held");
    let draft = p
        .stage_catalog(
            [("held".to_owned(), held_id.clone())].into(),
            Default::default(),
            detail(),
            Default::default(),
        )
        .unwrap();
    let replacement = p
        .stage_catalog(
            [("replacement".to_owned(), content("replacement"))].into(),
            [(content("replacement"), asset_with_height(5))].into(),
            detail(),
            Default::default(),
        )
        .unwrap();
    assert!(p.commit_catalog(replacement, Vec::new()).unwrap());
    assert!(
        draft.view(&p).is_err(),
        "retired held draft must be rejected cleanly"
    );
    assert!(names(&p).iter().all(|name| name == "replacement"));
}
fn names(p: &Presentation) -> Vec<String> {
    let w = p.world();
    w.appearances
        .iter()
        .map(|r| w.strings[r.name as usize..(r.name + r.name_len) as usize].to_owned())
        .collect()
}

/** A battle's world as a page loads it for `side`, from what it fields. */
fn battle_input(
    units: presentation::world::UnitsSection,
    effects: presentation::world::EffectsSection,
    side: &str,
) -> WorldInput {
    use presentation::world::{BattleInput, SoldierBody, Xray};
    WorldInput {
        battle: Some(BattleInput {
            units,
            effects,
            side: side.into(),
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
}

#[test]
fn pending_catalog_preserves_live_rows_and_models_and_commits_latest_models() {
    let mut p = Presentation::new(None, 30.0).unwrap();
    p.install_lab_assets([installed("committed", asset())].into(), detail());
    p.set_lab_models(model("committed", 1.0)).unwrap();
    let (wanted, facts) = staged("candidate", asset());
    let candidate = p
        .stage_catalog(wanted, facts, detail(), Default::default())
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
    p.install_lab_assets([installed("committed", asset())].into(), detail());
    let (wanted, facts) = staged("stale", asset());
    let stale = p
        .stage_catalog(wanted, facts, detail(), Default::default())
        .unwrap();
    let mut another = Presentation::new(None, 30.0).unwrap();
    assert!(
        !stale.is_current(&another),
        "catalog identity includes its Presentation owner"
    );
    assert!(stale.view(&another).is_err());
    assert!(!another.commit_catalog(stale, Vec::new()).unwrap());
    let (wanted, facts) = staged("stale", asset());
    let stale = p
        .stage_catalog(wanted, facts, detail(), Default::default())
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
    let (wanted, facts) = staged("invalid", asset());
    let draft = p
        .stage_catalog(wanted, facts, detail(), Default::default())
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
    use presentation::{fielded, world::Appearance};
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
    let input = battle_input(fielded.units, fielded.effects, "blue");
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
    let (wanted, facts) = staged(&name, asset());
    assert!(p
        .stage_catalog(wanted, facts, detail, Default::default())
        .is_err());
    assert_eq!(p.last_frame().instances[0].placement, instance.placement);
    let id = content(&name);
    let draft = p
        .stage_catalog(
            [(name, id.clone())].into(),
            [(
                id,
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

#[test]
fn a_newly_fielded_kind_joins_the_loaded_battle_without_reloading_its_world() {
    use contract::ids::Side;
    use presentation::fielded;
    use sim::{
        battle::Battle,
        publication::{self, Publisher},
    };
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let battle = Battle::new(
        &serde_json::from_value(json!({
            "map":{"size":[600,400],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35},
            "rules":sim::fixtures::test_rules(),
            "units":[
                {"side":"blue","kind":"test_tank","position":[0,0]},
                {"side":"blue","kind":"test_rifle","position":[12,0]}
            ],"events":[],"scripts":[]
        }))
        .unwrap(),
        1,
    );
    let tank = fielded::fielded(&root, battle.rules(), &["test_tank".into()]).unwrap();
    let detail = tank.units.detail;
    let mut p = Presentation::new(Some(&publication::layout_json(&battle)), 30.0).unwrap();
    ground::set_summer_ground(&mut p);
    p.load_world(
        battle_input(tank.units, tank.effects, "blue"),
        tank.tables,
        None,
    )
    .unwrap();
    // As in a page, the scenery is placed once its art arrives, after the
    // battle's rows.
    ground::place_scenery(&mut p, &ground::scenery_mesh(0.25, 4.0));
    let words: Vec<u32> = Publisher::new()
        .publish(&battle, Side::Blue)
        .unwrap()
        .iter()
        .map(|v| v.to_bits())
        .collect();
    p.push_publication(&words, 0.0).unwrap();
    let before = p.frame(0.0, &camera(), &Ui::default(), 0).unwrap();
    let (batches, instances) = (before.batches.clone(), before.instances.clone());
    let rows = &p.world().appearances;
    let tank = batches
        .iter()
        .find(|b| rows[b.appearance as usize].placed == 0)
        .unwrap();
    let tank_at = instances[tank.first as usize].placement;
    let drawn = units_drawn(&p, &batches);
    let scenery = static_draws(&p, &batches);
    assert!(!scenery.is_empty(), "the frame draws the ground's scenery");

    let both = || {
        fielded::fielded(
            &root,
            battle.rules(),
            &["test_tank".into(), "test_rifle".into()],
        )
        .unwrap()
    };
    let other_side = both();
    assert!(
        !p.stage_battle(
            battle_input(other_side.units, other_side.effects, "red"),
            other_side.tables
        )
        .unwrap(),
        "another side is another battle: its world loads anew"
    );
    let grown = both();
    assert!(p
        .stage_battle(
            battle_input(grown.units, grown.effects, "blue"),
            grown.tables
        )
        .unwrap());
    let staged = p
        .frame(0.0, &camera(), &Ui::default(), 0)
        .unwrap()
        .batches
        .clone();
    assert_eq!(
        units_drawn(&p, &staged),
        drawn,
        "a staged catalog changes nothing drawn until it commits"
    );
    let mut draft = p
        .stage_catalog(
            Default::default(),
            Default::default(),
            detail,
            Default::default(),
        )
        .unwrap();
    let mesh = ground::scenery_mesh(0.25, 4.0);
    let kinds = p.ground().unwrap().scenery_kinds().len();
    draft
        .stage_scenery(&p, &vec![mesh.as_slice(); kinds])
        .unwrap();
    assert!(p.commit_catalog(draft, Vec::new()).unwrap());
    let after = p.frame(0.0, &camera(), &Ui::default(), 0).unwrap();
    assert!(
        after.instances.iter().any(|i| i.placement == tank_at),
        "the units already drawn stay where they were"
    );
    let batches = after.batches.clone();
    assert!(
        units_drawn(&p, &batches) > drawn,
        "the newly fielded squad draws from the publication already taken"
    );
    assert_eq!(
        static_draws(&p, &batches),
        scenery,
        "the static world draws the same appearances after the battle grows"
    );
    let held = p
        .frame(0.0, &camera(), &Ui::default(), 0)
        .unwrap()
        .batches
        .clone();
    let rows = &p.world().appearances;
    assert!(
        held.iter()
            .filter(|b| b.first == presentation::records::NONE)
            .all(|b| rows[b.appearance as usize].placed != 0),
        "only the static world's batches are held between frames"
    );
    assert!(
        p.observation().is_some(),
        "the battle stream remains attached"
    );
}
