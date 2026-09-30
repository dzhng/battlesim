//! World geometry is judged against independent analytic surfaces, not against
//! a second copy of the triangulation code.
use contract::map::MapDefinition;
use sim::math::{v2, v3, V3};
use sim::world::{Collider, SurfaceKind, WorldGeometry};

fn lab_map() -> MapDefinition {
    serde_json::from_str(include_str!("../../../fixtures/geometry-lab.json")).unwrap()
}

fn lab() -> WorldGeometry {
    WorldGeometry::new(&lab_map(), &crate::common::rules())
}

fn flat(json_extra: &str) -> WorldGeometry {
    let map: MapDefinition = serde_json::from_str(&format!(
        r#"{{"size":[200,200],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35{json_extra}}}"#
    ))
    .unwrap();
    WorldGeometry::new(&map, &crate::common::rules())
}

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

#[test]
fn a_mesa_side_is_an_exact_plane_for_heights_normals_and_rays() {
    let w = flat(
        r#","relief":[{"kind":"mesa","rect":[120,40,40,120],"height_m":30,"side_degrees":20}]"#,
    );
    let tan = 20f64.to_radians().tan();
    // The west side is the plane z = (x - x0) tan between its foot x0 and the top edge x = 120.
    let x0 = 120.0 - 30.0 / tan;
    // Points well inside the side, whose triangles have every vertex on the plane.
    for (x, y) in [(50.0, 60.0), (77.3, 91.1), (101.9, 70.2)] {
        let h = w.height_at(x, y).unwrap();
        assert!(close(h, (x - x0) * tan, 1e-9), "height at {x},{y}: {h}");
        let s = w.surface_at(x, y).unwrap();
        let expect = v3(-20f64.to_radians().sin(), 0.0, 20f64.to_radians().cos());
        assert!((s.normal - expect).length() < 1e-9, "normal {:?}", s.normal);
        assert!(close(s.slope_deg, 20.0, 1e-9));
    }
    // Oblique ray against that plane, solved analytically.
    let o = v3(60.0, 80.0, 50.0);
    let d = v3(0.6, 0.1, -0.79).normalized();
    let t = (o.z - (o.x - x0) * tan) / (d.x * tan - d.z);
    let hit = w.raycast(o, d, 500.0).unwrap();
    assert_eq!(hit.collider, Collider::Terrain);
    assert!(close(hit.t, t, 1e-9), "t {} vs analytic {t}", hit.t);
}

#[test]
fn rendered_vertices_are_the_query_surface() {
    let w = lab();
    let (vertices, indices) = w.terrain_mesh();
    // Every exported vertex is exactly the queried height at its position.
    for v in vertices.iter().step_by(37) {
        assert_eq!(w.height_at(v.x, v.y), Some(v.z));
    }
    // Any point's height equals the barycentric interpolation of the exported
    // triangle containing it — computed here from the mesh alone.
    let bary = |p: (f64, f64), t: [V3; 3]| -> Option<f64> {
        let d = (t[1].y - t[2].y) * (t[0].x - t[2].x) + (t[2].x - t[1].x) * (t[0].y - t[2].y);
        let a = ((t[1].y - t[2].y) * (p.0 - t[2].x) + (t[2].x - t[1].x) * (p.1 - t[2].y)) / d;
        let b = ((t[2].y - t[0].y) * (p.0 - t[2].x) + (t[0].x - t[2].x) * (p.1 - t[2].y)) / d;
        let c = 1.0 - a - b;
        (a >= -1e-12 && b >= -1e-12 && c >= -1e-12).then(|| a * t[0].z + b * t[1].z + c * t[2].z)
    };
    for &(x, y) in &[
        (101.3, 207.9),
        (57.7, 181.1),
        (131.02, 233.3),
        (99.0, 210.0),
        (298.5, 57.1),
    ] {
        let found = indices
            .chunks(3)
            .find_map(|c| {
                bary(
                    (x, y),
                    [
                        vertices[c[0] as usize],
                        vertices[c[1] as usize],
                        vertices[c[2] as usize],
                    ],
                )
            })
            .unwrap();
        assert!(close(w.height_at(x, y).unwrap(), found, 1e-9), "{x},{y}");
    }
}

#[test]
fn ridge_vertices_sample_the_authored_function() {
    let w = lab();
    // 20 * (1-(d/80)^2)^2 at grid vertices (multiples of 4 m).
    for (x, y) in [
        (100.0, 208.0),
        (120.0, 228.0),
        (60.0, 200.0),
        (100.0, 292.0),
    ] {
        let d: f64 = f64::hypot(x - 100.0, y - 210.0);
        let q = (1.0 - (d / 80.0).powi(2)).max(0.0);
        let expect = if d >= 80.0 { 0.0 } else { 20.0 * q * q };
        assert!(close(w.height_at(x, y).unwrap(), expect, 1e-9), "{x},{y}");
    }
}

