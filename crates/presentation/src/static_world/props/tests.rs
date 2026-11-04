use super::*;
use crate::view::Camera;

/// Walls (modular), cars (painted), wrecks, a lamp with no art, a
/// forest's trunk and a building's part; a car and a wall stop movers, a
/// lamp and a deck don't.
fn layout() -> PropLayout {
    PropLayout::parse(
        &serde_json::json!({
            "propKinds": ["wall", "car", "heavy_wreck", "lamp", "trunk", "part", "deck"],
            "propAppearance": {
                "wall": { "drawn_by": "wall", "stand_in_tint": [0.5, 0.5, 0.5], "modular": true },
                "car": { "drawn_by": "car", "stand_in_tint": [0.5, 0.5, 0.5] },
                "heavy_wreck": { "drawn_by": "wreck", "stand_in_tint": [0.5, 0.5, 0.5] },
                "lamp": { "drawn_by": "lamp", "stand_in_tint": [1.0, 1.0, 0.0] },
                "trunk": { "drawn_by": "forest", "stand_in_tint": [0.5, 0.5, 0.5] },
                "part": { "drawn_by": "building", "stand_in_tint": [0.5, 0.5, 0.5], "map_only": true },
                "deck": { "drawn_by": "deck", "stand_in_tint": [0.5, 0.5, 0.5], "map_only": true },
            },
            "blockingPropKinds": { "wheeled": ["wall", "car", "heavy_wreck", "part"] },
            "unitKinds": ["tank", "jeep"],
            "unitAppearance": { "tank": "tank_model" },
        })
        .to_string(),
    )
    .unwrap()
}

fn art(name: &str, scenery: &str, family: Option<&str>, footprint: [f64; 3]) -> Art {
    Art {
        name: name.into(),
        scenery: Some(scenery.into()),
        family: family.map(Into::into),
        footprint,
        paints: None,
        // Drawn 2 m long, reaching 1 m from its foot.
        bounds: Corners {
            min: [-1.0, 0.0, 0.0],
            max: [1.0, 0.0, 0.0],
        },
    }
}

