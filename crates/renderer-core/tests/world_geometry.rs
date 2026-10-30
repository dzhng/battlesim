//! The shared world constructor preserves the shell's view inputs.
use renderer_core::runtime::WorldGeometry;

#[test]
fn traversal_carries_tint_and_extra_faces_while_surface_stays_bare() {
    let positions = [0.0, 0.0, 1.0, 4.0, 0.0, 2.0, 4.0, 4.0, 3.0, 0.0, 4.0, 0.5];
    let indices = [0, 1, 2, 0, 2, 3];
    let surfaces = [0, 0, 2, 8];
    // A disconnected traversal face above the terrain, with its own tint.
    let face = [
        1.0, 1.0, 7.0, 0.0, 0.0, 1.0, 0.2, 0.4, 0.6, 1.0, 2.0, 1.0, 7.0, 0.0, 0.0, 1.0, 0.2, 0.4,
        0.6, 1.0, 1.0, 2.0, 7.0, 0.0, 0.0, 1.0, 0.2, 0.4, 0.6, 1.0,
    ];
    let cells = [0.0, 0.0, 2.0, 4.0, 0.0, 0.0, 0.0, 0.0];
    let surface = WorldGeometry::from_triangles(&positions, &indices, &cells, 80.0, None, &[]);
    let traversal =
        WorldGeometry::from_triangles(&positions, &indices, &[], 80.0, Some((&surfaces, 8)), &face);
    // Traversal colours distinguish the flags, independently of palette tuning.
    let flipped = WorldGeometry::from_triangles(
        &positions,
        &indices,
        &[],
        80.0,
        Some((&[0, 8, 2, 0], 8)),
        &[],
    );
    let open = &traversal.terrain[6..10];
    let blocked = &traversal.terrain[36..40];
    assert_ne!(open, blocked);
    for (triangle, tint) in flipped
        .terrain
        .as_chunks::<30>()
        .0
        .iter()
        .zip([blocked, open])
    {
        for vertex in triangle.as_chunks::<10>().0 {
            assert_eq!(&vertex[6..], tint);
        }
    }
    assert_eq!(
        &traversal.skirt_and_props[surface.skirt_and_props.len()..],
        &face
    );
    assert!(surface
        .terrain
        .as_chunks::<10>()
        .0
        .iter()
        .all(|v| v[6..] == [0.0; 4]));
    assert!(surface
        .skirt_and_props
        .as_chunks::<10>()
        .0
        .iter()
        .all(|v| v[2] < 7.0));
    assert!(surface
        .skirt_and_props
        .as_chunks::<10>()
        .0
        .iter()
        .any(|v| v[2] < 0.5));
    assert_eq!(&surface.water[..3], &[0.0, 0.0, 2.0]);
    assert!(traversal.water.is_empty());
    assert_eq!(surface.backdrop, traversal.backdrop);
}

#[test]
fn prebuilt_terrain_keeps_its_faces_while_backdrop_uses_the_local_patch() {
    let local = [
        0.0, 0.0, 0.5, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 4.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 0.0, 4.0, 0.5, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0,
    ];
    let mut remote = local.to_vec();
    for vertex in remote.as_chunks_mut::<10>().0 {
        vertex[0] += 1000.0;
        vertex[2] += 100.0;
    }
    let mut world = WorldGeometry::from_mesh(remote.clone(), &[], &local, &[], 80.0);
    assert_eq!(world.backdrop[0], 920.0);
    world.set_backdrop(Some(&local), 80.0);
    assert_eq!(world.terrain, remote);
    assert_eq!(world.skirt_and_props, local);
    assert_eq!(&world.backdrop[..3], &[-80.0, -80.0, 0.5]);
}
