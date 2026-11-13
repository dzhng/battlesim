//! Smoke in fog of war's sight: a cloud that comes, steps or goes rebuilds
//! the eye maps it reaches, and only those, oldest first under the moved
//! eyes' budget; the renderer is handed the circles only when they change.
use contract::smoke::CloudState;
use presentation::fog::sight::{Eye, FogSight, Geometry, Sensors, Shape, SightWorld};
use presentation::records::Camera;

fn geometry(rebuild_eyes_per_frame: u32) -> Geometry {
    Geometry {
        azimuth_bins: 8,
        terrain_azimuth_bins: 8,
        first_bin_m: 1.0,
        terrain_step_m: [1.0, 5.0],
        terrain_step_fraction: 0.1,
        tile_eyes_max: 8,
        face_probe_m: 0.1,
        roof_reach_m: 0.0,
        rebuild_eyes_per_frame,
        whole_step_m: 1.0,
    }
}

fn fog(rebuild_eyes_per_frame: u32, eyes: &[Eye]) -> FogSight {
    let mut fog = FogSight::new(geometry(rebuild_eyes_per_frame)).unwrap();
    let world = SightWorld {
        nx: 2,
        ny: 2,
        spacing: 5000.0,
        playable: [-5000.0, -5000.0, 10000.0, 10000.0],
        sensors: Sensors {
            fog_target_height_m: 1.0,
            foliage_full_block: 1.0,
            min_sight_gap_m: 0.0,
        },
        map: vec![],
        ids: vec![],
        occluding: Default::default(),
    };
    fog.set_world(world, vec![]).unwrap();
    fog.set_sight(eyes, Some(&[])).unwrap();
    // New eyes build at once, past any budget.
    assert_eq!(frame(&mut fog).len(), eyes.len());
    fog
}

/// An eye at (x, y) that sees `range` metres every way.
fn eye(key: &str, x: f64, y: f64, range: f64) -> Eye {
    Eye {
        key: key.into(),
        position: [x, y, 2.0],
        forward: 0.0,
        shape: Shape {
            front: 1.0,
            side: 1.0,
            rear: 1.0,
        },
        range,
    }
}

/// Cloud `id` at its `phase`, standing at (x, y) out to `radius`.
fn cloud(id: u32, phase: u8, x: f64, y: f64, radius: f64) -> (u32, CloudState) {
    (
        id,
        CloudState {
            phase,
            centre: [x, y],
            radius_m: radius,
            height_m: 6.0,
            depth_per_m: 0.2,
            concealment: 0.1,
        },
    )
}

/// One drawn frame: the keys of the eyes whose maps it rebuilt.
fn frame(fog: &mut FogSight) -> Vec<String> {
    let up = fog.prepare(Some(&Camera::default()), 1, 1, 0).unwrap();
    let eyes = fog.eyes();
    let rebuilt: Vec<String> = (0..up.params.rebuild_count as usize)
        .map(|r| eyes[up.rebuild[r * 3] as usize].key.clone())
        .collect();
    assert_eq!(rebuilt.len() as u32, fog.rebuilt);
    rebuilt
}

#[test]
fn a_cloud_rebuilds_only_the_maps_it_reaches_as_it_comes_steps_and_goes() {
    // The cloud's circle meets the near eye's 100 m reach, never the far one's.
    let mut fog = fog(
        48,
        &[eye("near", 0.0, 0.0, 100.0), eye("far", 400.0, 0.0, 100.0)],
    );
    let blooming = |phase, radius| [cloud(7, phase, 110.0, 0.0, radius)];
    fog.set_smoke(&blooming(0, 12.0));
    assert_eq!(frame(&mut fog), ["near"]);
    assert_eq!(frame(&mut fog), Vec::<String>::new());
    // The same state again changes nothing.
    fog.set_smoke(&blooming(0, 12.0));
    assert_eq!(frame(&mut fog), Vec::<String>::new());
    fog.set_smoke(&blooming(1, 15.0));
    assert_eq!(frame(&mut fog), ["near"]);
    fog.set_smoke(&[]);
    assert_eq!(frame(&mut fog), ["near"]);
    assert_eq!(frame(&mut fog), Vec::<String>::new());
}

