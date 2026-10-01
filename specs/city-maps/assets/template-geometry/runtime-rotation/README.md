# Exact runtime rotation

The original materializer emitted normals that differed by one ULP between native and Wasm in seven of ninety-six placement frames. That is a real canonical-geometry failure, independent of the old simulation's initial-squad numeric differences.

Rust's [host trigonometry contract](https://doc.rust-lang.org/std/primitive.f64.html#method.sin_cos) permits platform-dependent precision. Physical template materialization and admission therefore use pinned software `libm`, with its [architecture feature disabled](https://docs.rs/crate/libm/0.2.16/source/Cargo.toml). The same evaluator owns frame rotations, local-part rotations, exposure transforms and facade norms. Scenario parsing and simulation arithmetic keep their own owners.

With the fix, every value matches over the original angle range plus signed-zero, subnormal and huge finite angles. The paired records in [`fixtures/parity/templates/runtime-rotation/`](../../../../../fixtures/parity/templates/runtime-rotation/) are the native side the browser test compares against; raw JSON keeps signed zero. The rejected arm is in `fixtures/parity/templates/rejected-runtime-rotation/`. The focused native regression failed on the original normal, then passed. The small physical reports still pass and were not repinned.

Canonical descriptor facts and catalogue hashes are unchanged; derived coordinates at the failing angles deliberately adopt the shared software result. So a compiled map's identity must include its complete materialized output and generator provenance: a dependency or algorithm change is not the same generated artifact. Full generator coverage, all type/size requests and whole-battle numeric parity are separate proofs.

Raw evidence: tag `city-maps-evidence-2026-09-30`.
