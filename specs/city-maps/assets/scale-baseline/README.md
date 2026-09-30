# Current-path allocation baseline

Frozen inputs and measured resource results for [S0](../../spikes/S0.md), before systems architecture changes. These files are evidence, not production code or tests. `identity.json` pins source and artifact hashes; `allocation-model.json` contains predictions, while `native/` and `browser.json` contain executed results.

To reproduce, copy the three probe sources to `throwaway/city-spike/` in the pinned source checkout. Copy `allocation_probe.rs` temporarily into `crates/sim/examples/city_allocation_probe.rs`, build the example with an isolated `CARGO_TARGET_DIR`, and run `run_native.py` with that variable set. Build wasm once, then run `browser_probe.mjs`. The browser probe serves its own localhost origin, runs headless Chrome/Metal, and closes its server/browser/worker. Remove the temporary example after the run. Keep these baseline files immutable; new representation proofs get separate evidence.

Elapsed times vary with host load. Allocator sizes and canonical boundary outputs are the reproducible contract; RSS includes allocator/platform overhead. The native experiment ceiling is enforced by admission arithmetic, not an allocator interception that recovers from arbitrary OOM.
