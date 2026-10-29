//! The packed publication carries integers exactly past float32's 2²⁴, read
//! back only through the published layout.
use std::collections::BTreeMap;

use contract::ids::Side;
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::publication;

use crate::common;
mod codec {
    include!("common/publication_codec.rs");
}

#[test]
fn publication_layout_exposes_player_rows_instead_of_side_economy() {
    let map = json!({ "size": [800, 600], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35, "props": [] });
    let battle = Battle::new(
        &common::scenario(
            &map.to_string(),
            json!([{ "side": "blue", "kind": "test_rifle", "position": [100, 300] }]),
            json!([]),
        ),
        1,
    );
    let layout: Value = serde_json::from_str(&publication::layout_json(&battle)).unwrap();
    let header = names(&layout["header"]);
    assert!(header.contains(&"playersCount".to_owned()));
    assert!(!header.contains(&"readyBlue".to_owned()));
    assert!(!header.contains(&"credits".to_owned()));
    assert_eq!(
        names(
            &layout["groups"]
                .as_array()
                .unwrap()
                .iter()
                .find(|group| group["name"] == "players")
                .expect("players group")["fields"]
        ),
        vec![
            "index",
            "id",
            "controller",
            "ready",
            "credits",
            "unitCount",
            "cap"
        ]
    );
}

#[test]
fn two_player_publication_keeps_player_rows_and_pending_owner() {
    let golden: Value = serde_json::from_str(include_str!(
        "../../../fixtures/parity/publication/2v2-frame.json"
    ))
    .unwrap();
    let mut setup = crate::skirmish::setup();
    let mut blue_two = setup.players[0].clone();
    blue_two.id = "blue-2".into();
    let mut red_two = setup.players[1].clone();
    red_two.id = "red-2".into();
    setup.players.extend([blue_two, red_two]);
    setup.skirmish.as_mut().unwrap().sites.entries.extend([
        contract::encounter::EntrySite {
            side: Side::Blue,
            center: [200.0, 10.0],
            yaw: std::f64::consts::FRAC_PI_2,
        },
        contract::encounter::EntrySite {
            side: Side::Red,
            center: [200.0, 590.0],
            yaw: -std::f64::consts::FRAC_PI_2,
        },
    ]);
    let battle = Battle::new(&setup, 1);
    let mut frame = battle.observe(Side::Blue).clone();
    frame.skirmish.as_mut().unwrap().players[1]
        .pending
        .push(contract::skirmish::PendingPurchase {
            id: contract::skirmish::PurchaseId(7),
            kind: common::unit_kind("test_tank"),
            destination: [100.0, 20.0],
            confirmed_tick: 4,
            blocked: false,
            arrival_tick: None,
            entry: 0,
        });
    let layout: Value = serde_json::from_str(&publication::layout_json(&battle)).unwrap();
    let patch = publication::GroundHeader {
        epoch: 1,
        side: Side::Blue,
        base: 0,
        revision: 1,
        full: true,
        count: 0,
    };
    let mut data = Vec::new();
    publication::pack_logical(&frame, &patch, &full_fog(), std::iter::empty(), &mut data).unwrap();
    let groups = decode(&layout, &data);
    assert_eq!(groups["players"].len(), 2);
    assert_eq!(groups["players"][1].fields["id"], 2.0);
    assert_eq!(groups["pendingPurchases"][0].fields["owner"], 2.0);
    assert_eq!(golden["players"][1]["pending"][0]["owner"], 2);
}

