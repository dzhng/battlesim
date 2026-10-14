use std::sync::Arc;

use presentation::effects::transit::{tracks, TransitFeel};
use sim::publication::reader::AirTransit;

fn transit(dir: &str, edge_tick: f64) -> AirTransit {
    AirTransit {
        kind: Arc::from("test_jet"),
        point: [100.0, 20.0, 250.0],
        heading: 0.0,
        speed_mps: 200.0,
        edge_tick,
        dir: Arc::from(dir),
    }
}

#[test]
fn an_inbound_track_meets_the_spawn_point_at_the_spawn_tick() {
    let feel = TransitFeel {
        inbound_s: 10.0,
        outbound_s: 10.0,
    };
    let edge_tick = 300.0;
    let poses = tracks(&[transit("in", edge_tick)], edge_tick, 30.0, &feel);
    assert_eq!(poses.len(), 1);
    assert_eq!(poses[0].position, [100.0, 20.0, 250.0]);
}

#[test]
fn the_enemy_frame_has_no_transit() {
    let feel = TransitFeel {
        inbound_s: 10.0,
        outbound_s: 10.0,
    };
    // The reader's enemy-side observation has no transit group to pass on.
    assert!(tracks(&[], 10.0, 30.0, &feel).is_empty());
}

#[test]
fn an_outbound_track_ends_after_its_duration() {
    let feel = TransitFeel {
        inbound_s: 10.0,
        outbound_s: 2.0,
    };
    let edge_tick = 300.0;
    let transit = transit("out", edge_tick);
    let before = tracks(
        std::slice::from_ref(&transit),
        edge_tick + 1.999 * 30.0,
        30.0,
        &feel,
    );
    assert_eq!(before.len(), 1);
    let after = tracks(&[transit], edge_tick + 2.0 * 30.0, 30.0, &feel);
    assert!(after.is_empty());
}
