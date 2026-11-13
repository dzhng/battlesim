//! The publication's paired records: the native halves of the streams and
//! codec vectors the web tests hold the Wasm build and its decoder to.
use contract::ids::Side;
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::publication;

use super::{full_fog, names};
use crate::common;

/// The native half of the publication stream record: each tick's battle
/// digest, published words and delivered fog, as hashes.
/// `web/tests/fogDelivery.test.ts` holds the Wasm build and the TypeScript
/// decoder to the same values.
#[test]
fn the_publication_stream_matches_its_paired_record() {
    let record: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/parity/publication/stream.json"
    ))
    .unwrap();
    publication_stream(record, "publication/stream.json", false);
}

#[test]
fn the_combat_stream_matches_its_paired_record() {
    let record: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/parity/publication/combat.json"
    ))
    .unwrap();
    publication_stream(record, "publication/combat.json", true);
}

#[test]
fn a_large_coordinate_arrangement_matches_its_paired_state_record() {
    let record: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/parity/publication/arrangement.json"
    ))
    .unwrap();
    publication_stream(record, "publication/arrangement.json", false);
}

#[test]
fn cover_facing_matches_its_paired_state_record() {
    let record: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/parity/publication/cover-facing.json"
    ))
    .unwrap();
    publication_stream(record, "publication/cover-facing.json", false);
}

#[test]
fn contact_lifecycle_matches_its_paired_state_record() {
    let record: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/parity/publication/contact-lifecycle.json"
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

/// The encoder's half of the codec vectors: every frame packs to exactly the
/// words `web/tests/observation.test.ts` decodes. The vectors keep their own
/// kind tables; the field order is this build's.
#[test]
fn the_encoder_packs_the_codec_vectors_the_web_decoder_reads() {
    let record: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/parity/publication/codec-vectors.json"
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
        "smokeLooks",
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
        let looks = publication::smoke_looks(probe.rules());
        publication::pack_logical(&frame, &looks, &ground, &full_fog(), runs, &mut data).unwrap();
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