#[test]
fn reader_rejects_a_publication_layout_mismatch() {
    let map = json!({ "size": [800, 600], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35, "props": [] });
    let battle = Battle::new(&common::scenario(&map.to_string(), json!([]), json!([])), 1);
    let mut layout: Value = serde_json::from_str(&publication::layout_json(&battle)).unwrap();
    layout["header"][0] = json!("wrong");
    let error = match sim::publication::reader::Reader::new(&layout.to_string()) {
        Ok(_) => panic!("layout mismatch was accepted"),
        Err(error) => error,
    };
    assert!(error.contains("layout"));
}

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
                { "side": "blue", "kind": "test_rifle", "position": [100, 300] },
                { "side": "red", "kind": "test_tank", "position": [300, 300], "yaw": std::f64::consts::PI },
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
    frame.own[0].member_active_mounts[0] = None;
    frame.own[0].member_active_mounts[3] = Some(1);
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
        kind: common::unit_kind("test_rifle"),
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
    publication::pack_logical(&frame, &patch, &full_fog(), std::iter::once(run), &mut data)
        .unwrap();
    let groups = decode(&layout, &data);

    let own = &groups["own"][0];
    assert_eq!(own.fields["fuelSeconds"], -1.0);
    assert_eq!(own.fields["afterburner"], 0.0);
    assert_eq!(own.fields["inDogfight"], 0.0);
    assert_eq!(own.fields["flareCharges"], -1.0);
    assert_eq!(own.fields["flareCooldown"], -1.0);
    let ids: Vec<u32> = own.sections["memberIds"]
        .iter()
        .map(|p| integer(bits, p, "id"))
        .collect();
    assert_eq!(ids, frame.own[0].member_ids);
    let selected: Vec<Option<u8>> = own.sections["memberIds"]
        .iter()
        .map(|p| (p["activeMount"] >= 0.0).then_some(p["activeMount"] as u8))
        .collect();
    assert_eq!(selected, frame.own[0].member_active_mounts);

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
    assert_eq!(layout["unitKinds"][corpse["kind"] as usize], "test_rifle");
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
fn a_contact_publishes_its_height_and_layer() {
    // A ground area and an air area: each is read back, by the layout alone,
    // at its cause's height and in its band.
    use contract::catalog::AltitudeLayer;
    use contract::observation::{ApproximateContact, ContactId, ContactSource};
    let map =
        json!({ "size": [400, 400], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 });
    let b = Battle::new(
        &common::scenario(
            &map.to_string(),
            json!([{ "side": "blue", "kind": "test_rifle", "position": [100, 100] }]),
            json!([]),
        ),
        1,
    );
    let mut frame = b.observe(Side::Blue).clone();
    let area = |id, center, layer| ApproximateContact {
        id: ContactId(id),
        source: ContactSource::Firing,
        center,
        layer,
        aloft: layer == AltitudeLayer::LowAir,
        radius: 30.0,
        evidence_tick: 4,
        expires_tick: 900,
        kind: None,
        heard: 1,
        bearing_deg: None,
        range_m: None,
    };
    frame.contacts = vec![
        area(1, [120.0, 80.0, 2.5], AltitudeLayer::Ground),
        area(2, [300.0, 250.0, 22.5], AltitudeLayer::LowAir),
    ];
    let layout: Value = serde_json::from_str(&publication::layout_json(&b)).unwrap();
    let patch = publication::GroundHeader {
        epoch: 1,
        side: Side::Blue,
        base: 0,
        revision: 0,
        full: false,
        count: 0,
    };
    let mut data = Vec::new();
    publication::pack_logical(&frame, &patch, &full_fog(), std::iter::empty(), &mut data).unwrap();
    let read: Vec<([f32; 3], &str, f32)> = decode(&layout, &data)["contacts"]
        .iter()
        .map(|c| {
            let f = &c.fields;
            let layer = layout["layers"][f["layer"] as usize].as_str().unwrap();
            ([f["x"], f["y"], f["z"]], layer, f["aloft"])
        })
        .collect();
    assert_eq!(
        read,
        [
            ([120.0, 80.0, 2.5], "ground", 0.0),
            ([300.0, 250.0, 22.5], "low_air", 1.0)
        ]
    );
}

