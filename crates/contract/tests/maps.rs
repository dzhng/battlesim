//! The one resolver over a saved map's documents: `map.json` in the compact
//! saved form, the `SOURCES.json` that pins it, and the physical library.
//! The tests change a resolved map, save it (`MapDefinition::saved`) and
//! resolve it again.
use contract::identity::json_hash;
use contract::map::MapDefinition;
use contract::maps::{resolve, MapAdmission};
use contract::templates::TemplateGeometryCatalog;
use serde_json::json;

mod common;
use common::{template, LIBRARY};

const GEOMETRY: &str = include_str!("../../../fixtures/maps/geometry/map.json");
const GEOMETRY_SOURCES: &str = include_str!("../../../fixtures/maps/geometry/SOURCES.json");
const ADMISSION: MapAdmission = MapAdmission {
    max_authored_parts: 128,
    max_bay_positions: 65_536,
};

/// The shipped geometry map, resolved.
fn geometry() -> MapDefinition {
    resolve(GEOMETRY, GEOMETRY_SOURCES, LIBRARY, ADMISSION)
        .unwrap()
        .definition
}

/// Sources that pin `definition` as an authored map of the shipped library,
/// selecting the shipped geometry map's templates.
fn sources(definition: &MapDefinition) -> serde_json::Value {
    let shipped: serde_json::Value = serde_json::from_str(GEOMETRY_SOURCES).unwrap();
    json!({
        "identity": {
            "kind": "authored",
            "map_hash": json_hash(definition).unwrap(),
            "template_catalog_hash": definition.template_catalog_hash,
        },
        "catalogue": shipped["catalogue"],
        "inputs": [{
            "kind": "repository",
            "path": "fixtures/geometry-lab.json",
            "revision": "8af952e61015937ed059de3d1c3c1c365ce62507",
            "sha256": "82314fcac5f08a0d1bbe9d7e810d593e72dc9ad6002c110e7c99c40c9ccf50fe",
        }],
    })
}

/// `definition` saved and resolved under `sources`.
fn admitted(
    definition: &MapDefinition,
    sources: &serde_json::Value,
) -> Result<contract::maps::ResolvedMap, contract::maps::ResolveError> {
    resolve(
        &serde_json::to_string(&definition.saved()).unwrap(),
        &sources.to_string(),
        LIBRARY,
        ADMISSION,
    )
}

#[test]
fn saved_visual_surroundings_keep_their_extent_without_changing_physical_geometry() {
    let original = geometry();
    let mut json = serde_json::to_value(&original).unwrap();
    json["render_margin_m"] = json!(500);
    let map: MapDefinition = serde_json::from_value(json).unwrap();
    let restored = admitted(&map, &sources(&map)).unwrap().definition;
    let stored = serde_json::to_value(restored.saved()).unwrap();
    assert_eq!(stored["render_margin_m"].as_f64(), Some(500.0));
    assert_eq!(restored.size, original.size);
    assert_eq!(restored.buildings, original.buildings);
    assert_eq!(
        serde_json::to_value(restored.props).unwrap(),
        serde_json::to_value(original.props).unwrap()
    );
}

#[test]
fn visual_surroundings_refuse_negative_or_unbounded_margins() {
    for margin in [-1.0, 5001.0, 1e300] {
        let mut map = serde_json::to_value(geometry()).unwrap();
        map["render_margin_m"] = json!(margin);
        let error = serde_json::from_value::<MapDefinition>(map).unwrap_err();
        assert!(error.to_string().contains("render margin"));
    }
}

#[test]
fn a_saved_building_resolves_to_its_template_materialized_at_its_frame() {
    let saved: serde_json::Value = serde_json::from_str(GEOMETRY).unwrap();
    let stored = &saved["buildings"][0];
    assert_eq!(
        stored.as_object().unwrap().keys().collect::<Vec<_>>(),
        ["frame", "kind", "owner", "parts", "template_id"],
        "a saved building stores its template, its frame and its own ids"
    );
    let definition = geometry();
    let building = &definition.buildings[0];
    let template = template(stored["template_id"].as_str().unwrap());
    // Read as the resolver reads it: every physical float exactly as written.
    let frame = serde_json::from_str::<contract::map::SavedMap>(GEOMETRY)
        .unwrap()
        .buildings[0]
        .frame;
    assert_eq!(building.geometry, template.materialize(frame).unwrap());
    assert_eq!(building.category, template.category);
    assert_eq!(building.regional_family, template.regional_family);
    assert_eq!(
        serde_json::to_value(&building.parts).unwrap(),
        stored["parts"]
    );

    // Saving the resolved map and resolving it again gives the same map
    // under the same identity.
    let sources = sources(&definition);
    let again = admitted(&definition, &sources).unwrap();
    assert_eq!(
        serde_json::to_string(&again.definition).unwrap(),
        serde_json::to_string(&definition).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&again.identity).unwrap(),
        sources["identity"]
    );
}

/// A map names the one region its buildings are of; saving and resolving
/// it keeps that name.
#[test]
fn a_map_names_its_buildings_region_and_keeps_it_when_saved() {
    let definition = geometry();
    assert_eq!(definition.regional_family.as_deref(), Some("china"));
    let again = admitted(&definition, &sources(&definition)).unwrap();
    assert_eq!(again.definition.regional_family, definition.regional_family);
}

