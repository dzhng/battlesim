//! A replay runs only on the battle its manifest names: a capture or a saved
//! map that differs from the manifest's identity is refused by the file's
//! name, so a plate is never compared against a different battle. Both files
//! are read strictly, as the browser's readers read them: a key the format
//! does not have, or one it has left out, refuses the file rather than being
//! dropped or defaulted. Reads the fixture run.
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
fn a_manifest_naming_no_scene_is_refused() {
    let dir = staged("unnamed", |capture, manifest| {
        manifest["identity"]
            .as_object_mut()
            .unwrap()
            .remove("scene");
        let mut parsed: Value = serde_json::from_str(capture).unwrap();
        parsed["workload"].as_object_mut().unwrap().remove("scene");
        *capture = parsed.to_string();
    });
    let e = refusal(&dir);
    assert!(e.contains("capture.json") && e.contains("scene"), "{e}");
    fs::remove_dir_all(dir).unwrap();
}

/// The manifest names `capture` as the file its control ran on, so only the
/// edit itself can refuse it.
fn stamp(capture: &str, manifest: &mut Value) {
    manifest["identity"]["captureHash"] = Value::from(format!("{:x}", Sha256::digest(capture)));
}

#[test]
fn a_manifest_with_a_key_its_format_lacks_is_refused() {
    let dir = staged("manifest-extra", |_, manifest| {
        manifest["stale"] = Value::from(1);
    });
    let e = refusal(&dir);
    assert!(e.contains("manifest.json") && e.contains("stale"), "{e}");
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_manifest_missing_a_count_table_is_refused() {
    let dir = staged("manifest-uncounted", |_, manifest| {
        manifest.as_object_mut().unwrap().remove("modelCounts");
    });
    let e = refusal(&dir);
    assert!(
        e.contains("manifest.json") && e.contains("modelCounts"),
        "{e}"
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_manifest_drawn_through_another_depth_convention_is_refused() {
    let dir = staged("manifest-depth", |_, manifest| {
        manifest["lens"]["depth"] = Value::from("forward-z");
    });
    let e = refusal(&dir);
    assert!(
        e.contains("manifest.json") && e.contains("forward-z"),
        "{e}"
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_capture_frame_with_a_key_its_format_lacks_is_refused() {
    let dir = staged("capture-extra", |capture, manifest| {
        let mut parsed: Value = serde_json::from_str(capture).unwrap();
        parsed["frames"][0]["stale"] = Value::from(1);
        *capture = parsed.to_string();
        stamp(capture, manifest);
    });
    let e = refusal(&dir);
    assert!(e.contains("capture.json") && e.contains("stale"), "{e}");
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn the_fixture_run_is_its_manifests_battle() {
    let library = Library::open(Path::new(RUNTIME)).unwrap();
    Replay::open(Path::new(RUN), &library).unwrap();
}

#[test]
fn a_capture_with_a_different_tick_rate_than_its_encounter_is_refused() {
    let dir = staged("clock", |capture, manifest| {
        let mut parsed: Value = serde_json::from_str(capture).unwrap();
        parsed["tickHz"] = Value::from(parsed["tickHz"].as_f64().unwrap() * 2.0);
        *capture = parsed.to_string();
        stamp(capture, manifest);
    });
    assert!(refusal(&dir).contains("tick_hz"));
    fs::remove_dir_all(dir).unwrap();
}
