# C58: first playable fixed-seed encounter

**Depends on:** C53, C09, C32/C14, C22/C23, C33, C40–C43, C57 and scale/fog changes required by G0. **Kind:** early delivery slice.

Final C31/C87 ground composition and complete family coverage are C54/C51 gates.

## Question
Can friends play the first reviewed fixed-seed town/plain encounter before runtime generation is integrated?

## Contract it unlocks
A saved generated Small map at [the current M04 extent](../procedural-maps.md#closed-decisions) and authored `encounters/<name>.json`, pinned to this exact map hash: deployments, capture objective, variants and defender inputs use legal physical anchors checked through existing sim queries. Authored placement uses the existing encounter shape; it adds no second planner. C59 later proves recipe planning on this same map before runtime delivery. Register replay/benchmark access through the catalogue/resolver. The encounter has a useful 1,800 m open approach/flank and an urban fight with infantry/vehicle access. Physical catalogue/map/preset identities stay pinned; compatible appearance identity is recorded separately. Use accepted release art for every selected template in this encounter; incomplete categories/families may remain explicit source gaps elsewhere. Existing ground appearance may serve this early checkpoint, with required physical surface/forest/river contracts intact.

This is the first useful offline checkpoint, not spec completion. C55 subsequently adds runtime acquisition to this same battle path; C54/C51 validate the complete preset and all-map release.

## API seam
Compiled map + encounter configuration → existing scenario builder and menu/catalogue route. Rename the builder for its actual role if necessary; do not add a generated-map-only battle implementation.

## What the human can run or see
A playable browser encounter on this Mac, replay and contact benchmark; plain-to-town movement and safe camera zoom/orbit/pan are reviewable.

## Verification
- Default camera ≥30 FPS during the matched encounter; G0's startup/memory/step/delivery bounds.
- Native/wasm replay and physical access/range demonstrations.
- Apply game-ui before menu changes. Compare whole-frame composition with accepted per-variable evidence using compare-screenshots; run unprimed screenshot-critique last; preview-shots non-blocking.
- Friends can play the review build; record feedback and affected slice owners. Numeric/type requirements cannot be waived by a pleasing single seed.

## Delegated to the implementer
Deployments/objective/script/camera station within G0's accepted access/range constraints.

## Must stay green
Pinned physical map identity, compatible art coverage and the original battle/scenario owners.

## Feedback that would change this slice
Playtest feedback changes encounter configuration or reopens the relevant layout/camera/rule owner.

## Outcome

**Not saved: the map is too large to commit as it stands.** A Mixed Small map's `map.json` is 9 to 15 MB (1.6 to 2.5 MB gzipped) over seeds 1 to 8, against a limit of about 2 MB set for this pass, so no generated map was added to `fixtures/maps/`. The size is the buildings: each of about 3,000 carries its template's whole materialized geometry (parts, floors, bays). A saved generated map also needs two things the catalogue does not have yet: the resolver's catalogue admission (`MapAdmission::CATALOGUE`, 4,096 parts and 65,536 bay positions) is far below a generated map, and the adapters hand every map the authored library (`building-templates.json`), while a generated map pins the prototype catalogue.

| Mixed Small, seed | `map.json` | gzipped |
|---|---|---|
| 1 | 10.3 MB | 1.9 MB |
| 2 | 14.8 MB | 2.5 MB |
| 3 | 14.1 MB | 2.4 MB |
| 4 | 9.2 MB | 1.6 MB |
| 5 | 12.7 MB | 2.2 MB |
| 6 | 12.9 MB | 2.2 MB |
| 7 | 14.3 MB | 2.4 MB |
| 8 | 13.6 MB | 2.4 MB |

**The seed to save when the size is settled: Mixed Small, seed 1.** Of the plans looked at (`mapgen inspect`, seeds 1, 4, 5, 6) it has the clearest town, in the middle of the map on the road between the two edges the sides start from, with a measured open approach beside that road on the attacker's side and no river to funnel the fight. It is also the cell the planner's report already measures. The menu plays it today as a generated map (`/battle?type=mixed&size=small&seed=1`).

**What exists for it.** The battle path this slice asked for is the one [C55](C55-runtime-generation.md#outcome) built: `PrepareBattleRequest` with `map_source: { kind: "catalogue", id }` resolves a saved map through the one resolver and plays its saved encounter `encounters/<recipe_id>.json` in the same battle view, and a test plays the village's `lean` that way. A saved generated map would be played by `/battle?map=<id>&recipe=assault` with no new code beyond the two admissions above. There is no generated-only battle implementation.

**Not done.** The saved map, its `SOURCES.json`, `meta.json` and saved `assault` encounter; the menu entry beside the village; the catalogue tests on it; the benchmark. The camera lab still compiles its own authored plan at run time (`fixtures/camera-lab.json` through `compile_map`): it is a plan, not a catalogue map or a generation request, so neither arm of `MapSource` fits it, and it was left alone.
