# Choices ledger

Decisions made where the spec was silent or wrong, with verdicts. Final consolidation happens at closeout.

## Slice 00

- **The clippy guard covers library code, not native-only tests and examples.** The slice said test code follows the rule too. That would mean rewriting 176 calls in test and example crates, which only ever run natively, so their values never meet WebAssembly. `crates/clippy.toml` lists the methods. Each of contract, sim, mapgen and presentation sets `disallowed_methods = "allow"` in its `Cargo.toml` `[lints.clippy]` and `#![deny(clippy::disallowed_methods)]` in its `lib.rs`. So the library, including its `#[cfg(test)]` modules, is guarded, and no test file changes. Per-file allow attributes were tried first, but the size ratchet refuses growth in test files already over budget. *Verdict: sound; the rule protects what runs in both builds.*
- **`powi` is not disallowed.** It isn't transcendental, and `contract/src/map.rs` squares with it. *Verdict: sound.*
- **The presentation crate had four production violations** of its own rule (`scars/cache.rs` `log2`, `sound/bank.rs` `log10`), plus test-module calls. All are converted, and the guard now covers it too. *Verdict: sound.*
- **No digest, parity record, menu reel or map moved.** The whole Rust suite passed except one pre-existing failure, which is fixed separately (see below). *Verdict: evidence.*
