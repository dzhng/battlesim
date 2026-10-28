use presentation::static_world::ground::{mesh, partition::TerrainPartition};
use presentation::view::{Camera, View};

fn patch(x: f32, y: f32) -> Vec<f32> {
    mesh::terrain(&[x, y, 0., x + 1., y, 0., x, y + 1., 0.], &[0, 1, 2], None)
}

fn camera(eye: [f64; 3], target: [f64; 3]) -> [f32; 16] {
    let camera = Camera {
        eye,
        target,
        up: [0., 1., 0.],
        fov_y: 0.8,
        near: 0.1,
        size: [800, 800],
    };
    View::new(&camera).camera(&camera, 0.).view_proj
}

fn selected(partition: &TerrainPartition, matrix: &[f32; 16]) -> (Vec<f32>, usize) {
    let mut ranges = vec![];
    let visited = partition.select(matrix, &mut ranges);
    let mut vertices = vec![];
    for range in ranges {
        vertices.extend_from_slice(
            &partition.vertices()[range.start as usize * 10..range.end as usize * 10],
        );
    }
    (vertices, visited)
}

#[test]
fn matched_local_views_preserve_triangles_and_ids_as_remote_extent_grows() {
    use presentation::{light::LightSettings, shadow::sun_shadow};
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/shadow-vectors.json")).unwrap();
    let light: LightSettings = serde_json::from_value(fixture["lights"]["game"].clone()).unwrap();
    let camera = Camera {
        eye: [0., 0., 10.],
        target: [0., 0., 0.],
        up: [0., 1., 0.],
        fov_y: 0.8,
        near: 0.1,
        size: [800, 800],
    };
    // Receiver fit is held fixed: extending storage must not change admitted light/view.
    let shadows = sun_shadow(
        &light,
        &camera,
        Some(&TerrainPartition::new(&patch(0., 0.), 8.)),
    );
    let matrices: Vec<_> = std::iter::once(View::new(&camera).camera(&camera, 0.).view_proj)
        .chain(shadows.casters.iter().map(|c| c.view_proj))
        .collect();
    let mut local = patch(0., 0.);
    local.extend(patch(1., 1.));
    // Attribute words carry stable triangle identities through the reorder.
    for (id, triangle) in local.as_chunks_mut::<30>().0.iter_mut().enumerate() {
        for vertex in triangle.as_chunks_mut::<10>().0 {
            vertex[6] = id as f32 + 10.;
        }
    }
    let baseline = TerrainPartition::new(&local, 8.);
    for remote_count in [64, 1024, 16384] {
        let mut extended = local.clone();
        for x in 0..remote_count {
            extended.extend(patch(1000. + x as f32 * 16., 1000.));
        }
        let partition = TerrainPartition::new(&extended, 8.);
        for (view, matrix) in matrices.iter().enumerate() {
            let (drawn, visited) = selected(&partition, matrix);
            assert_eq!(
                drawn,
                selected(&baseline, matrix).0,
                "extent={remote_count}, view={view}"
            );
            // One local leaf plus its binary ancestors/siblings, not a map-wide scan.
            let height = (remote_count + 1_usize).next_power_of_two().ilog2() as usize;
            assert!(
                visited <= 2 * height + 1,
                "extent={remote_count}, view={view}, visited={visited}"
            );
        }
    }
}

#[test]
fn a_triangle_crossing_into_view_keeps_all_its_original_vertex_words() {
    // Its centroid is far outside the view; its first corner is in view.
    let crossing = mesh::terrain(&[0., 0., 0., 100., 0., 0., 100., 1., 0.], &[0, 1, 2], None);
    let mut input = patch(-100., 0.);
    input.extend_from_slice(&crossing);
    let partition = TerrainPartition::new(&input, 8.);
    let (drawn, _) = selected(&partition, &camera([0., 0., 10.], [0., 0., 0.]));
    assert_eq!(
        drawn.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
        crossing.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
    );
    let words = |mesh: &[f32]| {
        let mut triangles: Vec<Vec<u32>> = mesh
            .as_chunks::<30>()
            .0
            .iter()
            .map(|t| t.iter().map(|v| v.to_bits()).collect())
            .collect();
        triangles.sort();
        triangles
    };
    assert_eq!(words(partition.vertices()), words(&input));
}

