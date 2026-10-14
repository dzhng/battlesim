# One renderer (placeholder)

**Status:** placeholder. Nothing is decided, sliced or implemented yet.
**Updated:** 2026-10-13.

## Next Agent Prompt

You are picking up an open decision: should the browser move onto the same Rust `wgpu` renderer native uses, so the game has one renderer instead of two? The user owns the decision. Don't start building until they make it. When the user starts this, run `explore-unknowns` with them, then (if they choose to go ahead) `write-spec`, and turn this page into a sliced plan. Load `renderer` first, and consult Bevy as a reference for engine architecture (not a dependency). Update this section before you end each pass.

- [ ] Walk the decision with the user (see the open questions below).
- [ ] If yes: research `wgpu`'s WebGPU backend in the browser (WASM), and measure it against today's TypeGPU renderer on the reel, browser first.
- [ ] If yes: slice the move so the browser keeps shipping the whole way, with plates, counts and frame time gated per slice.

## Why it exists

[Native rendering parity](../done/native-rendering-parity/README.md) gave the game two renderers that draw the same picture:

- **Browser:** TypeGPU over WebGPU, in TypeScript (`packages/battle-renderer`). It owns the shaders, written with TypeGPU and exported as WGSL (`packages/battle-renderer/pipelines`).
- **Native:** Rust `wgpu` on Metal (`native/client`), running the browser's exported WGSL unchanged.

Both draw only what the shared Rust presentation crate hands them (`crates/presentation`, the 18b contract), so the decisions already have one owner. What's still duplicated is the renderer itself:
- pass orchestration;
- render targets;
- GPU resource lifetimes;
- uploading `Frame`'s records;
- each layer's draw routing.

Every rendering change is made twice, and parity has to be re-proved. The archive deferred this decision until native was measured against TypeGPU. It now has been: native matches or beats the browser's frame rate and worst frames, with a GPU frame 5–7% longer, accepted because `wgpu` on Metal can't overlap passes the way Chrome's WebGPU does.

## The tension

- **For one renderer:**
  - one owner per concept, so renderer duplication goes away;
  - native and browser can't drift apart;
  - Rust stays the smart side end to end.
- **Against:**
  - The user chose TypeGPU (2026-09-27) because it performs well and the shaders are written in TypeScript with it. Moving the browser to `wgpu` would change where shaders are authored: in WGSL, or in Rust-side tooling.
  - The browser is the shipped game. A renderer swap is a large risk to the one client players actually use, for a gain that's mostly in maintenance.
  - The browser's GPU frame is currently the faster of the two.

## Open questions for the walk

- Is the goal one renderer, or only one shader source (which already holds: native runs the browser's WGSL)?
- If the browser moves, where are shaders authored: hand-written WGSL, TypeGPU kept as an authoring tool that only exports, or something Rust-side?
- Does `wgpu` in the browser (its WebGPU backend in WASM) match TypeGPU's frame time on the reel, and what does it cost in bundle size and startup (budget: 30 s to playable)?
- What happens to the labs and workbenches that drive the TypeScript renderer directly?
- Alternatively: keep two thin renderers, and shrink what's duplicated by moving more of the orchestration (pass list, target sizes) into `Frame` as data both clients follow.
