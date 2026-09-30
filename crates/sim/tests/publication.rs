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
    let map =
        json!({ "size": [800, 600], "height_grid_m": 4, "slope_cutoff_deg": 35, "props": [] });
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
    let big_cell = contract::observation::GroundCellPatch {
        cell: big + 8,
        crater: 255,
        scorch: 1,
        tracks: 128,
        trampled: 7,
        cleared: 255,
    };
    let patch = contract::observation::GroundPatch {
        epoch: 3,
        side: Side::Red,
        base_revision: 5,
        revision: 9,
        full: false,
        cells: vec![big_cell],
    };
    let mut data = Vec::new();
    publication::pack(&frame, &patch, &mut data);
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
    assert_eq!(integer(bits, &cell, "cell"), big + 8);
    assert_eq!(
        [cell["craterScorch"], cell["tracksTrampledCleared"]],
        [511.0, 16_713_600.0]
    );
}

#[test]
fn unchanged_visibility_is_not_retransmitted() {
    let setup = common::scenario(
        &json!({"size":[128,128],"height_grid_m":4,"slope_cutoff_deg":35}).to_string(),
        json!([]),
        json!([]),
    );
    let mut battle = Battle::new(&setup, 1);
    let mut publisher = publication::Publisher::new();
    battle.step();
    let snapshot = publisher.publish(&battle, Side::Blue).len();
    battle.step();
    let unchanged = publisher.publish(&battle, Side::Blue).len();
    assert!(
        unchanged < snapshot,
        "unchanged visibility must carry no full field: {unchanged} vs {snapshot}"
    );
    assert_eq!(publisher.last_patch().unwrap().cells.len(), 0);
}

#[test]
fn fog_delivery_preserves_the_frozen_complete_observation_and_digest() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../specs/city-maps/assets/fog-delivery/oracle.json"
    ))
    .unwrap();
    let setup = serde_json::from_value(oracle["scenario"].clone()).unwrap();
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
        let expected: contract::observation::ObservationFrame =
            serde_json::from_value(row["authoritative"].clone()).unwrap();
        // The digest pins authoritative f64 state; the browser oracle pins
        // every packed f32 row (JSON parsing need not preserve every f64 ULP).
        let record = publisher.publish(&battle, side);
        let head = |name: &str| record[header.iter().position(|f| f == name).unwrap()];
        let count = head("fogFloats") as usize;
        let words = (head("fogNx") as usize * head("fogNy") as usize).div_ceil(32);
        let at = record.len() - head("groundCellCount") as usize * 4 - count;
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
        assert_eq!(
            bits, expected.ground_visibility.bits,
            "tick {}",
            expected.tick
        );
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
    use contract::observation::{GroundPatch, ObservationFrame, VisibilityField};
    let patch = GroundPatch {
        epoch: 1,
        side: Side::Blue,
        base_revision: 0,
        revision: 0,
        full: true,
        cells: vec![],
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
            publication::pack(&frame, &patch, &mut out);
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
        &json!({"size":[64,64],"height_grid_m":4,"slope_cutoff_deg":35}).to_string(),
        json!([{"side":"blue","kind":"rifle","position":[16,32]}]),
        json!([{"tick":2,"add_prop":{"kind":"wall","center":[32,32],"yaw":0,"half_extents":[4,32,8]}}]),
    );
    let mut battle = Battle::new(&setup, 1);
    let mut publisher = publication::Publisher::new();
    let layout: Value = serde_json::from_str(&publication::layout_json(&battle)).unwrap();
    let header = names(&layout["header"]);
    battle.step();
    publisher.publish(&battle, Side::Blue);
    let before = battle.observe(Side::Blue).ground_visibility.bits.clone();
    for _ in 0..30 {
        battle.step();
        if battle.observe(Side::Blue).ground_visibility.bits != before {
            break;
        }
    }
    let expected = &battle.observe(Side::Blue).ground_visibility.bits;
    assert_ne!(&before, expected, "the new wall must remove visible cells");
    let record = publisher.publish(&battle, Side::Blue);
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
