//! The sun's cascades are fitted where shadows are received, and every map
//! point the camera sees within the reach is inside a cascade that shades
//! it. The fit is the browser's to the last bit: `fixtures/shadow-vectors.json`
//! records it over a camera tour of both
//! reel maps' boxes (high and ground-level cameras, two lenses, three
//! viewports) under the game's light, a low sun and a high one.
use presentation::light::LightSettings;
use presentation::records::SunShadow;
use presentation::shadow::{camera_record, map_box, sun_shadow, SunShadowFrame, CASCADES};
use presentation::view::{Camera, View};
use serde_json::Value;

fn vectors() -> Value {
    let at = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/shadow-vectors.json"
    );
    serde_json::from_str(&std::fs::read_to_string(at).unwrap()).unwrap()
}

/// Little-endian f32 words from the browser's hex.
fn words(hex: &Value) -> Vec<u32> {
    let s = hex.as_str().unwrap();
    (0..s.len() / 8)
        .map(|i| {
            u32::from_le_bytes(std::array::from_fn(|b| {
                u8::from_str_radix(&s[i * 8 + b * 2..i * 8 + b * 2 + 2], 16).unwrap()
            }))
        })
        .collect()
}

fn bits<T: bytemuck::Pod>(record: &T) -> Vec<u32> {
    bytemuck::cast_slice::<T, u32>(std::slice::from_ref(record)).to_vec()
}

fn v3(v: &Value) -> [f64; 3] {
    std::array::from_fn(|i| v[i].as_f64().unwrap())
}

fn camera_of(case: &Value) -> Camera {
    Camera {
        eye: v3(&case["eye"]),
        target: v3(&case["target"]),
        up: v3(&case["up"]),
        fov_y: case["fov_y"].as_f64().unwrap(),
        near: case["near"].as_f64().unwrap(),
        size: std::array::from_fn(|i| case["size"][i].as_u64().unwrap() as u32),
    }
}

#[test]
fn every_fit_is_the_browsers() {
    let v = vectors();
    let lights: Vec<(String, LightSettings)> = v["lights"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(n, l)| (n.clone(), serde_json::from_value(l.clone()).unwrap()))
        .collect();
    let light = |name: &str| &lights.iter().find(|(n, _)| n == name).unwrap().1;
    for (name, l) in &lights {
        assert_eq!(
            bits(&SunShadowFrame::cold(l).receiver),
            words(&v["cold"][name]),
            "cold {name}"
        );
    }
    let cases = v["cases"].as_array().unwrap();
    for (k, case) in cases.iter().enumerate() {
        let b: Option<[f64; 6]> = case["box"]
            .as_array()
            .map(|b| std::array::from_fn(|i| b[i].as_f64().unwrap()));
        let frame = sun_shadow(
            light(case["light"].as_str().unwrap()),
            &camera_of(case),
            b.as_ref(),
        );
        let at = format!("case {k} ({} {})", case["map"], case["light"]);
        assert_eq!(
            bits(&frame.receiver),
            words(&case["receiver"]),
            "{at} receiver"
        );
        let casters = case["casters"].as_array().unwrap();
        assert_eq!(frame.casters.len(), casters.len(), "{at} casters");
        for (i, (got, want)) in frame.casters.iter().zip(casters).enumerate() {
            assert_eq!(bits(got), words(want), "{at} caster {i}");
        }
    }
    assert!(cases.len() > 100);
}

/// The street test map's ground as one flat slab, 1600 m square.
fn street() -> [f64; 6] {
    let corners = [[0.0, 0.0], [1600.0, 0.0], [1600.0, 1600.0], [0.0, 1600.0]];
    let mesh: Vec<f32> = corners
        .iter()
        .flat_map(|[x, y]| [*x, *y, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0])
        .collect();
    map_box(&mesh).unwrap()
}

fn game_light() -> LightSettings {
    let at = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/game.json");
    let game: Value = serde_json::from_str(&std::fs::read_to_string(at).unwrap()).unwrap();
    serde_json::from_value(game["presentation"]["light"].clone()).unwrap()
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
    let range = |c: &Camera, b: Option<&[f64; 6]>| {
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
