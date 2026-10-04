# 00 Measure the scenery download and prove stand-in bodies

**Unlocks:** the two numbers every art slice inherits (a byte cap per new piece, and whether regional loading is needed beyond slice 07's filter), and proof that a catalog kind with no art places, simulates and draws.

**Seam:** none changed. Measurement only, through `web/asset.mjs` and the runtime catalog (`assets/runtime/catalog.json`: `appearances`, `gzip`).

**Do:**
1. Measure what a map downloads today: every non-kit appearance's served (gzip) and raw bytes, grouped by scenery kind; what dominates a street prop's bundle (`bench`, `parked_car`): textures, LODs or geometry.
2. Add one throwaway catalog kind with a body and no art on a scratch branch; confirm it places through the presets, passes asset validation and draws as the tinted stand-in (`presentation.stand_ins` in `fixtures/game.json`, `models/standInKit.ts`). Discard the branch.
3. Decide the per-piece cap (target ≤ 0.5 MiB served) and how pieces share materials. Write both into this spec's README under Design and into [choices.md](../choices.md).

**Run/see:** a short table in `throwaway/` and the decision in the README.

**Stays green:** everything; nothing ships.

**Delegated:** how to measure. **Feedback that changes it:** if served bytes per map are already near a limit the user cares about, slice 07 grows a lazy-loading step.
