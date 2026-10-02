//! Build identity separates simulation implementation from presentation rebuilds.
#[path = "../build.rs"]
mod build;

use std::{fs, path::PathBuf};

struct Inputs(PathBuf);
impl Inputs {
    fn new() -> Self {
        let root =
            std::env::temp_dir().join(format!("battle-build-identity-{}", std::process::id()));
        for dir in [
            "crates/sim/src",
            "crates/contract/src",
            "crates/game-wasm/src",
            "crates/mapgen",
            "assets",
        ] {
            fs::create_dir_all(root.join(dir)).unwrap();
        }
        for path in [
            "Cargo.toml",
            "Cargo.lock",
            "crates/sim/build.rs",
            "crates/sim/Cargo.toml",
            "crates/contract/Cargo.toml",
            "crates/game-wasm/Cargo.toml",
            "crates/mapgen/Cargo.toml",
            "crates/sim/src/lib.rs",
            "crates/contract/src/lib.rs",
            "crates/game-wasm/src/lib.rs",
        ] {
            fs::write(root.join(path), path).unwrap();
        }
        Self(root)
    }
}
impl Drop for Inputs {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn art_rebuilds_and_portable_targets_keep_identity_but_engine_inputs_change_it() {
    let inputs = Inputs::new();
    let native = build::semantic_cfg([
        ("CARGO_CFG_UNIX".into(), "".into()),
        ("CARGO_CFG_TARGET_ARCH".into(), "aarch64".into()),
        ("CARGO_CFG_DEBUG_ASSERTIONS".into(), "".into()),
        ("CARGO_CFG_PANIC".into(), "unwind".into()),
    ]);
    let wasm = build::semantic_cfg([
        ("CARGO_CFG_TARGET_ARCH".into(), "wasm32".into()),
        ("CARGO_CFG_TARGET_FEATURE".into(), "simd128".into()),
        ("CARGO_CFG_PANIC".into(), "abort".into()),
    ]);
    let original = build::fingerprint(&inputs.0, "rustc-test", &native).unwrap();
    assert_eq!(
        original,
        build::fingerprint(&inputs.0, "rustc-test", &wasm).unwrap()
    );
    fs::write(inputs.0.join("assets/appearance.gltf"), "new appearance").unwrap();
    assert_eq!(
        original,
        build::fingerprint(&inputs.0, "rustc-test", &native).unwrap()
    );
    for path in [
        "crates/sim/src/lib.rs",
        "crates/contract/src/lib.rs",
        "crates/game-wasm/src/lib.rs",
        "Cargo.lock",
    ] {
        fs::write(inputs.0.join(path), "changed implementation or dependency").unwrap();
        assert_ne!(
            original,
            build::fingerprint(&inputs.0, "rustc-test", &native).unwrap(),
            "{path}"
        );
        fs::write(inputs.0.join(path), path).unwrap();
    }
    assert_ne!(
        original,
        build::fingerprint(&inputs.0, "other compiler", &native).unwrap()
    );
    let custom = build::semantic_cfg([("CARGO_CFG_SIMULATION_MODE".into(), "alternative".into())]);
    assert_ne!(
        original,
        build::fingerprint(&inputs.0, "rustc-test", &custom).unwrap()
    );
}
