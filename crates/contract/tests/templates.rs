//! Physical template proofs use the generator's library and labelled API
//! geometry, never inferred art dimensions or released source claims.
use contract::templates::{
    BuildingTemplateDescriptor, MaterializedBuilding, PlacementFrame, TemplateGeometryCatalog,
};
use serde_json::json;

/// The generator's template `id` (`fixtures/prototype-building-templates.json`).
fn template(id: &str) -> BuildingTemplateDescriptor {
    serde_json::from_str::<Vec<BuildingTemplateDescriptor>>(include_str!(
        "../../../fixtures/prototype-building-templates.json"
    ))
    .unwrap()
    .into_iter()
    .find(|t| t.id == id)
    .unwrap()
}

/// Every template is a whole building: a descriptor missing its floors, its
/// entrances, an exposed facade's bays or part of a face is refused, never
/// admitted with the gap filled in.
#[test]
fn a_template_missing_any_building_fact_is_refused() {
    let shed = template("china-shed-15x24");
    shed.validate().unwrap();
    let mut missing = Vec::new();
    let mut t = shed.clone();
    t.floor_heights_m = None;
    missing.push(("floors", t));
    let mut t = shed.clone();
    t.entrances = None;
    missing.push(("entrances", t));
    let mut t = shed.clone();
    t.entrances = Some(vec![]);
    missing.push(("an entrance", t));
    let mut t = shed.clone();
    t.edges[0].bays = None;
    missing.push(("an exposed facade's bays", t));
    let mut t = shed.clone();
    t.edges.retain(|e| e.id != "body-north");
    missing.push(("a face", t));
    for (what, t) in missing {
        assert!(t.validate().is_err(), "admitted without {what}");
        assert!(
            t.materialize(PlacementFrame {
                translation: [0.0; 3],
                yaw: 0.0
            })
            .is_err(),
            "materialized without {what}"
        );
        assert!(
            TemplateGeometryCatalog::new(vec![t]).is_err(),
            "catalogued without {what}"
        );
    }
}

#[test]
fn an_asymmetric_rotated_part_keeps_its_own_box_frame() {
    let mut descriptor = template("china-shed-15x24");
    let part = &mut descriptor.parts[0];
    (part.center, part.yaw, part.base_z) = ([2.0, 1.0], std::f64::consts::FRAC_PI_2, 1.0);
    let half = part.half_extents;
    let placed = descriptor
        .materialize(PlacementFrame {
            translation: [10.0, 20.0, 5.0],
            yaw: std::f64::consts::FRAC_PI_2,
        })
        .unwrap();
    assert_eq!(placed.parts[0].center, [9.0, 22.0]);
    assert_eq!(placed.parts[0].half_extents, half);
    assert_eq!(placed.parts[0].yaw, std::f64::consts::PI);
    assert_eq!(placed.parts[0].base_z, 6.0);
    assert_eq!(placed.height_m, 1.0 + 2.0 * half[2]);
}

#[test]
fn physical_identity_is_canonical_and_changes_with_geometry() {
    let first = template("china-shed-15x24");
    let mut second = first.clone();
    second.id = "another-shed-probe".into();
    second.parts[0].center[0] = -0.0;
    let catalogue = TemplateGeometryCatalog::new(vec![first.clone(), second.clone()]).unwrap();
    let reordered = TemplateGeometryCatalog::new(vec![second, first.clone()]).unwrap();
    assert_eq!(catalogue.hash(), reordered.hash());
    assert_eq!(
        catalogue.canonical_json().unwrap(),
        reordered.canonical_json().unwrap()
    );
    let original = TemplateGeometryCatalog::new(vec![first.clone()]).unwrap();
    let mut geometry = first;
    geometry.parts[0].center[0] = 0.5;
    let changed = TemplateGeometryCatalog::new(vec![geometry]).unwrap();
    assert_ne!(original.hash(), changed.hash());
}

fn asymmetric() -> BuildingTemplateDescriptor {
    serde_json::from_str(include_str!(
        "../../../fixtures/parity/templates/asymmetric.json"
    ))
    .unwrap()
}

