//! The packed publication carries integers exactly past float32's 2²⁴, read
//! back only through the published layout.
use std::collections::BTreeMap;

use contract::ids::Side;
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::publication;

use crate::common;

/// One decoded row: its fields by name and its sections' points.
#[derive(Debug, Default)]
struct Row {
    fields: BTreeMap<String, f32>,
    sections: BTreeMap<String, Vec<BTreeMap<String, f32>>>,
}

fn names(v: &Value) -> Vec<String> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap().to_owned())
        .collect()
}

/// Walks `data` exactly as the web decoder does, by the layout alone.
fn decode(layout: &Value, data: &[f32]) -> BTreeMap<String, Vec<Row>> {
    let header = names(&layout["header"]);
    let head: BTreeMap<String, f32> = header.iter().cloned().zip(data.iter().copied()).collect();
    let mut at = header.len();
    let mut groups = BTreeMap::new();
    for group in layout["groups"].as_array().unwrap() {
        let fields = names(&group["fields"]);
        let count = head[group["count"].as_str().unwrap()] as usize;
        let mut rows: Vec<Row> = (0..count)
            .map(|_| {
                let row = Row {
                    fields: fields
                        .iter()
                        .cloned()
                        .zip(data[at..].iter().copied())
                        .collect(),
                    ..Row::default()
                };
                at += fields.len();
                row
            })
            .collect();
        for row in &mut rows {
            for section in group["sections"].as_array().unwrap() {
                let points = names(&section["fields"]);
                let n = row.fields[section["count"].as_str().unwrap()] as usize;
                let list = (0..n)
                    .map(|_| {
                        let p = points
                            .iter()
                            .cloned()
                            .zip(data[at..].iter().copied())
                            .collect();
                        at += points.len();
                        p
                    })
                    .collect();
                row.sections
                    .insert(section["name"].as_str().unwrap().to_owned(), list);
            }
        }
        groups.insert(group["name"].as_str().unwrap().to_owned(), rows);
    }
    let ground = &layout["ground"];
    assert_eq!(
        at + head["fogFloats"] as usize
            + head[ground["count"].as_str().unwrap()] as usize
                * ground["fields"].as_array().unwrap().len(),
        data.len(),
        "the layout covers the record"
    );
    groups
}

fn integer(limb_bits: u32, point: &BTreeMap<String, f32>, name: &str) -> u32 {
    let (lo, hi) = (point[&format!("{name}Lo")], point[&format!("{name}Hi")]);
    lo as u32 + ((hi as u32) << limb_bits)
}

