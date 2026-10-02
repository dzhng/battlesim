//! World geometry is judged against independent analytic surfaces, not against
//! a second copy of the triangulation code.
use contract::map::MapDefinition;
use sim::math::{v2, v3, V3};
use sim::world::{Collider, SurfaceKind, WorldGeometry};

fn lab_map() -> MapDefinition {
    sim::maps::load("geometry").unwrap().definition
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
fn a_removed_road_field_is_refused_instead_of_silently_erasing_the_road() {
    let result = serde_json::from_str::<MapDefinition>(
        r#"{"size":[200,200],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,
        "roads":[{"points":[[20,20],[180,20]],"width_m":12}]}"#,
    );
    assert!(
        result.is_err(),
        "the removed road schema silently lost physical geometry"
    );
}

#[test]
fn a_concave_road_polygon_includes_its_edges_and_excludes_its_cutout() {
    let w = flat(
        r#", "surfaces": [{ "kind": "road", "shape": {
        "kind": "polygon", "ring": [[32,32],[160,32],[160,64],[64,64],[64,160],[32,160]]
    }}]"#,
    );
    for (x, y, expected) in [
        (32.0, 32.0, SurfaceKind::Road),
        (160.0, 64.0, SurfaceKind::Road),
        (64.0, 160.0, SurfaceKind::Road),
        (63.9, 129.0, SurfaceKind::Road),
        (129.0, 129.0, SurfaceKind::Ground),
        (65.0, 66.0, SurfaceKind::Ground),
        (31.999_999_999, 64.0, SurfaceKind::Ground),
    ] {
        assert_eq!(w.surface_at(x, y).unwrap().kind, expected, "{x},{y}");
    }
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
    let stride = layout["surfaceStrokeStride"].as_u64().unwrap() as usize;
    let fields: Vec<&str> = layout["surfaceStrokeFields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f.as_str().unwrap())
        .collect();
    assert_eq!(
        fields,
        ["ax", "ay", "bx", "by", "halfWidth", "kind", "cuts"]
    );
    assert_eq!(stride, fields.len());
    let roads = w.export_surface_strokes();
    assert!(!roads.is_empty() && roads.len().is_multiple_of(stride));
    // A point is road exactly when it lies within an exported stretch's half
    // width and not past an end the stretch is cut square at: the renderer's
    // road mask is the simulation's rule. Water wins over road, so the
    // bridge's river is left out.
    let within = |x: f64, y: f64| {
        roads.chunks(stride).any(|s| {
            let (a, b) = (v2(s[0] as f64, s[1] as f64), v2(s[2] as f64, s[3] as f64));
            let (p, ab) = (v2(x, y), b - a);
            let along = (p - a).dot(ab) / ab.dot(ab);
            let cuts = s[6] as u8;
            let past = (cuts & contract::ground::CUT_A != 0 && along < 0.0)
                || (cuts & contract::ground::CUT_B != 0 && along > 1.0);
            !past && (p - (a + ab * along.clamp(0.0, 1.0))).length() <= s[4] as f64
        })
    };
    assert!(
        roads.chunks(stride).any(|s| s[6] != 0.0),
        "no stretch says where its stroke ends"
    );
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
fn terrain_queries_match_the_parity_oracle() {
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/parity/terrain/queries.json"
    ))
    .unwrap();
    let input = reference["map"].clone();
    let rules = crate::common::rules();
    let map = crate::common::physical_map(serde_json::from_value(input).unwrap(), &rules);
    let w = WorldGeometry::new(&map, &rules);
    let values = |v: &serde_json::Value| {
        v.as_array()
            .unwrap()
            .iter()
            .map(|x| f64::from_bits(u64::from_str_radix(x.as_str().unwrap(), 16).unwrap()))
            .collect::<Vec<_>>()
    };
    let hex = |v: Vec<f64>| {
        v.iter()
            .map(|x| format!("{:016x}", x.to_bits()))
            .collect::<Vec<_>>()
    };
    let mut blessed = reference.clone();
    for query in blessed["queries"].as_array_mut().unwrap() {
        let (p, r) = (values(&query["point"]), values(&query["ray"]));
        query["surface"] = serde_json::json!(hex(sim::world::export::surface_record(
            w.surface_at(p[0], p[1])
        )));
        query["hit"] = serde_json::json!(hex(sim::world::export::hit_record(w.raycast(
            v3(r[0], r[1], r[2]),
            v3(r[3], r[4], r[5]),
            r[6]
        ))));
    }
    if crate::common::bless_parity("terrain/queries.json", &blessed) {
        return;
    }
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

