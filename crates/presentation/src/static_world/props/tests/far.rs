//! The far mesh: a prop whose bundle carries one draws it below `far_px` and
//! casts with it; a prop without one never asks for it.
use super::*;
use crate::records::CAST_PASS;
use crate::view::{Camera, FAR};

/// The test's own thresholds: tiers at 200, 80 and 30 px, the far mesh under 12.
const FAR_PX: f64 = 12.0;

fn detail() -> Detail {
    Detail {
        lod_px: [200.0, 80.0, 30.0],
        impostor_px: 10.0,
        far_px: FAR_PX,
        shadow_caster_px: 0.0,
    }
}

/// One car at the origin drawn from art with `meshes` meshes, 2 m tall.
fn one_car(meshes: usize) -> PropPopulation {
    let mut input = input();
    input.detail = detail();
    input.appearances = vec![Art {
        bounds: Corners {
            min: [-1.0, -1.0, 0.0],
            max: [1.0, 1.0, 2.0],
        },
        meshes,
        ..art("test_car", "car", None, [1.0, 1.0, 1.0])
    }];
    let props = vec![MapProp {
        id: 1.0,
        body: PropBox {
            center: [0.0, 0.0],
            base_z: 0.0,
            yaw: 0.0,
            ..body("car", [1.0, 1.0, 1.0])
        },
    }];
    let mut p = PropPopulation::new(PropArt::new(layout(), input).unwrap(), props);
    p.rows = (0..p.looks.list.len() as u32).collect();
    p
}

/// A level camera whose eye is where the car's model, `size` metres, projects
/// `px` pixels: the tier rule's own measure.
fn at_px(p: &PropPopulation, px: f64) -> View {
    let m = &p.map[0];
    let z = m.size / 2.0;
    let probe = View::new(&camera([0.0, -10.0, z], z));
    let d = m.size * probe.pixels_per_metre / px;
    View::new(&camera([0.0, -d, z], z))
}

fn camera(eye: [f64; 3], z: f64) -> Camera {
    Camera {
        eye,
        target: [0.0, 0.0, z],
        up: [0.0, 0.0, 1.0],
        fov_y: 0.8,
        near: 1.0,
        size: [1280, 720],
    }
}

/// Every non-empty list: pass, tier and how many records.
fn lists(p: &PropPopulation) -> Vec<(u32, u32, usize)> {
    let mut out = Vec::new();
    p.each_list(&mut |l| {
        if !l.records.is_empty() {
            out.push((l.pass, l.tier, l.records.len()));
        }
    });
    out
}

fn drawn_tiers(p: &PropPopulation) -> Vec<u32> {
    lists(p)
        .into_iter()
        .filter(|l| l.0 == VIEW_PASS)
        .map(|l| l.1)
        .collect()
}

#[test]
fn a_far_carrying_prop_draws_its_far_mesh_just_below_the_threshold() {
    let mut p = one_car(MESH_SLOTS);
    p.select(&at_px(&p, FAR_PX * 0.95), None);
    assert_eq!(drawn_tiers(&p), [FAR as u32]);
    p.select(&at_px(&p, FAR_PX * 1.05), None);
    assert_eq!(drawn_tiers(&p), [3]);
}

#[test]
fn a_prop_without_a_far_mesh_draws_tier_3_at_every_small_size_and_never_a_tier_it_lacks() {
    let mut p = one_car(TIERS);
    for px in [29.0, FAR_PX * 1.05, FAR_PX * 0.95, 3.0, 0.5, 0.05] {
        p.select(&at_px(&p, px), None);
        assert_eq!(drawn_tiers(&p), [3], "at {px} px");
    }
    for px in [400.0, 150.0, 50.0, 20.0, 1.0] {
        p.select(&at_px(&p, px), None);
        assert!(
            drawn_tiers(&p).iter().all(|&t| (t as usize) < TIERS),
            "at {px} px: {:?}",
            lists(&p)
        );
    }
}

/// One caster volume over the whole test world: every prop casts into cascade 0.
fn everywhere() -> CasterVolumes {
    let mut m = crate::shadow::IDENTITY;
    (m[0], m[5], m[10]) = (1e-4, 1e-4, 1e-4);
    CasterVolumes::new(&[crate::shadow::camera_record(
        &m, [0.0; 3], 0.1, [64.0; 2], 0.0,
    )])
}

fn cast_tiers(p: &PropPopulation) -> Vec<u32> {
    lists(p)
        .into_iter()
        .filter(|l| l.0 == CAST_PASS)
        .map(|l| l.1)
        .collect()
}

#[test]
fn a_prop_casts_with_its_far_mesh_only_while_it_draws_it() {
    let mut p = one_car(MESH_SLOTS);
    let shadow = everywhere();
    p.select(&at_px(&p, FAR_PX * 0.95), Some(&shadow));
    assert_eq!(
        (drawn_tiers(&p), cast_tiers(&p)),
        (vec![FAR as u32], vec![FAR as u32])
    );
    p.select(&at_px(&p, FAR_PX * 1.05), Some(&shadow));
    assert_eq!((drawn_tiers(&p), cast_tiers(&p)), (vec![3], vec![3]));
    // Looking away at the same distance: drawn nowhere, it still casts, with tier 3.
    let behind = at_px(&p, FAR_PX * 0.95);
    let away = View::new(&Camera {
        target: [0.0, 2.0 * behind.eye[1], behind.eye[2]],
        ..camera(behind.eye, behind.eye[2])
    });
    p.select(&away, Some(&shadow));
    assert_eq!((drawn_tiers(&p), cast_tiers(&p)), (vec![], vec![3]));
}

#[test]
fn a_tiny_visible_far_drawn_prop_still_skips_its_shadow() {
    let mut p = one_car(MESH_SLOTS);
    p.art.detail.shadow_caster_px = 1e9;
    p.select(&at_px(&p, FAR_PX * 0.95), Some(&everywhere()));
    assert_eq!(
        (drawn_tiers(&p), cast_tiers(&p)),
        (vec![FAR as u32], vec![])
    );
}
