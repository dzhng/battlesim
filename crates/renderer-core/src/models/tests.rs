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

#[test]
fn admission_does_not_scroll_pitchless_track_nodes() {
    use crate::bundle::{decode, Shape};
    use presentation::presentation::{lab::Asset, Presentation};
    use presentation::world::Detail;
    use serde_json::json;

    let mesh = json!({
        "positions": {"$view": 0}, "normals": {"$view": 1},
        "uvs": {"$view": 2}, "colors": {"$view": 3},
        "indices": {"$view": 4}, "draws": []
    });
    let nodes: Vec<_> = [
        ("track_L", json!({})),
        ("track_R", json!({"link_pitch_m": 0})),
        ("track_L", json!({"link_pitch_m": -0.2})),
        ("hull", json!({"link_pitch_m": 0.2})),
        ("track_L", json!({"link_pitch_m": 0.2})),
        ("track_R", json!({"link_pitch_m": 0.2})),
    ]
    .into_iter()
    .map(|(name, extras)| {
        json!({
            "name": name, "parent": -1,
            "bind": {"t": [0,0,0], "r": [0,0,0,1], "s": [1,1,1]},
            "extras": extras, "tiers": [mesh]
        })
    })
    .collect();
    let header = serde_json::to_vec(&json!({
        "content": {"kind": "articulated", "nodes": nodes, "materials": []},
        "views": [["f32",0,3],["i16",12,4],["f32",20,2],["u8",28,4],["u32",32,0]]
    }))
    .unwrap();
    let mut bytes = Vec::new();
    for word in [0x5441_4742u32, 4, header.len() as u32, 32] {
        bytes.extend(word.to_le_bytes());
    }
    bytes.extend(header);
    bytes.extend([0; 32]);
    let mut p = Presentation::new(None, 30.0).unwrap();
    p.install_lab_assets(
        [("tracks".into(), Asset::from_bundle(&bytes, None).unwrap())].into(),
        Detail {
            lod_px: [80.0, 40.0, 20.0],
            impostor_px: 4.0,
            shadow_caster_px: 0.0,
        },
    );
    let bundle = decode("tracks", &bytes, &[]).unwrap();
    let Shape::Articulated(nodes) = &bundle.shape else {
        panic!("articulated fixture")
    };
    let meshes = admission::articulated_meshes(nodes, 0, 0, |node| p.catalog().track_side(0, node));
    let tracks: Vec<_> = meshes
        .iter()
        .map(|(_, vertices)| u16::from_le_bytes(vertices[46..48].try_into().unwrap()))
        .collect();
    assert_eq!(
        tracks,
        [0, 0, 0, 0, 1, 2],
        "only pitch-qualified track nodes scroll"
    );
    // A staged replacement must supply its own rig before it becomes active.
    let mut candidate = Asset::from_bundle(&bytes, None).unwrap();
    let Asset::Animated { appearance, .. } = &mut candidate else {
        unreachable!()
    };
    let presentation::world::Appearance::Articulated { rig, .. } = appearance.as_mut() else {
        unreachable!()
    };
    rig.tracks.clear();
    let draft = p
        .stage_catalog(
            [("tracks".into(), candidate)].into(),
            Detail {
                lod_px: [80.0, 40.0, 20.0],
                impostor_px: 4.0,
                shadow_caster_px: 0.0,
            },
            Default::default(),
        )
        .unwrap();
    let catalog = draft.view(&p).unwrap();
    for (_, vertices) in
        admission::articulated_meshes(nodes, 0, 0, |node| catalog.track_side(0, node))
    {
        assert_eq!(&vertices[46..48], &[0, 0]);
    }
    assert_eq!(
        p.catalog().track_side(0, 4),
        Some(presentation::world::TrackSide::Left)
    );
}
