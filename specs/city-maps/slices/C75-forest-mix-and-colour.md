# C75: forest mix and colour

**Depends on:** C72, C74. **Kind:** slice.

## Question
Does a wood read varied only in shape and colour: mostly one family per stand, plus the odd birch and snag?

## Contract it unlocks
The biome's `trees.species` holds weights and tints over all kinds. Snags are ≤5% and stay out of a forest's outer ring and out of strips. Placement stays one tree per sim trunk (`placement.ts:262-265`).

## API seam
`fixtures/biomes/summer.json` (`trees`), `scenery/placement.ts`.

## What the human can run or see
`forest-edge-65` and `patchwork-1100`.

## Verification
- The placement test still gives one tree per trunk.
- A frame-cost row.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**the forest canopy seen from above; critique: "does any darker crown read as cloud shadow?"**) against **battle-look's WARNO forest frames, `../assets/reference/ground/forest-road-summer.jpg` and C74's shot**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Density read, floor.


## Delegated to the implementer
Weights and tints. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
The one forest rule.

## Feedback that would change this slice
Species colours that obscure canopy depth change the mix/palette with skeletons and density held fixed.

## Outcome (2026-10-02)

**Contract as landed.** A species row in the biome's `trees.species` is `{ appearance, weight, tint, family?, interior_m? }`, and `trees.stands` is `{ size_m, purity }`. Placement (`speciesAt` in `scenery/placement.ts`) cuts the land into stands, each one family's, and draws a trunk as its stand's family most of the time; a species of no family is the odd tree among them. A species with `interior_m` stands only that deep inside a forest, never in a strip and never past the map. Placement is still exactly one tree per simulated trunk.

| Species | Family | Share of trees | Tint |
|---|---|---|---|
| broadleaf, spreading, tall | broadleaf | 25%, 15%, 10% | as before |
| spruce, pine | conifer | 25%, 15% | cooler green |
| birch | none | 6.6% | lighter, yellower |
| snag | none; 12 m inside, no strips | 2% | none |

Stands are about 140 m across and 0.88 pure.

**What changed from the slice as written.** Nothing in the contract. The first tints were softened after the critique (below); that change landed with C76.

**Frame cost.** Paired with today's trees at the village's wood edge (65 m, the default pitch; two source trees in two browsers, 150 forced frames a batch, 5 rounds, Apple Metal, 1920×1080): **+0.50 ms GPU** (median; four of five rounds between +0.44 and +0.63). The mix draws fewer triangles there (550k against 628k a view pass), so the cost is the extra kinds a chunk draws, not geometry. In the same run the mixed wood's trees cost +0.58 ms against no trees at all, and today's read −0.09 ms, so the run resolves about half a millisecond. The lane's bar for trees is 1.5 ms.

**Compare.** Against the before shots at the same stations: mean luminance moved by 0.1 to 2.7 of 255 and its 5th percentile by under 1.5, so the wood got no darker; the colours in a frame rose by 10 to 50%. Against the Broken Arrow frames the wood now reads as stands of different trees rather than one texture; ours is coarser and more saturated. Less wrong, not a match.

**Critique** (unprimed, seven stations). It named four kinds grouped in stands with yellow-green trees scattered singly, which is the contract. On the slice's question: dark trees read as conifers by their shape, not as cloud shadow, with two medium-confidence exceptions (a spruce's shaded side going blue-black at 25 m; one conifer group darkening gradually on the river lab). Findings and what was done:

- Conifers too cold on their shaded sides, birches "autumnal or sickly": both tints pulled toward the broadleaf green (C76).
- Needle-thin stretched trees at wood edges, a canopy that reads as polka dots at 250 m, blocky dark blotches on the forest floor: all C76's, fixed there.
- Trees stand in rows like a plantation: the simulation's trunk grid (9 m, 0.3 jitter). Not this lane's to change; for the sim lane or the user.
- At 2,500 m a wood is a smooth slate blob that could be a cloud shadow or a pond: today's far tier under haze, unchanged by this slice. Open for C87.
- Birch boles show as white specks at 250 m; the village's woods are hard rectangles (map data); no undergrowth (C79).

**Pictures** (scratch): `throwaway/shots/c75/` (after) against `throwaway/shots/before/ground/` at the same stations.
