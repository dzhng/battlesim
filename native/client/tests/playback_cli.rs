//! Playback chooses its layers on the command line: `--no-indicators`,
//! `--no-cards` and `--no-fog` are accepted, each on its own or in any
//! combination (the first two together are a clean clip), and an argument
//! the client doesn't have is still refused. Each request here fails on a
//! later check before any window or GPU, so parsing is all it exercises;
//! what each flag leaves out of a replay is `tests/cards.rs`'s.
use std::process::Command;

fn refusal(args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_native-client"))
        .args(["--run", "unused", "--scene", "unused"])
        .args(args)
        .output()
        .unwrap();
    assert!(!output.status.success(), "{args:?} ran");
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn indicators_cards_and_fog_are_left_out_independently() {
    for flags in [
        &["--no-indicators"][..],
        &["--no-cards"],
        &["--no-fog"],
        &["--no-indicators", "--no-fog"],
        &["--no-cards", "--no-fog"],
        // A clean clip, with and without the fog.
        &["--no-indicators", "--no-cards"],
        &["--no-indicators", "--no-cards", "--no-fog"],
    ] {
        // Without --out or --bench the request fails after its flags parse.
        let error = refusal(flags);
        assert!(
            error.contains("one of --out and --bench"),
            "{flags:?}: {error}"
        );
    }
}

#[test]
fn an_unknown_argument_is_still_refused() {
    let error = refusal(&["--out", "unused", "--no-sky"]);
    assert!(error.contains("unknown argument --no-sky"), "{error}");
}
