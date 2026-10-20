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
                "wall": { "drawn_by": "wall", "modular": true },
                "car": { "drawn_by": "car" },
                "heavy_wreck": { "drawn_by": "wreck" },
                "lamp": { "drawn_by": "lamp" },
                "trunk": { "drawn_by": "forest" },
                "part": { "drawn_by": "building", "map_only": true },
                "deck": { "drawn_by": "deck", "map_only": true },
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
        stand_in_tints: Some(
            [
                ("default".to_owned(), [0.5, 0.5, 0.5]),
                ("lamp".to_owned(), [1.0, 1.0, 0.0]),
            ]
            .into(),
        ),
        family: None,
        detail: Detail {
            lod_px: [200.0, 80.0, 30.0],
            impostor_px: 10.0,
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
    assert_eq!(p.stats(), [2, 0, 0, 2, 1]);
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
