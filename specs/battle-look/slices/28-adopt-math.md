# 28 — Adopt `math` as the one TypeScript math owner

**Status:** done. **Depends on:** 13 (so the renderer lane is quiet). **Lane:** renderer. Run it before 20 and 23 if possible, so they build on it.

## Contract

The user's decision (`decisions.md`, 2026-09-25): the pmndrs `math` package owns TypeScript vector, matrix, quaternion, shape, culling, noise, seeded-random, easing and spring math. This slice migrates every hand-rolled equivalent to it and deletes the originals. Load [`math`](../../../.agents/skills/math/SKILL.md) first; its API reference is `web/node_modules/math/API.md`.

## API seam

- `packages/renderer-core/src/mat4.ts` and the matrix and vector parts of `camera3d.ts` are replaced by `math`'s `mat4`, `vec3` and `vec4`. `Camera3DParams` stays; `liveCamera`, `projectPoint` and `screenRay` are built on `math`.
- `packages/renderer-core/src/math.ts`: keep only helpers `math` lacks. `hash2` stays unless `math/random` gives the same values, because CPU data builders depend on it for determinism.
- Picking and ray tests (`pickInstance`, `rayAt`) use `math/shapes` rays and boxes.
- Ported code from `~/dev/game` that carries its own vector math migrates too. Update its reuse-manifest entries' `local_changes`.
- Hot paths (per-frame instance packing, overlays, the camera controller) follow the skill: module-level scratch named `_owner_purpose`, `out`-first calls, no allocation per frame. Marshal into GPU typed arrays with `toBuffer` / `set`.

## What you can run or see

Nothing visible changes: every scene's pixels and every benchmark phase stay the same.

## Verification

- The camera uniform layout test, picking, `projectToCss` agreement and every scene stay green with no pixel retunes.
- A grep test finds no hand-rolled `mat4`/`vec3` implementations left outside `math`.
- A benchmark short run: CPU frame time does not regress. Record it in [`frame-cost.md`](../frame-cost.md).

## Decision budget

- **Delegated:** the order of migration, and module-scratch naming within the skill's rule.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Replay and digest parity. Every existing scene and test. `bun run check` and `bun run verify` at closeout.
