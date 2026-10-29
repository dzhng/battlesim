//! The sun's cascades are fitted where shadows are received, and every map
//! point the camera sees within the reach is inside a cascade that shades
//! it. Both clients consume this production fit over immutable terrain.
use presentation::light::LightSettings;
use presentation::records::SunShadow;
use presentation::shadow::{camera_record, map_box, sun_shadow, SunShadowFrame, CASCADES};
use presentation::static_world::ground::{
    mesh,
    partition::{TerrainPartition, CELL_M},
};
use presentation::view::{Camera, View};
use serde_json::Value;

#[test]
fn a_shadow_lies_on_the_ground_with_offset_proportional_to_actual_height() {
    let fall = [2.0, -1.0];
    let low = presentation::shadow::aircraft_shadow([12.0, 34.0, 17.5], 7.5, fall);
    let high = presentation::shadow::aircraft_shadow([12.0, 34.0, 27.5], 7.5, fall);
    assert_eq!(low, [32.0, 24.0, 7.5]);
    assert_eq!(high, [52.0, 14.0, 7.5]);
}

fn bits<T: bytemuck::Pod>(record: &T) -> Vec<u32> {
    bytemuck::cast_slice::<T, u32>(std::slice::from_ref(record)).to_vec()
}

/// Flat receiver terrain, triangulated through the production mesh writer.
fn slab(west: f32, south: f32, east: f32, north: f32, z: f32) -> TerrainPartition {
    TerrainPartition::new(
        &mesh::terrain(
            &[
                west, south, z, east, south, z, east, north, z, west, north, z,
            ],
            &[0, 1, 2, 0, 2, 3],
            None,
        ),
        CELL_M,
    )
}

fn street() -> TerrainPartition {
    slab(0., 0., 1600., 1600., 0.)
}

fn game_light() -> LightSettings {
    let at = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/presentation.json"
    );
    let game: Value = serde_json::from_str(&std::fs::read_to_string(at).unwrap()).unwrap();
    serde_json::from_value(game["light"].clone()).unwrap()
}

/// An orbit camera over `target`, `distance` out at `pitch` and `yaw`, as
/// the browser's lens frames it.
fn orbit(target: [f64; 3], distance: f64, pitch: f64, yaw: f64) -> Camera {
    let (cp, sp) = (pitch.cos(), pitch.sin());
    Camera {
        eye: [
            target[0] + distance * cp * yaw.cos(),
            target[1] + distance * cp * yaw.sin(),
            target[2] + distance * sp,
        ],
        target,
        up: [-sp * yaw.cos(), -sp * yaw.sin(), cp],
        fov_y: 0.8,
        near: 1.0,
        size: [1920, 1080],
    }
}

