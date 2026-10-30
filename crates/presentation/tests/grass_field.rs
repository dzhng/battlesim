//! The grass field's CPU side, word for word as recorded: every installed
//! grass kind's packed shapes and facts, and the window and frame parameters
//! over a camera tour of both reel maps (`fixtures/grass-vectors.json`, with
//! the bundles' meshes).
use presentation::static_world::ground::biome::GrassRules;
use presentation::static_world::ground::grass::{
    blade_vertices, frame_params, metres_per_pixel, pack_shapes, strip_indices, window, KindMesh,
    TierMesh, FIELD_SEGMENTS, MAX_TILES, SHAPE_FLOATS, TILE_M,
};
use serde_json::Value;

const VECTORS: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/grass-vectors.json"
);

fn vectors() -> Value {
    serde_json::from_str(&std::fs::read_to_string(VECTORS).unwrap()).unwrap()
}

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

fn list<T>(v: &Value, f: impl Fn(&Value) -> T) -> Vec<T> {
    v.as_array().unwrap().iter().map(f).collect()
}

struct Tier {
    positions: Vec<f32>,
    normals: Vec<i16>,
    uvs: Vec<f32>,
    colors: Vec<u8>,
}

fn tiers(kind: &Value) -> Vec<Tier> {
    list(&kind["tiers"], |t| Tier {
        positions: list(&t["positions"], |v| v.as_f64().unwrap() as f32),
        normals: list(&t["normals"], |v| v.as_i64().unwrap() as i16),
        uvs: list(&t["uvs"], |v| v.as_f64().unwrap() as f32),
        colors: list(&t["colors"], |v| v.as_u64().unwrap() as u8),
    })
}

fn kind<'a>(name: &'a str, tiers: &'a [Tier], top: f64) -> KindMesh<'a> {
    KindMesh {
        name,
        tiers: tiers
            .iter()
            .map(|t| TierMesh {
                positions: &t.positions,
                normals: &t.normals,
                uvs: &t.uvs,
                colors: &t.colors,
            })
            .collect(),
        top,
    }
}

/// Run with `UPDATE_GRASS_VECTORS=1 cargo test -p presentation --test grass_field`
/// to rerecord the packed shapes after an intentional packing change.
#[test]
fn the_installed_kinds_pack_as_recorded() {
    let mut v = vectors();
    let names: Vec<String> = list(&v["kinds"], |k| k["name"].as_str().unwrap().into());
    let meshes: Vec<Vec<Tier>> = list(&v["kinds"], tiers);
    let tops: Vec<f64> = list(&v["kinds"], |k| k["top"].as_f64().unwrap());
    let kinds: Vec<KindMesh> = (0..names.len())
        .map(|k| kind(&names[k], &meshes[k], tops[k]))
        .collect();
    let packed = pack_shapes(&kinds).unwrap();
    let shapes: Vec<u32> = packed.shapes.iter().map(|f| f.to_bits()).collect();
    if std::env::var_os("UPDATE_GRASS_VECTORS").is_some() {
        let hex: String = shapes
            .iter()
            .flat_map(|w| w.to_le_bytes())
            .map(|b| format!("{b:02x}"))
            .collect();
        v["shapes"] = hex.into();
        std::fs::write(VECTORS, serde_json::to_string(&v).unwrap()).unwrap();
        panic!("fixture updated; rerun without UPDATE_GRASS_VECTORS");
    }
    assert_eq!(shapes, words(&v["shapes"]));
    let facts: Vec<f64> = packed
        .facts
        .iter()
        .flat_map(|f| {
            [
                f.mesh_height,
                f.top,
                f.blades,
                f.root_width,
                f64::from(f.bases[0]),
                f64::from(f.bases[1]),
            ]
        })
        .collect();
    assert_eq!(facts, list(&v["facts"], |f| f.as_f64().unwrap()));
    assert_eq!(
        u64::from(packed.max_blades),
        v["max_blades"].as_u64().unwrap()
    );
}

#[test]
fn the_window_and_frame_params_are_the_browsers() {
    let v = vectors();
    let rules: GrassRules = serde_json::from_value(v["rules"].clone()).unwrap();
    for (k, case) in v["windows"].as_array().unwrap().iter().enumerate() {
        let eye: [f64; 3] = std::array::from_fn(|i| case["eye"][i].as_f64().unwrap());
        let height = case["height"].as_f64().unwrap();
        let scale = metres_per_pixel(case["fov_y"].as_f64().unwrap(), height);
        assert_eq!(scale, case["scale"].as_f64().unwrap(), "case {k} scale");
        let w = window(eye, case["lowest"].as_f64().unwrap(), scale, &rules);
        let want = &case["window"];
        assert_eq!(
            [
                w.x0,
                w.y0,
                f64::from(w.tiles_x),
                f64::from(w.tiles_y),
                w.reach
            ],
            ["x0", "y0", "tilesX", "tilesY", "reach"].map(|f| want[f].as_f64().unwrap()),
            "case {k} window"
        );
        let view_proj: [f32; 16] =
            std::array::from_fn(|i| f32::from_bits(words(&case["view_proj"])[i]));
        let mut params = vec![0; presentation::records::GrassParams::LAYOUT.size / 4];
        frame_params(&mut params, &view_proj, eye, scale, &w);
        let expected = words(&case["words"]);
        assert_eq!(params[..expected.len()], expected, "case {k} params");
    }
}