#[test]
fn a_fitted_cascade_retains_an_off_camera_caster_above_its_visible_receiver() {
    use presentation::{light::LightSettings, shadow::sun_shadow};
    // Recorded test light, independent of the live game's tuning.
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/shadow-vectors.json")).unwrap();
    let mut light: LightSettings =
        serde_json::from_value(fixture["lights"]["game"].clone()).unwrap();
    light.sun_azimuth = 0.;
    light.sun_elevation = std::f64::consts::FRAC_PI_4;
    let camera = Camera {
        eye: [0., 0., 100.],
        target: [0., 0., 0.],
        up: [0., 1., 0.],
        fov_y: 0.8,
        near: 0.1,
        size: [800, 800],
    };
    // Sunward and elevated by the same amount: its shadow lands at the origin.
    let caster = mesh::terrain(
        &[60., 0., 60., 61., 0., 60., 60., 1., 60.],
        &[0, 1, 2],
        None,
    );
    let partition = TerrainPartition::new(&caster, 8.);
    assert!(selected(
        &partition,
        &View::new(&camera).camera(&camera, 0.).view_proj
    )
    .0
    .is_empty());
    let shadows = sun_shadow(
        &light,
        &camera,
        Some(&TerrainPartition::new(
            &mesh::terrain(
                &[
                    -100., -100., 0., 100., -100., 0., 100., 100., 0., -100., 100., 0.,
                ],
                &[0, 1, 2, 0, 2, 3],
                None,
            ),
            8.,
        )),
    );
    assert!(shadows
        .casters
        .iter()
        .any(|camera| selected(&partition, &camera.view_proj).0 == caster));
}

#[test]
fn infinite_far_views_keep_distant_ground_but_reject_ground_behind_the_eye() {
    let distant = patch(1_000_000., 0.);
    let mut input = patch(-100., 0.);
    input.extend_from_slice(&distant);
    let partition = TerrainPartition::new(&input, 8.);
    assert_eq!(
        selected(&partition, &camera([0., 0., 1.], [1., 0., 1.])).0,
        distant
    );
    assert!(selected(
        &TerrainPartition::new(&[], 8.),
        &camera([0., 0., 1.], [1., 0., 1.])
    )
    .0
    .is_empty());
}

#[test]
fn clip_boundaries_are_inclusive_and_finite_depth_is_respected() {
    // An orthographic WebGPU clip volume: x,y in [-1,1], z in [0,1].
    let matrix = [
        1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
    ];
    let triangle = |x, z| mesh::terrain(&[x, 0., z, x, 0.2, z, x + 0.1, 0., z], &[0, 1, 2], None);
    for (x, z, visible) in [
        (1., 0.5, true),
        (0., 0., true),
        (0., 1., true),
        (1.01, 0.5, false),
        (0., -0.01, false),
        (0., 1.01, false),
    ] {
        let mesh = triangle(x, z);
        assert_eq!(
            selected(&TerrainPartition::new(&mesh, 0.1), &matrix).0,
            if visible { mesh } else { vec![] },
            "x={x}, z={z}"
        );
    }
}

