use contract::identity::json_hash;
use contract::map::MapDefinition;
use contract::maps::{resolve, MapAdmission};
use serde_json::json;

fn sources(definition: &MapDefinition) -> serde_json::Value {
    json!({
        "identity": {
            "kind": "authored",
            "map_hash": json_hash(definition).unwrap(),
            "template_catalog_hash": definition.template_catalog_hash,
        },
        "catalogue": { "template_ids": null },
        "inputs": [{
            "kind": "repository",
            "path": "fixtures/geometry-lab.json",
            "revision": "8af952e61015937ed059de3d1c3c1c365ce62507",
            "sha256": "82314fcac5f08a0d1bbe9d7e810d593e72dc9ad6002c110e7c99c40c9ccf50fe",
        }],
    })
}

fn admitted(
    definition: &MapDefinition,
    sources: &serde_json::Value,
) -> Result<contract::maps::ResolvedMap, contract::maps::ResolveError> {
    resolve(
        &serde_json::to_string(definition).unwrap(),
        &sources.to_string(),
        include_str!("../../../fixtures/building-templates.json"),
        MapAdmission {
            max_authored_parts: 128,
            max_bay_positions: 65_536,
        },
    )
}

#[test]
fn saved_geometry_resolves_complete_physical_definition_and_identity() {
    let input = include_str!("../../../fixtures/geometry-lab.json");
    let definition: MapDefinition = serde_json::from_str(input).unwrap();
    let sources = sources(&definition);
    let resolved = resolve(
        input,
        &sources.to_string(),
        include_str!("../../../fixtures/building-templates.json"),
        MapAdmission {
            max_authored_parts: 128,
            max_bay_positions: 65_536,
        },
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(&resolved.definition).unwrap(),
        serde_json::to_value(&definition).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&resolved.identity).unwrap(),
        sources["identity"]
    );
}

#[test]
fn altered_map_is_refused_against_saved_content_identity() {
    let mut definition: MapDefinition =
        serde_json::from_str(include_str!("../../../fixtures/geometry-lab.json")).unwrap();
    let sources = sources(&definition);
    definition.props[0].geometry.center[0] += 1.0;
    let error = admitted(&definition, &sources).expect_err("changed geometry must be refused");
    assert_eq!(
        serde_json::to_value(&error).unwrap()["code"],
        "identity_mismatch"
    );
    assert_eq!(error.location, "SOURCES.json.identity.map_hash");
}

#[test]
fn self_consistent_geometry_cannot_invent_unmeasured_catalogue_floors() {
    let mut definition: MapDefinition =
        serde_json::from_str(include_str!("../../../fixtures/geometry-lab.json")).unwrap();
    definition.buildings[0].geometry.floor_z = Some(vec![2.0]);
    definition.buildings[0].geometry.validate().unwrap();
    let sources = sources(&definition);
    let error = admitted(&definition, &sources)
        .expect_err("a matching content hash does not certify invented template facts");
    assert_eq!(
        serde_json::to_value(&error).unwrap()["code"],
        "template_mismatch"
    );
    assert_eq!(error.location, "map.json.buildings[0].geometry");
}

#[test]
fn selection_does_not_silently_relabel_an_existing_catalogue_identity() {
    let definition: MapDefinition =
        serde_json::from_str(include_str!("../../../fixtures/geometry-lab.json")).unwrap();
    let mut sources = sources(&definition);
    sources["catalogue"]["template_ids"] = json!(["api-box-12-9-4"]);
    let error = admitted(&definition, &sources)
        .expect_err("selected library geometry needs its own declared catalogue identity");
    assert_eq!(
        serde_json::to_value(&error).unwrap()["code"],
        "catalogue_mismatch"
    );
    assert_eq!(error.location, "map.json.template_catalog_hash");
}

#[test]
fn physical_part_admission_refuses_the_whole_map() {
    let input = include_str!("../../../fixtures/geometry-lab.json");
    let definition: MapDefinition = serde_json::from_str(input).unwrap();
    let error = resolve(
        input,
        &sources(&definition).to_string(),
        include_str!("../../../fixtures/building-templates.json"),
        MapAdmission {
            max_authored_parts: 4,
            max_bay_positions: 65_536,
        },
    )
    .expect_err("five physical parts exceed the caller's four-part allowance");
    assert_eq!(
        serde_json::to_value(&error).unwrap()["code"],
        "complexity_limit"
    );
    assert_eq!(error.location, "map.json.authored_parts");
}