#[test]
fn every_blade_strip_is_its_segments_pairs_then_its_tip() {
    for segments in FIELD_SEGMENTS {
        let per = blade_vertices(segments);
        let indices = strip_indices(3, segments);
        // Two triangles a segment but the last, which narrows to the tip.
        assert_eq!(indices.len() as u32, 3 * (2 * segments - 1) * 3);
        for (b, blade) in indices.chunks(indices.len() / 3).enumerate() {
            let own = b as u32 * per..(b as u32 + 1) * per;
            assert!(
                blade.iter().all(|&i| own.contains(&u32::from(i))),
                "blade {b} keeps to itself"
            );
            assert_eq!(
                u32::from(*blade.last().unwrap()),
                own.end - 1,
                "ends at its tip"
            );
        }
    }
    assert_eq!(FIELD_SEGMENTS, [4, 2]);
}

/// One blade of `segments` segments, 1 m tall, its colours `rgb`.
fn blade(segments: u32, rgb: [u8; 3]) -> Tier {
    let per = blade_vertices(segments) as usize;
    let mut t = Tier {
        positions: vec![],
        normals: vec![],
        uvs: vec![],
        colors: vec![],
    };
    for v in 0..per {
        let (k, side) = (v / 2, v % 2);
        let h = k as f32 / segments as f32;
        let x = if v == per - 1 {
            0.0
        } else {
            side as f32 * 0.02 - 0.01
        };
        t.positions.extend([x, 0.0, h]);
        t.normals.extend([0, -32767, 0, 0]);
        t.uvs.extend([0.25, h]);
        t.colors.extend([rgb[0], rgb[1], rgb[2], 255]);
    }
    t
}

#[test]
fn a_clump_tints_toward_the_weighted_mean_and_a_broken_kind_is_refused_by_name() {
    let tiers: Vec<Tier> = [4, 3, 2, 1].map(|s| blade(s, [60, 120, 30])).into();
    let packed = pack_shapes(&[kind("tuft", &tiers, 1.0)]).unwrap();
    // One colour throughout: the weighted mean equals every vertex's colour,
    // so every tint is one and the ground's colour shows through unchanged.
    for v in packed.shapes.chunks(SHAPE_FLOATS) {
        assert!(v[12..15].iter().all(|t| (t - 1.0).abs() < 1e-6), "{v:?}");
    }
    let f = &packed.facts[0];
    assert_eq!((f.blades, f.bases), (1.0, [0, 9]));
    assert!((f.root_width - 0.02).abs() < 1e-6 && f.mesh_height == 1.0);
    let short: Vec<Tier> = [4, 3].map(|s| blade(s, [60, 120, 30])).into();
    let refused = pack_shapes(&[kind("stub", &short, 1.0)]).err().unwrap();
    assert!(
        refused.contains("stub") && refused.contains("LOD2"),
        "{refused}"
    );
}

#[test]
fn the_window_holds_the_grass_reach_round_the_eye_and_nothing_from_high_up() {
    let rules: GrassRules = serde_json::from_value(vectors()["rules"].clone()).unwrap();
    let scale = metres_per_pixel(0.8, 1080.0);
    let reach = rules.fade_m_per_px[1] / scale;
    let low = window([100.0, 200.0, 20.0], 0.0, scale, &rules);
    assert_eq!(low.reach, reach);
    assert_eq!((low.x0 % TILE_M, low.y0 % TILE_M), (0.0, 0.0));
    assert!(low.x0 < 100.0 - reach + 20.0 && low.x0 + f64::from(low.tiles_x) * TILE_M > 100.0);
    assert!(low.tiles_x <= MAX_TILES && low.tiles_x > 0);
    let high = window([100.0, 200.0, reach + 1.0], 0.0, scale, &rules);
    assert_eq!((high.tiles_x, high.tiles_y), (0, 0));
}

#[test]
fn packed_tints_stay_bounded_and_every_vertex_keeps_its_wind_response() {
    for wind in [255, 64] {
        let mut tiers: Vec<Tier> = [4, 3, 2, 1].map(|s| blade(s, [30, 90, 20])).into();
        for tier in &mut tiers {
            let n = tier.colors.len() / 4;
            for (v, color) in tier.colors.chunks_mut(4).enumerate() {
                color[..3].copy_from_slice(if v == n - 1 {
                    &[240, 255, 90]
                } else {
                    &[10, 30, 5]
                });
                color[3] = wind;
            }
        }
        let packed = pack_shapes(&[kind("tuft", &tiers, 1.0)]).unwrap();
        for v in packed.shapes.chunks(SHAPE_FLOATS) {
            for tint in &v[12..15] {
                assert!(*tint >= 0.3 - 1e-6 && *tint < 1.3, "{v:?}");
            }
            assert_eq!(v[15], f32::from(wind) / 255.0);
            assert_eq!(v[6], 0.0, "widening a blade cannot add height");
        }
    }
}

#[test]
fn field_height_includes_the_drawn_far_tier() {
    let mut tiers: Vec<Tier> = [4, 3, 2, 1].map(|s| blade(s, [60, 120, 30])).into();
    *tiers[2].positions.last_mut().unwrap() = 2.0;
    let packed = pack_shapes(&[kind("far-tip", &tiers, 1.0)]).unwrap();
    assert_eq!(packed.facts[0].mesh_height, 2.0);
}
