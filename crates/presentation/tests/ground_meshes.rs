//! The ground's meshes are the browser's, float for float: on six saved maps
//! as the simulation builds them (both reel maps, the river, street,
//! geometry and ground labs), the terrain in both views, the skirt, the
//! water and the backdrop hash to the recorded browser vectors
//! (`fixtures/ground-mesh-vectors.json`: FNV-1a over each
//! mesh's f32 bytes, with the map's box and its water cells).
use presentation::shadow::map_box;
use presentation::static_world::ground::mesh::{self, VERTEX_FLOATS};
use presentation::static_world::ground::relief::BackdropRelief;
use serde_json::Value;
use sim::world::WorldGeometry;

fn vectors() -> Value {
    let at = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/ground-mesh-vectors.json"
    );
    serde_json::from_str(&std::fs::read_to_string(at).unwrap()).unwrap()
}

fn fnv(bytes: &[u8]) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

fn digest<T: bytemuck::Pod>(items: &[T]) -> Value {
    let bytes: &[u8] = bytemuck::cast_slice(items);
    serde_json::json!({ "bytes": bytes.len(), "fnv": fnv(bytes) })
}

fn floats(hex: &str) -> Vec<f32> {
    (0..hex.len() / 8)
        .map(|i| {
            f32::from_le_bytes(std::array::from_fn(|b| {
                u8::from_str_radix(&hex[i * 8 + b * 2..i * 8 + b * 2 + 2], 16).unwrap()
            }))
        })
        .collect()
}