#[test]
fn ids_and_shot_counters_stay_exact_past_two_to_the_twenty_four() {
    // A real frame with every group in use, then counters no float32 holds.
    let map = json!({ "size": [800, 600], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35, "props": [] });
    let mut b = Battle::new(
        &common::scenario(
            &map.to_string(),
            json!([
                { "side": "blue", "kind": "rifle", "position": [100, 300] },
                { "side": "red", "kind": "tank", "position": [300, 300], "yaw": std::f64::consts::PI },
            ]),
            json!([]),
        ),
        1,
    );
    b.step();
    let mut frame = b.observe(Side::Blue).clone();
    let big = (1u32 << 24) + 1;
    assert_ne!(big as f32 as u32, big, "one float would round it");
    frame.own[0].member_ids[3] = big;
    frame.own[0].weapon_poses[0].shots = u32::MAX;
    frame.identified[0].weapon_poses[1].shots = big + 2;
    frame.projectiles = vec![contract::observation::VisibleSegment {
        path: vec![[0.0; 3], [1.0, 2.0, 3.0], [4.0, 5.0, 6.0]],
        ricochets: vec![contract::observation::SegmentRicochet {
            point: 1,
            normal: [0.0, -1.0, 0.0],
        }],
        own: false,
        kind: 2,
        shooter_member: Some(big + 4),
        hit: contract::observation::SegmentHit::Hull,
        impact_normal: Some([0.0, 0.0, 1.0]),
    }];
    frame.corpses = vec![contract::observation::Corpse {
        position: [5.0; 3],
        own: true,
        soldier: big + 6,
        kind: common::unit_kind("rifle"),
        slot: 5,
        yaw: 0.25,
    }];
    let layout: Value = serde_json::from_str(&publication::layout_json(&b)).unwrap();
    let bits = layout["limbBits"].as_u64().unwrap() as u32;
    let marks = sim::ground::GroundCell {
        crater: 255,
        scorch: 1,
        tracks: 128,
        trampled: 7,
        cleared: 255,
    };
    let patch = publication::GroundHeader {
        epoch: 3,
        side: Side::Red,
        base: 5,
        revision: 9,
        full: false,
        count: 1,
    };
    let mut data = Vec::new();
    let cell = big + 8;
    let (x, y) = (cell % 18_000, cell / 18_000);
    let run = sim::ground::GroundRunPatch {
        tile: (y / 16) * 1125 + x / 16,
        start: ((y % 16) * 16 + x % 16) as u16,
        len: 1,
        marks,
    };
    publication::pack(&frame, &patch, &full_fog(), std::iter::once(run), &mut data).unwrap();
    let groups = decode(&layout, &data);

    let own = &groups["own"][0];
    let ids: Vec<u32> = own.sections["memberIds"]
        .iter()
        .map(|p| integer(bits, p, "id"))
        .collect();
    assert_eq!(ids, frame.own[0].member_ids);
    let poses = &own.sections["weaponPoses"];
    assert_eq!(integer(bits, &poses[0], "shots"), u32::MAX);
    let enemy = &groups["identified"][0].sections["weaponPoses"][1];
    assert_eq!(integer(bits, enemy, "shots"), big + 2);
    let segment = &groups["projectiles"][0];
    let path: Vec<[f32; 3]> = segment.sections["path"]
        .iter()
        .map(|p| [p["x"], p["y"], p["z"]])
        .collect();
    assert_eq!(path, [[0.0; 3], [1.0, 2.0, 3.0], [4.0, 5.0, 6.0]]);
    let bounce = &segment.sections["ricochets"][0];
    assert_eq!(
        [bounce["point"], bounce["nx"], bounce["ny"], bounce["nz"]],
        [1.0, 0.0, -1.0, 0.0]
    );
    let segment = &segment.fields;
    assert_eq!(integer(bits, segment, "shooter"), big + 4);
    assert_eq!(
        layout["roundKinds"][segment["kind"] as usize],
        b.arsenal().weapons[2].id.as_str()
    );
    assert_eq!(layout["hitKinds"][segment["hit"] as usize], "hull");
    let corpse = &groups["corpses"][0].fields;
    assert_eq!(integer(bits, corpse, "soldier"), big + 6);
    assert_eq!(layout["unitKinds"][corpse["kind"] as usize], "rifle");
    assert_eq!((corpse["slot"], corpse["yaw"]), (5.0, 0.25));
    // The ground patch trails the record: its cell index in limbs, its
    // marks two bytes, then three, to a float.
    let fields = names(&layout["ground"]["fields"]);
    let cell: BTreeMap<String, f32> = fields
        .iter()
        .cloned()
        .zip(data[data.len() - fields.len()..].iter().copied())
        .collect();
    let span = cell["span"] as u32;
    let tile = cell["tile"] as u32;
    let local = span % 256;
    assert_eq!(span / 256, 1);
    assert_eq!(
        ((tile / 1125) * 16 + local / 16) * 18_000 + (tile % 1125) * 16 + local % 16,
        big + 8
    );
    assert_eq!(
        [cell["craterScorch"], cell["tracksTrampledCleared"]],
        [511.0, 16_713_600.0]
    );
}

#[test]
fn unchanged_visibility_is_not_retransmitted() {
    let setup = common::scenario(
        &json!({"size":[128,128],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35})
            .to_string(),
        json!([]),
        json!([]),
    );
    let mut battle = Battle::new(&setup, 1);
    let mut publisher = publication::Publisher::new();
    battle.step();
    let snapshot = publisher.publish(&battle, Side::Blue).unwrap().len();
    battle.step();
    let unchanged = publisher.publish(&battle, Side::Blue).unwrap().len();
    assert!(
        unchanged < snapshot,
        "unchanged visibility must carry no full field: {unchanged} vs {snapshot}"
    );
    assert_eq!(publisher.ground_patch_bytes(), 0);
}

