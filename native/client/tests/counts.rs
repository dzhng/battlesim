//! Native's replay of a browser capture shows what the browser drew, frame
//! by frame: on every frame the browser counted (`frameCounts`, on tick
//! boundaries and between them), the models posed by kind, the effects
//! running by kind, their instances by shape and the lights, the building
//! modules, the props and the scenery in view all equal the browser's.
//! Whatever native reads or frames a hair off (a frame time one ULP past a
//! tick boundary starts or ends that tick's effects) shows here. Counted from
//! the presentation alone, so no GPU. Reads the count fixture's world and its
//! bundles (Git LFS).
//!
//! The fixture is a count-only browser run of paris-corner's whole reel
//! (`PARITY_SCENES=paris-corner PARITY_COUNTS=1`, `native-parity-capture`),
//! its `capture.json` and `manifest.json` gzipped as written (`gzip -9n`).
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;

use native_client::bundle::Library;
use native_client::capture::Replay;
use native_client::counts::Counted;
use presentation::assets::RUNTIME;

#[path = "../../../crates/sim/tests/common/capture_publication.rs"]
mod capture_publication;

const COUNTED: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/parity/capture/counts/paris-corner"
);

/// The fixture's files inflated into a scratch folder, byte for byte as the
/// browser wrote them.
fn inflated() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("native-counts-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    for name in ["capture.json", "manifest.json"] {
        let gz = fs::read(Path::new(COUNTED).join(format!("{name}.gz"))).unwrap();
        let mut bytes = Vec::new();
        GzDecoder::new(&gz[..]).read_to_end(&mut bytes).unwrap();
        fs::write(dir.join(name), bytes).unwrap();
    }
    dir
}

#[test]
fn every_frame_the_browser_counted_shows_the_same_counts_natively() {
    let library = Library::open(Path::new(RUNTIME)).unwrap();
    let dir = inflated();
    if std::env::var_os("BLESS_PARITY").is_some() {
        use sha2::{Digest, Sha256};
        use std::io::Write;
        let mut capture: serde_json::Value =
            serde_json::from_slice(&fs::read(dir.join("capture.json")).unwrap()).unwrap();
        capture_publication::refresh(&mut capture);
        let bytes = serde_json::to_vec(&capture).unwrap();
        fs::write(dir.join("capture.json"), &bytes).unwrap();
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(dir.join("manifest.json")).unwrap()).unwrap();
        manifest["identity"]["captureHash"] =
            serde_json::json!(format!("{:x}", Sha256::digest(&bytes)));
        fs::write(
            dir.join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        for name in ["capture.json", "manifest.json"] {
            let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
            gz.write_all(&fs::read(dir.join(name)).unwrap()).unwrap();
            fs::write(
                Path::new(COUNTED).join(format!("{name}.gz")),
                gz.finish().unwrap(),
            )
            .unwrap();
        }
    }
    let replay = Replay::open(&dir, &library);
    fs::remove_dir_all(&dir).unwrap();
    let mut replay = replay.unwrap();
    let counted = replay.frame_counts.clone();
    let records = replay.records.clone();
    let hz = replay.tick_hz;
    let on_boundary = |t: f64| (t * hz).round() / hz == t;
    let mut rows = counted.iter().peekable();
    let (mut boundaries, mut between) = (0, 0);
    let mut mismatches = Vec::new();
    for (index, record) in records.iter().enumerate() {
        replay.frame(record, 0, |_| {}).unwrap();
        let Some(row) = rows.next_if(|r| r["frame"] == index) else {
            continue;
        };
        let t = record.sim_time;
        if on_boundary(t) {
            boundaries += 1;
        } else {
            between += 1;
        }
        let at = format!("frame {index} at {t:?} s");
        mismatches.extend(Counted::of(replay.presentation()).mismatches(
            &at,
            &row["models"],
            &row["effects"],
            &row["drawn"],
        ));
    }
    assert_eq!(
        rows.next(),
        None,
        "every counted frame is one the capture drew"
    );
    // The fixture holds both: frames drawn on a tick boundary, where a frame
    // time read one ULP off changes what runs, and frames between.
    assert!(
        boundaries > 0 && between > 0,
        "{boundaries} counted frames on a tick boundary, {between} between"
    );
    assert!(
        mismatches.is_empty(),
        "{} differences from the browser's counts over {} counted frames:\n  {}",
        mismatches.len(),
        counted.len(),
        mismatches[..mismatches.len().min(30)].join("\n  ")
    );
}