#[test]
fn moving_camera_selection_contains_every_triangle_that_gpu_clip_can_keep() {
    // Independent homogeneous clip oracle, one triangle per tile. Conservative
    // bounds may keep extra triangles but must never lose a GPU-visible one.
    let mut triangles = Vec::new();
    for x in -12..=12 {
        for y in -12..=12 {
            triangles.push(patch(x as f32 * 16., y as f32 * 16.));
        }
    }
    let partition = TerrainPartition::new(&triangles.concat(), 8.);
    for distance in [10., 65., 260.] {
        for offset in [-32., 0., 40.] {
            let matrix = camera([offset, -distance, distance], [offset, 0., 0.]);
            let (drawn, _) = selected(&partition, &matrix);
            for triangle in &triangles {
                let clips: Vec<[f32; 4]> = triangle
                    .as_chunks::<10>()
                    .0
                    .iter()
                    .map(|v| {
                        std::array::from_fn(|r| {
                            matrix[r] * v[0]
                                + matrix[4 + r] * v[1]
                                + matrix[8 + r] * v[2]
                                + matrix[12 + r]
                        })
                    })
                    .collect();
                let outside = (0..6).any(|plane| {
                    clips.iter().all(|v| match plane {
                        0 => v[0] < -v[3],
                        1 => v[0] > v[3],
                        2 => v[1] < -v[3],
                        3 => v[1] > v[3],
                        4 => v[2] < 0.,
                        _ => v[2] > v[3],
                    })
                });
                if !outside {
                    assert!(
                        drawn
                            .as_chunks::<30>()
                            .0
                            .iter()
                            .any(|t| t.as_slice() == triangle.as_slice()),
                        "lost triangle at {:?}",
                        &triangle[..3]
                    );
                }
            }
        }
    }
}

#[test]
fn cancellation_at_gpu_clip_boundary_keeps_the_triangle() {
    // f32 multiply then add places x exactly on w=8. Evaluating the same
    // coefficients in f64 without a rounding envelope would reject it.
    let matrix = [
        1.0000001,
        0.,
        0.,
        0.,
        0.,
        1.,
        0.,
        0.,
        0.,
        0.,
        1.,
        0.,
        -100_000_000.,
        0.,
        0.,
        8.,
    ];
    let x = 100_000_000_f32;
    assert_eq!(matrix[0] * x + matrix[12], 8.);
    let triangle = mesh::terrain(&[x, 0., 0.5, x, 1., 0.5, x, 0., 0.6], &[0, 1, 2], None);
    assert_eq!(
        selected(&TerrainPartition::new(&triangle, 8.), &matrix).0,
        triangle
    );
}

#[test]
fn remote_water_and_boundary_growth_preserves_local_triangle_bytes_and_bounded_queries() {
    use presentation::static_world::ground::partition::CELL_M;
    let boundary = |x| mesh::skirt(&[x, 0., 0., x + 4., 0., 0., x + 4., 4., 0., x, 4., 0.]);
    let water = |x| mesh::water(&[x, 0., 0., 4., 0., 0., 0., 0.]);
    let matrices = [
        camera([2., -4., 10.], [2., 0., 0.]),
        camera([1026., -4., 10.], [1026., 0., 0.]),
        camera([5002., -4., 10.], [5002., 0., 0.]),
    ];
    for make_mesh in [boundary, water] {
        let mut local = make_mesh(0.);
        local.extend(make_mesh(1024.));
        let baseline = TerrainPartition::new(&local, CELL_M);
        for remote_count in [64_usize, 1024, 16384] {
            // Append exact existing geometry: regenerating a bigger map changes
            // the boundary itself and would not be a matched local comparison.
            let mut extended = local.clone();
            for i in 0..remote_count {
                extended.extend(make_mesh(10_000. + i as f32 * 512.));
            }
            let partition = TerrainPartition::new(&extended, CELL_M);
            for matrix in &matrices {
                let (drawn, visited) = selected(&partition, matrix);
                let expected = selected(&baseline, matrix).0;
                assert_eq!(
                    bytemuck::cast_slice::<_, u8>(&drawn),
                    bytemuck::cast_slice::<_, u8>(&expected)
                );
                let height = (remote_count + 2).next_power_of_two().ilog2() as usize;
                assert!(
                    visited <= 4 * height + 3,
                    "{remote_count}: visited {visited}"
                );
            }
        }
    }
}