#[test]
fn uniform_learned_ground_is_delivered_without_one_record_per_cell() {
    // A controlled burst saturates four ground tiles. This is the small-scale
    // form of the all-touched map; neither production storage nor delivery may
    // expand equal neighboring marks into a full-cell staging buffer.
    let mut setup = common::scenario(
        &json!({"size":[32,32],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35}).to_string(),
        json!([{"side":"blue","kind":"tank","position":[2,2],"engagement":"return_fire_only"}]),
        json!([{"tick":1,"burst":{"point":[16,16],"weapon":"tank_he"}}]),
    );
    let weapon = setup.rules.weapons.get_mut("tank_he").unwrap();
    weapon.blast_radius_m = 64.0;
    weapon.damage = 0.0;
    weapon.near_miss_suppression = 0.0;
    setup.rules.ground.crater_radius_fraction = 1.0;
    setup.rules.ground.scorch_radius_fraction = 1.0;
    setup.rules.ground.crater_depth_per_m = 512.0;
    setup.rules.ground.scorch_per_burst = 512.0;
    let mut battle = Battle::new(&setup, 1);
    for _ in 0..6 {
        battle.step();
    }
    let known: Vec<_> = battle.known_ground(Side::Blue).cells().collect();
    assert_eq!(known.len(), 32 * 32);
    assert!(known.iter().all(|(_, _, c)| {
        [c.crater, c.scorch, c.tracks, c.trampled, c.cleared] == [255, 255, 0, 0, 0]
    }));
    let mut publisher = publication::Publisher::new();
    let first = publisher.publish(&battle, Side::Blue).unwrap().len() * 4;
    assert!(
        first < 2048,
        "uniform learned cells must stay compact: {first} B"
    );
    publisher.resync();
    assert_eq!(
        publisher.publish(&battle, Side::Blue).unwrap().len() * 4,
        first
    );
}

#[test]
fn fog_delivery_preserves_the_frozen_complete_observation_and_digest() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../specs/city-maps/assets/fog-delivery/oracle.json"
    ))
    .unwrap();
    // C02 relocates only this input field; frozen observations stay untouched.
    let mut scenario = oracle["scenario"].clone();
    scenario["map"]["fog_cell_m"] = scenario["rules"]["sensors"]
        .as_object_mut()
        .unwrap()
        .remove("fog_cell_m")
        .unwrap();
    crate::common::migrate_original_surfaces(&mut scenario["map"]);
    let mut setup: contract::scenario::ScenarioDefinition =
        serde_json::from_value(scenario).unwrap();
    setup.map = crate::common::physical_map(setup.map, &setup.rules);
    let mut battle = Battle::new(&setup, oracle["seed"].as_u64().unwrap());
    let layout: Value = serde_json::from_str(&publication::layout_json(&battle)).unwrap();
    let header = names(&layout["header"]);
    let mut publisher = publication::Publisher::new();
    let mut bits = Vec::new();
    let mut snapshots = 0;
    let mut deltas = 0;
    for row in oracle["rows"].as_array().unwrap() {
        battle.step();
        let side: Side = serde_json::from_value(row["side"].clone()).unwrap();
        if row["resync"].as_bool().unwrap() {
            publisher.resync();
        }
        assert_eq!(
            format!("{:016x}", battle.digest()),
            row["digest"].as_str().unwrap()
        );
        let expected: contract::observation::VisibilityField =
            serde_json::from_value(row["authoritative"]["ground_visibility"].clone()).unwrap();
        // The digest pins authoritative f64 state; the browser oracle pins
        // every packed f32 row (JSON parsing need not preserve every f64 ULP).
        let record = publisher.publish(&battle, side).unwrap();
        let head = |name: &str| record[header.iter().position(|f| f == name).unwrap()];
        let count = head("fogFloats") as usize;
        let words = (head("fogNx") as usize * head("fogNy") as usize).div_ceil(32);
        let at = record.len() - head("groundRunCount") as usize * 4 - count;
        if head("fogFull") == 1.0 {
            snapshots += 1;
            bits = (0..words)
                .map(|i| record[at + i * 2] as u32 | (record[at + i * 2 + 1] as u32) << 16)
                .collect();
        } else {
            deltas += 1;
            for change in record[at..at + count].chunks_exact(3) {
                bits[change[0] as usize] = change[1] as u32 | (change[2] as u32) << 16;
            }
        }
        assert_eq!(bits, expected.bits, "tick {}", row["authoritative"]["tick"]);
    }
    assert!(
        snapshots >= 4,
        "initial, each side switch and resync replace the field"
    );
    assert!(
        deltas > 0,
        "the active oracle must exercise incremental fields"
    );
}

