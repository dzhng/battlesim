//! Portable simulation-source build fingerprint; presentation files are outside
//! its scope. Target/profile cfgs differ between native/Wasm without changing
//! the supported deterministic simulation contract.
use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[cfg(not(test))]
use std::{env, process::Command};

#[cfg(not(test))]
fn main() {
    let manifest =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("Cargo manifest directory"));
    let root = manifest.parent().unwrap().parent().unwrap();
    let compiler = Command::new(env::var_os("RUSTC").expect("Cargo compiler"))
        .arg("--version")
        .output()
        .expect("read compiler identity");
    assert!(compiler.status.success(), "compiler identity refused");
    let compiler = String::from_utf8(compiler.stdout).expect("compiler identity UTF-8");
    let cfg = semantic_cfg(env::vars());
    for (key, _) in &cfg {
        println!("cargo:rerun-if-env-changed={key}");
    }
    println!("cargo:rerun-if-env-changed=RUSTC");
    for path in [
        "Cargo.toml",
        "Cargo.lock",
        "crates/sim/build.rs",
        "crates/contract",
        "crates/sim/src",
        "crates/game-wasm",
        "crates/mapgen/Cargo.toml",
        "crates/sim/Cargo.toml",
    ] {
        println!("cargo:rerun-if-changed={}", root.join(path).display());
    }
    let identity = fingerprint(root, &compiler, &cfg).expect("read simulation build inputs");
    println!("cargo:rustc-env=SIM_ENGINE_BUILD_ID={identity}");
}

pub fn semantic_cfg(vars: impl IntoIterator<Item = (String, String)>) -> Vec<(String, String)> {
    let mut cfg: Vec<_> = vars
        .into_iter()
        .filter(|(key, _)| {
            key.starts_with("CARGO_FEATURE_")
                || (key.starts_with("CARGO_CFG_")
                    && !key.starts_with("CARGO_CFG_TARGET_")
                    && !matches!(
                        key.as_str(),
                        "CARGO_CFG_UNIX"
                            | "CARGO_CFG_WINDOWS"
                            | "CARGO_CFG_DEBUG_ASSERTIONS"
                            | "CARGO_CFG_PANIC"
                            | "CARGO_CFG_TEST"
                            | "CARGO_CFG_CLIPPY"
                            | "CARGO_CFG_UB_CHECKS"
                    ))
        })
        .collect();
    cfg.sort();
    cfg
}

pub fn fingerprint(root: &Path, compiler: &str, cfg: &[(String, String)]) -> io::Result<String> {
    let mut files: Vec<PathBuf> = [
        "Cargo.toml",
        "Cargo.lock",
        "crates/sim/build.rs",
        "crates/sim/Cargo.toml",
        "crates/contract/Cargo.toml",
        "crates/game-wasm/Cargo.toml",
        "crates/mapgen/Cargo.toml",
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect();
    for dir in [
        "crates/sim/src",
        "crates/contract/src",
        "crates/game-wasm/src",
    ] {
        sources(root, Path::new(dir), &mut files)?;
    }
    files.sort();
    let mut input = Vec::new();
    let mut part = |label: &[u8], bytes: &[u8]| {
        input.extend_from_slice(&(label.len() as u64).to_le_bytes());
        input.extend_from_slice(label);
        input.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
        input.extend_from_slice(bytes);
    };
    part(b"rustc", compiler.trim().as_bytes());
    for (key, value) in cfg {
        part(key.as_bytes(), value.as_bytes());
    }
    for path in files {
        let label = path.to_str().expect("source path UTF-8").replace('\\', "/");
        part(label.as_bytes(), &fs::read(root.join(path))?);
    }
    Ok(contract::identity::bytes_hash(&input))
}

fn sources(root: &Path, dir: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(root.join(dir))? {
        let entry = entry?;
        let path = dir.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            sources(root, &path, files)?;
        } else if path.extension().is_some_and(|e| e == "rs") {
            files.push(path);
        }
    }
    Ok(())
}