#[test]
fn polygon_export_covers_the_concave_shape_and_marks_only_its_outer_edges() {
    let ring = [
        [32.0, 32.0],
        [160.0, 32.0],
        [160.0, 64.0],
        [64.0, 64.0],
        [64.0, 160.0],
        [32.0, 160.0],
    ];
    let mut map = serde_json::json!({"size":[200,200],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35});
    map["surfaces"] = serde_json::json!([{"kind":"road","shape":{"kind":"polygon","ring":ring}}]);
    let map = serde_json::from_value(map).unwrap();
    let world = WorldGeometry::new(&map, &crate::common::rules());
    let triangles = world.export_surface_triangles();
    assert_eq!(triangles.len(), 4 * 7);
    for x in (0..=200).step_by(4) {
        for y in (0..=200).step_by(4) {
            let inside = triangles.chunks_exact(7).any(|t| {
                let cross = |a: usize, b: usize| {
                    (t[b] - t[a]) * (y as f32 - t[a + 1])
                        - (t[b + 1] - t[a + 1]) * (x as f32 - t[a])
                };
                let signs = [cross(0, 2), cross(2, 4), cross(4, 0)];
                signs.iter().all(|s| *s >= 0.0) || signs.iter().all(|s| *s <= 0.0)
            });
            assert_eq!(
                inside,
                world.surface_at(x as f64, y as f64).unwrap().kind == SurfaceKind::Road,
                "{x},{y}"
            );
        }
    }
    let mut boundary = Vec::new();
    let mut area = 0.0f64;
    for row in triangles.chunks_exact(7) {
        let points = [[row[0], row[1]], [row[2], row[3]], [row[4], row[5]]];
        area += (((points[1][0] - points[0][0]) * (points[2][1] - points[0][1])
            - (points[1][1] - points[0][1]) * (points[2][0] - points[0][0]))
            / 2.0) as f64;
        assert_eq!(row[6], 1.0);
    }
    assert_eq!(area, 7168.0);
    for edge in world.export_surface_boundaries().chunks_exact(5) {
        assert_eq!(edge[4], 1.0);
        boundary.push(([edge[0], edge[1]], [edge[2], edge[3]]));
    }
    assert_eq!(boundary.len(), 6);
    for i in 0..6 {
        assert!(boundary.contains(&(
            ring[i].map(|v| v as f32),
            ring[(i + 1) % 6].map(|v| v as f32)
        )));
    }
}

#[test]
fn sidewalk_edges_are_closed_and_road_water_and_bridge_take_precedence() {
    let w = flat(
        r#", "surfaces":[
      {"kind":"sidewalk","shape":{"kind":"polygon","ring":[[32,32],[160,32],[160,160],[32,160]]}},
      {"kind":"road","shape":{"kind":"stroke","points":[[0,64],[180,64]],"width_m":8}}
    ],"rivers":[{"points":[{"xy":[104,0],"width_m":12,"depth_m":1.5},{"xy":[104,180],"width_m":12,"depth_m":1.5}],"surface_z":-0.5}],
    "bridges":[{"deck":"bridge_deck","center":[104,64],"half_extents":[12,5],"yaw":0,"deck_z":0.1,"thickness_m":0.8}]"#,
    );
    for (p, kind) in [
        ([32.0, 40.0], SurfaceKind::Sidewalk),
        ([31.999999999, 40.0], SurfaceKind::Ground),
        ([32.0, 60.0], SurfaceKind::Road),
        ([104.0, 100.0], SurfaceKind::Water),
        ([104.0, 64.0], SurfaceKind::Bridge),
    ] {
        assert_eq!(w.surface_at(p[0], p[1]).unwrap().kind, kind, "{p:?}");
    }
}

