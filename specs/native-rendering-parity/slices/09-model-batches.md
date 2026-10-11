# 09 — Native model drawing architecture

*Rewritten after 08.* The anatomy found no remaining waiting once the harness drew its own frames (07). The cost is work: the pre-parity `legacy_map_layer.gd` draws each unit as an unbatched multi-part model and each prop at full detail, and every shadow cascade draws them again. With that layer off, a market-town frame drops from about 40 ms to 9.

**Contract:** native draws models the way the browser does, as a general architecture that slice 13's authored models will reuse:
- one instanced batch per appearance and detail tier, not one node per placed part;
- distance tiers and impostors chosen by the browser's rules;
- shadow casters culled to what each cascade sees.

Draw calls and primitives per frame scale with appearances × tiers, not with placed objects.

**Decision** (orchestrator, 2026-10-11): neither tune the throwaway layer nor measure around it. Build the lasting draw path now, on the content the legacy layer places today. Slice 13 then changes identity, materials and placement, not how models are drawn.

**Seam:**
- **Browser owners to mirror:**
  - `packages/battle-renderer/src/models/modelLayer.ts`: instancing and tiers;
  - `models/impostor.ts`: the impostor atlas and its pitches;
  - `apps/battle-lab/src/buildingTier.ts`: pixel-sized tiers;
  - `frame/` shadow caster selection.
- **Native:**
  - `world/model_batches.gd`: one MultiMesh per appearance part and tier, rebuilt only when membership changes;
  - per-frame updates write instance transforms only;
  - an impostor path for the far tier;
  - shadow casting limited per cascade (Godot cascade blend and caster culling, plus per-tier `cast_shadow`, so far tiers and impostors don't cast).
- **`legacy_map_layer.gd` shrinks to placement** (what stands where), and drawing moves to the batches. Units keep their published poses; articulation is slice 13's.

**Runnable artifact:** Godot's per-frame draw-call and primitive counts, which don't move with machine load, plus untraced frame ms, before and after, on market-town records 1000–1299 and the whole reel, in `throwaway/perf/batches/`.

**Focused gates:**
- **Draw calls per frame.** Market-town drops from about 20,000 to the low hundreds. Report the exact count and its formula: appearances × parts × tiers × passes.
- **Frame time.** The median untraced frame in the 1000–1299 window, minimised, three runs, is at or below the browser's market-town p50 (16.7 ms).
- **Plates.**
  - Batching alone is byte-identical within the noise floor.
  - Tiers and impostors may change distant pixels. Judge them against the browser control with `compare-screenshots`, and they must not move further from the browser. Then an unprimed `screenshot-critique`.
- `native/godot/tests/run.sh` stays green.

**Visual variable:** none intended. Distant models may move toward the browser's own impostors.

**Delegated:** MultiMesh grouping and the update strategy, provided the counts gate holds.