#[test]
fn fog_snapshots_fit_the_admitted_extents_and_preserve_padding() {
    use contract::observation::{ObservationFrame, VisibilityField};
    let patch = publication::GroundHeader {
        epoch: 1,
        side: Side::Blue,
        base: 0,
        revision: 0,
        full: true,
        count: 0,
    };
    for side in [12_000u32, 15_000, 18_000] {
        for cell in [2u32, 4, 8] {
            let nx = side.div_ceil(cell);
            let cells = nx as usize * nx as usize;
            let words = cells.div_ceil(32);
            let mut bits = vec![0; words];
            bits[words - 1] = u32::MAX;
            let frame = ObservationFrame {
                ground_visibility: VisibilityField {
                    cell_m: cell as f64,
                    nx,
                    ny: nx,
                    bits,
                },
                ..Default::default()
            };
            let mut out = Vec::new();
            publication::pack(&frame, &patch, &full_fog(), std::iter::empty(), &mut out).unwrap();
            assert!(
                out.len() * 4 <= 20_250_108,
                "18 km at 2 m is the snapshot bound"
            );
            let tail = if cells.is_multiple_of(32) {
                u32::MAX
            } else {
                (1u32 << (cells % 32)) - 1
            };
            assert_eq!(
                out[out.len() - 2] as u32 | (out[out.len() - 1] as u32) << 16,
                tail
            );
        }
    }
}

#[test]
fn high_churn_replaces_the_field_inside_the_same_stream() {
    let setup = common::scenario(
        &json!({"size":[64,64],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35}).to_string(),
        json!([{"side":"blue","kind":"rifle","position":[16,32]}]),
        json!([{"tick":2,"add_prop":{"kind":"wall","center":[32,32],"yaw":0,"half_extents":[4,32,8]}}]),
    );
    let mut battle = Battle::new(&setup, 1);
    let mut publisher = publication::Publisher::new();
    let layout: Value = serde_json::from_str(&publication::layout_json(&battle)).unwrap();
    let header = names(&layout["header"]);
    battle.step();
    publisher.publish(&battle, Side::Blue).unwrap();
    let before = battle.observe(Side::Blue).ground_visibility.bits.clone();
    for _ in 0..30 {
        battle.step();
        if battle.observe(Side::Blue).ground_visibility.bits != before {
            break;
        }
    }
    let expected = &battle.observe(Side::Blue).ground_visibility.bits;
    assert_ne!(&before, expected, "the new wall must remove visible cells");
    let record = publisher.publish(&battle, Side::Blue).unwrap();
    let head = |name: &str| record[header.iter().position(|field| field == name).unwrap()];
    assert_eq!(head("groundEpoch"), 1.0);
    assert_eq!(head("groundFull"), 0.0);
    assert_eq!(
        head("fogFull"),
        1.0,
        "dense word replacements must use the smaller snapshot"
    );
    assert_eq!(head("fogBaseLo"), 1.0);
    assert_eq!(head("fogRevisionLo"), 2.0);
    assert_eq!(head("fogFloats") as usize, expected.len() * 2);
}

fn full_fog() -> publication::FogPatch<'static> {
    publication::FogPatch {
        full: true,
        base: 0,
        revision: 1,
        changed: &[],
    }
}

#[test]
fn an_over_budget_record_is_rejected_before_output_allocation() {
    let frame = contract::observation::ObservationFrame::default();
    let ground = publication::GroundHeader {
        epoch: 1,
        side: Side::Blue,
        base: 0,
        revision: 1,
        full: true,
        count: publication::MAX_PUBLICATION_BYTES / 16 + 1,
    };
    let mut out = vec![1.0, 2.0];
    let capacity = out.capacity();
    let error =
        publication::pack(&frame, &ground, &full_fog(), std::iter::empty(), &mut out).unwrap_err();
    assert!(error.contains("allocation allowance"));
    assert_eq!(
        out,
        [1.0, 2.0],
        "the previous complete record remains intact"
    );
    assert_eq!(
        out.capacity(),
        capacity,
        "rejection cannot allocate output staging"
    );
}