#[test]
fn a_falling_airframe_publishes_its_place_and_attitude() {
    // Read back by the layout alone: who it is to this side, its type, and
    // how it hangs where it falls.
    use contract::observation::FallingAirframe;
    let map =
        json!({ "size": [400, 400], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 });
    let b = Battle::new(
        &common::scenario(
            &map.to_string(),
            json!([{ "side": "blue", "kind": "test_rifle", "position": [100, 100] }]),
            json!([]),
        ),
        1,
    );
    let mut frame = b.observe(Side::Blue).clone();
    frame.crashes = vec![FallingAirframe {
        id: 7,
        own: false,
        kind: common::unit_kind("test_heli"),
        position: [120.5, 80.25, 14.5],
        yaw: 1.25,
        pitch: -0.25,
        roll: 0.125,
    }];
    let layout: Value = serde_json::from_str(&publication::layout_json(&b)).unwrap();
    let patch = publication::GroundHeader {
        epoch: 1,
        side: Side::Blue,
        base: 0,
        revision: 0,
        full: false,
        count: 0,
    };
    let mut data = Vec::new();
    publication::pack_logical(&frame, &patch, &full_fog(), std::iter::empty(), &mut data).unwrap();
    let crashes = &decode(&layout, &data)["crashes"];
    assert_eq!(crashes.len(), 1);
    let f = &crashes[0].fields;
    assert_eq!(layout["unitKinds"][f["kind"] as usize], "test_heli");
    assert_eq!(
        [f["id"], f["own"], f["x"], f["y"], f["z"], f["yaw"], f["pitch"], f["roll"]],
        [7.0, 0.0, 120.5, 80.25, 14.5, 1.25, -0.25, 0.125]
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
        json!([{"side":"blue","kind":"test_tank","position":[2,2],"engagement":"return_fire_only"}]),
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
fn a_detailed_learned_ground_burst_stays_within_the_delivery_budget() {
    let mut setup = common::scenario(
        &json!({"size":[64,64],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35}).to_string(),
        json!([{"side":"blue","kind":"test_tank","position":[2,2],"engagement":"return_fire_only"}]),
        json!([{"tick":1,"burst":{"point":[32,32],"weapon":"tank_he"}}]),
    );
    let weapon = setup.rules.weapons.get_mut("tank_he").unwrap();
    weapon.blast_radius_m = 28.0;
    weapon.damage = 0.0;
    weapon.near_miss_suppression = 0.0;
    setup.rules.ground.crater_radius_fraction = 1.0;
    setup.rules.ground.scorch_radius_fraction = 1.0;
    setup.rules.ground.crater_depth_per_m = 4.0;
    setup.rules.ground.scorch_per_burst = 128.0;
    let mut battle = Battle::new(&setup, 1);
    for _ in 0..6 {
        battle.step();
    }
    let canonical: Vec<_> = battle
        .known_ground(Side::Blue)
        .change_runs_since(0)
        .collect();
    assert!(
        canonical.len() * 16 > 19_800,
        "the learned burst must expose uncompressed-run amplification: {} rows",
        canonical.len()
    );
    let digest = battle.digest();
    let mut publisher = publication::Publisher::new();
    let wire = publisher.publish(&battle, Side::Blue).unwrap();
    assert!(
        wire.len() * 4 <= 19_800,
        "complete detailed-ground publication: {} B for {} rows",
        wire.len() * 4,
        canonical.len()
    );
    let layout: Value = serde_json::from_str(&publication::layout_json(&battle)).unwrap();
    let header = names(&layout["header"]);
    let mut at = header.len();
    for _ in layout["groups"].as_array().unwrap() {
        at += 3 + wire[at + 2] as usize;
    }
    at += wire[header.iter().position(|name| name == "fogFloats").unwrap()] as usize;
    let restored = codec::ground(&wire[at..], canonical.len());
    let expected: Vec<f32> = canonical
        .iter()
        .flat_map(|run| {
            let c = run.marks;
            [
                run.tile as f32,
                (u32::from(run.start) + u32::from(run.len) * 256) as f32,
                (u32::from(c.crater) + (u32::from(c.scorch) << 8)) as f32,
                (u32::from(c.tracks) + (u32::from(c.trampled) << 8) + (u32::from(c.cleared) << 16))
                    as f32,
            ]
        })
        .collect();
    assert_eq!(
        restored.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
        expected.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
    );
    assert_eq!(digest, battle.digest());
}

/// The native half of the publication stream record: each tick's battle
/// digest, published words and delivered fog, as hashes.
/// `web/tests/fogDelivery.test.ts` holds the Wasm build and the TypeScript
/// decoder to the same values.
#[test]
fn the_publication_stream_matches_its_paired_record() {
    let record: Value = serde_json::from_str(include_str!(
        "../../../fixtures/parity/publication/stream.json"
    ))
    .unwrap();
    publication_stream(record, "publication/stream.json", false);
}

#[test]
fn the_combat_stream_matches_its_paired_record() {
    let record: Value = serde_json::from_str(include_str!(
        "../../../fixtures/parity/publication/combat.json"
    ))
    .unwrap();
    publication_stream(record, "publication/combat.json", true);
}

#[test]
fn a_large_coordinate_arrangement_matches_its_paired_state_record() {
    let record: Value = serde_json::from_str(include_str!(
        "../../../fixtures/parity/publication/arrangement.json"
    ))
    .unwrap();
    publication_stream(record, "publication/arrangement.json", false);
}

#[test]
fn cover_facing_matches_its_paired_state_record() {
    let record: Value = serde_json::from_str(include_str!(
        "../../../fixtures/parity/publication/cover-facing.json"
    ))
    .unwrap();
    publication_stream(record, "publication/cover-facing.json", false);
}

#[test]
fn contact_lifecycle_matches_its_paired_state_record() {
    let record: Value = serde_json::from_str(include_str!(
        "../../../fixtures/parity/publication/contact-lifecycle.json"
    ))
    .unwrap();
    publication_stream(record, "publication/contact-lifecycle.json", false);
}

fn publication_stream(record: Value, path: &str, combat: bool) {
    let map = match record["map"].as_str() {
        Some(id) => sim::maps::load(id).unwrap().definition,
        None => serde_json::from_value(record["map"].clone()).unwrap(),
    };
    let mut setup = common::scenario_with(
        &serde_json::to_string(&map).unwrap(),
        record["units"].clone(),
        record["events"].clone(),
        record["scripts"].clone(),
    );
    // The web half runs a lab's catalog, the test set (the game's units and
    // the test units): the published type table must be the same one.
    setup.rules.catalog = contract::catalog::resolve(&sim::fixtures::catalog_documents(
        sim::fixtures::CatalogSet::Test,
    ))
    .unwrap();
    let mut battle = Battle::new(&setup, record["seed"].as_u64().unwrap());
    let layout: Value = serde_json::from_str(&publication::layout_json(&battle)).unwrap();
    let header = names(&layout["header"]);
    let mut publisher = publication::Publisher::new();
    let mut bits: Vec<u32> = Vec::new();
    let mut snapshots = 0;
    let mut deltas = 0;
    let mut blessed = record.clone();
    blessed["initial_digest"] = json!(format!("{:016x}", battle.digest()));
    let mut saw_shot = false;
    let mut saw_impact = false;
    for row in blessed["rows"].as_array_mut().unwrap() {
        battle.step();
        let side: Side = serde_json::from_value(row["side"].clone()).unwrap();
        if row["resync"].as_bool().unwrap() {
            publisher.resync();
        }
        let words = publisher.publish(&battle, side).unwrap();
        let head = |name: &str| words[header.iter().position(|f| f == name).unwrap()];
        let count = head("fogFloats") as usize;
        let field_words = (head("fogNx") as usize * head("fogNy") as usize).div_ceil(32);
        let mut at = header.len();
        for _ in layout["groups"].as_array().unwrap() {
            at += 3 + words[at + 2] as usize;
        }
        if head("fogFull") == 1.0 {
            snapshots += 1;
            bits = (0..field_words)
                .map(|i| words[at + i * 2] as u32 | (words[at + i * 2 + 1] as u32) << 16)
                .collect();
        } else {
            deltas += 1;
            for change in words[at..at + count].as_chunks::<3>().0 {
                bits[change[0] as usize] = change[1] as u32 | (change[2] as u32) << 16;
            }
        }
        let observation = battle.observe(side);
        // Snapshots and deltas alike deliver the side's authoritative field.
        assert_eq!(
            bits,
            observation.ground_visibility.bits,
            "tick {}",
            battle.tick()
        );
        saw_shot |= observation
            .own
            .iter()
            .flat_map(|unit| &unit.weapon_poses)
            .any(|pose| pose.shots > 0);
        saw_impact |= observation
            .projectiles
            .iter()
            .any(|projectile| projectile.hit != contract::observation::SegmentHit::None);
        let hash = |bytes: Vec<u8>| json!(contract::identity::bytes_hash(&bytes));
        row["digest"] = json!(format!("{:016x}", battle.digest()));
        row["publication_sha256"] = hash(
            words
                .iter()
                .flat_map(|w| w.to_bits().to_le_bytes())
                .collect(),
        );
        row["fog_sha256"] = hash(bits.iter().flat_map(|w| w.to_le_bytes()).collect());
    }
    assert!(
        snapshots >= 4,
        "initial, each side switch and resync replace the field"
    );
    assert!(deltas > 0, "the stream must exercise incremental fields");
    if combat {
        assert!(saw_shot, "the paired stream must observe combat firing");
        assert!(
            saw_impact,
            "the paired stream must observe a projectile impact"
        );
    }
    if common::bless_parity(path, &blessed) {
        return;
    }
    assert_eq!(record["initial_digest"], blessed["initial_digest"]);
    for (tick, (row, expected)) in blessed["rows"]
        .as_array()
        .unwrap()
        .iter()
        .zip(record["rows"].as_array().unwrap())
        .enumerate()
    {
        assert_eq!(row, expected, "tick {}", tick + 1);
    }
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
            publication::pack_logical(&frame, &patch, &full_fog(), std::iter::empty(), &mut out)
                .unwrap();
            assert!(
                out.len() * 4 <= 20_250_000 + publication::HEADER_WORDS * 4,
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
        json!([{"side":"blue","kind":"test_rifle","position":[16,32]}]),
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
        publication::pack_logical(&frame, &ground, &full_fog(), std::iter::empty(), &mut out)
            .unwrap_err();
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
        publication::pack_logical(&frame, &ground, &full_fog(), std::iter::empty(), &mut out)
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
            json!([{ "side":"blue","kind":"test_rifle","position":[100,300]}]),
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
            wreck_of: None,
        },
        contract::observation::KnownProp {
            kind: common::kind("heavy_wreck"),
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
            wreck_of: Some(common::unit_kind("test_tank")),
        },
    ];
    frame.own[0].hold = Some(contract::observation::HoldState {
        host: contract::observation::HostRef::Building {
            id: u32::MAX,
            center: [100.0, 200.0],
            half: [4.0, 3.0],
        },
        phase: contract::observation::HoldPhase::Inside,
        progress: 1.0,
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
    publication::pack_logical(&frame, &header, &full_fog(), std::iter::empty(), &mut data).unwrap();
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
    // A wreck names its unit type by its index in `unitKinds`; nothing else does.
    assert_eq!(p["wreckOf"], -1.0);
    assert_eq!(
        dynamic["wreckOf"],
        f32::from(common::unit_kind("test_tank").0)
    );
    for name in ["building", "structureOwner", "authoredProp", "replaces"] {
        assert_eq!(
            [dynamic[&format!("{name}Lo")], dynamic[&format!("{name}Hi")]],
            [-1.0, -1.0]
        );
    }
    assert_eq!(integer(bits, &groups["own"][0].fields, "host"), u32::MAX);
}

/// The encoder's half of the codec vectors: every frame packs to exactly the
/// words `web/tests/observation.test.ts` decodes. The vectors keep their own
/// kind tables; the field order is this build's.
#[test]
fn the_encoder_packs_the_codec_vectors_the_web_decoder_reads() {
    let record: Value = serde_json::from_str(include_str!(
        "../../../fixtures/parity/publication/codec-vectors.json"
    ))
    .unwrap();
    let probe = Battle::new(
        &common::scenario(
            r#"{"size":[32,32],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35}"#,
            json!([]),
            json!([]),
        ),
        1,
    );
    let current: Value = serde_json::from_str(&publication::layout_json(&probe)).unwrap();
    let mut blessed = record.clone();
    for key in [
        "header",
        "groups",
        "groupDelivery",
        "objectiveIds",
        "layers",
        "soundCategories",
        "shoutTriggers",
        "intensities",
        "hostKinds",
        "holdPhases",
        "orderErrors",
    ] {
        blessed["layout"][key] = current[key].clone();
    }
    blessed["layout"]["ground"]["packed"] = current["ground"]["packed"].clone();
    for row in blessed["vectors"].as_object_mut().unwrap().values_mut() {
        let frame: contract::observation::ObservationFrame =
            serde_json::from_value(row["frame"].clone()).unwrap();
        let runs = row["patch"]["cells"].as_array().unwrap().iter().map(|c| {
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
        publication::pack_logical(&frame, &ground, &full_fog(), runs, &mut data).unwrap();
        row["bits"] = json!(data.iter().map(|v| v.to_bits()).collect::<Vec<_>>());
    }
    if common::bless_parity("publication/codec-vectors.json", &blessed) {
        return;
    }
    assert_eq!(blessed["layout"], record["layout"], "field order");
    for (phase, row) in blessed["vectors"].as_object().unwrap() {
        assert_eq!(row["bits"], record["vectors"][phase]["bits"], "{phase}");
    }
}

#[test]
fn unchanged_observation_groups_do_not_retransmit_own_rows() {
    let setup = common::scenario(
        &json!({"size":[128,128],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35})
            .to_string(),
        json!([{"side":"blue","kind":"test_rifle","position":[32,32]}]),
        json!([]),
    );
    let battle = Battle::new(&setup, 1);
    let mut publisher = publication::Publisher::new();
    let snapshot = publisher.publish(&battle, Side::Blue).unwrap().to_vec();
    let own = codec::group(&snapshot, publication::HEADER_WORDS, &[]);
    assert!(!own.is_empty());
    let steady = publisher.publish(&battle, Side::Blue).unwrap();
    let metadata = publication::HEADER_WORDS;
    assert_eq!(
        steady[metadata + 2],
        0.0,
        "unchanged own rows transmit no payload"
    );
    assert_eq!(
        codec::group(steady, metadata, &own)
            .iter()
            .map(|v| v.to_bits())
            .collect::<Vec<_>>(),
        own.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
    );
}

#[test]
fn group_delivery_reconstructs_the_logical_oracle_across_side_and_epoch_changes() {
    let setup = common::scenario(
        &json!({"size":[128,128],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35})
            .to_string(),
        json!([{"side":"blue","kind":"test_rifle","position":[32,32]}, {"side":"red","kind":"test_rifle","position":[64,32]}]),
        json!([]),
    );
    let mut battle = Battle::new(&setup, 1);
    let mut publisher = publication::Publisher::new();
    let mut previous: Vec<Vec<f32>> = Vec::new();
    for tick in 0..12 {
        battle.step();
        let side = if !(4..8).contains(&tick) {
            Side::Blue
        } else {
            Side::Red
        };
        if tick == 2 {
            publisher.resync();
        }
        let wire = publisher.publish(&battle, side).unwrap();
        if wire[25] == 1.0 {
            previous.clear();
        }
        let mut at = publication::HEADER_WORDS;
        previous.resize_with(publication::GROUPS, Vec::new);
        for group in &mut previous {
            let decoded = codec::group(wire, at, group);
            at += 3 + wire[at + 2] as usize;
            *group = decoded;
        }
        let mut oracle = Vec::new();
        let ground = publication::GroundHeader {
            epoch: 1,
            side,
            base: 0,
            revision: 0,
            full: true,
            count: 0,
        };
        let frame = battle.observe(side);
        publication::pack_logical(frame, &ground, &full_fog(), std::iter::empty(), &mut oracle)
            .unwrap();
        let row_end = oracle.len() - frame.ground_visibility.bits.len() * 2;
        let actual: Vec<u32> = previous.iter().flatten().map(|f| f.to_bits()).collect();
        assert_eq!(
            actual,
            oracle[publication::HEADER_WORDS..row_end]
                .iter()
                .map(|f| f.to_bits())
                .collect::<Vec<_>>(),
            "tick {tick}"
        );
    }
}

#[test]
fn one_variable_route_change_does_not_resend_other_own_units() {
    use contract::command::{CommandEnvelope, MoveDirection, Order, RoutePolicy};
    use contract::ids::UnitId;
    let units: Vec<_> = (0..80)
        .map(|i| {
            // The mover has a clear corridor beside the 79 stationary squads.
            let position = if i == 0 {
                [320, 240]
            } else {
                [32 + i % 10 * 24, 32 + i / 10 * 24]
            };
            json!({"side":"blue","kind":"test_rifle","position":position})
        })
        .collect();
    let setup = common::scenario(
        &json!({"size":[512,512],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35})
            .to_string(),
        json!(units),
        json!([]),
    );
    let mut battle = Battle::new(&setup, 1);
    battle.step();
    let mut publisher = publication::Publisher::new();
    let initial = publisher.publish(&battle, Side::Blue).unwrap().to_vec();
    let old = codec::group(&initial, publication::HEADER_WORDS, &[]);
    assert!(battle
        .accept(CommandEnvelope {
            side: Side::Blue,
            player: None,
            seq: 1,
            queued: false,
            order: Order::Move {
                units: vec![UnitId(0)],
                gesture: 1,
                goal: [400.0, 400.0],
                route: RoutePolicy::Shortest,
                direction: MoveDirection::Forward,
                facing: None
            }
        })
        .error
        .is_none());
    for _ in 0..240 {
        battle.step();
        if !battle.observe(Side::Blue).own[0].route.is_empty() {
            break;
        }
    }
    assert!(
        !battle.observe(Side::Blue).own[0].route.is_empty(),
        "the route must finish planning within eight simulated seconds"
    );
    let wire = publisher.publish(&battle, Side::Blue).unwrap();
    eprintln!(
        "own route payload {} B",
        (3 + wire[publication::HEADER_WORDS + 2] as usize) * 4
    );
    assert!(
        wire[publication::HEADER_WORDS] > initial[publication::HEADER_WORDS],
        "the own variable section must actually grow"
    );
    let own_bytes = (3 + wire[publication::HEADER_WORDS + 2] as usize) * 4;
    assert!(
        own_bytes < 2000,
        "one route change must retain the other 79 own units: {own_bytes} B"
    );
    let words = codec::group(wire, publication::HEADER_WORDS, &old);
    let mut snapshot = publication::Publisher::new();
    let fresh = snapshot.publish(&battle, Side::Blue).unwrap();
    assert_eq!(
        words.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
        codec::group(fresh, publication::HEADER_WORDS, &[])
            .iter()
            .map(|v| v.to_bits())
            .collect::<Vec<_>>()
    );
}

#[test]
fn cold_own_delivery_compacts_sparse_values_without_losing_the_logical_words() {
    let setup = common::scenario(
        r#"{"size":[128,128],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35}"#,
        json!([{"side":"blue","kind":"test_rifle","position":[32,32]},
               {"side":"blue","kind":"test_rifle","position":[96,96]}]),
        json!([]),
    );
    let battle = Battle::new(&setup, 1);
    let mut publisher = publication::Publisher::new();
    let data = publisher.publish(&battle, Side::Blue).unwrap();
    assert!(
        data[publication::HEADER_WORDS + 2] < data[publication::HEADER_WORDS],
        "sparse cold own rows should reduce their canonical byte count: {} / {}",
        data[publication::HEADER_WORDS + 2] * 4.0,
        data[publication::HEADER_WORDS] * 4.0
    );
    let frame = battle.observe(Side::Blue);
    let ground = publication::GroundHeader {
        epoch: 1,
        side: Side::Blue,
        base: 0,
        revision: 0,
        full: true,
        count: 0,
    };
    let mut logical = Vec::new();
    publication::pack_logical(
        frame,
        &ground,
        &full_fog(),
        std::iter::empty(),
        &mut logical,
    )
    .unwrap();
    assert_eq!(
        codec::group(data, publication::HEADER_WORDS, &[])
            .iter()
            .map(|v| v.to_bits())
            .collect::<Vec<_>>(),
        logical[publication::HEADER_WORDS
            ..publication::HEADER_WORDS + data[publication::HEADER_WORDS] as usize]
            .iter()
            .map(|v| v.to_bits())
            .collect::<Vec<_>>()
    );
}

/// A launcher holding for want of a clear launch path is published as such,
/// read back by the layout alone: its weapon row, the blocked-trajectory
/// reason, and every round still in it.
#[test]
fn a_launcher_held_for_its_launch_path_publishes_the_blocked_reason() {
    let mut b = crate::launch_clearance::held_under_trees();
    for _ in 0..300 {
        b.step();
    }
    let frame = b.observe(Side::Blue).clone();
    let layout: Value = serde_json::from_str(&publication::layout_json(&b)).unwrap();
    let patch = publication::GroundHeader {
        epoch: 1,
        side: Side::Blue,
        base: 0,
        revision: 0,
        full: false,
        count: 0,
    };
    let mut data = Vec::new();
    publication::pack_logical(&frame, &patch, &full_fog(), std::iter::empty(), &mut data).unwrap();
    let own = &decode(&layout, &data)["own"];
    let mount = own[0].sections["mounts"]
        .iter()
        .find(|m| m["mount"] == 1.0)
        .expect("the launcher's row is published");
    let reason = layout["actionReasons"][mount["reason"] as usize].as_str();
    assert_eq!(reason, Some("blocked_trajectory"));
    let full = b
        .arsenal()
        .weapons
        .iter()
        .find(|w| w.id == "test_launcher")
        .unwrap();
    assert_eq!(
        Some(mount["ammo0"] as u32),
        match full.def.ammo {
            contract::weapons::AmmoCapacity::Rounds(n) => Some(n),
            _ => None,
        },
        "no round spent"
    );
}

#[test]
fn visible_members_publish_the_weapon_they_are_using() {
    let setup = common::scenario_with(
        &json!({"size":[300,200],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,"props":[]}).to_string(),
        json!([
            {"side":"blue","kind":"test_at","position":[40,100]},
            {"side":"red","kind":"test_tank","position":[180,100],"yaw":std::f64::consts::PI,"engagement":"return_fire_only"}
        ]), json!([]), json!([]));
    let mut battle = Battle::new(&setup, 1);
    battle.step();
    let unit = serde_json::to_value(&battle.observe(Side::Blue).own[0]).unwrap();
    assert_eq!(unit["member_active_mounts"], json!([1, 0, 0]));
    let seen = &battle.observe(Side::Red).identified[0];
    assert_eq!(
        seen.member_active_mounts,
        [Some(1), Some(0), Some(0)],
        "an enemy sees the same selected weapons for visible soldiers"
    );
}

#[test]
fn a_shout_publishes_its_speaker_kind_place_trigger_and_intensity() {
    // Read back by the layout alone: whose voice (own unit or seen enemy,
    // each id exact past float32's 2²⁴), its type, where it was said, which
    // bark and how. An unseen enemy's cue rides in `audible` as a shout.
    use contract::ids::UnitId;
    use contract::observation::{
        Intensity, ObservedTargetId, Shout, ShoutSpeaker, ShoutTrigger, SoundBand, SoundCategory,
        SoundCue,
    };
    let map =
        json!({ "size": [400, 400], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 });
    let b = Battle::new(
        &common::scenario(
            &map.to_string(),
            json!([{ "side": "blue", "kind": "test_rifle", "position": [100, 100] }]),
            json!([]),
        ),
        1,
    );
    let mut frame = b.observe(Side::Blue).clone();
    let big = (1u32 << 24) + 3;
    frame.shouts = vec![
        Shout {
            speaker: ShoutSpeaker::Own(UnitId(u32::MAX)),
            kind: common::unit_kind("test_rifle"),
            position: [120.5, 80.25, 1.5],
            trigger: ShoutTrigger::B17FriendlyDestroyedNearby,
            intensity: Intensity::Stressed,
        },
        Shout {
            speaker: ShoutSpeaker::Seen(ObservedTargetId(big)),
            kind: common::unit_kind("test_tank"),
            position: [300.0, 250.0, 2.0],
            trigger: ShoutTrigger::B1ContactInfantry,
            intensity: Intensity::Steady,
        },
    ];
    frame.audible = vec![SoundCue {
        listener: UnitId(0),
        category: SoundCategory::Shout,
        sector: 3,
        band: SoundBand::Far,
        moving: false,
    }];
    let layout: Value = serde_json::from_str(&publication::layout_json(&b)).unwrap();
    let bits = layout["limbBits"].as_u64().unwrap() as u32;
    let patch = publication::GroundHeader {
        epoch: 1,
        side: Side::Blue,
        base: 0,
        revision: 0,
        full: false,
        count: 0,
    };
    let mut data = Vec::new();
    publication::pack_logical(&frame, &patch, &full_fog(), std::iter::empty(), &mut data).unwrap();
    let groups = decode(&layout, &data);
    let tag = |table: &str, i: f32| layout[table][i as usize].as_str().unwrap().to_owned();
    let read: Vec<_> = groups["shouts"]
        .iter()
        .map(|s| {
            let f = &s.fields;
            (
                f["own"],
                integer(bits, f, "id"),
                tag("unitKinds", f["kind"]),
                [f["x"], f["y"], f["z"]],
                tag("shoutTriggers", f["trigger"]),
                tag("intensities", f["intensity"]),
            )
        })
        .collect();
    assert_eq!(
        read,
        [
            (
                1.0,
                u32::MAX,
                "test_rifle".to_owned(),
                [120.5, 80.25, 1.5],
                "b17_friendly_destroyed_nearby".to_owned(),
                "stressed".to_owned()
            ),
            (
                0.0,
                big,
                "test_tank".to_owned(),
                [300.0, 250.0, 2.0],
                "b1_contact_infantry".to_owned(),
                "steady".to_owned()
            ),
        ]
    );
    let cue = &groups["audible"][0].fields;
    assert_eq!(tag("soundCategories", cue["category"]), "shout");
    assert_eq!([cue["listener"], cue["sector"]], [0.0, 3.0]);
}

#[test]
fn the_layout_names_every_bark_and_intensity_in_contract_order() {
    // The tags are the contract's serde names, B1 first: TypeScript reads
    // them from here and never hard-codes them.
    use contract::observation::{Intensity, ShoutTrigger};
    let b = Battle::new(
        &common::scenario(
            &json!({ "size": [64, 64], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 })
                .to_string(),
            json!([]),
            json!([]),
        ),
        1,
    );
    let layout: Value = serde_json::from_str(&publication::layout_json(&b)).unwrap();
    let triggers = names(&layout["shoutTriggers"]);
    assert_eq!(triggers.len(), 17);
    for (i, name) in triggers.iter().enumerate() {
        assert!(name.starts_with(&format!("b{}_", i + 1)), "{name}");
        let parsed: ShoutTrigger = serde_json::from_value(json!(name)).unwrap();
        assert_eq!(json!(parsed), json!(name));
    }
    assert_eq!(
        names(&layout["intensities"]),
        [Intensity::Steady, Intensity::Stressed].map(|i| json!(i).as_str().unwrap().to_owned())
    );
    assert_eq!(
        names(&layout["soundCategories"]),
        ["infantry", "vehicle", "shot", "shout"]
    );
}

/// A red tank 500 m from a blue tank, beyond its sight, gives itself away
/// by firing every second; the blue crew shells the area and the red crew
/// calls the rounds landing round it. Each bark lasts a tick and may be
/// said again on the next, so the red crew talks often.
fn unseen_shouter() -> Battle {
    let fire: Vec<Value> = (0..20)
        .map(|k| json!({ "tick": 5 + k * 30, "fire": { "unit": 2 } }))
        .collect();
    let mut setup = common::scenario(
        &json!({ "size": [1200, 600], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35, "props": [] })
            .to_string(),
        json!([
            { "side": "blue", "kind": "test_tank", "position": [100, 300] },
            { "side": "blue", "kind": "test_supply", "position": [60, 340] },
            { "side": "red", "kind": "test_tank", "position": [600, 300], "engagement": "return_fire_only" },
        ]),
        json!(fire),
    );
    let tick = 1.0 / setup.rules.tick_hz as f64;
    setup.rules.shouts.cooldown_s = tick;
    setup.rules.shouts.talk_s = tick;
    Battle::new(&setup, 3)
}

#[test]
fn an_unseen_enemys_shout_is_published_only_as_an_anonymous_audible_cue() {
    use contract::observation::{ShoutSpeaker, SoundCategory};
    use sim::publication::reader::Reader;
    let mut battle = unseen_shouter();
    let layout = publication::layout_json(&battle);
    let mut publisher = publication::Publisher::new();
    let mut reader = Reader::new(&layout).unwrap();
    let (mut spoken, mut cues, mut own) = (0, 0, 0);
    for _ in 0..20 * battle.rules().tick_hz {
        battle.step();
        spoken += battle
            .observe(Side::Red)
            .shouts
            .iter()
            .filter(|s| s.speaker == ShoutSpeaker::Own(contract::ids::UnitId(2)))
            .count();
        let words: Vec<u32> = publisher
            .publish(&battle, Side::Blue)
            .unwrap()
            .iter()
            .map(|w| w.to_bits())
            .collect();
        let o = reader.decode(&words).unwrap().unwrap();
        assert!(o.identified.is_empty(), "blue never sees the red tank");
        assert!(
            o.shouts.iter().all(|s| s.own),
            "tick {}: an unseen enemy's bark was published: {:?}",
            o.tick,
            o.shouts
        );
        own += o.shouts.len();
        // Every shout the side heard arrives as a cue, and nothing more.
        let heard = o.audible.iter().filter(|c| &*c.category == "shout").count();
        let frame = battle.observe(Side::Blue);
        let said = frame.audible.iter();
        assert_eq!(
            heard,
            said.filter(|c| c.category == SoundCategory::Shout).count()
        );
        cues += heard;
    }
    assert!(spoken > 3, "the red crew talks: {spoken}");
    assert!(cues > 0, "blue hears it shout");
    assert!(own > 0, "blue's own barks are published");
}