#[test]
fn a_ray_across_the_ridge_stops_at_its_first_crossing() {
    let w = lab();
    let o = v3(10.0, 205.0, 8.0);
    let d = v3(1.0, 0.03, 0.0).normalized();
    let hit = w.raycast(o, d, 400.0).expect("the ridge rises above 8 m");
    assert_eq!(hit.collider, Collider::Terrain);
    assert!(close(
        w.height_at(hit.point.x, hit.point.y).unwrap(),
        hit.point.z,
        1e-6
    ));
    // Nothing earlier along the ray is below ground.
    let mut t = 0.0;
    while t < hit.t - 0.01 {
        let p = o + d * t;
        assert!(
            w.height_at(p.x, p.y).unwrap() < p.z + 1e-9,
            "went underground at t={t}"
        );
        t += 0.05;
    }
}

#[test]
fn slope_access_uses_one_shared_cutoff() {
    let w = lab();
    let gentle = w.surface_at(215.0, 50.0).unwrap();
    let steep = w.surface_at(315.0, 50.0).unwrap();
    assert!(gentle.slope_deg < w.slope_cutoff_deg() && gentle.traversable);
    assert!(steep.slope_deg > w.slope_cutoff_deg() && !steep.traversable);
}

#[test]
fn water_blocks_ground_and_the_bridge_deck_is_walkable_and_solid() {
    let w = lab();
    let water = w.surface_at(192.0, 120.0).unwrap();
    assert_eq!(water.kind, SurfaceKind::Water);
    assert!(!water.traversable);
    let deck_z = lab_map().bridges[0].deck_z;
    let deck = w.surface_at(192.0, 160.0).unwrap();
    assert_eq!(deck.kind, SurfaceKind::Bridge);
    assert!(deck.traversable && close(deck.z, deck_z, 1e-12));
    assert!(
        deck.z > w.height_at(192.0, 160.0).unwrap(),
        "deck spans above the water bed"
    );
    // A falling ray lands on the deck prop, not on the bed below.
    let hit = w
        .raycast(v3(192.0, 160.0, 30.0), v3(0.0, 0.0, -1.0), 100.0)
        .unwrap();
    assert!(matches!(hit.collider, Collider::Prop(_)));
    assert!(close(hit.point.z, deck_z, 1e-9));
    let road = w.surface_at(60.0, 161.0).unwrap();
    assert_eq!(road.kind, SurfaceKind::Road);
}

#[test]
fn bounds_are_closed() {
    let w = lab();
    assert_eq!(w.height_at(-0.01, 10.0), None);
    assert_eq!(w.height_at(10.0, 300.01), None);
    assert!(w.surface_at(400.5, 10.0).is_none());
    // A low ray leaving the map through its edge hits nothing beyond it.
    assert!(w
        .raycast(v3(390.0, 20.0, 1.0), v3(1.0, 0.0, 0.0), 1000.0)
        .is_none());
    // A ray starting outside and pointing away never hits.
    assert!(w
        .raycast(v3(-10.0, 20.0, 1.0), v3(-1.0, 0.0, 0.0), 1000.0)
        .is_none());
}

#[test]
fn props_occlude_by_their_actual_height() {
    let w = lab();
    let eye = |y: f64, z: f64| v3(60.0, y, z);
    // Low wall at y=60 is 1 m tall: a 1.6 m sightline passes over, 0.5 m does not.
    assert!(w.segment_clear(eye(40.0, 1.6), eye(75.0, 1.6)));
    assert!(!w.segment_clear(eye(40.0, 0.5), eye(75.0, 0.5)));
    // Tall wall at y=90 is 4 m tall: blocks a 1.6 m sightline.
    assert!(!w.segment_clear(eye(75.0, 1.6), eye(105.0, 1.6)));
}

