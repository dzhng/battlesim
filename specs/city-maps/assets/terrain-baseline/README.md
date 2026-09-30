# Exact terrain representation evidence

This freezes the dense surface oracle and the first bounded terrain candidate. `identity.json` pins the production sources and evidence by SHA-256; the owning production commit preserves their code. These are physical and resource proofs, not released city artwork or complete world-scale acceptance.

The oracle retains the original sampled heights, normals, first ray hits and classifications over the existing relief/water/bridge fixture. The village trace retains every tick's battle digest. Tests consume the oracle directly, so changing the representation cannot silently change its expected answers.

`native/` records serial allocation-counter arms at the fixed extents, including an end-to-end narrow river. Peaks include the still-dense foliage owner; they must not be interpreted as terrain-only storage. `gpu.json` records actual Apple Metal height readback using the production page reader and triangle rule, its binding limits and released-resource counts. The browser scene metadata identifies the production screenshots under `visual/`; `diff.json` compares original and candidate frames at matched framing. The three side-by-side shots are the human review set.

## Reproduction

From a checkout of the owning production revision with shared web dependencies, build wasm into `web/src/wasm`. Copy `allocation_probe.rs` temporarily into `crates/sim/examples/terrain_allocation_probe.rs`, then run it in release mode with arguments `12000 world`, `15000 world`, `18000 world`, and `18000 world-river`, serially. Remove the temporary example afterward. Use a distinct worktree Cargo target and preflight source/temporary overlap against S0's resource ceiling.

Run `node specs/city-maps/assets/terrain-baseline/gpu_probe.mjs` from the repo root with Google Chrome installed. It starts its own Vite server, runs headless on Apple Metal and writes `throwaway/city-spike/terrain-gpu.json`. The runner resolves TypeGPU through Vite's exact transformed import URL so the production helpers and probe use the same runtime instance.

The ordinary geometry, ground and fog browser scenes own screenshot framing and behavioral assertions. Ground frames here predate SA3's scar-cache correction; later SA3 evidence owns that separate change. Preserve these originals when judging the terrain candidate.
