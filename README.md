# battlegame

A browser real-time tactics game, built around reconnaissance, physical fire and positioning. Every round is a flown projectile, and every side sees only what its own units can see. The first playable checkpoint is a replayable assault on a defended village. The full design lives in [`specs/`](specs/).

## How it fits together

The simulation is the one authority; everything else observes it.

- **`crates/`** — the Rust side.
  - `contract` holds the shared data shapes: scenario, commands, and the per-side observation.
  - `sim` owns every rule: movement, sight, sensing, knowledge, weapons, flight, damage, garrisons, supply and deployment.
  - `game-wasm` is the thin WebAssembly boundary.

  New battle state must enter `Battle::digest`, so replays and parity checks catch drift.
- **`web/src/battle/`** — the browser side of that boundary:
  - the worker that runs the simulation (one authority, ordered commands, bounded publications);
  - decoding of the packed observation;
  - player input;
  - player readouts.

  Presentation reads only the observation, never simulation state.
- **`packages/`** — the TypeGPU renderer.
  - `renderer-core` holds device, camera and projection primitives.
  - `battle-renderer` holds scene resources, meshes and overlays.
- **`apps/battle-lab/`** — the lab app. Each lab route is a focused, deterministic fixture for one mechanic, and the village routes are the playable game. `src/fixtures.json` is the registry of lab routes.
- **`fixtures/`** — authored maps, units and rule numbers. `village.json` is the one owner of the game's rules; labs reuse it.
- **`web/scenes/`** — one headless browser scene per registered fixture. These scenes are the visual and behavioural checks, run by `web/scene.mjs`.

## Running it

You need [Bun](https://bun.sh), a Rust toolchain with the `wasm32-unknown-unknown` target, [wasm-pack](https://rustwasm.github.io/wasm-pack/), and a WebGPU browser.

```bash
bun run setup   # install web dependencies
bun run dev     # build the WebAssembly, start the lab app
```

`/` is the main menu: play the village, watch a replay, run the benchmark, or open the lab index at `/labs`, which links every route. The benchmark (`/benchmark`) is the one frame-cost measure.

## Checks

`package.json` names the gates:
- `check` covers format, lint, typecheck and every Rust and web test.
- `verify` runs every browser scene.
- `bun run --cwd web scene -- <fixture-id>` runs one scene.

[`AGENTS.md`](AGENTS.md) says which runner to reach for while iterating. It also covers the worktree recipe (Git LFS, shared `node_modules` and build directory) that keeps parallel work from exhausting the machine.

## Plans and decisions

Work is planned as specs in [`specs/<feature>/`](specs/). Each spec has:
- a README whose "Next Agent Prompt" is the live handoff;
- a slice ladder;
- `choices.md`, the ledger of decisions made where the spec was silent.

[`design/`](design/) keeps the earliest planning map.