/// The yes/no line test (`segment_clear`, which stops at the first body it
/// meets and skips cells whose ground lies below the line) answers exactly
/// whether the nearest-hit ray meets anything, over lines hugging the relief,
/// the walls and the forest, and still after a body is shoved.
#[test]
fn the_line_test_agrees_with_the_nearest_hit_ray() {
    let mut w = lab();
    let mut rng = sim::rng::Rng::new(7);
    let point = |rng: &mut sim::rng::Rng| {
        let (x, y) = (400.0 * rng.unit(), 300.0 * rng.unit());
        v3(x, y, w.height_at(x, y).unwrap() + 3.0 * rng.unit())
    };
    let mut lines: Vec<(V3, V3)> = (0..4000)
        .map(|_| (point(&mut rng), point(&mut rng)))
        .collect();
    // Short lines through the forest and past the walls.
    lines.extend((0..4000).map(|_| {
        let a = point(&mut rng);
        (
            a,
            a + v3(
                40.0 * rng.unit() - 20.0,
                40.0 * rng.unit() - 20.0,
                rng.unit() - 0.5,
            ),
        )
    }));
    let agree = |w: &WorldGeometry, lines: &[(V3, V3)]| {
        let mut blocked = 0;
        for &(a, b) in lines {
            let d = b - a;
            let ray = w.raycast(a, d * (1.0 / d.length()), d.length());
            assert_eq!(
                w.segment_clear(a, b),
                ray.is_none(),
                "{a:?} → {b:?}: {ray:?}"
            );
            blocked += ray.is_some() as usize;
        }
        blocked
    };
    let blocked = agree(&w, &lines);
    assert!(
        blocked > 1000 && blocked < lines.len() - 1000,
        "{blocked} blocked"
    );
    let wall = w
        .props_near(v2(140.0, 60.0), 1.0)
        .into_iter()
        .find(|p| p.half.x < 0.2)
        .unwrap()
        .id;
    w.move_prop(wall, v2(200.0, 150.0), 0.7, 1);
    agree(&w, &lines);
}

#[test]
fn a_thin_wall_is_hit_on_its_near_face() {
    let w = lab();
    let hit = w
        .raycast(v3(100.0, 60.0, 1.5), v3(1.0, 0.0, 0.0), 200.0)
        .unwrap();
    assert!(matches!(hit.collider, Collider::Prop(_)));
    assert!(close(hit.point.x, 139.85, 1e-9), "x {}", hit.point.x);
    assert!((hit.normal - v3(-1.0, 0.0, 0.0)).length() < 1e-9);
}

#[test]
fn removing_a_prop_bumps_the_revision_and_clears_the_line() {
    let mut w = lab();
    let r0 = w.obstacle_revision();
    let a = v3(100.0, 60.0, 1.5);
    let b = v3(180.0, 60.0, 1.5);
    assert!(!w.segment_clear(a, b));
    let wall = w
        .props_near(v2(140.0, 60.0), 1.0)
        .into_iter()
        .find(|p| p.half.x < 0.2)
        .unwrap()
        .id;
    w.remove_prop(wall);
    assert!(w.obstacle_revision() > r0);
    assert!(w.segment_clear(a, b));
}

#[test]
fn forests_plant_trunks_clear_of_roads_and_props() {
    let w = lab();
    let trunks: Vec<_> = w
        .props()
        .filter(|p| p.kind == crate::common::kind("trunk"))
        .collect();
    assert!(trunks.len() > 20);
    for t in trunks {
        assert!(w.surface_at(t.center.x, t.center.y).unwrap().forest);
        assert_ne!(
            w.surface_at(t.center.x, t.center.y).unwrap().kind,
            SurfaceKind::Road
        );
    }
}

#[test]
fn a_vertical_ray_on_a_grid_vertex_and_diagonal_still_hits() {
    let w = lab();
    for (x, y) in [(100.0, 208.0), (102.0, 210.0), (104.0, 212.0)] {
        let hit = w
            .raycast(v3(x, y, 100.0), v3(0.0, 0.0, -1.0), 200.0)
            .unwrap();
        assert!(close(hit.point.z, w.height_at(x, y).unwrap(), 1e-9));
    }
}

#[test]
fn exports_follow_their_published_layout() {
    use sim::world::export;
    let w = lab();
    let layout: serde_json::Value =
        serde_json::from_str(&export::layout_json(crate::common::props())).unwrap();
    let stride = layout["propStride"].as_u64().unwrap() as usize;
    assert_eq!(layout["propFields"].as_array().unwrap().len(), stride);
    let props = w.export_props();
    assert_eq!(props.len(), w.props().count() * stride);
    let kinds = layout["surfaceKinds"].as_array().unwrap();
    let surfaces = w.export_terrain_triangle_surfaces();
    let (vertices, indices) = w.terrain_mesh();
    assert_eq!(surfaces.len(), indices.len() / 3 * 2);
    // Each triangle's tag agrees with the surface query at its centroid —
    // including under the bridge, where the ground is still water.
    for (t, tri) in indices.chunks(3).enumerate().step_by(97) {
        let c = (vertices[tri[0] as usize] + vertices[tri[1] as usize] + vertices[tri[2] as usize])
            * (1.0 / 3.0);
        let s = w.ground_surface_at(c.x, c.y).unwrap();
        assert_eq!(
            kinds[surfaces[t * 2] as usize],
            format!("{:?}", s.kind).to_lowercase()
        );
        assert_eq!(
            surfaces[t * 2 + 1] & export::FLAG_BLOCKED != 0,
            !s.traversable
        );
    }
    let under_bridge = w.ground_surface_at(192.0, 160.0).unwrap();
    assert_eq!(under_bridge.kind, SurfaceKind::Water);
}

