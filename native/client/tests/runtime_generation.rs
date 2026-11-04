//! Device recreation republishes current held static lists without reframing
//! moving units, UI selections, picks or effects from another moment.
use std::path::Path;

use native_client::bundle::Library;
use native_client::capture::Replay;
use presentation::assets::RUNTIME;
use presentation::presentation::Ui;
use presentation::records::{Batch, NONE};
use presentation::view::Camera;

fn listed(replay: &Replay) -> Vec<(u32, u32, u32, u32, u32, Vec<u8>)> {
    let frame = replay.last_frame();
    frame
        .batches
        .iter()
        .map(|batch| {
            assert_ne!(batch.first, NONE, "all current batches are materialized");
            let instances =
                &frame.instances[batch.first as usize..(batch.first + batch.count) as usize];
            (
                batch.appearance,
                batch.tier,
                batch.pass,
                batch.cascade_mask,
                batch.fog,
                bytemuck::cast_slice(instances).to_vec(),
            )
        })
        .collect()
}

#[test]
fn renderer_recreation_materializes_current_static_lists_without_replaying_dynamic_frame() {
    let library = Library::open(Path::new(RUNTIME)).unwrap();
    let run = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/parity/capture/run/paris-corner");
    let mut replay = Replay::open(&run, &library).unwrap();
    let first = replay.records.first().unwrap().clone();
    let current = replay.records[replay.records.len() / 2].clone();
    replay.frame(&first, 11, |_| {}).unwrap();
    replay.frame(&current, 11, |_| {}).unwrap();
    let selected = [replay
        .last_frame()
        .picks
        .iter()
        .find(|pick| pick.flags & 1 != 0)
        .expect("a visible own unit")
        .unit];
    let camera = Camera {
        eye: current.eye,
        target: current.target,
        up: current.up,
        fov_y: 0.8,
        near: 1.0,
        size: current.canvas,
    };
    let ui = Ui {
        selected: &selected,
        xray: true,
        ..Ui::default()
    };
    replay
        .presentation_mut()
        .frame(current.sim_time, &camera, &ui, 11)
        .unwrap();
    replay
        .presentation_mut()
        .frame(current.sim_time, &camera, &ui, 11)
        .unwrap();
    let frame = replay.last_frame();
    assert!(frame.batches.iter().any(|batch| batch.first == NONE));
    let dynamic = |batches: &[Batch]| {
        batches
            .iter()
            .filter(|batch| batch.first != NONE)
            .copied()
            .collect::<Vec<_>>()
    };
    let batches = dynamic(&frame.batches);
    let instances = bytemuck::cast_slice::<_, u8>(&frame.instances).to_vec();
    let palettes = bytemuck::cast_slice::<_, u8>(&frame.palettes).to_vec();
    let picks = bytemuck::cast_slice::<_, u8>(&frame.picks).to_vec();
    let effects = bytemuck::cast_slice::<_, u8>(replay.presentation().effects()).to_vec();
    let lights = bytemuck::cast_slice::<_, u8>(replay.presentation().lights()).to_vec();
    let (index, tick, clock) = (frame.index, frame.tick, frame.clock);
    replay.presentation_mut().resend_static_frame(12);
    let frame = replay.last_frame();
    assert_eq!((frame.index, frame.tick, frame.clock), (index, tick, clock));
    assert_eq!(
        &bytemuck::cast_slice::<_, u8>(&frame.instances)[..instances.len()],
        &instances
    );
    assert_eq!(bytemuck::cast_slice::<_, u8>(&frame.palettes), palettes);
    assert_eq!(bytemuck::cast_slice::<_, u8>(&frame.picks), picks);
    assert_eq!(
        bytemuck::cast_slice::<_, u8>(replay.presentation().effects()),
        effects
    );
    assert_eq!(
        bytemuck::cast_slice::<_, u8>(replay.presentation().lights()),
        lights
    );
    for batch in batches {
        assert!(frame
            .batches
            .iter()
            .any(|current| bytemuck::bytes_of(current) == bytemuck::bytes_of(&batch)));
    }
    let materialized = listed(&replay);
    // A normal frame following materialization still rewrites for the new owner.
    replay
        .presentation_mut()
        .frame(current.sim_time, &camera, &ui, 12)
        .unwrap();
    assert!(replay
        .last_frame()
        .batches
        .iter()
        .all(|batch| batch.first != NONE));
    assert_eq!(listed(&replay), materialized);
    replay
        .presentation_mut()
        .frame(current.sim_time, &camera, &ui, 13)
        .unwrap();
    assert_eq!(
        listed(&replay),
        materialized,
        "current authoritative populations match an explicit fresh generation"
    );
}
