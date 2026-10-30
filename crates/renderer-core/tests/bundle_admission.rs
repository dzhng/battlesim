use renderer_core::bundle::{decode, Shape};
use serde_json::{json, Value};

fn bundle(kind: &str, draws: Value) -> Vec<u8> {
    let mut body = Vec::new();
    let mut views = Vec::new();
    let mut array = |ty, bytes: Vec<u8>, length| {
        let at = views.len();
        views.push(json!([ty, body.len(), length]));
        body.extend(bytes);
        json!({"$view": at})
    };
    let mesh = json!({
        "positions": array("f32", vec![0; 36], 9),
        "normals": array("i16", vec![0; 24], 12),
        "uvs": array("f32", vec![0; 24], 6),
        "colors": array("u8", vec![255; 12], 12),
        "indices": array("u16", vec![0, 0, 1, 0, 2, 0], 3),
        "draws": draws,
    });
    let mut content = json!({"kind": kind, "materials": [{
        "name": "test", "base_color": [1,1,1,1], "metallic": 0,
        "roughness": 1, "tint": 0, "coverage": {"kind":"opaque"}
    }]});
    match kind {
        "skinned" => content["tiers"] = json!([mesh]),
        "articulated" => content["nodes"] = json!([{"name":"body", "tiers":[mesh]}]),
        _ => content["states"] = json!([{"name":"whole", "tiers":[mesh]}]),
    }
    let mut header = serde_json::to_vec(&json!({"content": content, "views": views})).unwrap();
    header.resize(header.len().next_multiple_of(4), b' ');
    let mut bytes = Vec::new();
    for word in [0x5441_4742_u32, 4, header.len() as u32, body.len() as u32] {
        bytes.extend(word.to_le_bytes());
    }
    bytes.extend(header);
    bytes.extend(body);
    bytes
}

#[test]
fn decoding_refuses_draws_outside_the_mesh_or_material_table() {
    for kind in ["static", "articulated", "skinned"] {
        for (draws, error) in [
            (
                json!([{"material":0,"first":1,"count":3}]),
                "draw 0 exceeds mesh indices",
            ),
            (
                json!([{"material":0,"first":usize::MAX,"count":3}]),
                "draw 0 exceeds mesh indices",
            ),
            (
                json!([{"material":1,"first":0,"count":3}]),
                "draw 0 names missing material 1",
            ),
        ] {
            let result = decode("test-body", &bundle(kind, draws), &[]);
            assert_eq!(
                result.err().as_deref(),
                Some(format!("bundle test-body: {error}").as_str()),
                "{kind}"
            );
        }
        let decoded = decode(
            "test-body",
            &bundle(
                kind,
                json!([
                    {"material":0,"first":0,"count":1},
                    {"material":0,"first":1,"count":2}
                ]),
            ),
            &[],
        )
        .unwrap();
        let mesh = match &decoded.shape {
            Shape::Skinned(tiers) => &tiers[0],
            Shape::Static(nodes) | Shape::Articulated(nodes) => &nodes[0].tiers[0],
        };
        assert_eq!(mesh.indices, [0, 1, 2]);
        assert_eq!(mesh.draws[1].first + mesh.draws[1].count, 3);
    }
}
