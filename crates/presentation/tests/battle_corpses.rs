//! Real battle publications carry fallen soldiers into the battle frame's palettes.
use contract::ids::Side;
use presentation::fielded;
use presentation::presentation::{Presentation, Ui};
use presentation::records::NONE;
use presentation::view::Camera;
use presentation::world::{BattleInput, SoldierBody, WorldInput, Xray};
use serde_json::json;
use sim::battle::Battle;
use sim::publication::{self, Publisher};

#[test]
fn fallen_battle_soldiers_name_their_rows_rest_palette_and_share_it() {
    let mut scenario: contract::scenario::ScenarioDefinition = serde_json::from_value(json!({
        "map":{"size":[600,400],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35},
        "rules":sim::fixtures::test_rules(),
        "units":[
            {"side":"blue","kind":"test_tank","position":[100,200],"engagement":"fire_at_will"},
            {"side":"red","kind":"test_rifle","position":[220,200],"engagement":"return_fire_only"},
            {"side":"red","kind":"test_rifle","position":[220,210],"engagement":"return_fire_only"}
        ],"events":[],"scripts":[]
    }))
    .unwrap();
    scenario.rules = scenario
        .rules
        .overridden(
            json!({
                "units":{"test_tank":{"mounts":[{"id":"cannon","weapons":["tank_he"]}]}},
                "soldiers":{"test_rifleman":{"mounts":[]},"test_grenadier":{"mounts":[]}}
            })
            .as_object()
            .unwrap(),
        )
        .unwrap();
    let mut battle = Battle::new(&scenario, 1);
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fielded = fielded::fielded(
        &root,
        battle.rules(),
        &["test_tank".into(), "test_rifle".into()],
    )
    .unwrap();
    let mut presentation =
        Presentation::new(Some(&publication::layout_json(&battle)), 30.).unwrap();
    presentation
        .load_world(
            WorldInput {
                battle: Some(BattleInput {
                    units: fielded.units,
                    effects: fielded.effects,
                    side: "blue".into(),
                    tick_hz: 30.,
                    soldier: SoldierBody {
                        radius_m: 0.2,
                        height_m: 1.8,
                    },
                    xray: Xray {
                        own: [0.; 4],
                        selected: [0.; 4],
                        ally: [0.; 4],
                    },
                    blocked: [0.; 4],
                    stand_in: None,
                }),
                sun_azimuth: 0.,
                sun_elevation: 0.7,
                buildings: None,
            },
            fielded.tables,
            None,
        )
        .unwrap();
    let camera = Camera {
        eye: [200., -150., 160.],
        target: [200., 200., 0.],
        up: [0., 0., 1.],
        fov_y: 0.9,
        near: 0.1,
        size: [1280, 720],
    };
    let mut publisher = Publisher::new();
    let mut reader =
        sim::publication::reader::Reader::new(&publication::layout_json(&battle)).unwrap();
    for _ in 0..1200 {
        let now = battle.tick() as f64 / f64::from(battle.rules().tick_hz);
        let words: Vec<_> = publisher
            .publish(&battle, Side::Blue)
            .unwrap()
            .iter()
            .map(|f| f.to_bits())
            .collect();
        let observed = reader.decode(&words).unwrap().unwrap();
        presentation.push_publication(&words, now).unwrap();
        let frame = presentation
            .frame(now + 1. / 30., &camera, &Ui::default(), 0)
            .unwrap();
        let (batches, instances, palettes) = (
            frame.batches.clone(),
            frame.instances.clone(),
            frame.palettes.clone(),
        );
        for batch in batches {
            let row = &presentation.world().appearances[batch.appearance as usize];
            if row.lies == NONE || row.skin == 0 || batch.count < 2 {
                continue;
            }
            let rest = presentation.rest_palette(batch.appearance).unwrap();
            let bodies = &instances[batch.first as usize..(batch.first + batch.count) as usize];
            let first = bodies[0].data[0] as usize;
            assert_ne!(first, 0, "a battle corpse names a real rest palette");
            for body in bodies {
                let palette = body.data[0] as usize;
                assert_eq!(palette, first, "bodies of one corpse row share one palette");
                assert_eq!(
                    &palettes[palette..palette + rest.len()],
                    rest.as_slice(),
                    "a battle corpse's palette equals its row's rest palette"
                );
            }
            assert!(observed.corpses.len() >= bodies.len());
            return;
        }
        battle.step();
    }
    panic!("the battle must draw two fallen soldiers of one corpse row");
}
