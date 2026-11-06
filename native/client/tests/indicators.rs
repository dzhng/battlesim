//! A replay draws the indicators the browser recorded (`overlays.bin`) when
//! its manifest names the `overlays` layer, and only then: a capture from
//! before indicators were recorded replays as clean as it always did, a
//! manifest naming the layer without its file (or with a file of another
//! run's length) is refused before the world loads, and playback may leave
//! them out. Reads the fixture run; its capture is never edited, so its
//! identity holds.
use std::fs;
use std::path::{Path, PathBuf};

use native_client::bundle::Library;
use native_client::capture::{Playback, Replay};
use native_client::overlays;
use presentation::assets::RUNTIME;
use serde_json::Value;

const RUN: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/parity/capture/run/paris-corner"
);

fn library() -> Library {
    Library::open(Path::new(RUNTIME)).unwrap()
}

fn frame_count() -> usize {
    let capture: Value =
        serde_json::from_slice(&fs::read(Path::new(RUN).join("capture.json")).unwrap()).unwrap();
    capture["frames"].as_array().unwrap().len()
}

/// `frames` frames each drawing one painted vertex, `marks`, in the
/// browser's layout (`overlays.rs`).
fn recorded(frames: usize, marks: &[f32]) -> Vec<u8> {
    let mut bytes = b"BOVL".to_vec();
    for n in [1, frames as u32, 1, 0, 0, marks.len() as u32, 0] {
        bytes.extend(n.to_le_bytes());
    }
    for v in marks {
        bytes.extend(v.to_le_bytes());
    }
    for _ in 0..frames {
        bytes.extend([1u32, 0].iter().flat_map(|n| n.to_le_bytes()));
    }
    bytes
}

/// The fixture run in a scratch folder, its manifest naming the `overlays`
/// layer, beside `sidecar` when given.
fn staged(name: &str, sidecar: Option<Vec<u8>>) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("native-indicators-{name}-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    fs::copy(
        Path::new(RUN).join("capture.json"),
        dir.join("capture.json"),
    )
    .unwrap();
    let mut manifest: Value =
        serde_json::from_slice(&fs::read(Path::new(RUN).join("manifest.json")).unwrap()).unwrap();
    let layers = manifest["layers"].as_array_mut().unwrap();
    assert!(
        !layers.contains(&Value::from("overlays")),
        "the fixture predates indicators"
    );
    layers.push(Value::from("overlays"));
    fs::write(dir.join("manifest.json"), manifest.to_string()).unwrap();
    if let Some(bytes) = sidecar {
        fs::write(dir.join(overlays::FILE), bytes).unwrap();
    }
    dir
}

fn refusal(dir: &Path) -> String {
    match Replay::open(dir, &library()) {
        Ok(_) => panic!("{} was replayed", dir.display()),
        Err(e) => e,
    }
}

#[test]
fn a_manifest_naming_overlays_without_their_file_is_refused_by_name() {
    let dir = staged("missing", None);
    let e = refusal(&dir);
    assert!(e.contains(overlays::FILE), "{e}");
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn overlays_recorded_for_another_run_are_refused() {
    let frames = frame_count();
    let dir = staged("stale", Some(recorded(frames + 1, &[0.0; 10])));
    let e = refusal(&dir);
    assert!(
        e.contains(overlays::FILE) && e.contains(&format!("for {frames} frame records")),
        "{e}"
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_capture_from_before_indicators_replays_clean() {
    let replay = Replay::open(Path::new(RUN), &library()).unwrap();
    assert!(replay.indicators.is_none());
}

#[test]
fn recorded_indicators_play_back_frame_by_frame_unless_left_out() {
    let marks: Vec<f32> = (0..20).map(|v| v as f32).collect();
    let dir = staged("recorded", Some(recorded(frame_count(), &marks)));
    let replay = Replay::open(&dir, &library()).unwrap();
    let shown = replay.indicators.as_ref().expect("the recorded indicators");
    let last = replay.records.len() - 1;
    assert_eq!(shown.overlays.channels(last)[2], marks, "the painted marks");
    let clean = Playback {
        indicators: false,
        ..Playback::default()
    };
    let replay = Replay::open_played(
        &dir,
        &library(),
        Path::new(native_client::capture::ROOT),
        clean,
    )
    .unwrap();
    assert!(replay.indicators.is_none(), "--no-indicators draws none");
    fs::remove_dir_all(dir).unwrap();
}