/// The street map's cameras: ground level, the default, the street at
/// 160 m, WARNO's 420 m, the route overview and the strategic camera.
fn cameras() -> [(&'static str, Camera); 6] {
    [
        ("ground", orbit([170.0, 800.0, 0.0], 25.0, 0.22, -1.57)),
        ("default", orbit([170.0, 800.0, 0.0], 65.0, 0.85, -1.57)),
        ("game", orbit([990.0, 790.0, 0.0], 160.0, 0.85, -1.2)),
        ("warno", orbit([700.0, 800.0, 0.0], 420.0, 0.85, -1.2)),
        ("route", orbit([800.0, 800.0, 0.0], 1150.0, 0.85, -1.57)),
        ("strategic", orbit([170.0, 800.0, 0.0], 2000.0, 0.85, -1.57)),
    ]
}

/// A point through a matrix, divided by its w.
fn project(m: &[f32], p: [f64; 3]) -> [f64; 3] {
    let m: Vec<f64> = m.iter().map(|&v| f64::from(v)).collect();
    let row = |r: usize| m[r] * p[0] + m[4 + r] * p[1] + m[8 + r] * p[2] + m[12 + r];
    let w = row(3);
    [row(0) / w, row(1) / w, row(2) / w]
}

/// Every point of the map the camera sees, with its view depth.
fn visible_ground(camera: &Camera) -> Vec<([f64; 3], f64)> {
    let view = View::new(camera);
    let view_proj = multiply(&view.projection, &view.view);
    let inverse = camera_record(&view_proj, [0.0; 3], 1.0, [1.0, 1.0], 0.0).inv_view_proj;
    let n = 24;
    let mut out = vec![];
    for iy in 0..=n {
        for ix in 0..=n {
            let ndc = |i: i32| f64::from(i) / f64::from(n) * 2.0 - 1.0;
            let near = project(&inverse, [ndc(ix), ndc(iy), 1.0]);
            let dir: [f64; 3] = std::array::from_fn(|i| near[i] - camera.eye[i]);
            if dir[2] >= 0.0 {
                continue;
            }
            let t = -camera.eye[2] / dir[2];
            let p = [camera.eye[0] + dir[0] * t, camera.eye[1] + dir[1] * t, 0.0];
            if p[0] < 0.0 || p[1] < 0.0 || p[0] > 1600.0 || p[1] > 1600.0 {
                continue;
            }
            out.push((p, -project(&view.view, p)[2]));
        }
    }
    out
}

fn multiply(a: &[f32; 16], b: &[f32; 16]) -> [f32; 16] {
    std::array::from_fn(|i| {
        let (c, r) = (i / 4, i % 4);
        (0..4)
            .map(|k| f64::from(a[k * 4 + r]) * f64::from(b[c * 4 + k]))
            .sum::<f64>() as f32
    })
}

struct Cascade<'a> {
    matrix: &'a [f32],
    interval: [f64; 2],
}

fn cascades(r: &SunShadow, active: usize) -> Vec<Cascade<'_>> {
    (0..active)
        .map(|i| {
            let c = &r.cascades[i];
            Cascade {
                matrix: &c.matrix,
                interval: [f64::from(c.interval[0]), f64::from(c.interval[1])],
            }
        })
        .collect()
}

/// The receiver shader's blend weight of a cascade at a receiver's depth
/// (as a fraction of the receiver range): a private oracle of
/// `shaders/shadow.ts`'s fade.
fn blend(depth: f64, [start, end]: [f64; 2], first: bool, last: bool) -> f64 {
    let centre = (start + end) / 2.0;
    let edge = if depth < centre { start } else { end };
    let margin = 0.25 * edge * edge;
    let low = start - margin / 2.0;
    let high = if last { end } else { end + margin / 2.0 };
    if depth < low || depth > high {
        return 0.0;
    }
    if (first && depth <= centre) || margin <= 0.0 {
        return 1.0;
    }
    ((depth - low).min(high - depth) / margin).clamp(0.0, 1.0)
}

#[test]
fn every_map_point_seen_within_the_reach_is_inside_a_cascade_that_shades_it() {
    let light = game_light();
    let map = street();
    for (name, camera) in cameras() {
        let frame = sun_shadow(&light, &camera, Some(&map));
        assert_eq!(frame.casters.len(), CASCADES, "{name}");
        let [far, _, near, _] = frame.receiver.control.map(f64::from);
        assert!(far <= light.cascades.max_far_m, "{name}");
        let list = cascades(&frame.receiver, CASCADES);
        let last = list.len() - 1;
        let points: Vec<_> = visible_ground(&camera)
            .into_iter()
            .filter(|(_, d)| *d <= far)
            .collect();
        assert!(points.len() > 20, "{name}");
        for (p, depth) in points {
            let linear = ((depth - near) / (far - near)).max(0.0);
            let mut weight = 0.0;
            for (i, c) in list.iter().enumerate() {
                let w = blend(linear, c.interval, i == 0, i == last);
                if w == 0.0 {
                    continue;
                }
                weight += w;
                let [u, v, z] = project(c.matrix, p);
                assert!(
                    u.abs() <= 1.0 && v.abs() <= 1.0 && (0.0..=1.0).contains(&z),
                    "{name}: {p:?} at {depth} m outside cascade {i}"
                );
            }
            if linear < list[last].interval[0] {
                assert!((weight - 1.0).abs() < 1e-6, "{name}: {p:?} weight {weight}");
            } else {
                assert!(weight > 0.0, "{name}: {p:?} unshaded");
            }
        }
    }
}

