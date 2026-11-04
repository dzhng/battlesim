//! A clip is framed on the reel's clock: the page's records carry its
//! presentation clock, read off its own wall clock, which jitters against the
//! reel the camera is keyed to. Frames at even reel steps draw the battle at
//! even sim steps, a constant lag behind the reel, never past the newest
//! publication the run had. Reads the fixture run, its records rewritten.
use std::fs;
use std::path::{Path, PathBuf};

use native_client::bundle::Library;
use native_client::capture::Replay;
use presentation::assets::RUNTIME;
use serde_json::Value;
use sha2::{Digest, Sha256};

const RUN: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/parity/capture/run/paris-corner"
);
const FRAME_MS: f64 = 1000.0 / 60.0;
/// The page's clock: this far ahead of the reel at most, by `JITTER_MS` behind at least.
const LEAD_S: f64 = 2.0;
const JITTER_MS: [f64; 4] = [0.0, 9.0, 15.0, 4.0];

/// The fixture run with `records` page frames: one every `FRAME_MS`, but
/// for one the page skipped, its clock behind the reel by a jitter that
/// changes every two frames and held at the start while the battle warms.
/// `samples` keeps that many of the run's publications. Answers the run, its
/// tick rate and its last publication's tick.
fn jittered(name: &str, records: usize, samples: usize) -> (PathBuf, f64, f64) {
    let dir = std::env::temp_dir().join(format!("native-clip-clock-{name}-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let mut capture: Value =
        serde_json::from_slice(&fs::read(Path::new(RUN).join("capture.json")).unwrap()).unwrap();
    let tick_hz = capture["tickHz"].as_f64().unwrap();
    let template = capture["frames"][0].clone();
    let mut sim_time: f64 = 0.0;
    let frames: Vec<Value> = (1..=records + 1)
        .filter(|&k| k != records / 2)
        .enumerate()
        .map(|(index, k)| {
            let reel_ms = k as f64 * FRAME_MS;
            let page = LEAD_S + (reel_ms - JITTER_MS[(k / 2) % JITTER_MS.len()]) / 1000.0;
            // Warming up, the page's clock holds at its first reading.
            sim_time = if k <= 3 { LEAD_S } else { sim_time.max(page) };
            let mut frame = template.clone();
            frame["index"] = Value::from(index);
            frame["reelMs"] = Value::from(reel_ms);
            frame["simTime"] = Value::from(sim_time);
            frame["tick"] = Value::from((sim_time * tick_hz).ceil() as u64);
            frame["plate"] = Value::Null;
            frame
        })
        .collect();
    capture["frames"] = Value::from(frames);
    let kept = capture["samples"].as_array_mut().unwrap();
    kept.truncate(samples);
    let last_tick = kept.last().unwrap()["tick"].as_f64().unwrap();
    let capture = capture.to_string();
    let mut manifest: Value =
        serde_json::from_slice(&fs::read(Path::new(RUN).join("manifest.json")).unwrap()).unwrap();
    manifest["identity"]["captureHash"] = Value::from(format!("{:x}", Sha256::digest(&capture)));
    fs::write(dir.join("capture.json"), capture).unwrap();
    fs::write(dir.join("manifest.json"), manifest.to_string()).unwrap();
    (dir, tick_hz, last_tick)
}

/// Each clip frame up to the run's last record, as the clip writes it: its
/// reel time, the sim time it drew and the tick of the newest publication.
fn clip(dir: &Path) -> Vec<(f64, f64, f64)> {
    let library = Library::open(Path::new(RUNTIME)).unwrap();
    let mut replay = Replay::open(dir, &library).unwrap();
    let last = replay.records.last().unwrap().reel_ms;
    let mut drawn = Vec::new();
    for at in (0..)
        .map(|i| i as f64 * FRAME_MS)
        .take_while(|&at| at < last)
    {
        replay.frame_on_reel(at, 0, |_| {}).unwrap();
        let p = replay.presentation();
        drawn.push((at, p.clock_of_frame(), p.observation().unwrap().tick));
    }
    fs::remove_dir_all(dir).unwrap();
    drawn
}

#[test]
fn jittered_records_clip_at_even_sim_steps_a_constant_lag_behind_the_reel() {
    let (dir, tick_hz, _) = jittered("even", 120, usize::MAX);
    let drawn = clip(&dir);
    let lags: Vec<f64> = drawn
        .iter()
        .map(|&(at, time, _)| at / 1000.0 - (time - LEAD_S))
        .collect();
    // Constant: every frame, the skipped one too, advances one reel step.
    let (lo, hi) = lags
        .iter()
        .fold((f64::MAX, f64::MIN), |(lo, hi), &l| (lo.min(l), hi.max(l)));
    assert!(
        hi - lo < 1e-9,
        "the lag varies over {} ms",
        (hi - lo) * 1000.0
    );
    // Within the page's own jitter: where it drew the battle as the camera moved.
    let max_jitter = JITTER_MS.iter().copied().fold(0.0, f64::max) / 1000.0;
    assert!((0.0..=max_jitter).contains(&lo), "lag {} ms", lo * 1000.0);
    for &(at, time, tick) in &drawn {
        assert!(
            time * tick_hz <= tick + 1e-9,
            "at {at} ms: drew {time} s past tick {tick}"
        );
    }
}

#[test]
fn a_clip_stops_at_the_runs_last_publication() {
    // The run's publications end about a second into the reel's two.
    let (dir, tick_hz, last_tick) = jittered("end", 120, ((LEAD_S + 1.0) * 30.0) as usize);
    let drawn = clip(&dir);
    for &(at, time, tick) in &drawn {
        assert!(
            time * tick_hz <= tick + 1e-9,
            "at {at} ms: drew {time} s past tick {tick}"
        );
    }
    let &(_, time, tick) = drawn.last().unwrap();
    assert_eq!(tick, last_tick, "the newest publication pushed");
    assert_eq!(time, last_tick / tick_hz, "held at the last publication");
}

#[test]
fn a_clips_first_frame_shows_the_battle_through_the_pages_first_camera() {
    // The reel starts a frame before the page records one: its first frame
    // is still the battle, seen as the page's first record saw it, never the
    // empty sky.
    let (dir, _, _) = jittered("first", 12, usize::MAX);
    let library = Library::open(Path::new(RUNTIME)).unwrap();
    let mut replay = Replay::open(&dir, &library).unwrap();
    let first = replay.records[0].reel_ms;
    assert!(
        first > 0.0,
        "the page's first record is a frame into the reel"
    );
    let view = |r: &mut Replay, at: f64| {
        let frame = r.frame_on_reel(at, 0, |_| {}).unwrap();
        bytemuck::bytes_of(&frame.view).to_vec()
    };
    let at_start = view(&mut replay, 0.0);
    assert!(
        replay.presentation().observation().is_some(),
        "no battle at the reel's start"
    );
    assert_eq!(
        at_start,
        view(&mut replay, first),
        "not the first record's camera"
    );
    fs::remove_dir_all(dir).unwrap();
}
