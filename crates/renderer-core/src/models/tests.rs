use super::*;
use crate::bundle::Draw;

fn mesh(draws: &[(usize, usize)]) -> Mesh {
    let mut first = 0;
    let draws: Vec<_> = draws
        .iter()
        .map(|&(material, triangles)| {
            let draw = Draw {
                material,
                first,
                count: triangles * 3,
            };
            first += draw.count;
            draw
        })
        .collect();
    Mesh {
        positions: vec![0.0; first * 3],
        indices: (0..first as u32).collect(),
        draws,
        ..Default::default()
    }
}

fn triangles(indices: &[u32], part: Part) -> Vec<[u32; 3]> {
    indices[part.first as usize..(part.first + part.count) as usize]
        .as_chunks::<3>()
        .0
        .to_vec()
}

#[test]
fn opaque_order_and_each_surface_triangle_are_preserved() {
    let opaque = mesh(&[(OPAQUE, 1), (OPAQUE, 3)]);
    let (indices, parts) = order_surfaces(&[&opaque], &|m| m);
    assert_eq!(indices, opaque.indices);
    assert_eq!(
        triangles(&indices, parts[OPAQUE]),
        vec![[0, 1, 2], [3, 4, 5], [6, 7, 8], [9, 10, 11]]
    );
    assert!(parts
        .iter()
        .enumerate()
        .all(|(c, p)| c == OPAQUE || p.count == 0));

    let mixed = mesh(&[
        (OPAQUE, 2),
        (CUTOUT, 1),
        (ROOM, 1),
        (BLENDED, 2),
        (OPAQUE, 1),
    ]);
    let (indices, parts) = order_surfaces(&[&mixed], &|m| m);
    let mut all = indices.clone();
    all.sort_unstable();
    assert_eq!(all, mixed.indices, "every triangle exactly once");
    assert_eq!(triangles(&indices, parts[CUTOUT]), vec![[6, 7, 8]]);
    assert_eq!(triangles(&indices, parts[ROOM]), vec![[9, 10, 11]]);
    assert_eq!(
        triangles(&indices, parts[BLENDED]),
        vec![[12, 13, 14], [15, 16, 17]]
    );
    assert_eq!(
        triangles(&indices, parts[OPAQUE]),
        vec![[0, 1, 2], [3, 4, 5], [18, 19, 20]]
    );
    assert_eq!(
        parts[OPAQUE].first + parts[OPAQUE].count,
        parts[ROOM].first,
        "depth and shadows read walls and rooms as one solid range"
    );
}

#[test]
fn merged_meshes_index_their_own_vertices() {
    let a = mesh(&[(OPAQUE, 1)]);
    let b = mesh(&[(CUTOUT, 1), (OPAQUE, 1)]);
    let (indices, parts) = order_surfaces(&[&a, &b], &|m| m);
    assert_eq!(triangles(&indices, parts[CUTOUT]), vec![[3, 4, 5]]);
    assert_eq!(
        triangles(&indices, parts[OPAQUE]),
        vec![[0, 1, 2], [6, 7, 8]]
    );
}

#[test]
fn rooms_are_solid_and_glass_and_cutouts_keep_their_coverage_class() {
    let material = |coverage, interior| Material {
        name: String::new(),
        base_color: [1.0; 4],
        metallic: 0.0,
        roughness: 1.0,
        tint: 0.0,
        textures: Default::default(),
        colour_scale: None,
        wear: None,
        coverage,
        interior,
    };
    assert_eq!(class_of(&material(Coverage::Opaque, None)), OPAQUE);
    assert_eq!(
        class_of(&material(Coverage::Opaque, Some("rooms".into()))),
        ROOM
    );
    assert_eq!(
        class_of(&material(Coverage::Cutout { cutoff: 0.5 }, None)),
        CUTOUT
    );
    assert_eq!(class_of(&material(Coverage::Blended, None)), BLENDED);
}

#[test]
fn hiding_a_surface_removes_it_from_solid_depth_and_colour_together() {
    let m = mesh(&[
        (OPAQUE, 1),
        (ROOM, 1),
        (BLENDED, 1),
        (OPAQUE, 1),
        (CUTOUT, 1),
    ]);
    let (indices, parts) = order_surfaces(&[&m], &|m| m);
    let solid = Part {
        first: parts[OPAQUE].first,
        count: parts[OPAQUE].count + parts[ROOM].count,
    };
    let drawn = |requested, hidden| {
        let mut shown = [true; 4];
        if let Some(class) = hidden {
            shown[class] = false;
        }
        let mut out: Vec<_> = visibility::visible_parts(&parts, requested, &shown)
            .into_iter()
            .flat_map(|part| triangles(&indices, part))
            .collect();
        out.sort_unstable();
        out
    };
    assert_eq!(drawn(solid, None), vec![[0, 1, 2], [3, 4, 5], [9, 10, 11]]);
    assert_eq!(drawn(solid, Some(ROOM)), vec![[0, 1, 2], [9, 10, 11]]);
    assert!(drawn(parts[ROOM], Some(ROOM)).is_empty());
    assert_eq!(drawn(solid, Some(OPAQUE)), vec![[3, 4, 5]]);
}