#[test]
fn each_cascade_looks_down_the_sun_and_draws_its_casters_as_its_light() {
    let light = game_light();
    let sun = light.sun_direction();
    let frame = sun_shadow(&light, &cameras()[1].1, Some(&street()));
    for (i, (caster, c)) in frame
        .casters
        .iter()
        .zip(cascades(&frame.receiver, CASCADES))
        .enumerate()
    {
        // A caster's camera is its cascade's light: the same matrix, over a
        // square viewport the size of its map.
        assert_eq!(caster.view_proj[..], *c.matrix, "cascade {i}");
        assert_eq!([caster.width, caster.height], [2048.0; 2]);
        assert_eq!(caster.znear, 1.0);
        // Along the sun a point keeps its place on the map, nearer the light.
        let p = [170.0, 800.0, 0.0];
        let up: [f64; 3] = std::array::from_fn(|k| p[k] + sun[k] * 10.0);
        let (a, b) = (project(c.matrix, p), project(c.matrix, up));
        assert!(
            (a[0] - b[0]).abs() < 1e-4 && (a[1] - b[1]).abs() < 1e-4,
            "cascade {i}"
        );
        assert!(b[2] > a[2], "reverse-Z: the light is nearer");
        // Its inverse undoes it.
        let back = project(&caster.inv_view_proj, a);
        assert!(
            (0..3).all(|k| (back[k] - p[k]).abs() < 0.05),
            "cascade {i}: {back:?}"
        );
    }
}

#[test]
fn the_strategic_camera_spends_its_cascades_on_the_map_not_the_air_above_it() {
    let light = game_light();
    let camera = cameras()[5].1;
    let frame = sun_shadow(&light, &camera, Some(&street()));
    let depths: Vec<f64> = visible_ground(&camera).iter().map(|(_, d)| *d).collect();
    let nearest = depths.iter().cloned().fold(f64::INFINITY, f64::min);
    let farthest = depths.iter().cloned().fold(0.0, f64::max);
    assert!(nearest > 1400.0);
    let [far, _, near, _] = frame.receiver.control.map(f64::from);
    assert!(near > 0.95 * nearest, "{near} against {nearest}");
    assert!(far >= farthest.min(light.cascades.max_far_m) - 1e-3);
    // About two metres a texel or finer: the normal bias is its texels'
    // width per texel of PCF radius.
    for c in 0..CASCADES {
        let [normal, radius] = [1, 2].map(|k| f64::from(frame.receiver.cascades[c].bias[k]));
        let texel = normal / (light.cascades.normal_bias_texels * radius);
        assert!(texel < 2.5, "cascade {c}: {texel} m a texel");
    }
}

#[test]
fn a_dropping_camera_widens_the_range_and_without_a_map_it_collapses_past_the_near_plane() {
    let light = game_light();
    let map = street();
    let range = |c: &Camera, b: Option<&TerrainPartition>| {
        let [far, _, near, _] = sun_shadow(&light, c, b).receiver.control;
        [near, far]
    };
    let high = range(&orbit([800.0, 800.0, 0.0], 1000.0, 0.85, -1.57), Some(&map));
    let low = range(&orbit([800.0, 800.0, 0.0], 25.0, 0.22, -1.57), Some(&map));
    assert!(
        high[0] > 500.0,
        "a high camera's range starts at the map: {high:?}"
    );
    assert!(low[0] < high[0]);
    assert!(low[1] / low[0] > high[1] / high[0]);
    // Just past the near plane: 1 m to 1.5 times it.
    let none = range(&orbit([800.0, 800.0, 0.0], 1000.0, 0.85, -1.57), None);
    let away = range(
        &orbit([800.0, 800.0, 5000.0], 100.0, -0.3, -1.57),
        Some(&map),
    );
    assert_eq!(none, [1.0, 2.0]);
    assert_eq!(away, [1.0, 2.0]);
}

#[test]
fn a_cold_frame_shades_nothing_and_casts_nothing() {
    let light = game_light();
    let cold = SunShadowFrame::cold(&light);
    assert!(cold.casters.is_empty());
    for c in cascades(&cold.receiver, CASCADES) {
        assert_eq!(c.interval, [1.0, 1.0]);
    }
}

