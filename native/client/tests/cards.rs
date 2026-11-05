//! A replay draws the unit info cards the browser recorded (`cards.bin`)
//! when its manifest names the `cards` layer, and only then: a capture from
//! before cards were recorded replays as it always did, a manifest naming
//! the layer without its file (or with a file of another run's length) is
//! refused before the world loads, and playback may leave them out on their
//! own, apart from the indicators and the fog, or with the indicators for a
//! clean clip. Reads the fixture run; its capture is
//! never edited, so its identity holds.
use std::fs;
use std::path::{Path, PathBuf};

use native_client::bundle::Library;
use native_client::capture::{Playback, Replay};
use native_client::cards;
use presentation::assets::RUNTIME;
use serde_json::{json, Value};

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

/// `frames` frames, the last of which shows `card` (a bare backing), in the
/// browser's layout (`cards.rs`).
fn recorded(frames: usize, card: &str) -> Vec<u8> {
    let mut shown = vec![json!([]); frames];
    shown[frames - 1] = json!([[0, 0, -1, 1, 10, 10, 0, 0, 0, 0, 0, 0, 0, 0, 0]]);
    let layout = json!({
        "viewport": [1280, 720],
        "scale": 1,
        "cards": [card],
        "fonts": [],
        "sprites": [],
        "leaders": [],
        "parts": [[{ "rect": {
            "box": [0, 0, 40, 12], "fill": [0, 0, 0, 0.6], "border": [0, 0, 0, 0],
            "borderPx": 0, "blurPx": 10
        } }]],
        "frames": shown,
    })
    .to_string();
    let mut bytes = b"BCRD".to_vec();
    for n in [1, layout.len() as u32] {
        bytes.extend(n.to_le_bytes());
    }
    bytes.extend(layout.as_bytes());
    bytes.extend([0u32, 0].iter().flat_map(|n| n.to_le_bytes()));
    bytes
}

/// The fixture run in a scratch folder, its manifest naming the `cards`
/// layer, beside `sidecar` when given.
fn staged(name: &str, sidecar: Option<Vec<u8>>) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("native-cards-{name}-{}", std::process::id()));
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
        !layers.contains(&Value::from("cards")),
        "the fixture predates cards"
    );
    layers.push(Value::from("cards"));
    fs::write(dir.join("manifest.json"), manifest.to_string()).unwrap();
    if let Some(bytes) = sidecar {
        fs::write(dir.join(cards::FILE), bytes).unwrap();
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
fn a_manifest_naming_cards_without_their_file_is_refused_by_name() {
    let dir = staged("missing", None);
    let e = refusal(&dir);
    assert!(e.contains(cards::FILE), "{e}");
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn cards_recorded_for_another_run_are_refused() {
    let frames = frame_count();
    let dir = staged("stale", Some(recorded(frames + 1, "own-1")));
    let e = refusal(&dir);
    assert!(
        e.contains(cards::FILE) && e.contains(&format!("for {frames} frame records")),
        "{e}"
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_capture_from_before_cards_replays_without_them() {
    let replay = Replay::open(Path::new(RUN), &library()).unwrap();
    assert!(replay.cards.is_none());
}

#[test]
fn recorded_cards_play_back_frame_by_frame_unless_left_out() {
    let dir = staged(
        "recorded",
        Some(recorded(frame_count(), "contact-5 retiring")),
    );
    let replay = Replay::open(&dir, &library()).unwrap();
    let shown = replay.cards.as_ref().expect("the recorded cards");
    let last = replay.records.len() - 1;
    assert_eq!(shown.cards_of(last), ["contact-5 retiring"]);
    assert!(
        shown.cards_of(0).is_empty(),
        "a frame before the card has none"
    );
    assert_eq!(
        shown.draw(last, [1280, 720]).quads().len(),
        1,
        "the backing is drawn"
    );
    let open = |playback: Playback| {
        Replay::open_played(
            &dir,
            &library(),
            Path::new(native_client::capture::ROOT),
            playback,
        )
        .unwrap()
    };
    let clean = open(Playback {
        cards: false,
        ..Playback::default()
    });
    assert!(clean.cards.is_none(), "--no-cards draws none");
    let without_indicators = open(Playback {
        indicators: false,
        ..Playback::default()
    });
    assert!(
        without_indicators.cards.is_some(),
        "--no-indicators leaves the cards"
    );
    let without_fog = open(Playback {
        fog: false,
        ..Playback::default()
    });
    assert!(without_fog.cards.is_some(), "--no-fog leaves the cards");
    for fog in [true, false] {
        let clean = open(Playback {
            fog,
            indicators: false,
            cards: false,
        });
        assert!(
            clean.cards.is_none() && clean.indicators.is_none(),
            "--no-indicators --no-cards is a clean clip (fog {fog})"
        );
    }
    fs::remove_dir_all(dir).unwrap();
}