#[test]
fn road_segments_export_the_road_rule() {
    use sim::world::export;
    let w = lab();
    let layout: serde_json::Value =
        serde_json::from_str(&export::layout_json(crate::common::props())).unwrap();
    let stride = layout["roadStride"].as_u64().unwrap() as usize;
    let fields: Vec<&str> = layout["roadFields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f.as_str().unwrap())
        .collect();
    assert_eq!(fields, ["ax", "ay", "bx", "by", "halfWidth"]);
    assert_eq!(stride, fields.len());
    let roads = w.export_roads();
    assert!(!roads.is_empty() && roads.len().is_multiple_of(stride));
    // A point is road exactly when it lies within an exported segment's half
    // width: the renderer's road mask is the simulation's rule. Water wins
    // over road, so the bridge's river is left out.
    let within = |x: f64, y: f64| {
        roads.chunks(stride).any(|s| {
            let (a, b) = (v2(s[0] as f64, s[1] as f64), v2(s[2] as f64, s[3] as f64));
            let (p, ab) = (v2(x, y), b - a);
            let t = ((p - a).dot(ab) / ab.dot(ab)).clamp(0.0, 1.0);
            (p - (a + ab * t)).length() <= s[4] as f64
        })
    };
    let mut roads_seen = 0;
    for j in 0..300 {
        for i in 0..400 {
            let (x, y) = (i as f64 + 0.37, j as f64 + 0.61);
            let s = w.ground_surface_at(x, y).unwrap();
            if s.kind == SurfaceKind::Water {
                continue;
            }
            assert_eq!(s.kind == SurfaceKind::Road, within(x, y), "at ({x}, {y})");
            roads_seen += (s.kind == SurfaceKind::Road) as usize;
        }
    }
    assert!(roads_seen > 1000);
}

#[test]
fn navigation_regions_cover_every_nonuniform_surface() {
    let w = lab();
    let regions = w.navigation_regions();
    for y in (1..300).step_by(2) {
        for x in (1..400).step_by(2) {
            let (x, y) = (x as f64, y as f64);
            let s = w.surface_at(x, y).unwrap();
            if s.kind != SurfaceKind::Ground || s.forest || !s.traversable || s.slope_deg != 0.0 {
                assert!(
                    regions
                        .iter()
                        .any(|r| x >= r[0] && y >= r[1] && x <= r[0] + r[2] && y <= r[1] + r[3]),
                    "surface variation at {x},{y} was omitted"
                );
            }
        }
    }
    assert!(flat("").navigation_regions().is_empty());
}

#[test]
fn terrain_queries_match_the_frozen_dense_surface() {
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "../../../specs/city-maps/assets/terrain-baseline/queries.json"
    ))
    .unwrap();
    let mut input = reference["map"].clone();
    input["fog_cell_m"] = serde_json::json!(8);
    let map = serde_json::from_value(input).unwrap();
    let w = WorldGeometry::new(&map, &crate::common::rules());
    let values = |v: &serde_json::Value| {
        v.as_array()
            .unwrap()
            .iter()
            .map(|x| f64::from_bits(u64::from_str_radix(x.as_str().unwrap(), 16).unwrap()))
            .collect::<Vec<_>>()
    };
    for query in reference["queries"].as_array().unwrap() {
        let p = values(&query["point"]);
        assert_eq!(
            sim::world::export::surface_record(w.surface_at(p[0], p[1])),
            values(&query["surface"])
        );
        let r = values(&query["ray"]);
        assert_eq!(
            sim::world::export::hit_record(w.raycast(
                v3(r[0], r[1], r[2]),
                v3(r[3], r[4], r[5]),
                r[6]
            )),
            values(&query["hit"])
        );
    }
}

#[test]
fn empty_terrain_export_does_not_grow_with_empty_area() {
    for size in [200, 1600] {
        let w = crate::common::flat([size as f64; 2], "");
        assert!(
            w.export_terrain_positions().len() * 4 + w.export_terrain_indices().len() * 4 < 16_384,
            "empty {size} m map expands a sampled grid"
        );
        assert_eq!(w.height_at(size as f64, size as f64), Some(0.0));
    }
}