#[test]
fn clipped_surface_bounds_keep_the_exact_edge_in_the_next_bucket() {
    // Recovering max as min + width rounds below 128 for this authored extent.
    let left = -1965.113464680063;
    let w = flat(&format!(
        ", \"surfaces\":[{}]",
        serde_json::json!({"kind":"road","shape":{"kind":"polygon","ring":[[left,0.0],[128.0,0.0],[128.0,100.0],[left,100.0]]}})
    ));
    assert_eq!(w.surface_at(128.0, 40.0).unwrap().kind, SurfaceKind::Road);
}

#[test]
fn joined_polygon_boundaries_exclude_the_shared_pavement_edge() {
    let world = flat(
        r#", "surfaces":[
        {"kind":"road","shape":{"kind":"polygon","ring":[[2,2],[12,2],[12,12],[2,12]]}},
        {"kind":"road","shape":{"kind":"polygon","ring":[[12,2],[22,2],[22,12],[12,12]]}}
    ]"#,
    );
    assert_eq!(world.surface_at(12.0, 7.0).unwrap().kind, SurfaceKind::Road);
    let boundaries = world.export_surface_boundaries();
    let mut nearest = f64::INFINITY;
    let mut perimeter = 0.0;
    for edge in boundaries.chunks_exact(5) {
        let a = v2(edge[0] as f64, edge[1] as f64);
        let b = v2(edge[2] as f64, edge[3] as f64);
        let ab = b - a;
        let t = ((v2(12.0, 7.0) - a).dot(ab) / ab.dot(ab)).clamp(0.0, 1.0);
        nearest = nearest.min((v2(12.0, 7.0) - (a + ab * t)).length());
        perimeter += ab.length();
        assert_eq!(edge[4], 1.0);
    }
    assert_eq!(nearest, 5.0, "the shared edge is not a pavement boundary");
    assert_eq!(perimeter, 60.0);
}

#[test]
fn overlapping_polygon_boundaries_clip_covered_edges_and_keep_one_exterior() {
    let world = flat(
        r#", "surfaces":[
        {"kind":"road","shape":{"kind":"polygon","ring":[[2,2],[12,2],[12,12],[2,12]]}},
        {"kind":"road","shape":{"kind":"polygon","ring":[[7,2],[17,2],[17,12],[7,12]]}}
    ]"#,
    );
    let mut nearest = f64::INFINITY;
    let mut perimeter = 0.0;
    for edge in world.export_surface_boundaries().chunks_exact(5) {
        let a = v2(edge[0] as f64, edge[1] as f64);
        let ab = v2(edge[2] as f64, edge[3] as f64) - a;
        let p = v2(9.5, 7.0);
        let t = ((p - a).dot(ab) / ab.dot(ab)).clamp(0.0, 1.0);
        nearest = nearest.min((p - (a + ab * t)).length());
        perimeter += ab.length();
    }
    assert_eq!(nearest, 5.0);
    assert_eq!(perimeter, 50.0);
}

/// C65: a rounded stroke's samples are the same bits in every runtime, and
/// so are a river's with the width and grade at each (C69). The web test
/// holds Wasm to this file's native samples.
#[test]
fn rounded_strokes_match_the_parity_oracle() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/parity/ground/curve-strokes.json"
    ))
    .unwrap();
    let map: MapDefinition = serde_json::from_value(oracle["map"].clone()).unwrap();
    let w = WorldGeometry::new(&map, &crate::common::rules());
    let strokes: Vec<String> = w
        .export_surface_strokes()
        .iter()
        .map(|v| format!("{:08x}", v.to_bits()))
        .collect();
    assert!(
        strokes.len() / sim::world::export::SURFACE_STROKE_STRIDE > 40,
        "the bends were not rounded"
    );
    let rivers: Vec<String> = w
        .export_rivers()
        .iter()
        .map(|v| format!("{:08x}", v.to_bits()))
        .collect();
    assert!(
        rivers.len() / sim::world::export::RIVER_STRIDE > 40,
        "the river's bends were not rounded"
    );
    let mut blessed = oracle.clone();
    blessed["strokes"] = serde_json::json!(strokes);
    blessed["rivers"] = serde_json::json!(rivers);
    if crate::common::bless_parity("ground/curve-strokes.json", &blessed) {
        return;
    }
    assert_eq!(serde_json::json!(strokes), oracle["strokes"]);
    assert_eq!(serde_json::json!(rivers), oracle["rivers"]);
}
