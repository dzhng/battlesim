use super::*;

#[test]
fn a_broken_prop_type_fails_at_load_naming_it() {
    let with = |props: Value| {
        let mut docs = units(json!({ "base": base_tank(), "m1": { "extends": "base" } }));
        docs.push(json!({ "props": props }));
        resolve(&docs).unwrap_err()
    };
    let body = json!({ "blocks": { "infantry": true, "vehicle": true }, "stops_rounds": true,
        "occludes": true, "weight_class": "immovable" });
    let app = json!({ "drawn_by": "wall", "stand_in_tint": [0.5, 0.5, 0.5] });
    let prop = |id: &str, error: &str| CatalogError::Prop {
        id: id.into(),
        error: error.into(),
    };
    let mut hp = body.clone();
    hp["hp"] = json!(100);
    assert_eq!(
        with(json!({ "wall": { "body": hp, "appearance": app } })),
        prop("wall", "hp and destroyed go together")
    );
    assert_eq!(
        with(json!({ "wall": { "body": hp, "appearance": app,
            "destroyed": { "into": { "prop": "nothing", "height_m": 1 } } } })),
        prop(
            "wall",
            "destroyed into \"nothing\", which is not a prop type"
        )
    );
    assert_eq!(
        with(json!({
            "a": { "body": hp, "appearance": app, "destroyed": { "into": { "prop": "b", "height_m": 1 } } },
            "b": { "body": hp, "appearance": app, "destroyed": { "into": { "prop": "a", "height_m": 1 } } },
        })),
        prop("a", "its destroyed states loop")
    );
    assert_eq!(
        with(json!({ "wall": { "body": hp, "appearance": app, "destroyed": "cleared" } })),
        prop(
            "wall",
            "only a toppling body's destroyed state is cleared ground"
        )
    );
    // A unit's wreck must be a prop type.
    let orphan = resolve(&[
        roles(),
        json!({ "units": { "base": base_tank(), "m1": { "extends": "base" } } }),
    ])
    .unwrap_err();
    assert_eq!(
        orphan,
        CatalogError::Rule {
            id: "m1".into(),
            error: "its wreck \"heavy_wreck\" is not a prop type".into()
        }
    );
}