#[test]
fn bays_and_entrances_materialize_from_the_same_asymmetric_frame() {
    let descriptor = asymmetric();
    descriptor.validate().unwrap();
    let placed = descriptor
        .materialize(PlacementFrame {
            translation: [10.0, 20.0, 5.0],
            yaw: std::f64::consts::FRAC_PI_2,
        })
        .unwrap();
    let entrances = placed.entrances.unwrap();
    assert_eq!(entrances[0].position, [13.0, 21.0, 5.0]);
    assert!(entrances[0].normal[0] > 0.999999999);
    assert!(entrances[0].normal[1].abs() < 1e-14);
    assert_eq!(entrances[1].position, [8.5, 28.0, 5.0]);
    let north = placed.edges.iter().find(|e| e.id == "main-north").unwrap();
    assert_eq!(
        north.bays.as_ref().unwrap(),
        &vec![[7.0, 21.5], [7.0, 18.5]]
    );
    assert_eq!(placed.floor_z, vec![5.0, 8.0]);
    assert!(placed
        .edges
        .iter()
        .filter(|e| !e.exposed)
        .all(|e| e.bays.is_none()));
}

#[test]
fn invalid_source_facts_never_reach_materialization_or_catalogue_identity() {
    let mut bad = Vec::new();
    let mut shape = asymmetric();
    shape.parts[0].half_extents[0] = 0.0;
    bad.push(shape);
    let mut shape = asymmetric();
    shape.parts[0].center[1] = f64::NAN;
    bad.push(shape);
    let mut shape = asymmetric();
    shape.parts[1].id = shape.parts[0].id.clone();
    bad.push(shape);
    let mut shape = asymmetric();
    shape.floor_heights_m = Some(vec![0.0, 8.0]);
    bad.push(shape);
    let mut shape = asymmetric();
    shape.edges[1].exposed = true;
    bad.push(shape);
    let mut shape = asymmetric();
    shape.entrances.as_mut().unwrap()[0].edge = "main-east-join".into();
    bad.push(shape);
    let mut shape = asymmetric();
    shape.edges[2].bays.as_mut().unwrap().pitch_m = 0.0;
    bad.push(shape);
    let mut shape = asymmetric();
    shape.edges[2].part = "missing".into();
    bad.push(shape);
    for shape in bad {
        assert!(
            shape
                .materialize(PlacementFrame {
                    translation: [0.0; 3],
                    yaw: 0.0
                })
                .is_err(),
            "invalid physical source reached materialization: {shape:?}"
        );
        assert!(TemplateGeometryCatalog::new(vec![shape]).is_err());
    }
}

#[test]
fn declared_joins_do_not_allow_false_exposed_walls_or_unproved_connections() {
    let shape = asymmetric();
    assert_eq!(
        shape.join("flush-api-join").unwrap().edges,
        ["main-east-join", "wing-west-join"]
    );
    assert!(shape.join("unbaked-corner").is_err());
    let mut incomplete = shape.clone();
    incomplete.edges.retain(|e| e.id != "main-west");
    assert!(
        incomplete.validate().is_err(),
        "a face without its facade is refused"
    );
    let mut concealed = shape.clone();
    concealed.edges[0].span_m = [-3.0, 1.0];
    assert!(
        concealed.validate().is_err(),
        "a partly interior wall cannot claim exposed bays"
    );
    let mut gap = shape.clone();
    gap.parts[1].center[0] += 0.01;
    assert!(
        gap.validate().is_err(),
        "join tolerance does not admit a physical source gap"
    );
}

#[test]
fn physical_json_preserves_authoritative_f64_bits_and_its_own_identity() {
    // The default JSON float reader rounds this finite metre value by one ULP.
    let value = f64::from_bits(0x4044000000005ccd);
    let mut descriptor = template("china-shed-15x24");
    descriptor.parts[0].center[0] = value;
    let catalogue = TemplateGeometryCatalog::new(vec![descriptor.clone()]).unwrap();
    let written = serde_json::to_string(catalogue.templates()).unwrap();
    let loaded = TemplateGeometryCatalog::new(serde_json::from_str(&written).unwrap()).unwrap();
    assert_eq!(
        loaded.templates()[0].parts[0].center[0].to_bits(),
        value.to_bits()
    );
    assert_eq!(loaded.hash(), catalogue.hash());
    let placed = descriptor
        .materialize(PlacementFrame {
            translation: [0.0; 3],
            yaw: 0.0,
        })
        .unwrap();
    let decoded: MaterializedBuilding =
        serde_json::from_str(&serde_json::to_string(&placed).unwrap()).unwrap();
    assert_eq!(decoded, placed);
}

