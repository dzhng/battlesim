//! World geometry is judged against independent analytic surfaces, not against
//! a second copy of the triangulation code.
use contract::map::MapDefinition;
use sim::math::{v2, v3, V3};
use sim::world::{Collider, SurfaceKind, WorldGeometry};

fn lab_map() -> MapDefinition {
    serde_json::from_str(include_str!("../../../fixtures/geometry-lab.json")).unwrap()
}

fn lab() -> WorldGeometry {
    WorldGeometry::new(&lab_map())
}

fn flat(json_extra: &str) -> WorldGeometry {
    let map: MapDefinition = serde_json::from_str(&format!(
        r#"{{"size":[200,200],"height_grid_m":4,"slope_cutoff_deg":35{json_extra}}}"#
    ))
    .unwrap();
    WorldGeometry::new(&map)
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
        .filter(|p| p.kind == contract::map::PropKind::Trunk)
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
    let layout: serde_json::Value = serde_json::from_str(&export::layout_json()).unwrap();
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
