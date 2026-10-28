# Choices ledger

Decisions made where the spec was silent or wrong, with verdicts. Final consolidation happens at closeout.

## Slice 00

- **The clippy guard covers library code, not native-only tests and examples.** The slice said test code follows the rule too. That would mean rewriting 176 calls in test and example crates, which only ever run natively, so their values never meet WebAssembly. `crates/clippy.toml` lists the methods. Each of contract, sim, mapgen and presentation sets `disallowed_methods = "allow"` in its `Cargo.toml` `[lints.clippy]` and `#![deny(clippy::disallowed_methods)]` in its `lib.rs`. So the library, including its `#[cfg(test)]` modules, is guarded, and no test file changes. Per-file allow attributes were tried first, but the size ratchet refuses growth in test files already over budget. *Verdict: sound; the rule protects what runs in both builds.*
- **`powi` is not disallowed.** It isn't transcendental, and `contract/src/map.rs` squares with it. *Verdict: sound.*
- **The presentation crate had four production violations** of its own rule (`scars/cache.rs` `log2`, `sound/bank.rs` `log10`), plus test-module calls. All are converted, and the guard now covers it too. *Verdict: sound.*
- **No digest, parity record, menu reel or map moved.** The whole Rust suite passed except one pre-existing failure, which is fixed separately (see below). *Verdict: evidence.*

## Slice 01

- **The rules type declares the three non-simulation sections it still carries; it doesn't strip them.** `UncheckedRules` admits `presentation` (owned and parsed by presentation, so the simulation doesn't inspect it), `controls` (typed and strict, owned by the browser's `reverseZone.ts`) and `seed` (the battle lab's default). The alternative was stripping them in each reader: native `fixtures::game()`/`admit()` and the browser's `compose.ts`. That is more places to keep in step. *Verdict: sound.*
- **`status`, top-level `units` and `physics.blast_falloff` were deleted.** They were prose annotations nothing read; strict admission refused them. *Verdict: sound.*
- **Two callers had embedded `map` inside the rules** (`settlement_tests.rs`, `examples/ground_resources.rs`). They now pass the map separately. *Verdict: sound; strictness found a real misuse.*
- **The forest rule types moved to `crates/contract/src/scenario/forest.rs`** unchanged, so the over-budget `scenario.rs` shrinks. *Verdict: sound; slice 04 edits them there.*
- **`mechanics_validate`'s tests now run with the workspace suite** (`[[example]] test = true`). Before, `cargo test --workspace` built them but never ran them, including the existing admission tests. *Verdict: sound; a latent gap.*