#[test]
fn remote_ground_growth_preserves_actual_fitted_cascade_records() {
    let light = game_light();
    // Append triangles to one fixed local mesh, not a resized/recompiled
    // map: that can retessellate local ground and is a different workload.
    let ground = |east: f32, elevation: f32| {
        let mut mesh = Vec::new();
        let mut quad = |west, east, z| {
            for [x, y] in [
                [west, 0.0],
                [east, 0.0],
                [east, 8192.0],
                [west, 0.0],
                [east, 8192.0],
                [west, 8192.0],
            ] {
                mesh.extend_from_slice(&[x, y, z, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]);
            }
        };
        quad(0.0, 8192.0, 0.0);
        if east > 8192.0 {
            quad(8192.0, east, elevation);
        }
        mesh
    };
    let local_mesh = ground(8192.0, 0.0);
    let local_box = map_box(&local_mesh).unwrap();
    for (distance, pitch) in [(65.0, 0.85), (260.0, 0.85), (2400.0, 0.95)] {
        let camera = orbit([4096.0, 4096.0, 0.0], distance, pitch, -1.57);
        let local = sun_shadow(
            &light,
            &camera,
            Some(&TerrainPartition::new(&local_mesh, CELL_M)),
        );
        assert!(!local.casters.is_empty());
        for (east, elevation) in [
            (16384.0, 0.0),
            (32768.0, 0.0),
            (16384.0, 100.0),
            (32768.0, 100.0),
        ] {
            let mesh = ground(east, elevation);
            assert_eq!(&mesh[..local_mesh.len()], local_mesh.as_slice());
            let grown_box = map_box(&mesh).unwrap();
            assert!(grown_box[3] > local_box[3]);
            assert!(
                !View::new(&camera).sees_box(&[
                    8192.0,
                    0.0,
                    elevation as f64,
                    east as f64,
                    8192.0,
                    elevation as f64
                ]),
                "added terrain must be outside the camera"
            );
            let grown = sun_shadow(&light, &camera, Some(&TerrainPartition::new(&mesh, CELL_M)));
            assert_eq!(
                bits(&local.receiver),
                bits(&grown.receiver),
                "receiver at {distance} m, east {east}"
            );
            assert_eq!(local.casters.len(), grown.casters.len());
            for (i, (a, b)) in local.casters.iter().zip(&grown.casters).enumerate() {
                assert_eq!(bits(a), bits(b), "caster {i} at {distance} m, east {east}");
            }
        }
    }
}

#[test]
fn local_elevation_changes_the_production_frame_fit() {
    use presentation::presentation::{Presentation, Ui};
    use std::sync::Arc;
    let light = game_light();
    let camera = orbit([800., 800., 0.], 260., 0.85, -1.57);
    let mut presentation = Presentation::new(None, 30.).unwrap();
    let mut fit = |terrain| {
        presentation.set_shadow(Some((light.clone(), Arc::new(terrain))));
        presentation
            .frame(0., &camera, &Ui::default(), 0)
            .unwrap()
            .shadow
            .clone()
            .unwrap()
    };
    let low = fit(street());
    let raised = fit(slab(0., 0., 1600., 1600., 100.));
    assert_ne!(bits(&low.receiver), bits(&raised.receiver));
    assert!(raised.receiver.control[2] < low.receiver.control[2]);
    assert_eq!(
        fit(street()),
        low,
        "replacing terrain must discard the prior fit"
    );
}

