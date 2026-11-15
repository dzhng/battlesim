//! Publication across sides: one publisher whose reader changes side, and
//! a battle's publishers serving each side its own stream.
use contract::ids::Side;
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::publication;

use super::{codec, full_fog, names};
use crate::common;

#[test]
fn each_side_keeps_its_own_stream_and_reopening_one_leaves_the_other_running() {
    let setup = common::scenario(
        &json!({"size":[128,128],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35})
            .to_string(),
        json!([{"side":"blue","kind":"test_rifle","position":[32,32]}, {"side":"red","kind":"test_rifle","position":[64,32]}]),
        json!([]),
    );
    let mut battle = Battle::new(&setup, 1);
    let layout: Value = serde_json::from_str(&publication::layout_json(&battle)).unwrap();
    let header = names(&layout["header"]);
    let field = |name: &str| header.iter().position(|f| f == name).unwrap();
    let (epoch, full) = (field("groundEpoch"), field("groundFull"));
    let mut publishers = publication::Publishers::new();
    // Each record as (its epoch, whether it is a full snapshot).
    let mut tick = |publishers: &mut publication::Publishers, sides: &[Side]| {
        battle.step();
        sides
            .iter()
            .map(|&side| {
                let words = publishers.publish(&battle, side).unwrap();
                (words[epoch] as u32, words[full] == 1.0)
            })
            .collect::<Vec<_>>()
    };
    let both = [Side::Blue, Side::Red];
    assert_eq!(tick(&mut publishers, &both), [(1, true), (1, true)]);
    assert_eq!(
        tick(&mut publishers, &both),
        [(1, false), (1, false)],
        "two sides served in one tick each get a delta"
    );
    publishers.resync(Side::Blue);
    assert_eq!(
        tick(&mut publishers, &both),
        [(2, true), (1, false)],
        "reopening Blue leaves Red's stream running"
    );
    // One consumer goes over to Red and back: each stream it joins opens
    // later than the one it left, or its reader would drop it as stale.
    publishers.resync(Side::Red);
    assert_eq!(tick(&mut publishers, &[Side::Red]), [(3, true)]);
    publishers.resync(Side::Blue);
    assert_eq!(tick(&mut publishers, &[Side::Blue]), [(4, true)]);
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
        publication::pack_logical(
            frame,
            &[],
            &ground,
            &full_fog(),
            std::iter::empty(),
            &mut oracle,
        )
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
