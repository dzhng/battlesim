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

**Saved: `fixtures/maps/market-town/`, Mixed Small seed 1 with its planned `assault`, playable from the main menu beside the village** (`/battle?map=market-town&recipe=assault`). What made it possible is a compact saved form for every map; the decisions are in the [choices ledger](../choices.md#compact-saved-maps).

**The saved-map contract** (one rule for all fifteen folders; the folder guide is [`fixtures/README.md`](../../../fixtures/README.md#saved-maps)).

```text
map.json      contract::map::SavedMap = MapDefinition<SavedBuilding>
  building    { owner, kind, template_id, frame: { translation, yaw }, parts: [{ part, prop }] }
              no geometry, category or regional_family: those are the template's
SOURCES.json  contract::maps::MapSources
  identity    unchanged; map_hash is the hash of the RESOLVED definition
  catalogue   { library: "<file>.json", template_ids: [..] | null }
  inputs      unchanged
```

- **The resolver** (`contract::maps::resolve(map_json, sources_json, library_json, admission)`) materializes each building from its template at its frame and answers the same resolved `MapDefinition` as before. A resolved map cannot hold geometry a template did not make, because there is none in the file to disagree. `MapDefinition::saved()` is the way back, and the `mapgen` CLI writes it. A `map.json` that carries building geometry is refused (`invalid_map`); a building naming a template its catalogue lacks is refused at `map.json.buildings[i].template_id`.
- **The library** is what `SOURCES.json` names: a file name beside `maps/` (`building-templates.json` for the authored maps, `prototype-building-templates.json` for a generated one), never a path (`invalid_sources` at `SOURCES.json.catalogue.library`). The name only says where the library is; the catalogue hash the map names is what admits it. The resolver reads a library as a canonical catalogue or as the descriptor list the generator reads. The adapters fetch the named file: `sim::maps::Catalogue { maps, libraries }`, `openCatalogue(maps, libraries)` in Node, a glob of both files in the browser. `sim::maps::load_folder(path)` resolves a saved map's folder anywhere, which the `encounter_report` and `city_report` examples now take.
- **The admission** `MapAdmission::CATALOGUE` is 60,000 parts and 600,000 bay positions: the generator's limits in `fixtures/generated-battle.json`, so any map the game may generate can be saved. A test holds the two equal. Both are counted from the templates before any building is materialized.
- **JavaScript** gets `ResolvedMap.json`, the definition as the resolver wrote it, and preparation's catalogue arm splices that text into the scenario where it used to print the parsed definition again.

**Sizes.** `map.json` for Mixed Small seed 1 went from 10,333,689 bytes to 687,586 (98,783 gzipped); the folder is 712,860 bytes with `SOURCES.json`, `meta.json`, `sites.json` (21,772) and `encounters/assault.json` (2,409). Of the map, the 3,111 buildings are 597 KB (192 bytes each), the 532 surfaces 72 KB and the 28 forests 18 KB. A further cut is there if wanted (the `kind` and the one-part `parts` list repeat on every building) and was not taken: the file is a third of the 2 MB budget.

| Folder | `map.json` before | after |
|---|---|---|
| ambush | 3,103 | 649 |
| endurance | 13,967 | 2,333 |
| garrison | 1,841 | 619 |
| geometry | 3,349 | 1,799 |
| movement | 3,642 | 2,103 |
| readouts | 1,653 | 430 |
| sensors | 2,052 | 829 |
| village | 10,926 | 7,251 |
| weapons | 1,748 | 526 |
| consequences, deployment, ground, river, supply | no buildings | unchanged |
| market-town | (10,333,689 resolved) | 687,586 |

**Nothing moved.** Before and after the conversion, each of the fourteen folders was resolved natively and the whole answer (definition and identity) written as JSON: the fourteen files are byte for byte identical. No `map_hash` in any `SOURCES.json` changed, and the resolver still checks each against the resolved definition; the only edit to those files is the added `library`. The simulation's tests, which include the village's and endurance's digest and replay tests, pass on the converted folders.

**The menu.** `Play Market Town` is one entry under `Play village`, the same card: "A fixed battlefield: attack the defended town as blue." The menu lists every released playable map of the catalogue that has the game's encounter (`assault`) saved on it, so the next saved map needs no menu change. The loading screen and the pause menu name it `MARKET TOWN · ASSAULT`.

**Reproducing it.** `mapgen request <type> <size> <seed> <presets> <catalogue> <game.json>` prints the pinned request the game makes for that choice; `mapgen generate-map` writes the folder's `map.json`, `SOURCES.json` and `sites.json`; `encounter_report --save` writes `encounters/assault.json` from the planner. The three commands are in the fixtures guide.

**Proof.**

- `crates/contract/tests/maps.rs`: a saved building resolves to its template materialized at its frame, and saving and resolving again gives the same map; an unknown template is refused by the building's index; a map carrying geometry is refused; a library named as a path is refused.
- `crates/sim/tests/maps.rs`: every folder resolves to the identity it pins and every saved encounter (the town's included) makes a battle; the catalogue's allowance is the generator's; a library the catalogue lacks is a missing document.
- `crates/mapgen/tests`: the CLI's saved map is `saved()` of the compiled map, names its catalogue's file, and resolves back to the compiled map against that file as given; every generated cell does so under the catalogue's allowance.
- `web/tests`: `mapCatalogue` (the same refusals through Wasm, a saved building's fields, the definition handed on), `prepareBattle` (the town plays its saved assault through the catalogue arm, on the resolver's text, and runs), `battleStart` and `router` (the menu's entry and its address).
- The `generated` scene starts the town from the menu and checks the map, the encounter and the subject it shows. In that run (development build, a loaded machine) the saved map resolved in 0.24 s and was playable 3.9 s after the menu's link; the same map generated from the menu was playable after 8.8 s.

**Not done.** The saved map is the `layout-5` generator's seed 1, made on this branch before it met the `layout-6` generator: once the two are merged, save it again with the three commands and look at the new town before committing it. The benchmark on the saved map. The camera lab still compiles its own plan at run time (`fixtures/camera-lab.json`): saving it would pin the prototype library's hash, and a lab that other work on those templates must re-save each time is worse than one that compiles what it is given. The same holds for `market-town`, by intent: it is a reviewed map, and it stops resolving, with the catalogue tests saying so, when a prototype template changes. A catalogue map is still not planned at run time: its `sites.json` is saved but only the report reads it.
