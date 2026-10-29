//! The ground's meshes are the browser's, float for float: on six saved maps
//! as the simulation builds them (both reel maps, the river, street,
//! geometry and ground labs), the terrain in both views, the skirt, the
//! water and the backdrop hash to the recorded browser vectors
//! (`fixtures/ground-mesh-vectors.json`: FNV-1a over each
//! mesh's f32 bytes, with the map's box and its water cells).
use presentation::shadow::map_box;
use presentation::static_world::ground::mesh::{self, VERTEX_FLOATS};
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
        serde_json::from_value(sim::fixtures::test_game()).unwrap();
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
            digest(&mesh::backdrop(&b, reach)),
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
    let backdrop = mesh::backdrop(&b, 1000.0);
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