#[test]
fn finite_inputs_cannot_publish_overflowed_world_geometry() {
    let mut descriptor = template("china-shed-15x24");
    descriptor.parts[0].center[0] = f64::MAX;
    assert!(descriptor
        .materialize(PlacementFrame {
            translation: [f64::MAX, 0.0, 0.0],
            yaw: 0.0
        })
        .is_err());
}

#[test]
fn vertically_disjoint_boxes_do_not_hide_each_others_facades() {
    let mut descriptor = asymmetric();
    descriptor.parts[1].base_z = 12.0;
    descriptor.joins.clear();
    for edge in &mut descriptor.edges {
        edge.exposed = true;
        edge.bays = Some(contract::templates::FacadeBays {
            pitch_m: 3.0,
            phase_m: 0.0,
        });
    }
    descriptor.validate().unwrap();
    let placed = descriptor
        .materialize(PlacementFrame {
            translation: [0.0; 3],
            yaw: 0.0,
        })
        .unwrap();
    assert!(placed.edges.iter().all(|edge| edge.exposed));
    assert_eq!(placed.parts[1].base_z, 12.0);
}

#[test]
fn unsupported_unequal_height_joins_cannot_hide_upper_exterior_walls() {
    let descriptor: BuildingTemplateDescriptor = serde_json::from_str(include_str!(
        "../../../fixtures/parity/templates/rejected-unequal-join/descriptor.json"
    ))
    .unwrap();
    assert!(descriptor.validate().is_err());
    assert!(descriptor
        .materialize(PlacementFrame {
            translation: [0.0; 3],
            yaw: 0.0
        })
        .is_err());
    assert!(TemplateGeometryCatalog::new(vec![descriptor]).is_err());
}

#[test]
fn cumulative_tiny_pitch_bays_fail_before_any_materialization_allocation() {
    let mut descriptor = asymmetric();
    // Each of these faces has fewer than 65,536 positions, but together they
    // exceed the descriptor's allocation allowance. These are invalid API data.
    for edge in &mut descriptor.edges {
        if ["main-north", "main-west", "main-south"].contains(&edge.id.as_str()) {
            edge.bays = Some(contract::templates::FacadeBays {
                pitch_m: 0.0003,
                phase_m: 0.0,
            });
        }
    }
    assert!(descriptor.validate().is_err());
    assert!(descriptor
        .materialize(PlacementFrame {
            translation: [0.0; 3],
            yaw: 0.0
        })
        .is_err());
}

#[test]
fn large_xy_translation_cannot_relax_vertical_join_geometry() {
    let mut descriptor = asymmetric();
    for part in &mut descriptor.parts {
        part.center[0] += 1e15;
    }
    let mut rebased = descriptor
        .materialize(PlacementFrame {
            translation: [-1e15, 0.0, 0.0],
            yaw: 0.0,
        })
        .unwrap();
    let ordinary = asymmetric()
        .materialize(PlacementFrame {
            translation: [0.0; 3],
            yaw: 0.0,
        })
        .unwrap();
    // A common offset itself is legal; only the genuine join defect must fail.
    rebased.frame = ordinary.frame;
    assert_eq!(rebased, ordinary);
    descriptor.parts[1].base_z = 12.0;
    assert!(
        descriptor.validate().is_err(),
        "XY magnitude cannot accept Z-disjoint join faces"
    );
    assert!(descriptor
        .materialize(PlacementFrame {
            translation: [-1e15, 0.0, 0.0],
            yaw: 0.0
        })
        .is_err());
}

#[test]
fn large_xy_translation_cannot_accept_a_real_join_gap() {
    let mut descriptor = asymmetric();
    for part in &mut descriptor.parts {
        part.center[0] += 1e15;
    }
    descriptor.parts[1].center[0] += 1.0;
    assert!(
        descriptor.validate().is_err(),
        "common XY offset cannot turn a metre gap into rotation roundoff"
    );
    assert!(descriptor
        .materialize(PlacementFrame {
            translation: [-1e15, 0.0, 0.0],
            yaw: 0.0
        })
        .is_err());
}

#[test]
fn unrepresentable_bay_lattice_is_rejected_before_materialization() {
    let descriptor: BuildingTemplateDescriptor = serde_json::from_str(include_str!(
        "../../../fixtures/parity/templates/rejected-numeric/lattice.json"
    ))
    .unwrap();
    assert!(
        descriptor.validate().is_err(),
        "993 lattice indices emitted only 46 distinct points and included span boundaries"
    );
    assert!(descriptor
        .materialize(PlacementFrame {
            translation: [0.0; 3],
            yaw: 0.0
        })
        .is_err());
    assert!(TemplateGeometryCatalog::new(vec![descriptor]).is_err());
}