fn input() -> PropInput {
    let mut car = art("car_a", "car", None, [2.0, 1.0, 0.8]);
    car.paints = Some(vec![[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]);
    PropInput {
        appearances: vec![
            art("wall_long", "wall", None, [2.0, 0.2, 1.0]),
            art("wall_short", "wall", None, [0.5, 0.2, 1.0]),
            car,
            art("car_b", "car", None, [4.0, 2.0, 1.5]),
            art("car_paris", "car", Some("paris"), [2.0, 1.0, 0.8]),
            art("car_china", "car", Some("china"), [2.0, 1.0, 0.8]),
            art("tank_wreck", "wreck", None, [3.5, 1.8, 1.2]),
            art("deck_a", "deck", None, [5.0, 2.0, 0.5]),
        ],
        wrecks: [("tank_model".to_owned(), "tank_wreck".to_owned())].into(),
        stand_in: Some(Corners {
            min: [-0.5, -0.5, 0.0],
            max: [0.5, 0.5, 1.0],
        }),
        family: None,
        detail: Detail {
            lod_px: [200.0, 80.0, 30.0],
            impostor_px: 10.0,
            shadow_caster_px: 0.0,
        },
        dynamic: true,
    }
}

fn body(kind: &str, half: [f64; 3]) -> PropBox {
    PropBox {
        kind: kind.into(),
        center: [10.0, 20.0],
        yaw: 0.3,
        half,
        base_z: 1.5,
        wreck_of: None,
    }
}

/// The models `b` is drawn as, by look.
fn drawn(art: &PropArt, b: &PropBox, authored: Option<f64>) -> Vec<(Look, Record)> {
    let (mut looks, mut out) = (Looks::default(), Vec::new());
    art.fit(b, authored, &mut looks, &mut out);
    out.iter()
        .map(|m| (looks.list[m.look].clone(), m.record))
        .collect()
}

fn names(models: &[(Look, Record)]) -> Vec<&str> {
    models.iter().map(|(l, _)| l.0.as_str()).collect()
}

#[test]
fn a_box_takes_the_footprint_nearest_it_scaled_per_axis_onto_it() {
    let art = PropArt::new(layout(), input()).unwrap();
    let big = drawn(&art, &body("car", [4.2, 2.1, 1.4]), Some(1.0));
    assert_eq!(names(&big), ["car_b"]);
    let r = big[0].1;
    assert_eq!(&r[..4], &[10.0, 20.0, 1.5, 0.3]);
    assert_eq!(&r[12..15], &[1.05, 1.05, (1.4f64 / 1.5) as f32]);
    // Unpainted art wears no tint; a mesh draws no card.
    assert_eq!(&r[8..12], &[1.0, 1.0, 1.0, -1.0]);
    assert_eq!(
        names(&drawn(&art, &body("car", [2.0, 1.0, 0.8]), None)),
        ["car_a"]
    );
}

#[test]
fn a_maps_region_draws_its_own_looks_and_never_anothers() {
    let paris = PropArt::new(
        layout(),
        PropInput {
            family: Some("paris".into()),
            ..input()
        },
    )
    .unwrap();
    assert_eq!(
        names(&drawn(&paris, &body("car", [4.0, 2.0, 1.5]), None)),
        ["car_paris"]
    );
    let none = PropArt::new(layout(), input()).unwrap();
    let mut looks = Looks::default();
    none.looks(&[], &mut looks);
    assert!(looks
        .list
        .iter()
        .all(|(n, ..)| n != "car_paris" && n != "car_china"));
}

#[test]
fn a_wreck_is_its_own_units_whatever_box_it_lies_on() {
    let art = PropArt::new(layout(), input()).unwrap();
    let mut wreck = body("heavy_wreck", [9.0, 9.0, 9.0]);
    wreck.wreck_of = Some("tank".into());
    assert_eq!(names(&drawn(&art, &wreck, None)), ["tank_wreck"]);
    // A unit type with no wreck of its own is the stand-in box.
    wreck.wreck_of = Some("jeep".into());
    assert_eq!(names(&drawn(&art, &wreck, None)), [STAND_IN_KIT]);
}

#[test]
fn a_module_repeats_along_the_boxs_long_side_unstretched() {
    let art = PropArt::new(layout(), input()).unwrap();
    // 10 m long along its x: the nearest whole number of 2 m modules (2.5
    // rounds up), each fitted to a third of it.
    let wall = drawn(&art, &body("wall", [5.0, 0.2, 1.0]), None);
    assert_eq!(names(&wall), ["wall_long"; 3]);
    // 4.5 m along its y: five short modules turned to it, end to end.
    let wall = drawn(&art, &body("wall", [0.2, 2.25, 1.0]), None);
    assert_eq!(names(&wall), ["wall_short"; 5]);
    let yaw = 0.3 + FRAC_PI_2;
    for (k, (_, r)) in wall.iter().enumerate() {
        let along = -2.25 + 0.9 * (k as f64 + 0.5);
        assert_eq!(r[0], (10.0 + along * libm::cos(yaw)) as f32);
        assert_eq!(r[3], yaw as f32);
        assert_eq!(&r[12..15], &[0.9, 1.0, 1.0]);
    }
}

#[test]
fn a_car_keeps_its_paint_wherever_it_is_shoved() {
    let art = PropArt::new(layout(), input()).unwrap();
    let car = body("car", [2.0, 1.0, 0.8]);
    let mut shoved = car.clone();
    shoved.center = [40.0, -3.0];
    let tint = |b: &PropBox, a| drawn(&art, b, a)[0].1[8..11].to_vec();
    assert_eq!(tint(&car, Some(17.0)), tint(&shoved, Some(17.0)));
    let paints: HashSet<_> = (0..40)
        .map(|id| {
            tint(&car, Some(id as f64))
                .iter()
                .map(|c| c.to_bits())
                .collect::<Vec<_>>()
        })
        .collect();
    assert_eq!(paints.len(), 3, "every paint is worn");
}

#[test]
fn a_kind_without_art_is_the_stand_in_box_and_a_tree_or_part_never_is() {
    let art = PropArt::new(layout(), input()).unwrap();
    let lamp = drawn(&art, &body("lamp", [0.2, 0.3, 3.0]), None);
    assert_eq!(
        lamp[0].0,
        (STAND_IN_KIT.into(), STAND_IN_MODULE.into(), DRAWS_PROP)
    );
    assert_eq!(&lamp[0].1[12..15], &[0.4, 0.6, 6.0]);
    assert_eq!(&lamp[0].1[8..11], &linear([1.0, 1.0, 0.0]));
    assert!(drawn(&art, &body("trunk", [0.3, 0.3, 8.0]), None).is_empty());
    assert!(drawn(&art, &body("part", [5.0, 5.0, 5.0]), None).is_empty());
    let bare = PropArt::new(
        layout(),
        PropInput {
            stand_in: None,
            ..input()
        },
    )
    .unwrap();
    assert!(drawn(&bare, &body("lamp", [0.2, 0.3, 3.0]), None).is_empty());
}

#[test]
fn a_surface_movers_stand_on_takes_the_ground_paint() {
    let art = PropArt::new(layout(), input()).unwrap();
    let deck = drawn(&art, &body("deck", [5.0, 2.0, 0.5]), None);
    assert_eq!(deck[0].0 .2, DRAWS_PROP_GROUND);
    assert_eq!(
        drawn(&art, &body("car", [2.0, 1.0, 0.8]), None)[0].0 .2,
        DRAWS_PROP
    );
}

/// The map: a car, a wall, a deck and a forest trunk.
fn town() -> PropPopulation {
    let layout = layout();
    let record = |id: f32, kind: f32, x: f32, half: [f32; 3]| {
        [
            id, 0.0, kind, x, 0.0, 0.0, half[0], half[1], half[2], 0.0, -1.0,
        ]
    };
    let records = [
        record(1.0, 1.0, 0.0, [2.0, 1.0, 0.8]),
        record(2.0, 0.0, 20.0, [2.0, 0.2, 1.0]),
        record(3.0, 6.0, 40.0, [5.0, 2.0, 0.5]),
        record(4.0, 4.0, 60.0, [0.3, 0.3, 8.0]),
    ]
    .concat();
    let props = map_props(&records, &layout);
    let mut p = PropPopulation::new(PropArt::new(layout, input()).unwrap(), props);
    p.rows = (0..p.looks.list.len() as u32).collect();
    p
}

fn appearances(p: &PropPopulation) -> Vec<&str> {
    let mut names: Vec<_> = p.models().map(|(l, _)| l.0.as_str()).collect();
    names.sort();
    names
}

fn known(authored: Option<f64>, kind: &str, x: f64, destroyed: bool) -> KnownBody {
    KnownBody {
        authored,
        id: Some(100.0 + x),
        kind: kind.into(),
        destroyed,
        body: Some(PropBox {
            center: [x, 0.0],
            ..body(kind, [2.0, 1.0, 0.8])
        }),
    }
}

#[test]
fn a_side_draws_the_map_less_what_it_knows_replaced_plus_what_it_knows() {
    let mut p = town();
    assert_eq!(appearances(&p), ["car_a", "deck_a", "wall_long"]);
    // The car shoved, and the wall seen destroyed with nothing left.
    assert!(p.set_known(vec![
        known(Some(1.0), "car", 5.0, false),
        known(Some(2.0), "wall", 20.0, true),
    ]));
    assert_eq!(appearances(&p), ["car_a", "deck_a"]);
    let car = p.models().find(|(l, _)| l.0 == "car_a").unwrap().1;
    assert_eq!(car[0], 5.0);
    assert_eq!(&p.stats()[..5], &[2, 0, 0, 2, 1]);
    // Knowing it again changes nothing; forgetting stands the map again.
    assert!(!p.set_known(p.known.clone()));
    assert!(p.set_known(vec![]));
    assert_eq!(appearances(&p).len(), 3);
}

#[test]
fn a_renderer_installs_the_maps_looks_and_every_body_a_battle_can_leave() {
    let art = PropArt::new(layout(), input()).unwrap();
    let installs = |props: &[PropBox]| {
        let mut looks = Looks::default();
        art.looks(props, &mut looks);
        let mut names: Vec<_> = looks.list.into_iter().map(|l| l.0).collect();
        names.sort();
        names.dedup();
        names
    };
    // A deck is only ever a map's: installed for a map that has one.
    let none = installs(&[]);
    assert_eq!(
        none,
        [
            "car_a",
            "car_b",
            STAND_IN_KIT,
            "tank_wreck",
            "wall_long",
            "wall_short"
        ]
    );
    assert!(installs(&[body("deck", [5.0, 2.0, 0.5])]).contains(&"deck_a".to_owned()));
}

#[test]
fn a_second_replacement_never_stands_the_map_prop_again() {
    let mut p = town();
    // The wall fell to rubble, and that rubble was shoved: both name the
    // wall as the prop they came from.
    p.set_known(vec![
        known(Some(2.0), "wall", 20.0, true),
        known(Some(2.0), "car", 30.0, false),
    ]);
    assert_eq!(appearances(&p), ["car_a", "car_a", "deck_a"]);
}

#[test]
fn a_wreck_drawn_moving_is_left_out_until_it_lies_still() {
    let mut p = town();
    let mut wreck = known(None, "heavy_wreck", 80.0, false);
    wreck.body.as_mut().unwrap().wreck_of = Some("tank".into());
    p.set_known(vec![wreck]);
    assert!(appearances(&p).contains(&"tank_wreck"));
    assert!(p.set_moving(vec![180.0]));
    assert!(!appearances(&p).contains(&"tank_wreck"));
    assert!(p.set_moving(vec![]));
    assert!(appearances(&p).contains(&"tank_wreck"));
}

#[test]
fn a_body_watched_shoved_is_drawn_where_it_is_until_its_knowledge_moves() {
    let mut p = town();
    // A wall two modules long, known where it stood.
    let mut wall = known(Some(2.0), "wall", 20.0, false);
    wall.body.as_mut().unwrap().half = [4.0, 0.2, 1.0];
    p.set_known(vec![wall.clone()]);
    let drawn =
        |p: &PropPopulation| -> Vec<Record> { p.known_models.iter().map(|m| m.record).collect() };
    let stood = drawn(&p);
    assert_eq!(stood.len(), 2);
    // Shoved 3 m ahead of what the side knows: every module goes with it.
    let shoved = PropDisplayPose {
        prop_id: 120.0,
        center: [23.0, 0.0],
        yaw: 0.3,
        base_z: 1.5,
    };
    assert!(p.set_shoved(std::slice::from_ref(&shoved)));
    let moved = drawn(&p);
    for (m, s) in moved.iter().zip(&stood) {
        assert!((m[0] - s[0] - 3.0).abs() < 1e-4, "{m:?} from {s:?}");
        assert_eq!(m[1..4], s[1..4]);
    }
    assert!(!p.set_shoved(&[shoved]), "the same pose draws nothing new");
    // The shove ends: the wall holds where it was last seen, through the
    // side re-learning it there, and moves on with what the side learns next.
    assert!(!p.set_shoved(&[]));
    assert_eq!(drawn(&p), moved);
    wall.body.as_mut().unwrap().center = [23.0, 0.0];
    assert!(p.set_known(vec![wall.clone()]));
    assert_eq!(drawn(&p), moved);
    wall.body.as_mut().unwrap().center = [30.0, 0.0];
    assert!(p.set_known(vec![wall]));
    assert!((drawn(&p)[0][0] - stood[0][0] - 10.0).abs() < 1e-4);
}

#[test]
fn a_shove_one_stream_watched_is_forgotten_by_the_next() {
    let mut p = town();
    let car = known(Some(1.0), "car", 5.0, false);
    p.set_known(vec![car.clone()]);
    let at = |p: &PropPopulation| p.known_models[0].record[0];
    let shoved = PropDisplayPose {
        prop_id: 105.0,
        center: [9.0, 0.0],
        yaw: 0.3,
        base_z: 1.5,
    };
    p.set_shoved(&[shoved]);
    assert_eq!(at(&p), 9.0);
    // Another side that knows the car where it stood never saw it go.
    assert!(p.forget_shoved());
    assert!(!p.set_known(vec![car]));
    assert_eq!(at(&p), 5.0);
}

#[test]
fn a_view_draws_what_it_sees_by_its_projected_height() {
    let mut p = town();
    let camera = |eye: [f64; 3], target: [f64; 3]| {
        View::new(&Camera {
            eye,
            target,
            up: [0.0, 0.0, 1.0],
            fov_y: 0.8,
            near: 1.0,
            size: [1280, 720],
        })
    };
    let drawn = |p: &PropPopulation| {
        let mut out = Vec::new();
        p.each_list(&mut |l| {
            if !l.records.is_empty() {
                out.push((
                    p.looks.list[l.row as usize].0.clone(),
                    l.tier,
                    l.records.len(),
                ));
            }
        });
        out
    };
    // Close on the car, looking along +x: it is near, the far deck small.
    assert!(p.select(&camera([-8.0, 0.0, 2.0], [0.0, 0.0, 0.0]), None));
    let near = drawn(&p);
    assert!(near.contains(&("car_a".into(), 0, 1)), "{near:?}");
    assert!(
        near.iter().any(|(n, t, _)| n == "deck_a" && *t > 0),
        "{near:?}"
    );
    // Looking away, nothing is drawn; the same view again changes nothing.
    assert!(p.select(&camera([-8.0, 0.0, 2.0], [-20.0, 0.0, 0.0]), None));
    assert!(drawn(&p).is_empty());
    assert!(!p.select(&camera([-8.0, 0.0, 2.0], [-20.0, 0.0, 0.0]), None));
}

#[test]
fn caster_lists_keep_offscreen_shadows_without_changing_camera_meshes() {
    for (base_z, uphill) in [(0.0, 0.0), (-100.0, 0.0), (100.0, 0.0), (-100.0, 10.0)] {
        let offscreen_x = -80.0 - uphill * 10.0;
        use crate::records::{CAST_PASS, NONE};
        let mut input = input();
        input.detail.shadow_caster_px = 2.0;
        input.appearances = vec![Art {
            bounds: Corners {
                min: [-1.0, -1.0, 0.0],
                max: [1.0, 1.0, 10.0],
            },
            ..art("test_caster", "car", None, [1.0, 1.0, 5.0])
        }];
        let props = [offscreen_x, 0.0, -1000.0]
            .into_iter()
            .enumerate()
            .map(|(id, x)| MapProp {
                id: id as f64,
                body: PropBox {
                    center: [x, 0.0],
                    base_z: base_z + if id == 0 { uphill } else { 0.0 },
                    yaw: 0.0,
                    ..body("car", [1.0, 1.0, 5.0])
                },
            })
            .collect();
        let mut p = PropPopulation::new(PropArt::new(layout(), input).unwrap(), props);
        p.rows = (0..p.looks.list.len() as u32).collect();
        let camera_input = Camera {
            eye: [0.0, -30.0, base_z + 10.0],
            target: [0.0, 100.0, base_z],
            up: [0.0, 0.0, 1.0],
            fov_y: 0.8,
            near: 1.0,
            size: [1280, 720],
        };
        let view = View::new(&camera_input);
        let lists = |p: &PropPopulation, pass| {
            let mut out = Vec::new();
            p.each_list(&mut |l| {
                if l.pass == pass {
                    for r in l.records {
                        out.push((l.row, l.tier, *r));
                    }
                }
            });
            out
        };
        p.select(&view, None);
        let camera = lists(&p, VIEW_PASS);
        assert_eq!(
            camera.iter().map(|(_, _, r)| r[0]).collect::<Vec<_>>(),
            [0.0]
        );
        // The offscreen tower's shadow reaches x=0, in the same visible ground as the centre tower.
        assert!(view.sees_sphere([0.0, 0.0, base_z], 1.0));
        let shadow = fitted_shadow(&camera_input, base_z);
        assert!(p.select(&view, Some(&shadow)));
        assert_eq!(lists(&p, VIEW_PASS), camera);
        let mut casters = lists(&p, CAST_PASS);
        casters.sort_by(|a, b| a.2[0].total_cmp(&b.2[0]));
        casters.dedup();
        assert_eq!(
            casters.iter().map(|(_, _, r)| r[0]).collect::<Vec<_>>(),
            [offscreen_x as f32, 0.0]
        );
        p.each_list(&mut |l| {
            assert_eq!(l.caster, if l.pass == CAST_PASS { l.tier } else { NONE });
        });
        assert!(
            !p.select(&view, Some(&shadow)),
            "unchanged lists retain their publication"
        );
        p.select(&view, None);
        assert!(lists(&p, CAST_PASS).is_empty());
        assert_eq!(lists(&p, VIEW_PASS), camera);
    }
}

#[test]
fn visible_props_outside_receiver_reach_keep_their_camera_mesh_only() {
    use crate::records::CAST_PASS;
    let mut p = town();
    p.set_known(vec![known(None, "car", 1000.0, false)]);
    let camera_input = Camera {
        eye: [-30.0, 0.0, 10.0],
        target: [100.0, 0.0, 0.0],
        up: [0.0, 0.0, 1.0],
        fov_y: 0.8,
        near: 1.0,
        size: [1280, 720],
    };
    let view = View::new(&camera_input);
    let selected = |p: &PropPopulation, pass| {
        let mut out = Vec::new();
        p.each_list(&mut |l| {
            if l.pass == pass {
                out.extend(l.records.iter().map(|r| (l.row, l.tier, *r)));
            }
        });
        out
    };
    p.select(&view, None);
    let camera = selected(&p, VIEW_PASS);
    assert!(camera.iter().any(|(_, _, r)| r[0] == 1000.0));
    p.select(&view, Some(&fitted_shadow(&camera_input, 0.0)));
    assert_eq!(selected(&p, VIEW_PASS), camera);
    let mut xs: Vec<_> = selected(&p, CAST_PASS)
        .iter()
        .map(|(_, _, r)| r[0])
        .collect();
    xs.sort_by(f32::total_cmp);
    xs.dedup();
    assert_eq!(xs, [0.0, 20.0, 40.0]);
}

fn fitted_shadow(camera: &Camera, base_z: f64) -> CasterVolumes {
    use crate::static_world::ground::{
        mesh,
        partition::{TerrainPartition, CELL_M},
    };
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/fixtures/shadow-vectors.json")).unwrap();
    let mut light: crate::light::LightSettings =
        serde_json::from_value(fixture["lights"]["game"].clone()).unwrap();
    light.sun_azimuth = std::f64::consts::PI;
    light.sun_elevation = libm::atan(0.1_f64);
    light.cascades.max_far_m = 100.;
    let z = base_z as f32;
    let terrain = TerrainPartition::new(
        &mesh::terrain(
            &[
                -100., -100., z, 100., -100., z, 100., 100., z, -100., 100., z,
            ],
            &[0, 1, 2, 0, 2, 3],
            None,
        ),
        CELL_M,
    );
    let fit = crate::shadow::sun_shadow(&light, camera, Some(&terrain));
    CasterVolumes::new(&fit.casters)
}

#[test]
fn spatial_query_preserves_complete_lists_through_remote_growth_and_knowledge() {
    let camera = Camera {
        eye: [0., -30., 10.],
        target: [0., 100., 0.],
        up: [0., 0., 1.],
        fov_y: 0.8,
        near: 1.,
        size: [1280, 720],
    };
    let shadow = fitted_shadow(&camera, 0.);
    let mut baseline = Vec::new();
    for remote in [64, 1024, 4096] {
        let mut sample = 0;
        let props: Vec<_> = (0..remote + 4)
            .map(|i| MapProp {
                id: i as f64,
                body: PropBox {
                    center: [
                        if i < 4 {
                            i as f64 * 15. - 80.
                        } else {
                            10000. + i as f64 * 128.
                        },
                        0.,
                    ],
                    ..body("car", [2., 1., 5.])
                },
            })
            .collect();
        let make = || PropPopulation::new(PropArt::new(layout(), input()).unwrap(), props.clone());
        let (mut indexed, mut brute) = (make(), make());
        for state in 0..4 {
            let knowledge = match state {
                1 => vec![
                    known(Some(0.), "car", 0., false),
                    known(Some(1.), "car", -80., true),
                ],
                2 => (0..remote)
                    .map(|i| {
                        known(
                            None,
                            "car",
                            if i == 0 { 5. } else { 20000. + i as f64 * 128. },
                            false,
                        )
                    })
                    .collect(),
                _ => vec![],
            };
            indexed.set_known(knowledge.clone());
            brute.set_known(knowledge);
            // Admit every original candidate, retaining the production leaf tests
            // and ordering: the brute-force walk before spatial selection.
            brute.chunk_index = crate::static_world::index::BoundsIndex::new(
                brute
                    .chunks
                    .iter()
                    .map(|_| [-1e12, -1e12, -1e12, 1e12, 1e12, 1e12]),
            );
            brute.known_index = crate::static_world::index::BoundsIndex::new(
                brute
                    .known_models
                    .iter()
                    .map(|_| [-1e12, -1e12, -1e12, 1e12, 1e12, 1e12]),
            );
            for shadow in [None, Some(&shadow)] {
                indexed.select(&View::new(&camera), shadow);
                brute.select(&View::new(&camera), shadow);
                assert_eq!(indexed.lists, brute.lists, "remote={remote} state={state}");
                assert_eq!(indexed.prints, brute.prints);
                if remote == 64 {
                    baseline.push(indexed.lists.clone());
                } else {
                    assert_eq!(
                        indexed.lists, baseline[sample],
                        "remote growth changed local output"
                    );
                }
                sample += 1;
                assert!(
                    indexed.stats()[5] < 100,
                    "local traversal grew with remote world: {:?}",
                    indexed.stats()
                );
            }
        }
    }
}

#[test]
fn prop_caster_lists_route_disjoint_and_overlapping_offscreen_models() {
    let mut input = input();
    input.appearances = vec![art("one_car", "car", None, [2., 2., 2.])];
    let props = [-200., 0., 200., 1000.]
        .into_iter()
        .enumerate()
        .map(|(id, x)| MapProp {
            id: id as f64,
            body: PropBox {
                center: [x, 0.],
                base_z: 0.,
                yaw: 0.,
                ..body("car", [2., 2., 2.])
            },
        })
        .collect();
    let mut p = PropPopulation::new(PropArt::new(layout(), input).unwrap(), props);
    p.rows = (0..p.looks.list.len() as u32).collect();
    let camera = Camera {
        eye: [0., -30., 10.],
        target: [0., 100., 0.],
        up: [0., 0., 1.],
        fov_y: 0.8,
        near: 1.,
        size: [640, 480],
    };
    let view = View::new(&camera);
    let casters = [-200., 0., -100., 5000.].map(|x| {
        let mut m = crate::shadow::IDENTITY;
        m[0] = 1. / 110.;
        m[5] = 1. / 110.;
        m[10] = 1. / 100.;
        m[12] = -x / 110.;
        crate::shadow::camera_record(&m, [0.; 3], 0.1, [64.; 2], 0.)
    });
    p.select(&view, Some(&CasterVolumes::new(&casters)));
    let mut xs: [Vec<f32>; 4] = Default::default();
    let mut visible = Vec::new();
    p.each_list(&mut |l| {
        if l.pass == CAST_PASS {
            xs[l.cascade_mask.trailing_zeros() as usize].extend(l.records.iter().map(|r| r[0]));
        } else {
            visible.extend(l.records.iter().map(|r| r[0]));
        }
    });
    for list in &mut xs {
        list.sort_by(f32::total_cmp);
    }
    assert_eq!(xs, [vec![-200.], vec![0.], vec![-200., 0.], vec![]]);
    assert_eq!(visible, [0.]);
}

#[test]
fn tiny_visible_prop_skips_shadow_only_when_control_is_enabled() {
    let props = vec![MapProp {
        id: 1.0,
        body: PropBox {
            center: [0.0, 0.0],
            base_z: 0.0,
            yaw: 0.0,
            ..body("car", [1.0, 1.0, 1.0])
        },
    }];
    let camera = Camera {
        eye: [0.0, -4_000.0, 2_000.0],
        target: [0.0, 0.0, 0.0],
        up: [0.0, 0.0, 1.0],
        fov_y: 0.8,
        near: 1.0,
        size: [640, 480],
    };
    let view = View::new(&camera);
    let mut identity = crate::shadow::IDENTITY;
    identity[10] = 1.0;
    let caster = crate::shadow::camera_record(&identity, [0.0; 3], 0.1, [64.0; 2], 0.0);
    let selected = |shadow_caster_px| {
        let mut prop_input = input();
        prop_input.detail.shadow_caster_px = shadow_caster_px;
        let mut p = PropPopulation::new(PropArt::new(layout(), prop_input).unwrap(), props.clone());
        p.rows = (0..p.looks.list.len() as u32).collect();
        p.select(&view, Some(&CasterVolumes::new(&[caster])));
        let mut counts = [0, 0];
        p.each_list(&mut |list| counts[usize::from(list.pass == CAST_PASS)] += list.records.len());
        counts
    };
    assert!(selected(0.0)[1] > 0);
    assert_eq!(selected(2.0), [1, 0]);
    assert_eq!(selected(1e9), [1, 0]);
}
