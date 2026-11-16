//! Art replacement refits the same known world without resetting its observer.
use contract::identity::ContentId;
use presentation::{
    presentation::{catalog::Placement, lab::Asset, Presentation, Ui},
    static_world::props::{Art, PropInput},
    world::{Corners, Detail, WorldInput},
};
use serde_json::json;
#[path = "common/ground.rs"]
mod ground;
fn detail() -> Detail {
    Detail {
        lod_px: [80.0, 40.0, 20.0],
        impostor_px: 4.0,
        far_px: 2.0,
        shadow_caster_px: 0.0,
    }
}
fn asset() -> Asset {
    asset_with(presentation::view::TIERS)
}
/// A static asset whose one state has `meshes` meshes (five carry a far mesh).
fn asset_with(meshes: usize) -> Asset {
    let tiers = vec![json!({}); meshes];
    let head=serde_json::to_vec(&json!({"content":{"kind":"static","states":[{"name":"default","bounds":{"min":[-2,-1,0],"max":[2,1,2]},"tiers":tiers}]},"views":[]})).unwrap();
    let mut bytes = Vec::new();
    for word in [0x5441_4742u32, 4, head.len() as u32, 0] {
        bytes.extend(word.to_le_bytes());
    }
    bytes.extend(head);
    Asset::from_bundle(&bytes, None).unwrap()
}
fn id(name: &str) -> ContentId {
    ContentId::bundle(contract::identity::bytes_hash(name.as_bytes())).unwrap()
}
fn installed(name: &str, asset: Asset) -> (String, (ContentId, Asset)) {
    (name.into(), (id(name), asset))
}
fn staged(
    name: &str,
    asset: Asset,
) -> (
    std::collections::BTreeMap<String, ContentId>,
    std::collections::BTreeMap<ContentId, Asset>,
) {
    let content = id(name);
    (
        [(name.into(), content.clone())].into(),
        [(content, asset)].into(),
    )
}
/// A presentation with one crate prop drawn from four-mesh `crate-art` under `detail`.
fn one_crate(detail: Detail) -> Presentation {
    let mut p = Presentation::new(None, 30.0).unwrap();
    let layout=json!({"propKinds":["crate"],"propAppearance":{"crate":{"drawn_by":"crate","stand_in_tint":[0.5,0.5,0.5]}},"blockingPropKinds":{"wheeled":["crate"]},"unitKinds":[],"unitAppearance":{}}).to_string();
    let mut record = vec![0.0; sim::world::export::PROP_STRIDE];
    record[0] = 1.0;
    record[6..9].copy_from_slice(&[2.0, 1.0, 1.0]);
    record[10] = -1.0;
    p.place_props(
        &record,
        &layout,
        PropInput {
            appearances: vec![Art {
                name: "crate-art".into(),
                scenery: Some("crate".into()),
                family: None,
                footprint: [1.0; 3],
                paints: Some(vec![[1.0, 0.0, 0.0]]),
                bounds: Corners {
                    min: [-1.0, -1.0, 0.0],
                    max: [1.0, 1.0, 2.0],
                },
                meshes: presentation::view::TIERS,
            }],
            wrecks: Default::default(),
            stand_in: None,
            family: None,
            detail,
            dynamic: true,
        },
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
    p
}

#[test]
fn replacing_a_props_art_with_a_far_mesh_and_back_changes_its_tier_and_keeps_what_the_side_knows() {
    use presentation::view::{FAR, TIERS};
    // Every size this camera sees the crate at is below tier 3's threshold
    // and the far mesh's.
    let mut p = one_crate(Detail {
        far_px: 19.0,
        ..detail()
    });
    let camera = presentation::view::Camera {
        eye: [0.0, -1000.0, 300.0],
        target: [0.0; 3],
        up: [0.0, 0.0, 1.0],
        fov_y: 1.0,
        near: 0.1,
        size: [640, 480],
    };
    let known: Vec<_> = serde_json::from_value(json!([{"id":10,"authoredProp":1,"kind":"crate","center":[3,0],"half":[4,2,1],"yaw":0,"baseZ":0}])).unwrap();
    let ui = Ui {
        known: Some(&known),
        ..Default::default()
    };
    let drawn = |p: &mut Presentation| {
        let f = p.frame(1.0, &camera, &ui, 0).unwrap();
        f.batches
            .iter()
            .filter(|b| b.pass == presentation::records::VIEW_PASS)
            .map(|b| (b.tier, f.instances[b.first as usize].placement[0]))
            .collect::<Vec<_>>()
    };
    assert_eq!(drawn(&mut p), [(3, 3.0)]);
    let replace = |p: &mut Presentation, meshes, placements| {
        let (wanted, facts) = staged("crate-art", asset_with(meshes));
        let candidate = p
            .stage_catalog(wanted, facts, detail(), placements)
            .unwrap();
        assert!(p.commit_catalog(candidate, Vec::new()).unwrap());
    };
    // New bytes under the same name and placement: the far mesh draws.
    replace(&mut p, TIERS + 1, Default::default());
    assert_eq!(drawn(&mut p), [(FAR as u32, 3.0)]);
    // Placed again from four-mesh art: tier 3 again, the known crate where it was.
    let placement: Placement = serde_json::from_value(
        json!({"footprint":[1,1,1],"scenery":"crate","family":null,"paints":null,"wreck":null}),
    )
    .unwrap();
    replace(&mut p, TIERS, [("crate-art".into(), placement)].into());
    assert_eq!(drawn(&mut p), [(3, 3.0)]);
}

#[test]
fn same_name_props_refit_new_footprints_and_paint_using_knowledge_current_at_commit() {
    let mut p = one_crate(detail());
    let camera = presentation::view::Camera {
        eye: [0.0, -20.0, 15.0],
        target: [0.0; 3],
        up: [0.0, 0.0, 1.0],
        fov_y: 1.0,
        near: 0.1,
        size: [640, 480],
    };
    let known = |x| {
        serde_json::from_value(json!([{"id":10,"authoredProp":1,"kind":"crate","center":[x,0],"half":[4,2,1],"yaw":0,"baseZ":0}])).unwrap()
    };
    let first: Vec<_> = known(3.0);
    p.frame(
        0.0,
        &camera,
        &Ui {
            known: Some(&first),
            ..Default::default()
        },
        0,
    )
    .unwrap();
    let replacement:Placement=serde_json::from_value(json!({"footprint":[2,1,1],"scenery":"crate","family":null,"paints":[[0,1,0]],"wreck":null})).unwrap();
    let (wanted, facts) = staged("crate-art", asset());
    let candidate = p
        .stage_catalog(
            wanted,
            facts,
            detail(),
            [("crate-art".into(), replacement)].into(),
        )
        .unwrap();
    let newest: Vec<_> = known(9.0);
    p.frame(
        1.0,
        &camera,
        &Ui {
            known: Some(&newest),
            ..Default::default()
        },
        0,
    )
    .unwrap();
    let before = p.props().unwrap().models().next().unwrap().1;
    assert_eq!(
        (before[0], before[12], before[8], before[9]),
        (9.0, 4.0, 1.0, 0.0)
    );
    assert!(p.commit_catalog(candidate, Vec::new()).unwrap());
    p.frame(
        1.0,
        &camera,
        &Ui {
            known: Some(&newest),
            ..Default::default()
        },
        0,
    )
    .unwrap();
    let after = p.props().unwrap().models().next().unwrap().1;
    assert_eq!((after[0],after[12],after[8],after[9]),(9.0,2.0,0.0,1.0),"new fitting facts must use the latest learned body, even when the known list did not change after commit");
}

fn container(
    magic: u32,
    version: u32,
    content: serde_json::Value,
    views: serde_json::Value,
    body: &[u8],
) -> Vec<u8> {
    let head = serde_json::to_vec(&json!({"content":content,"views":views})).unwrap();
    let mut bytes = Vec::new();
    for word in [magic, version, head.len() as u32, body.len() as u32] {
        bytes.extend(word.to_le_bytes());
    }
    bytes.extend(head);
    bytes.extend(body);
    bytes
}

#[test]
fn building_replacement_updates_culling_extents_and_keeps_the_latest_known_ruin() {
    use presentation::static_world::{
        buildings::{Placed, PlacedBuildings},
        library::Library,
    };
    let mut body = bytemuck::cast_slice::<u16, u8>(&[0, 1]).to_vec();
    body.extend(bytemuck::cast_slice::<f32, u8>(
        &[0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0].repeat(2),
    ));
    body.extend([15u8, 15]);
    body.extend([255u8; 6]);
    let library=Library::decode(&container(0x4c54_4742,3,json!({
        "kits":[{"appearance":"kit"}],"modules":[{"kit":0,"module":"whole"},{"kit":0,"module":"broken"}],
        "templates":[{"id":"hut","states":{"intact":{"first":0,"count":1},"ruin":{"first":1,"count":1},"gutted":{"first":1,"count":1}}}],
        "rows":{"module":{"$view":0},"transform":{"$view":1},"tiers":{"$view":2},"tint":{"$view":3}}
    }),json!([["u16",0,2],["f32",4,14],["u8",60,2],["u8",62,6]]),&body)).unwrap();
    let mut p = Presentation::new(None, 30.0).unwrap();
    p.load_world(
        WorldInput {
            battle: None,
            sun_azimuth: 0.0,
            sun_elevation: 0.7,
            buildings: Some(presentation::world::BuildingStyle {
                lod_px_per_m: [4.0, 2.0, 1.0],
                chunk_m: 64.0,
                pool_records: 1024,
                tint_jitter: 0.0,
            }),
        },
        Default::default(),
        Some(PlacedBuildings {
            library,
            bounds: vec![Some([-1.0, -1.0, 0.0, 1.0, 1.0, 2.0]); 2],
            placed: vec![Placed {
                owner: 1,
                template: "hut".into(),
                frame: [1000.0, 0.0, 0.0, 0.0],
            }],
            parts: vec![(5, 0)],
            shells: vec!["shell".into()],
        }),
    )
    .unwrap();
    let camera = presentation::view::Camera {
        eye: [0.0, -20.0, 15.0],
        target: [0.0; 3],
        up: [0.0, 0.0, 1.0],
        fov_y: 1.0,
        near: 0.1,
        size: [640, 480],
    };
    assert!(p
        .frame(0.0, &camera, &Ui::default(), 0)
        .unwrap()
        .batches
        .is_empty());
    let replacement = Asset::from_bundle(
        &container(
            0x5441_4742,
            4,
            json!({"kind":"static","states":[
                {"name":"whole","bounds":{"min":[-1200,-2,0],"max":[1,2,2]},"tiers":[{},{},{},{}]},
                {"name":"broken","bounds":{"min":[-1200,-2,0],"max":[1,2,1]},"tiers":[{},{},{},{}]}
            ]}),
            json!([]),
            &[],
        ),
        None,
    )
    .unwrap();
    let id = id("kit");
    let candidate = p
        .stage_catalog(
            [("kit".into(), id.clone())].into(),
            [(id, replacement)].into(),
            detail(),
            Default::default(),
        )
        .unwrap();
    let known: Vec<presentation::static_world::fallen::Known> =
        serde_json::from_value(json!([{"authoredProp":5,"kind":"rubble","destroyed":false}]))
            .unwrap();
    let ui = Ui {
        known: Some(&known),
        ..Default::default()
    };
    assert!(p.frame(1.0, &camera, &ui, 0).unwrap().batches.is_empty());
    assert_eq!(p.fallen(), &[(0, 1)]);
    assert!(p.commit_catalog(candidate, Vec::new()).unwrap());
    let frame = p.frame(1.0, &camera, &ui, 0).unwrap();
    assert!(
        !frame.batches.is_empty(),
        "new bounds intersect the view even though the placed origin stays distant"
    );
    let row = frame.batches[0].appearance as usize;
    let world = p.world();
    let r = &world.appearances[row];
    assert_eq!(
        &world.strings[r.state as usize..(r.state + r.state_len) as usize],
        "broken",
        "unchanged known evidence must still draw the ruin"
    );
    assert_eq!(p.fallen(), &[(0, 1)]);
}

#[test]
fn prospective_scenery_uses_new_trunk_and_stump_geometry_before_atomic_commit() {
    scenery_replacement(false);
}

#[test]
fn scenery_admitted_after_world_and_tooling_art_stays_canonical_through_replacement() {
    scenery_replacement(true);
}

#[test]
fn adding_an_appearance_carries_placed_scenery_without_requesting_placement() {
    let mut p = Presentation::new(None, 30.0).unwrap();
    ground::set_summer_ground(&mut p);
    let mesh = ground::scenery_mesh(0.25, 4.0);
    let kinds = p.ground().unwrap().scenery_kinds().len();
    p.place_scenery(&vec![mesh.as_slice(); kinds]).unwrap();
    let before = p.scenery_stats();

    let (wanted, facts) = staged("new-unit", asset());
    let candidate = p
        .stage_catalog(wanted, facts, detail(), Default::default())
        .unwrap();
    assert!(p.commit_catalog(candidate, Vec::new()).unwrap());
    assert_eq!(p.scenery_stats(), before);
}

fn scenery_replacement(late: bool) {
    let biome = presentation::static_world::ground::biome::Biome::load(
        include_str!("../../../fixtures/biomes/summer.json"),
        None,
    )
    .unwrap();
    let tree_name = biome.trees.species[0].appearance.clone();
    let mut p = Presentation::new(None, 30.0).unwrap();
    ground::set_summer_ground(&mut p);
    let mesh = ground::scenery_mesh;
    let count = p.ground().unwrap().scenery_kinds().len();
    let original = mesh(0.25, 4.0);
    let original_refs = vec![original.as_slice(); count];
    if !late {
        p.place_scenery(&original_refs).unwrap();
    }
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
    if late {
        p.install_lab_assets([installed("tool-crate", asset())].into(), detail());
        p.place_scenery(&original_refs).unwrap();
        // Replacing the tooling suffix must retain admitted scenery rows.
        p.install_lab_assets(Default::default(), detail());
    }
    let world = p.world();
    let stump = world
        .appearances
        .iter()
        .position(|r| {
            r.draws == 6
                && world.strings[r.name as usize..(r.name + r.name_len) as usize] == tree_name
        })
        .unwrap() as u32;
    let radius = |vertices: &[f32]| {
        vertices
            .as_chunks::<10>()
            .0
            .iter()
            .map(|v| v[0].abs())
            .fold(0.0, f32::max)
    };
    assert_eq!(radius(p.stump_mesh(stump).unwrap()), 0.25);
    let mut candidate = p
        .stage_catalog(
            Default::default(),
            Default::default(),
            detail(),
            Default::default(),
        )
        .unwrap();
    let replacement = mesh(0.75, 8.0);
    let refs = vec![replacement.as_slice(); count];
    assert!(candidate.stage_scenery(&p, &[]).is_err());
    assert_eq!(
        radius(p.stump_mesh(stump).unwrap()),
        0.25,
        "failed preparation preserves active geometry"
    );
    candidate.stage_scenery(&p, &refs).unwrap();
    assert_eq!(
        radius(candidate.view(&p).unwrap().stump_mesh(stump).unwrap()),
        0.75,
        "GPU admission must see the prospective stump"
    );
    assert_eq!(
        radius(p.stump_mesh(stump).unwrap()),
        0.25,
        "staging cannot change active scenery"
    );
    assert!(p.commit_catalog(candidate, Vec::new()).unwrap());
    assert_eq!(radius(p.stump_mesh(stump).unwrap()), 0.75);
}