#[test]
fn placement_cannot_collapse_distinct_bays_into_repeated_world_points() {
    let mut descriptor: BuildingTemplateDescriptor = serde_json::from_str(include_str!(
        "../../../fixtures/parity/templates/rotated.json"
    ))
    .unwrap();
    for edge in &mut descriptor.edges {
        edge.bays = Some(contract::templates::FacadeBays {
            pitch_m: 0.01,
            phase_m: 0.0,
        });
    }
    descriptor.validate().unwrap();
    assert!(
        descriptor
            .materialize(PlacementFrame {
                translation: [1e15, 1e15, 0.0],
                yaw: 0.0
            })
            .is_err(),
        "placement must retain distinct strictly interior XY bays"
    );
}

#[test]
fn common_vertical_datum_cannot_hide_real_facade_coverage() {
    let mut descriptor = asymmetric();
    for part in &mut descriptor.parts {
        part.base_z = 1e15;
    }
    descriptor.joins.clear();
    for edge in &mut descriptor.edges {
        edge.exposed = true;
        edge.bays = Some(contract::templates::FacadeBays {
            pitch_m: 3.0,
            phase_m: 0.0,
        });
    }
    assert!(
        descriptor.validate().is_err(),
        "eight metres of vertical overlap must still hide an internal facade"
    );
}

#[test]
fn rotation_materialization_uses_the_same_values_as_the_portable_runtime() {
    /// What the Wasm build materialized at a frame where the platform's own
    /// sine and cosine once gave native a different answer.
    #[derive(serde::Deserialize)]
    struct WasmRecord {
        frame: PlacementFrame,
        wasm: MaterializedBuilding,
    }
    let record: WasmRecord = serde_json::from_str(include_str!(
        "../../../fixtures/parity/templates/wasm-asymmetric.json"
    ))
    .unwrap();
    let actual = asymmetric().materialize(record.frame).unwrap();
    assert_eq!(actual.edges[0].normal, record.wasm.edges[0].normal);
    assert_eq!(actual, record.wasm);
}

#[test]
fn saved_local_spans_cannot_contradict_the_emitted_world_geometry() {
    let placed = asymmetric()
        .materialize(PlacementFrame {
            translation: [12000.0, 15000.0, 5.0],
            yaw: 0.7,
        })
        .unwrap();
    let mut record = serde_json::to_value(&placed).unwrap();
    assert_eq!(record["edges"][0]["span_m"], json!([-3.0, -1.0]));
    let admitted:contract::map::MapDefinition=serde_json::from_value(json!({"size":[18000,18000],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,"template_catalog_hash":"0".repeat(64),"regional_family":"api_fixture","buildings":[{"owner":0,"kind":"building","category":"attached_home","regional_family":"api_fixture","parts":[{"part":"main","prop":0},{"part":"wing","prop":1}],"geometry":record}]})).unwrap();
    admitted.authored_props().unwrap();
    record["edges"][0]["span_m"] = json!([-2.0, -1.0]);
    let map:contract::map::MapDefinition=serde_json::from_value(json!({"size":[18000,18000],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,"template_catalog_hash":"0".repeat(64),"regional_family":"api_fixture","buildings":[{"owner":0,"kind":"building","category":"farmstead","regional_family":"api_fixture","parts":[{"part":"main","prop":0},{"part":"wing","prop":1}],"geometry":record}]})).unwrap();
    assert!(map.authored_props().is_err());
}

/// A building far along one axis and near the origin of the other: its bays
/// and entrances are still on their facades, whatever way it is turned.
#[test]
fn every_placement_on_a_playable_map_materializes() {
    let descriptor = asymmetric();
    let mut refused = Vec::new();
    for step in 0..4000u32 {
        let frame = PlacementFrame {
            translation: [
                9_900.0 - 0.37 * f64::from(step),
                81.0 + 0.011 * f64::from(step),
                0.0,
            ],
            yaw: 0.001_571 * f64::from(step),
        };
        if let Err(error) = descriptor.materialize(frame) {
            refused.push((frame, error));
        }
    }
    assert!(
        refused.is_empty(),
        "{} refused: {:?}",
        refused.len(),
        refused.first()
    );
}