#[test]
fn bay_admission_counts_all_placements_before_verification_materializes_them() {
    use contract::map::{BuildingDefinition, BuildingPartReference};
    use contract::templates::{
        BuildingTemplateDescriptor, PlacementFrame, TemplateGeometryCatalog,
    };
    let template: BuildingTemplateDescriptor = serde_json::from_str(include_str!(
        "../../../specs/city-maps/assets/template-geometry/asymmetric.json"
    ))
    .unwrap();
    let catalogue = TemplateGeometryCatalog::new(vec![template.clone()]).unwrap();
    let mut definition: MapDefinition = serde_json::from_str(
        r#"{"size":[100,100],"height_grid_m":4,"fog_cell_m":8,"slope_cutoff_deg":35}"#,
    )
    .unwrap();
    definition.template_catalog_hash = Some(catalogue.hash().into());
    for (owner, x) in [(0, 20.0), (2, 50.0)] {
        definition.buildings.push(
            BuildingDefinition::materialize(
                &template,
                PlacementFrame {
                    translation: [x, 20.0, 0.0],
                    yaw: 0.0,
                },
                "building".into(),
                owner,
                vec![
                    BuildingPartReference {
                        part: "main".into(),
                        prop: owner,
                    },
                    BuildingPartReference {
                        part: "wing".into(),
                        prop: owner + 1,
                    },
                ],
            )
            .unwrap(),
        );
    }
    let error = resolve(
        &serde_json::to_string(&definition).unwrap(),
        &sources(&definition).to_string(),
        &catalogue.canonical_json().unwrap(),
        MapAdmission {
            max_authored_parts: 4,
            max_bay_positions: 15,
        },
    )
    .expect_err("two ten-bay placements exceed fifteen admitted positions");
    assert_eq!(
        serde_json::to_value(&error).unwrap()["code"],
        "complexity_limit"
    );
    assert_eq!(error.location, "map.json.bay_positions");
}

#[test]
fn missing_input_receipts_cannot_claim_saved_provenance() {
    let definition: MapDefinition =
        serde_json::from_str(include_str!("../../../fixtures/geometry-lab.json")).unwrap();
    let mut sources = sources(&definition);
    sources["inputs"] = json!([]);
    let error = admitted(&definition, &sources).expect_err("saved source needs an input receipt");
    assert_eq!(
        serde_json::to_value(&error).unwrap()["code"],
        "invalid_sources"
    );
    assert_eq!(error.location, "SOURCES.json.inputs");
}

#[test]
fn historical_receipts_require_relative_paths_revisions_and_content_hashes() {
    let definition: MapDefinition =
        serde_json::from_str(include_str!("../../../fixtures/geometry-lab.json")).unwrap();
    for (field, bad) in [
        ("path", "../other-project/map.json"),
        ("path", "/tmp/map.json"),
        ("path", "fixtures//map.json"),
        ("path", "C:/fixtures/map.json"),
        ("path", "fixtures/map\0.json"),
        ("path", "fixtures/map\n.json"),
        ("revision", "  "),
        ("sha256", "unverified"),
    ] {
        let mut sources = sources(&definition);
        sources["inputs"][0][field] = json!(bad);
        let error = admitted(&definition, &sources).expect_err("malformed source receipt");
        assert_eq!(
            serde_json::to_value(&error).unwrap()["code"],
            "invalid_sources"
        );
        assert_eq!(error.location, format!("SOURCES.json.inputs[0].{field}"));
    }
}

#[test]
fn matching_content_identity_does_not_admit_duplicate_authored_ids() {
    let mut definition: MapDefinition =
        serde_json::from_str(include_str!("../../../fixtures/geometry-lab.json")).unwrap();
    definition.props[1].id = definition.props[0].id;
    let error = admitted(&definition, &sources(&definition))
        .expect_err("saved authored IDs must still form one dense unique namespace");
    assert_eq!(
        serde_json::to_value(&error).unwrap()["code"],
        "invalid_authored_ids"
    );
    assert_eq!(error.location, "map.json.authored_parts");
}

