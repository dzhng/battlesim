# 17 — Effects

**Contract:** native draws every effect the browser draws, from the same published causes, on the same clock and with the same seeds. That covers tracers, impacts and ricochet sparks, muzzle flashes, blasts, dust, aircraft smoke trails, wreck and ruin fire and smoke, cook-offs and rotor blur.

**Seam:**
- The capture already carries every cause (slice 13's decode), so no new data path is needed.
- Port the browser's derivations:
  - `apps/battle-lab/src/effectFeed.ts`;
  - `packages/battle-renderer/src/effects/effectFrame.ts`, `effectPass.ts` and `launches.ts`;
  - `apps/battle-lab/src/cookOffs.ts` and `cookOff.ts`.
- **Determinism:**
  - Per-tick bursts use `mulberry32((tick·2654435761)>>>0)`, consumed in publication order.
  - Dust, smoke trails and smoke sources use their `hashString` seeds.
  - Time is the frame record's `simTime`.
- The manifest gains `presentation.effects` and the catalog rows effects read: muzzles, wrecks and `buildingRemains`.
- Muzzle flashes sit on the posed muzzles from slice 14.

**Runnable artifact:** plate pairs across the reel, including the explosion-filled paris-corner plate that slice 03 couldn't judge.

**Focused gates:**
- **Counts.** Per plate, live effects of each kind equal the browser's, which the browser exports from `effectFrame` in the plate's frame.
- **Seeds.** A test checks seeded particle positions for fixed causes against TS vectors.
- **Visual.** `compare-screenshots`, then an unprimed `screenshot-critique`, then `preview-shots`.
- **Cost.** Pass cost reported, and slice 12's measurement re-run with native at or below the browser.

**Visual variable:** effects.

**Delegated:** the particle drawing technique, provided counts and seeded positions match.