#[test]
fn every_ground_mesh_is_the_browsers() {
    let rules: contract::scenario::Rules =
        serde_json::from_value(sim::fixtures::test_rules()).unwrap();
    let maps = sim::maps::Catalogue::shipped();
    for (id, want) in vectors().as_object().unwrap() {
        let map = maps.load(id).unwrap().definition;
        let world = WorldGeometry::new(&map, &rules);
        let positions = world.export_terrain_positions();
        let indices = world.export_terrain_indices();
        let surfaces = world.export_terrain_triangle_surfaces();
        // The simulation's ground is the one the browser drew.
        assert_eq!(digest(&positions), want["positions"], "{id} positions");
        assert_eq!(digest(&indices), want["indices"], "{id} indices");
        assert_eq!(digest(&surfaces), want["surfaces"], "{id} surfaces");

        let flag = want["blocked"].as_u64().unwrap() as u8;
        let terrain = mesh::terrain(&positions, &indices, None);
        let cells = floats(want["cells"].as_str().unwrap());
        let reach = want["reach"].as_f64().unwrap();
        let b = map_box(&terrain).unwrap();
        assert_eq!(digest(&terrain), want["terrain"], "{id} terrain");
        let traversal = mesh::terrain(&positions, &indices, Some((&surfaces, flag)));
        assert_eq!(digest(&traversal), want["traversal"], "{id} traversal");
        assert_eq!(
            digest(&mesh::skirt(&positions)),
            want["skirt"],
            "{id} skirt"
        );
        assert_eq!(digest(&mesh::water(&cells)), want["water"], "{id} water");
        let want_box: Vec<f64> = want["box"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect();
        assert_eq!(b.to_vec(), want_box, "{id} box");
        assert_eq!(
            digest(&mesh::backdrop(&b, reach, &BackdropRelief::default())),
            want["backdrop"],
            "{id} backdrop"
        );
    }
}

/// A two-triangle square, the second triangle blocked to ground units.
fn square() -> (Vec<f32>, Vec<u32>, Vec<u8>) {
    let positions = vec![
        0.0, 0.0, 1.0, 10.0, 0.0, 1.0, 10.0, 10.0, 3.0, 0.0, 10.0, 3.0,
    ];
    (positions, vec![0, 1, 2, 0, 2, 3], vec![0, 0, 0, 0b10])
}

#[test]
fn the_ground_is_its_own_triangles_flat_shaded_and_tinted_only_in_the_traversal_view() {
    let (positions, indices, surfaces) = square();
    let terrain = mesh::terrain(&positions, &indices, None);
    assert_eq!(terrain.len(), 6 * VERTEX_FLOATS);
    for v in terrain.chunks(VERTEX_FLOATS) {
        // Up the slope's normal, untinted.
        assert!(v[5] > 0.9 && v[4] < 0.0, "{v:?}");
        assert_eq!(v[6..], [0.0; 4]);
    }
    let traversal = mesh::terrain(&positions, &indices, Some((&surfaces, 0b10)));
    let alpha: Vec<f32> = traversal.chunks(VERTEX_FLOATS).map(|v| v[9]).collect();
    assert_eq!(alpha, [1.0; 6], "every triangle tinted");
    let red =
        |t: usize| traversal[t * 3 * VERTEX_FLOATS + 6] > traversal[t * 3 * VERTEX_FLOATS + 7];
    assert!(!red(0) && red(1), "the blocked triangle is the red one");
}

#[test]
fn the_skirt_hangs_below_the_lowest_ground_from_every_edge() {
    let (positions, ..) = square();
    let skirt = mesh::skirt(&positions);
    // Four edges, one upright face each.
    assert_eq!(skirt.len(), 4 * 6 * VERTEX_FLOATS);
    let lowest = skirt
        .chunks(VERTEX_FLOATS)
        .map(|v| v[2])
        .fold(f32::INFINITY, f32::min);
    assert_eq!(lowest, 1.0 - 6.0);
    for v in skirt.chunks(VERTEX_FLOATS) {
        assert_eq!(v[5], 0.0, "upright: {v:?}");
        assert!(
            v[0] == 0.0 || v[0] == 10.0 || v[1] == 0.0 || v[1] == 10.0,
            "on an edge: {v:?}"
        );
    }
}

#[test]
fn water_and_the_backdrop_are_flat_squares_facing_up() {
    let water = mesh::water(&[
        10.0, 20.0, -1.5, 8.0, 0.0, 0.0, 0.0, 0.0, 18.0, 20.0, -1.5, 8.0, 1.0, 0.0, 0.0, 0.0,
    ]);
    assert_eq!(water.len(), 2 * 6 * VERTEX_FLOATS);
    let b = [0.0, 0.0, -2.0, 100.0, 50.0, 30.0];
    let backdrop = mesh::backdrop(&b, 1000.0, &BackdropRelief::default());
    assert_eq!(backdrop.len(), 4 * 6 * VERTEX_FLOATS);
    for v in water
        .chunks(VERTEX_FLOATS)
        .chain(backdrop.chunks(VERTEX_FLOATS))
    {
        assert_eq!(v[3..6], [0.0, 0.0, 1.0]);
    }
    // The backdrop lies at the lowest ground, and reaches out all round.
    let xs = backdrop.chunks(VERTEX_FLOATS).map(|v| v[0]);
    assert_eq!(xs.clone().fold(f32::INFINITY, f32::min), -1000.0);
    assert_eq!(xs.fold(f32::NEG_INFINITY, f32::max), 1100.0);
    assert!(backdrop.chunks(VERTEX_FLOATS).all(|v| v[2] == -1.0));
}

/// A 100 × 50 map, and a hill 30 m high and 150 m round east of it, its top
/// on a sample, sampled every 10 m.
const MAP: [f64; 4] = [0.0, 0.0, 100.0, 50.0];

fn hill(x: f64, radius: f64) -> String {
    format!(
        r#"{{"cell_m": 10, "features": [{{"kind": "ridge", "center": [{x}, 20], "peak_m": 30, "radius_m": {radius}}}]}}"#
    )
}

#[test]
fn the_backdrop_rises_with_its_relief_and_leaves_the_map_to_its_own_ground() {
    let relief = BackdropRelief::admit(&hill(300.0, 150.0), MAP).unwrap();
    // The map's lowest ground is at -1.
    let (b, reach) = ([MAP[0], MAP[1], -2.0, MAP[2], MAP[3], 30.0], 1000.0);
    let backdrop = mesh::backdrop(&b, reach, &relief);
    let vertices: Vec<&[f32]> = backdrop.chunks(VERTEX_FLOATS).collect();
    // Every corner stands on the relief, from the lowest ground, facing up.
    for v in &vertices {
        let want = -1.0 + relief.height(v[0] as f64, v[1] as f64);
        assert!((v[2] as f64 - want).abs() < 1e-3, "{v:?} stands off {want}");
        assert!(v[5] > 0.0, "faces down: {v:?}");
    }
    let top = vertices
        .iter()
        .map(|v| v[2])
        .fold(f32::NEG_INFINITY, f32::max);
    assert_eq!(top, -1.0 + 30.0, "it rises to the hill's top");
    // Shaded smooth: a corner on the hill's west flank leans west.
    let flank = vertices
        .iter()
        .find(|v| v[0] == 250.0 && v[1] == 20.0)
        .unwrap();
    assert!(flank[3] < 0.0, "{flank:?}");
    covers_the_frame_once(&backdrop, MAP, reach);
}

/// The backdrop's triangles cover the land round `map` out to `reach` once,
/// counter-clockwise: none lies over the map, and together they are the
/// frame's area.
fn covers_the_frame_once(backdrop: &[f32], map: [f64; 4], reach: f64) {
    let mut area = 0.0;
    for t in backdrop.chunks(3 * VERTEX_FLOATS) {
        let p = |k: usize| [0, 1].map(|i| t[k * VERTEX_FLOATS + i] as f64);
        let [a, q, c] = [p(0), p(1), p(2)];
        let centre = [0, 1].map(|i| (a[i] + q[i] + c[i]) / 3.0);
        let over =
            centre[0] > map[0] && centre[0] < map[2] && centre[1] > map[1] && centre[1] < map[3];
        assert!(!over, "a triangle over the map at {centre:?}");
        area += ((q[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (q[1] - a[1])) / 2.0;
    }
    let [w, h] = [map[2] - map[0], map[3] - map[1]];
    let frame = (w + 2.0 * reach) * (h + 2.0 * reach) - w * h;
    assert!((area - frame).abs() < 1.0, "{area} m² against {frame} m²");
}

#[test]
fn hills_past_a_map_off_the_relief_grid_shade_smooth_and_stand_on_the_relief() {
    // A 95 × 45 map, its north and east edges between the 10 m sample lines,
    // and a hill east of it that runs across both edges' lines past its
    // corners, its top off a sample.
    let map = [0.0, 0.0, 95.0, 45.0];
    let text = r#"{"cell_m": 10, "features": [{"kind": "ridge", "center": [191.3, 47.7], "peak_m": 30, "radius_m": 80}]}"#;
    let relief = BackdropRelief::admit(text, map).unwrap();
    let (b, reach) = ([map[0], map[1], -2.0, map[2], map[3], 30.0], 400.0);
    let backdrop = mesh::backdrop(&b, reach, &relief);
    // Every raised corner leans with the hill: none is shaded as if level.
    let raised: Vec<&[f32]> = backdrop
        .chunks(VERTEX_FLOATS)
        .filter(|v| v[2] > -1.0 + 1e-3)
        .collect();
    assert!(!raised.is_empty(), "the hill rises");
    for v in &raised {
        assert!(
            v[3] != 0.0 || v[4] != 0.0,
            "a raised corner faces straight up: {v:?}"
        );
    }
    // Inside every triangle the mesh is the relief's own surface.
    for t in backdrop.chunks(3 * VERTEX_FLOATS) {
        let mean = |i: usize| (0..3).map(|k| t[k * VERTEX_FLOATS + i] as f64).sum::<f64>() / 3.0;
        let (x, y, z) = (mean(0), mean(1), mean(2));
        let want = -1.0 + relief.height(x, y);
        assert!(
            (z - want).abs() < 1e-3,
            "({x}, {y}) stands at {z}, not {want}"
        );
    }
    covers_the_frame_once(&backdrop, map, reach);
}

#[test]
fn relief_reaching_the_map_or_the_cells_beside_it_is_refused() {
    // The hill's foot 10 m past the map's edge, then on the map, then within
    // a cell of it.
    assert!(BackdropRelief::admit(&hill(260.0, 150.0), MAP).is_ok());
    assert!(BackdropRelief::admit(&hill(240.0, 150.0), MAP).is_err());
    assert!(BackdropRelief::admit(&hill(255.0, 150.0), MAP).is_err());
    let unsampled = r#"{"cell_m": 0, "features": [{"kind": "ridge", "center": [300, 20], "peak_m": 30, "radius_m": 150}]}"#;
    assert!(BackdropRelief::admit(unsampled, MAP).is_err());
    // Level land needs no samples.
    assert!(BackdropRelief::admit(r#"{"cell_m": 0, "features": []}"#, MAP).is_ok());
}