#[test]
fn generated_sources_refuse_empty_versions_and_noncanonical_identity_hashes() {
    let definition: MapDefinition =
        serde_json::from_str(include_str!("../../../fixtures/geometry-lab.json")).unwrap();
    for (field, bad) in [
        ("generator_version", ""),
        ("preset_revision", " \t"),
        ("config_hash", "unverified"),
        ("map_hash", "unverified"),
        ("template_catalog_hash", "unverified"),
    ] {
        let mut sources = sources(&definition);
        sources["identity"] = json!({
            "kind": "generated",
            "generation": {
                "generator_version": "api-provenance-fixture-v1",
                "preset_revision": "api-fixture-v1",
                "seed": "18446744073709551615",
                "config_hash": json_hash(&json!({"placement": "api-fixture"})).unwrap(),
                "template_catalog_hash": definition.template_catalog_hash,
                "map_hash": json_hash(&definition).unwrap(),
            },
        });
        sources["identity"]["generation"][field] = json!(bad);
        let error = admitted(&definition, &sources).expect_err("malformed generation identity");
        assert_eq!(
            serde_json::to_value(&error).unwrap()["code"],
            "invalid_sources"
        );
        assert_eq!(
            error.location,
            format!("SOURCES.json.identity.generation.{field}")
        );
    }
}

#[test]
fn recomputing_content_hash_cannot_admit_invalid_physical_headers() {
    let original: MapDefinition =
        serde_json::from_str(include_str!("../../../fixtures/geometry-lab.json")).unwrap();
    for field in ["size", "fog_cell_m", "height_grid_m", "slope_cutoff_deg"] {
        let mut definition = original.clone();
        match field {
            "size" => definition.size[0] = 0.0,
            "fog_cell_m" => definition.fog_cell_m = 0.0,
            "height_grid_m" => definition.height_grid_m = -1.0,
            "slope_cutoff_deg" => definition.slope_cutoff_deg = 91.0,
            _ => unreachable!(),
        }
        let error = admitted(&definition, &sources(&definition))
            .expect_err("a content hash is not physical header validation");
        assert_eq!(serde_json::to_value(&error).unwrap()["code"], "invalid_map");
        assert_eq!(error.location, format!("map.json.{field}"));
    }
}

#[test]
fn catalogue_references_refuse_paths_before_source_io() {
    for id in [
        "../village",
        "/village",
        "C:/village",
        "village/map",
        "village\0",
        "Village",
        "",
    ] {
        let source = json!({"kind": "catalogue", "id": id});
        assert!(
            serde_json::from_value::<contract::maps::MapSource>(source).is_err(),
            "{id:?}"
        );
    }
}

#[test]
fn single_template_selection_preserves_the_frozen_endurance_catalogue_identity() {
    use contract::templates::TemplateGeometryCatalog;
    let library = TemplateGeometryCatalog::from_json(include_str!(
        "../../../fixtures/building-templates.json"
    ))
    .unwrap();
    let template = library
        .templates()
        .iter()
        .find(|t| t.id == "api-box-12-10-4")
        .unwrap();
    let mut definition: MapDefinition =
        serde_json::from_str(include_str!("../../../fixtures/geometry-lab.json")).unwrap();
    let building = &mut definition.buildings[0];
    building.geometry = template.materialize(building.geometry.frame).unwrap();
    definition.template_catalog_hash =
        Some("fd62882f6114c99b42ee939109f7ca17dda1a37a41c6f0a8a50ee95461c297e3".into());
    let mut sources = sources(&definition);
    sources["catalogue"]["template_ids"] = json!(["api-box-12-10-4"]);
    let resolved = admitted(&definition, &sources).unwrap();
    assert_eq!(
        serde_json::to_value(&resolved.definition).unwrap(),
        serde_json::to_value(&definition).unwrap()
    );
    assert_eq!(
        resolved.identity.template_catalog_hash(),
        definition.template_catalog_hash.as_deref()
    );
}