#[test]
fn oversized_fog_dimensions_cannot_wrap_past_the_delivery_bound() {
    let frame = contract::observation::ObservationFrame {
        ground_visibility: contract::observation::VisibilityField {
            cell_m: 2.0,
            nx: 65536,
            ny: 65536,
            bits: vec![],
        },
        ..Default::default()
    };
    let ground = publication::GroundHeader {
        epoch: 1,
        side: Side::Blue,
        base: 0,
        revision: 0,
        full: true,
        count: 0,
    };
    let mut out = vec![3.0];
    assert!(
        publication::pack(&frame, &ground, &full_fog(), std::iter::empty(), &mut out)
            .unwrap_err()
            .contains("admitted bound")
    );
    assert_eq!(out, [3.0]);
}

#[test]
fn known_bodies_publish_exact_current_building_owner_and_authored_source_ids() {
    let mut b = Battle::new(
        &common::scenario(
            &json!({"size":[800,600],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35})
                .to_string(),
            json!([{ "side":"blue","kind":"rifle","position":[100,300]}]),
            json!([]),
        ),
        1,
    );
    b.step();
    let layout: Value = serde_json::from_str(&publication::layout_json(&b)).unwrap();
    let fields = layout["groups"]
        .as_array()
        .unwrap()
        .iter()
        .find(|g| g["name"] == "knownProps")
        .unwrap()["fields"]
        .as_array()
        .unwrap();
    for name in [
        "idLo",
        "idHi",
        "buildingLo",
        "buildingHi",
        "structureOwnerLo",
        "structureOwnerHi",
        "authoredPropLo",
        "authoredPropHi",
        "replacesLo",
        "replacesHi",
    ] {
        assert!(
            fields.contains(&json!(name)),
            "missing exact identity field {name}"
        );
    }
    let big = (1u32 << 24) + 1;
    let mut frame = b.observe(Side::Blue).clone();
    frame.known_props = vec![
        contract::observation::KnownProp {
            kind: common::kind("wall"),
            center: [100.0, 200.0],
            yaw: 0.25,
            half_extents: [4.0, 3.0, 2.0],
            base_z: 1.0,
            replaces: Some(big + 6),
            destroyed: false,
            id: u32::MAX,
            building: Some(big + 2),
            structure_owner: Some(big + 4),
            authored_prop: Some(big + 8),
        },
        contract::observation::KnownProp {
            kind: common::kind("crate"),
            center: [400.0, 300.0],
            yaw: 0.0,
            half_extents: [1.0; 3],
            base_z: 0.0,
            replaces: None,
            destroyed: false,
            id: big + 10,
            building: None,
            structure_owner: None,
            authored_prop: None,
        },
    ];
    frame.own[0].garrison = Some(contract::observation::GarrisonState {
        building: u32::MAX,
        phase: contract::observation::GarrisonPhase::Inside,
        progress: 1.0,
        center: [100.0, 200.0],
        half: [4.0, 3.0],
    });
    let header = publication::GroundHeader {
        epoch: 0,
        side: Side::Blue,
        base: 0,
        revision: 0,
        full: true,
        count: 0,
    };
    let mut data = Vec::new();
    publication::pack(&frame, &header, &full_fog(), std::iter::empty(), &mut data).unwrap();
    let groups = decode(&layout, &data);
    let bits = layout["limbBits"].as_u64().unwrap() as u32;
    let p = &groups["knownProps"][0].fields;
    for (name, expected) in [
        ("id", u32::MAX),
        ("building", big + 2),
        ("structureOwner", big + 4),
        ("authoredProp", big + 8),
        ("replaces", big + 6),
    ] {
        assert_eq!(integer(bits, p, name), expected, "{name}");
    }
    assert_eq!(
        [p["x"], p["y"], p["yaw"], p["hx"], p["hy"], p["hz"], p["baseZ"]],
        [100.0, 200.0, 0.25, 4.0, 3.0, 2.0, 1.0]
    );
    let dynamic = &groups["knownProps"][1].fields;
    assert_eq!(integer(bits, dynamic, "id"), big + 10);
    for name in ["building", "structureOwner", "authoredProp", "replaces"] {
        assert_eq!(
            [dynamic[&format!("{name}Lo")], dynamic[&format!("{name}Hi")]],
            [-1.0, -1.0]
        );
    }
    assert_eq!(
        integer(bits, &groups["own"][0].fields, "garrisonBuilding"),
        u32::MAX
    );
}