#[test]
fn receivers_at_the_horizon_and_between_screen_samples_are_not_lost() {
    let mut light = game_light();
    light.cascades.max_far_m = 100_000.;
    let camera = Camera {
        eye: [0., 0., 2.],
        target: [0., 1., 2.],
        up: [0., 0., 1.],
        fov_y: 0.8,
        near: 1.,
        size: [1000, 1000],
    };
    // Tiny elevated patch between the old screen-ray grid's sample lines.
    // Its distance exceeds the game's ordinary shadow reach: use an explicit
    // test reach to prove fitting itself does not introduce a distance cutoff.
    let terrain = slab(731., 20_000., 732., 20_001., 57.);
    let fit = sun_shadow(&light, &camera, Some(&terrain));
    let [far, _, near, _] = fit.receiver.control;
    assert!(near > 19_999. && near <= 20_000.);
    assert!(far >= 20_001.);
    let p = [731.5, 20_000.5, 57.];
    assert!(fit.casters.iter().any(|c| {
        let [x, y, z] = project(&c.view_proj, p);
        x.abs() <= 1. && y.abs() <= 1. && (0.0..=1.).contains(&z)
    }));
}

#[test]
fn near_plane_inside_receiver_and_empty_terrain_have_finite_fits() {
    let light = game_light();
    let camera = Camera {
        eye: [0., 0., 2.],
        target: [0., 1., 2.],
        up: [0., 0., 1.],
        fov_y: 0.8,
        near: 1.,
        size: [1000, 1000],
    };
    let terrain = slab(-100., -100., 100., 100., 0.);
    let fit = sun_shadow(&light, &camera, Some(&terrain));
    assert_eq!(fit.receiver.control[2], 1.);
    assert!(fit.receiver.control[0] >= 100.);
    let empty = sun_shadow(&light, &camera, Some(&TerrainPartition::new(&[], CELL_M)));
    assert_eq!(empty, sun_shadow(&light, &camera, None));
    for f in [&fit, &empty] {
        assert!(
            bytemuck::cast_slice::<_, f32>(std::slice::from_ref(&f.receiver))
                .iter()
                .all(|x| x.is_finite())
        );
    }
}

#[test]
fn an_offscreen_triangle_in_the_same_tile_cannot_expand_local_receivers() {
    let light = game_light();
    let camera = Camera {
        eye: [2., 2., 60.],
        target: [2., 2., 0.],
        up: [0., 1., 0.],
        fov_y: 0.8,
        near: 1.,
        size: [1000, 1000],
    };
    let local = slab(1., 1., 3., 3., 0.);
    let mut mesh = local.vertices().to_vec();
    mesh.extend_from_slice(slab(200., 200., 201., 201., 100.).vertices());
    assert_eq!(
        sun_shadow(&light, &camera, Some(&local)),
        sun_shadow(&light, &camera, Some(&TerrainPartition::new(&mesh, CELL_M))),
    );
    // Both triangles of the offscreen patch share the local tile. They incur
    // triangle-bound tests but are rejected before exact depth clipping.
    let mut p = presentation::presentation::Presentation::new(None, 30.).unwrap();
    p.set_shadow(Some((
        light,
        std::sync::Arc::new(TerrainPartition::new(&mesh, CELL_M)),
    )));
    p.frame(0., &camera, &Default::default(), 0).unwrap();
    let stats = p.shadow_stats();
    assert_eq!(
        (stats.bounds_visited, stats.triangle_tests, stats.clip_tests),
        (1, 4, 2)
    );
}

#[test]
fn caster_mask_keeps_disjoint_overlap_and_boundary_membership() {
    use presentation::shadow::{camera_record, CasterVolumes};
    let camera = |x: f32| {
        let mut m = [0.; 16];
        for i in [0, 5, 10, 15] {
            m[i] = 1.;
        }
        m[12] = -x;
        camera_record(&m, [0.; 3], 0.1, [64.; 2], 0.)
    };
    let volumes = CasterVolumes::new(&[camera(0.), camera(2.), camera(10.), camera(20.)]);
    for (bounds, expected) in [
        ([-0.5, -0.5, 0.2, 0.5, 0.5, 0.8], 1),
        ([1.5, -0.5, 0.2, 2.5, 0.5, 0.8], 2),
        ([0.5, -0.5, 0.2, 1.5, 0.5, 0.8], 3),
        ([1., 0., 0., 1., 0., 1.], 3),
        ([9.5, -0.5, 0.2, 10.5, 0.5, 0.8], 4),
        ([19.5, -0.5, 0.2, 20.5, 0.5, 0.8], 8),
        ([30., 0., 0., 31., 1., 1.], 0),
    ] {
        assert_eq!(volumes.mask(&bounds), expected);
        assert_eq!(volumes.intersects(&bounds), expected != 0);
    }
    assert_eq!(CasterVolumes::new(&[]).mask(&[0.; 6]), 0);
}

