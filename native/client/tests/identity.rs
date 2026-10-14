//! A replay runs only on the battle its manifest names: a capture or a saved
//! map that differs from the manifest's identity is refused by the file's
//! name, so a plate is never compared against a different battle. Reads the
//! fixture run.
use std::fs;
use std::path::{Path, PathBuf};

use native_client::bundle::Library;
use native_client::capture::Replay;
use presentation::assets::RUNTIME;
use serde_json::Value;

const RUN: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/parity/capture/run/paris-corner"
);

/// The fixture run copied into a scratch folder, its capture's text and its
/// manifest passed through `edit` first.
fn staged(name: &str, edit: impl FnOnce(&mut String, &mut Value)) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("native-identity-{name}-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let mut capture = fs::read_to_string(Path::new(RUN).join("capture.json")).unwrap();
    let mut manifest: Value =
        serde_json::from_slice(&fs::read(Path::new(RUN).join("manifest.json")).unwrap()).unwrap();
    edit(&mut capture, &mut manifest);
    fs::write(dir.join("capture.json"), capture).unwrap();
    fs::write(dir.join("manifest.json"), manifest.to_string()).unwrap();
    dir
}

fn refusal(dir: &Path) -> String {
    let library = Library::open(Path::new(RUNTIME)).unwrap();
    match Replay::open(dir, &library) {
        Ok(_) => panic!("{} was replayed", dir.display()),
        Err(e) => e,
    }
}

#[test]
fn a_capture_that_is_not_the_manifests_is_refused_by_name() {
    // One byte more than the browser wrote: still valid JSON, another file.
    let dir = staged("capture", |capture, _| capture.push('\n'));
    let e = refusal(&dir);
    assert!(e.contains("capture.json"), "{e}");
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_map_edited_after_the_control_ran_is_refused_by_name() {
    let dir = staged("map", |_, manifest| {
        manifest["identity"]["mapHash"] = Value::from("0".repeat(64));
    });
    let e = refusal(&dir);
    assert!(e.contains("map.json"), "{e}");
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn the_fixture_run_is_its_manifests_battle() {
    let library = Library::open(Path::new(RUNTIME)).unwrap();
    Replay::open(Path::new(RUN), &library).unwrap();
}