#[test]
fn aggregate_codec_preserves_every_original_animation_word() {
    let original: Value = serde_json::from_str(include_str!(
        "../../../specs/city-maps/assets/ground-transport/animation-codec-vectors.json"
    ))
    .unwrap();
    let receipt: Value = serde_json::from_str(include_str!(
        "../../../specs/city-maps/assets/building-aggregate/animation-codec-vectors.json"
    ))
    .unwrap();
    let layout = &receipt["layout"];
    let header = names(&layout["header"]);
    for phase in ["base", "entering", "inside", "exiting", "wide"] {
        let row = &receipt["vectors"][phase];
        let frame: contract::observation::ObservationFrame =
            serde_json::from_value(row["frame"].clone()).unwrap();
        let patch = &row["patch"];
        let runs = patch["cells"].as_array().unwrap().iter().map(|c| {
            let cell = c["cell"].as_u64().unwrap() as u32;
            let (x, y) = (cell % 18000, cell / 18000);
            sim::ground::GroundRunPatch {
                tile: y / 16 * 1125 + x / 16,
                start: ((y % 16) * 16 + x % 16) as u16,
                len: 1,
                marks: sim::ground::GroundCell {
                    crater: c["crater"].as_u64().unwrap() as u8,
                    scorch: c["scorch"].as_u64().unwrap() as u8,
                    tracks: c["tracks"].as_u64().unwrap() as u8,
                    trampled: c["trampled"].as_u64().unwrap() as u8,
                    cleared: c["cleared"].as_u64().unwrap() as u8,
                },
            }
        });
        let ground = publication::GroundHeader {
            epoch: 4,
            side: Side::Red,
            base: 6,
            revision: 9,
            full: false,
            count: 2,
        };
        let mut data = Vec::new();
        publication::pack(&frame, &ground, &full_fog(), runs, &mut data).unwrap();
        assert_eq!(
            data.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            row["bits"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap() as u32)
                .collect::<Vec<_>>(),
            "production receipt {phase}"
        );
        if phase == "wide" {
            continue;
        }
        // The only animation changes are one scalar owner split and exact
        // KnownProp identities. Traverse variable sections unchanged.
        let mut legacy = data[..header.len()].to_vec();
        let mut at = header.len();
        for group in layout["groups"].as_array().unwrap() {
            let fields = names(&group["fields"]);
            let count = data[header
                .iter()
                .position(|n| n == group["count"].as_str().unwrap())
                .unwrap()] as usize;
            let mut rows = Vec::new();
            for _ in 0..count {
                let row = &data[at..at + fields.len()];
                at += fields.len();
                rows.push(row);
                for (i, name) in fields.iter().enumerate() {
                    if group["name"] == "knownProps"
                        && [
                            "idLo",
                            "idHi",
                            "buildingLo",
                            "buildingHi",
                            "structureOwnerLo",
                            "structureOwnerHi",
                            "authoredPropLo",
                            "authoredPropHi",
                        ]
                        .contains(&name.as_str())
                    {
                        continue;
                    }
                    if name == "garrisonBuildingHi" || name == "replacesHi" {
                        continue;
                    }
                    if name == "garrisonBuildingLo" || name == "replacesLo" {
                        legacy.push(if row[i] < 0.0 {
                            -1.0
                        } else {
                            row[i] + row[i + 1] * 65536.0
                        });
                    } else {
                        legacy.push(row[i]);
                    }
                }
            }
            for row in rows {
                for section in group["sections"].as_array().unwrap() {
                    let count = row[fields
                        .iter()
                        .position(|n| n == section["count"].as_str().unwrap())
                        .unwrap()] as usize;
                    let len = count * section["fields"].as_array().unwrap().len();
                    legacy.extend_from_slice(&data[at..at + len]);
                    at += len;
                }
            }
        }
        let tail = original[phase]["bits"].as_array().unwrap();
        let old_tail = tail.len() - 8;
        // Run storage alone changes the ground tail. The exact old packed
        // cells remain its independently frozen oracle in the web decoder.
        legacy.extend_from_slice(&data[at..data.len() - 8]);
        assert_eq!(
            legacy.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            tail[..old_tail]
                .iter()
                .map(|v| v.as_u64().unwrap() as u32)
                .collect::<Vec<_>>(),
            "all original animation/fog/header words {phase}"
        );
    }
}
