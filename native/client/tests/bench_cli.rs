//! Invalid benchmark requests fail before creating a window or admitting a GPU.
use std::process::Command;

#[test]
fn a_held_benchmark_must_leave_reportable_frames_after_warmup() {
    let output = Command::new(env!("CARGO_BIN_EXE_native-client"))
        .args([
            "--run",
            "unused",
            "--scene",
            "unused",
            "--bench",
            "unused",
            "--bench-freeze-index",
            "660",
            "--bench-frames",
            "61",
        ])
        .output()
        .unwrap();
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(
        error.contains("--bench-frames") && error.contains("warm-up"),
        "{error}"
    );
}

#[test]
fn a_held_benchmark_requires_an_index_a_draw_count_and_bench_output() {
    for extra in [
        vec!["--bench-freeze-index", "660"],
        vec!["--bench-frames", "181"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_native-client"))
            .args(["--run", "unused", "--scene", "unused", "--bench", "unused"])
            .args(extra)
            .output()
            .unwrap();
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success());
        assert!(error.contains("together"), "{error}");
    }
    let output = Command::new(env!("CARGO_BIN_EXE_native-client"))
        .args([
            "--run",
            "unused",
            "--scene",
            "unused",
            "--out",
            "unused",
            "--bench-freeze-index",
            "660",
            "--bench-frames",
            "181",
        ])
        .output()
        .unwrap();
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(error.contains("require --bench"), "{error}");
}
