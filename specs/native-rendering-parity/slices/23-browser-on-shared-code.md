# 23 — The browser on shared code: whole-game check

**Contract:** after 19 and 20, the shipped browser runs its publication reader and its presentation derivations from Rust through WASM. This slice proves the whole game, not just the reel, is unchanged. That covers battles, replays, the labs that use poses or effects, and the menu.

**Gates:**
- Every browser scene that draws units or effects passes, along with the visual suite (`bun run --cwd web visual`) against its approved pictures.
- Battle frame time on a medium generated battle (the method in `specs/performance/README.md`) is no worse than before 19, with main-thread CPU and GPU reported.
- Digest and replay unchanged.

The repository's full check still runs once, at 24.
