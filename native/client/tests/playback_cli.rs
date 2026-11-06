//! Playback chooses its layers on the command line: `--no-indicators` and
//! `--no-fog` are accepted, each on its own or together, and an argument the
//! client doesn't have is still refused. Each request here fails on a later
//! check before any window or GPU, so parsing is all it exercises.
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
fn indicators_and_fog_are_left_out_independently() {
    for flags in [
        &["--no-indicators"][..],
        &["--no-fog"],
        &["--no-indicators", "--no-fog"],
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
    let error = refusal(&["--out", "unused", "--no-cards"]);
    assert!(error.contains("unknown argument --no-cards"), "{error}");
}