#[test]
fn a_change_reaches_an_eye_through_the_larger_of_its_two_circles() {
    // 120 m off: past the near eye's reach plus the cloud's first radius,
    // within it once the cloud has grown.
    let mut fog = fog(48, &[eye("near", 0.0, 0.0, 100.0)]);
    fog.set_smoke(&[cloud(1, 0, 120.0, 0.0, 5.0)]);
    assert_eq!(frame(&mut fog), Vec::<String>::new());
    fog.set_smoke(&[cloud(1, 1, 120.0, 0.0, 25.0)]);
    assert_eq!(frame(&mut fog), ["near"]);
    fog.set_smoke(&[cloud(1, 0, 120.0, 0.0, 5.0)]);
    assert_eq!(frame(&mut fog), ["near"]);
}

#[test]
fn touched_maps_wait_their_turn_under_the_budget_and_are_never_dropped() {
    let eyes: Vec<Eye> = (0..5)
        .map(|i| eye(&i.to_string(), i as f64, 0.0, 100.0))
        .collect();
    let mut fog = fog(2, &eyes);
    fog.set_smoke(&[cloud(1, 0, 50.0, 0.0, 15.0)]);
    // Oldest first, two a frame: none skips the queue as a dropped map would.
    assert_eq!(frame(&mut fog), ["0", "1"]);
    assert_eq!(fog.stats().pending, 3);
    assert_eq!(frame(&mut fog), ["2", "3"]);
    assert_eq!(frame(&mut fog), ["4"]);
    assert_eq!(frame(&mut fog), Vec::<String>::new());
}

#[test]
fn a_moved_eye_a_cloud_also_touches_rebuilds_once() {
    let mut fog = fog(48, &[eye("a", 0.0, 0.0, 100.0)]);
    fog.set_sight(&[eye("a", 1.0, 0.0, 100.0)], None).unwrap();
    fog.set_smoke(&[cloud(1, 0, 50.0, 0.0, 15.0)]);
    assert_eq!(frame(&mut fog), ["a"]);
    assert_eq!(fog.stats().pending, 0);
    assert_eq!(frame(&mut fog), Vec::<String>::new());
}

#[test]
fn the_renderer_is_handed_the_circles_only_when_they_change() {
    let mut fog = fog(48, &[eye("a", 0.0, 0.0, 100.0)]);
    let upload = |fog: &mut FogSight| fog.prepare(Some(&Camera::default()), 1, 1, 0).unwrap();
    let clouds = [cloud(1, 0, 50.0, 0.0, 15.0), cloud(2, 3, -50.0, 10.0, 12.0)];
    fog.set_smoke(&clouds);
    let up = upload(&mut fog);
    let sent = up.smoke.expect("new circles are sent");
    assert_eq!(up.params.smoke_count, 2);
    for (record, (_, c)) in sent.iter().zip(&clouds) {
        assert_eq!(record.center, c.centre.map(|v| v as f32));
        assert_eq!(record.radius, c.radius_m as f32);
        assert_eq!(record.height, c.height_m as f32);
        assert_eq!(record.depth_per_m, c.depth_per_m as f32);
    }
    let held = upload(&mut fog);
    assert!(held.smoke.is_none());
    assert_eq!(held.params.smoke_count, 2, "the GPU still holds two");
    // A renderer that lost its tables is sent them again.
    fog.resend();
    assert_eq!(upload(&mut fog).smoke.map(|s| s.len()), Some(2));
    fog.set_smoke(&[]);
    let gone = upload(&mut fog);
    assert_eq!(gone.smoke.map(|s| s.len()), Some(0));
    assert_eq!(gone.params.smoke_count, 0);
}
