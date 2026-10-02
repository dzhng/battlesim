# Exact runtime rotation

The original materializer emitted normals that differed by one ULP between native and Wasm in seven of ninety-six placement frames. That is a real canonical-geometry failure, independent of the old simulation's initial-squad numeric differences.

Rust's [host trigonometry contract](https://doc.rust-lang.org/std/primitive.f64.html#method.sin_cos) permits platform-dependent precision. Physical template materialization and admission therefore use pinned software `libm`, with its [architecture feature disabled](https://docs.rs/crate/libm/0.2.16/source/Cargo.toml). The same evaluator owns frame rotations, local-part rotations, exposure transforms and facade norms. Scenario parsing and simulation arithmetic keep their own owners.

With the fix, every value matches over the original angle range plus signed-zero, subnormal and huge finite angles. The angle corpus (`fixtures/parity/templates/runtime-rotation/`, at tag `data-files-before-prune-2026-10-01`) was the native side the browser test compared against; raw JSON keeps signed zero. What stays is the frame that failed: `fixtures/parity/templates/wasm-asymmetric.json` holds the Wasm build's output there, and the native test must equal it. The generator's paired records hold both runtimes to the same bytes over every rotated building of whole maps. The focused native regression failed on the original normal, then passed. The small physical reports still pass and were not repinned.

Canonical descriptor facts and catalogue hashes are unchanged; derived coordinates at the failing angles deliberately adopt the shared software result. So a compiled map's identity must include its complete materialized output and generator provenance: a dependency or algorithm change is not the same generated artifact. Full generator coverage, all type/size requests and whole-battle numeric parity are separate proofs.

Raw evidence: tag `city-maps-evidence-2026-09-30`.