#[test]
fn production_shadow_hold_reuses_fit_and_camera_changes_refit() {
    use presentation::presentation::{Presentation, Ui};
    use std::sync::Arc;
    let light = game_light();
    let terrain = Arc::new(street());
    let mut p = Presentation::new(None, 30.).unwrap();
    p.set_shadow(Some((light.clone(), terrain.clone())));
    let mut camera = cameras()[1].1;
    for change in 0..8 {
        match change {
            1 => camera.eye[0] += 1.,
            2 => camera.target[1] += 1.,
            3 => camera.up[0] += 0.01,
            4 => camera.fov_y += 0.01,
            5 => camera.near += 0.1,
            6 => camera.size[0] += 10,
            7 => camera.size[1] += 10,
            _ => (),
        }
        let expected = sun_shadow(&light, &camera, Some(&terrain));
        assert_eq!(
            p.frame(0., &camera, &Ui::default(), 0)
                .unwrap()
                .shadow
                .as_ref(),
            Some(&expected)
        );
        let work = p.shadow_stats();
        assert!(!work.reused);
        assert!(work.bounds_visited > 0 && work.triangle_tests > 0 && work.clip_tests > 0);
        // The same fit, its casters on the new frame's clock.
        let mut held = expected.clone();
        held.casters.iter_mut().for_each(|c| c.time = 1.);
        assert_eq!(
            p.frame(1., &camera, &Ui::default(), 0)
                .unwrap()
                .shadow
                .as_ref(),
            Some(&held)
        );
        let hold = p.shadow_stats();
        assert!(hold.reused);
        assert_eq!(
            (hold.bounds_visited, hold.triangle_tests, hold.clip_tests),
            (0, 0, 0)
        );
    }
}

#[test]
fn installed_shadow_inputs_and_world_reload_invalidate_the_fit() {
    use presentation::presentation::{Presentation, Ui};
    use presentation::world::WorldInput;
    use std::sync::Arc;
    let mut p = Presentation::new(None, 30.).unwrap();
    let mut light = game_light();
    let camera = cameras()[1].1;
    let mut terrain = Arc::new(street());
    for change in 0..5 {
        match change {
            1 => light.cascades.depth_bias *= 2.,
            2 => terrain = Arc::new(slab(0., 0., 1600., 1600., 100.)),
            3 => {
                p.set_shadow(None);
                assert!(p
                    .frame(0., &camera, &Ui::default(), 0)
                    .unwrap()
                    .shadow
                    .is_none());
                let stats = p.shadow_stats();
                assert!(!stats.reused);
                assert_eq!(
                    (stats.bounds_visited, stats.triangle_tests, stats.clip_tests),
                    (0, 0, 0)
                );
            }
            _ => (),
        }
        if change == 4 {
            p.load_world(
                WorldInput {
                    battle: None,
                    buildings: None,
                    sun_azimuth: 0.,
                    sun_elevation: 0.5,
                },
                Default::default(),
                None,
            )
            .unwrap();
        } else {
            p.set_shadow(Some((light.clone(), terrain.clone())));
        }
        let expected = sun_shadow(&light, &camera, Some(&terrain));
        assert_eq!(
            p.frame(0., &camera, &Ui::default(), 0)
                .unwrap()
                .shadow
                .as_ref(),
            Some(&expected)
        );
        assert!(!p.shadow_stats().reused);
        assert!(p.shadow_stats().triangle_tests > 0);
        p.frame(0., &camera, &Ui::default(), 0).unwrap();
        assert!(p.shadow_stats().reused);
    }
}