/// A building of another region than its map's, or on a map that names
/// none, is refused: the map's region is the one every reader trusts.
#[test]
fn a_building_of_another_region_than_its_maps_is_refused() {
    for family in [Some("paris"), None] {
        let mut definition = geometry();
        definition.regional_family = family.map(String::from);
        let error = admitted(&definition, &sources(&definition))
            .expect_err("the map and its building disagree");
        assert!(error.message.contains("region"), "{family:?}: {error}");
    }
    // A map without buildings may name none.
    let bare: MapDefinition = serde_json::from_value(
        json!({"size":[64,64],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35}),
    )
    .unwrap();
    assert!(bare.regional_family.is_none() && bare.authored_props().is_ok());
}

#[test]
fn a_building_naming_a_template_its_catalogue_lacks_is_refused_by_name() {
    let definition = geometry();
    let mut saved = definition.saved();
    saved.buildings[0].template_id = "no-such-template".into();
    let error = resolve(
        &serde_json::to_string(&saved).unwrap(),
        &sources(&definition).to_string(),
        LIBRARY,
        ADMISSION,
    )
    .expect_err("no template, no building");
    assert_eq!(
        serde_json::to_value(&error).unwrap()["code"],
        "template_mismatch"
    );
    assert_eq!(error.location, "map.json.buildings[0].template_id");
    assert!(error.message.contains("no-such-template"), "{error}");
}

#[test]
fn a_map_carrying_its_own_building_geometry_is_refused() {
    // The resolved form is not a saved form: geometry comes only from the
    // template, so there is none to disagree with it.
    let definition = geometry();
    let error = resolve(
        &serde_json::to_string(&definition).unwrap(),
        &sources(&definition).to_string(),
        LIBRARY,
        ADMISSION,
    )
    .expect_err("a saved building holds no geometry");
    assert_eq!(serde_json::to_value(&error).unwrap()["code"], "invalid_map");
    assert_eq!(error.location, "map.json");
}

#[test]
fn the_library_a_map_names_is_one_file_name_never_a_path() {
    let definition = geometry();
    for bad in [
        "../prototype-building-templates.json",
        "templates/a.json",
        "library",
        "",
    ] {
        let mut sources = sources(&definition);
        sources["catalogue"]["library"] = json!(bad);
        let error = admitted(&definition, &sources).expect_err("a path is not a library name");
        assert_eq!(
            serde_json::to_value(&error).unwrap()["code"],
            "invalid_sources"
        );
        assert_eq!(error.location, "SOURCES.json.catalogue.library");
    }
}

#[test]
fn altered_map_is_refused_against_saved_content_identity() {
    let mut definition = geometry();
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
fn selection_does_not_silently_relabel_an_existing_catalogue_identity() {
    let definition = geometry();
    let mut sources = sources(&definition);
    sources["catalogue"]["template_ids"] = json!(["china-home-12x9-2f", "china-shed-15x24"]);
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
    let definition = geometry();
    let error = resolve(
        GEOMETRY,
        &sources(&definition).to_string(),
        LIBRARY,
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
    use contract::templates::{BuildingTemplateDescriptor, PlacementFrame};
    let template: BuildingTemplateDescriptor = serde_json::from_str(include_str!(
        "../../../fixtures/parity/templates/asymmetric.json"
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
    let mut sources = sources(&definition);
    sources["catalogue"] = json!({ "library": "asymmetric.json", "template_ids": null });
    let error = resolve(
        &serde_json::to_string(&definition.saved()).unwrap(),
        &sources.to_string(),
        &serde_json::to_string(&[template]).unwrap(),
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
    let definition = geometry();
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
    let definition = geometry();
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
    let mut definition = geometry();
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
    let definition = geometry();
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
    let original = geometry();
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
fn recomputing_content_hash_cannot_admit_a_river_the_terrain_cannot_carry() {
    let mut map = serde_json::to_value(geometry()).unwrap();
    map["rivers"][0]["surface_z"] = json!(5.0);
    let definition: MapDefinition = serde_json::from_value(map).unwrap();
    let error = admitted(&definition, &sources(&definition))
        .expect_err("a content hash is not river validation");
    assert_eq!(serde_json::to_value(&error).unwrap()["code"], "invalid_map");
    assert!(error.message.contains("above its bank"), "{error}");
}

#[test]
fn catalogue_references_refuse_paths_before_source_io() {
    for id in [
        "../geometry",
        "/geometry",
        "C:/geometry",
        "geometry/map",
        "geometry\0",
        "Geometry",
        "",
    ] {
        assert!(contract::maps::MapId::new(id).is_err(), "{id:?}");
        assert!(serde_json::from_value::<contract::maps::MapId>(json!(id)).is_err());
    }
}

/// A map pins the templates it selects and nothing else of the library, so a
/// library that grows or changes elsewhere leaves its identity alone.
#[test]
fn a_selection_is_identified_by_its_own_templates_alone() {
    let definition = geometry();
    let selected = TemplateGeometryCatalog::new(vec![template("china-shed-15x24")]).unwrap();
    assert_eq!(
        definition.template_catalog_hash.as_deref(),
        Some(selected.hash())
    );
    let whole = TemplateGeometryCatalog::new(serde_json::from_str(LIBRARY).unwrap()).unwrap();
    assert_ne!(selected.hash(), whole.hash());
    let mut sources = sources(&definition);
    sources["catalogue"]["template_ids"] = json!(null);
    let error =
        admitted(&definition, &sources).expect_err("the whole library is another catalogue");
    assert_eq!(error.location, "map.json.template_catalog_hash");
}
