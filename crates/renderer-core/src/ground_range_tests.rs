use super::*;
use presentation::static_world::ground::mesh;

#[test]
fn mesh_ranges_route_camera_and_cascades_without_full_mesh_fallback() {
    let local = mesh::water(&[0., 0., 0.5, 0.25, 0., 0., 0., 0.]);
    let remote = mesh::water(&[1024., 0., 0.5, 0.25, 0., 0., 0., 0.]);
    let words = [remote.clone(), local.clone()].concat();
    let partition = std::sync::Arc::new(TerrainPartition::new(&words, CELL_M));
    // Only the GPU allocation is omitted: prepare and ranges are the methods
    // the shared ground's depth, colour and cascade draws consume.
    let mut mesh = Mesh {
        buffer: None,
        visited: 0,
        partition: Some(partition.clone()),
        view: vec![],
        cascades: std::array::from_fn(|_| vec![]),
    };
    let camera = |x: f32| Camera {
        view_proj: [
            1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., -x, 0., 0., 1.,
        ],
        ..Default::default()
    };
    mesh.prepare(
        &camera(0.),
        &[camera(1024.), camera(0.), camera(5000.), camera(1024.)],
    );
    for (cascade, expected) in [
        (None, &local),
        (Some(0), &remote),
        (Some(1), &local),
        (Some(2), &vec![]),
        (Some(3), &remote),
    ] {
        let drawn: Vec<f32> = mesh
            .ranges(cascade)
            .iter()
            .flat_map(|r| {
                partition.vertices()
                    [r.start as usize * VERTEX_FLOATS..r.end as usize * VERTEX_FLOATS]
                    .iter()
                    .copied()
            })
            .collect();
        assert_eq!(
            bytemuck::cast_slice::<_, u8>(&drawn),
            bytemuck::cast_slice::<_, u8>(expected)
        );
        assert_eq!(
            mesh.ranges(cascade)
                .iter()
                .map(|r| r.len() / 3)
                .sum::<usize>(),
            expected.len() / 30
        );
    }
    mesh.prepare(&camera(5000.), &[]);
    assert!(mesh.ranges(None).is_empty());
    for c in 0..CASCADES {
        assert!(mesh.ranges(Some(c)).is_empty());
    }
}