#[test]
fn receiver_work_stays_local_as_remote_triangle_population_grows() {
    use presentation::presentation::{Presentation, Ui};
    use std::sync::Arc;
    let light = game_light();
    let camera = Camera {
        eye: [2., 2., 60.],
        target: [2., 2., 0.],
        up: [0., 1., 0.],
        fov_y: 0.8,
        near: 1.,
        size: [1000, 1000],
    };
    let local = slab(1., 1., 3., 3., 0.);
    let expected = sun_shadow(&light, &camera, Some(&local));
    for count in [64_usize, 1024, 16384] {
        let mut mesh = local.vertices().to_vec();
        for i in 0..count {
            let x = 1024. + i as f32 * 256.;
            mesh.extend_from_slice(slab(x, 0., x + 1., 1., 100.).vertices());
        }
        let mut p = Presentation::new(None, 30.).unwrap();
        p.set_shadow(Some((
            light.clone(),
            Arc::new(TerrainPartition::new(&mesh, CELL_M)),
        )));
        assert_eq!(
            p.frame(0., &camera, &Ui::default(), 0)
                .unwrap()
                .shadow
                .as_ref(),
            Some(&expected)
        );
        let work = p.shadow_stats();
        // One local leaf: a balanced search may test both children at each level,
        // but no remote triangle needs either narrow-phase test.
        assert!(work.bounds_visited <= 2 * (count + 1).next_power_of_two().ilog2() as usize + 1);
        assert_eq!((work.triangle_tests, work.clip_tests), (2, 2));
        p.frame(1., &camera, &Ui::default(), 0).unwrap();
        let hold = p.shadow_stats();
        assert!(hold.reused);
        assert_eq!(
            (hold.bounds_visited, hold.triangle_tests, hold.clip_tests),
            (0, 0, 0)
        );
        println!("remote tiles={count}: fit={work:?}, hold={hold:?}");
    }
}

/// Whatever moves in the wind casts as it is drawn: the frame's camera
/// carries the ground's wind, and every cascade's camera runs on the
/// frame's clock and in that wind, though the fit is reused.
#[test]
fn every_cascade_camera_runs_on_the_frames_clock_and_wind() {
    use presentation::presentation::{Presentation, Ui};
    use presentation::static_world::ground::{self, biome::Biome, Tuning};
    use std::sync::Arc;
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let read = |path: &str| std::fs::read_to_string(format!("{root}/{path}")).unwrap();
    let rules: contract::scenario::Rules =
        serde_json::from_value(sim::fixtures::test_game()).unwrap();
    let map = sim::maps::Catalogue::shipped()
        .load("wetlands")
        .unwrap()
        .definition;
    let world = sim::world::WorldGeometry::new(&map, &rules);
    let layout = sim::world::export::layout_json(&rules.catalog);
    let source = ground::source(&world, &map.extents(), &layout).unwrap();
    let mut biome = Biome::load(&read("fixtures/biomes/summer.json"), None).unwrap();
    // A wind of the test's own, blowing somewhere and gusting.
    biome.wind.heading_deg = 120.0;
    biome.wind.gust_mps = 7.5;
    let wind = biome.wind.record();
    assert!(wind.gust_mps > 0.0 && wind.toward[1] > 0.0);
    let game: serde_json::Value =
        serde_json::from_str(&read("fixtures/presentation.json")).unwrap();
    let tuning: Tuning = presentation::settings::PresentationSettings::from_value(&game)
        .unwrap()
        .tuning();
    let mut presentation = Presentation::new(None, 30.).unwrap();
    presentation.set_ground(source, biome, tuning).unwrap();
    presentation.set_shadow(Some((game_light(), Arc::new(street()))));
    let camera = orbit([240., 180., 0.], 260., 0.85, -1.57);
    for time in [0.0, 3.5, 7.25] {
        let frame = presentation
            .frame(time, &camera, &Ui::default(), 0)
            .unwrap();
        assert_eq!(frame.camera.time, time as f32);
        assert_eq!(bits(&frame.camera.wind), bits(&[wind]), "the ground's wind");
        let shadow = frame.shadow.as_ref().unwrap();
        assert!(!shadow.casters.is_empty());
        for caster in &shadow.casters {
            assert_eq!(caster.time, frame.camera.time, "at {time} s");
            assert_eq!(bits(&caster.wind), bits(&[wind]), "at {time} s");
        }
    }
}
