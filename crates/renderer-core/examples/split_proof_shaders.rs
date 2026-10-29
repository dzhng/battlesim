//! Transitional split-game-json control; remove with slice 16.
//! Composed sources have no fixture dependency; the data-file split must
//! leave their bytes untouched.
//! cargo run -p renderer-core --example split_proof_shaders -- <output>/shaders
use std::{fs, path::PathBuf};

fn main() {
    let out = PathBuf::from(std::env::args_os().nth(1).expect("output directory"));
    fs::create_dir_all(&out).unwrap();
    let entries = include_str!("../src/shaders/pipeline_sources.txt");
    let mut count = 0;
    for line in entries.lines().filter(|line| !line.trim().is_empty()) {
        let (name, _) = line.split_once('|').expect("pipeline source entry");
        let source = renderer_core::pipelines::builtin_source(name).expect("built-in source");
        fs::write(out.join(name), source).unwrap();
        count += 1;
    }
    eprintln!("wrote {count} composed shader sources to {}", out.display());
}
