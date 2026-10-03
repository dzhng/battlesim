# C55: runtime map generation

**Depends on:** C53, C09, C32/C14, C22, C33, C59 and C58's accepted offline encounter; S6/G0 parity contract. **Kind:** required slice.

## Question
Can a player start an arbitrary supported seed from type/size presets with the same compiled map and template art as offline generation?

## Contract it unlocks
The pre-battle preparation worker invokes the same `crates/mapgen` library through wasm. `GenerationRequest` carries Open/Mixed/Metro, Small/Medium/Large, a lossless canonical seed string, generator/preset versions and physical template catalogue hash. Regional-family choice follows versioned preset data/seed, not another required player control.

Map-generation seed and battle RNG input remain distinct recorded inputs. Map and encounter seeds are canonical u64 decimal strings; battle RNG input stays an integer in 0..Number.MAX_SAFE_INTEGER under the existing f64 adapter, checked at preparation. The new generator adapter accepts the canonical string directly; it does not pass an arbitrary u64 through the existing f64 battle constructor.

`GenerateMapResult` returns compiled MapDefinition, immutable provenance/identity, stable diagnostics and bounded generation statistics. After map resolution, C59's shared recipe planner derives legal deployments, capture zones, defender inputs and garrison references for this exact map; coordinate-based saved encounters are not reused on another seed. The preparation result includes the resolved encounter/rules and their identity.

C09's one resolver handles `MapSource::Catalogue { id } | Generated { request }` and passes the same resolved definition to battle initialization. The sim/renderer never branch on generated versus saved source. Transient maps need no catalogue folder or committed index.

Generate before constructing/running the battle, outside ticks and frames. Bound retries. Superseded preparation requests release/discard resources through worker ownership; obsolete results cannot start a battle. Failures show a useful player message with developer diagnostics available separately, never another seed or hidden fallback.

The menu exposes the two composition controls and current seed/share identity. Pending preparation and failure are visible; gameplay starts only from a complete resolved result. Apply game-ui before changing the player flow. Runtime replay data stores the exact compiled map, prepared encounter, resolved rules, RNG inputs, request/physical identity and engine build identity. Playback is same-build only and refuses mismatched build/scenario/config digests; no older generator is required. Appearance hash is diagnostic metadata outside simulation identity. Current validated art covering the physical catalogue may render playback; pixel-exact historical art, cross-build compatibility and archival asset retention are outside scope.

## API seam
`PrepareBattleRequest { map_source, recipe_id, encounter_seed, battle_seed }` resolves through the common map owner, then C59. `PreparedBattleResult` carries compiled map, prepared encounter, resolved rules and composite simulation identity (map/physical catalogue, recipe/result, rules, engine build and RNG inputs), with appearance hash recorded separately, plus diagnostics. This preparation contract composes generation and scenario planning without making either own the other's data.

`contract` request/identity types → thin `game-wasm` adapter → existing browser map resolver/preparation worker → Battle initialization. TypeScript orchestrates and displays status; all generation rules remain Rust. C32's pure template resolver and C22's chunks serve both sources.

## What the human can run or see
Choose type and size, generate a fresh seed and play; share/replay it; generate the same request via CLI and compare its complete canonical identity. The map uses legal varied templates without a Blender job.

## Verification
- Frozen S6 parity and all nine type × size requests; include seeds above JS's exact integer range, failures and matching template hashes.
- Complete startup meets [the startup contract](../scale-direction.md#startup-and-loading), including required renderer readiness; a loading screen is allowed. Peak memory/downloads stay within G0 budgets; input/render remain responsive.
- Replace/cancel a pending request and switch maps without stale battle startup or leaked resources; bounded failure reports retain the requested seed.
- Same-build replay uses the stored compiled scenario/rules and matching digests; changed engine build or physical/config identity refuses clearly. A compatible art-only rebake leaves simulation replay identity unchanged; native/wasm battle parity holds.
- Menu/control/status variable only: compare with the existing menu's design conventions via compare-screenshots, run unprimed screenshot-critique last, and preview-shots non-blocking. Whole map appearance remains C54/C51's integration evidence.

## Delegated to the implementer
Preparation-worker wiring within G0's resource architecture, selector placement in the existing menu and status wording. Generation/identity rules and extra player controls are not delegated.

## Must stay green
One generator/compiler/resolver, immutable replay identity and no generation work in active simulation/render loops.

## Feedback that would change this slice
Rejected startup latency or workflow reopens preparation/residency or menu composition, preserving the required runtime outcome.

## Outcome

A player starts a battle on a generated map from the main menu, and it plays in the village's battle view. The developer route `/lab/generated` is gone: `/battle` is the one route, and the `generated` scene drives it from the menu. The decisions the spec left open are in the [choices ledger](../choices.md).

**The seam.**

```text
PrepareBattleRequest { map_source, recipe_id, encounter_seed, battle_seed }
  map_source     = { kind: "catalogue", id } | { kind: "generated", request: GenerationRequest }
  recipe_id      a recipe of fixtures/encounters.json (generated map), or the map's saved encounter of that name (catalogue map)
  encounter_seed canonical u64 decimal text
  battle_seed    a whole number, at most 2^53 - 1

PreparedBattle { scenario, report }
  scenario  the ScenarioDefinition JSON any battle authority runs
  report    { request, identity: MapIdentity, size, counts, planned: { recipe_hash, encounter_seed, placement } | null,
              objective: { center, radius_m, hold_s } | null, start: { at, yaw }, timings: { map, encounter }, wasmBytes }
```

- **Rust** owns the request's shape and its check. `contract::generation` holds `GenerationRequest`, `MapType`, `MapSize` and `CompileLimits` (moved from `mapgen`, which re-exports them), `contract::maps::MapSource` gains `Generated { request }`, and `contract::preparation::PrepareBattleRequest::from_json` refuses a seed that is not canonical decimal text, a battle seed above 2^53 - 1, an encounter name or catalogue id that is not one address, and an unknown field. The Wasm export `check_prepare_request` is that check.
- **The one map owner in JavaScript** is `web/src/maps/source.ts`: `resolveMap(source, access)` answers a catalogue id through the adapter's `loadMap` and a generation request through the simulation's `generate_map`, both as `{ definition, identity, json, sites }`. `generationRequest` pins a player's choice to this build's generator version, preset revision and catalogue hash; `canonicalSeed` and `newSeed` keep a seed as text.
- **Preparation** (`web/src/battle/prepare/`) is `prepare(wasm, memory, request, documents, saved)`: check the request, resolve the map, lay the encounter (the simulation's planner for a recipe on a generated map; the saved encounter on a catalogue map), splice the scenario. The worker runs it; `prepareBattle(message, onStage)` on the page is one worker per request, and `cancel()` closes the worker and silences it. A refusal names its stage: `request`, `map` or `encounter`.
- **The route** `/battle` reads its address (`apps/battle-lab/src/battleLinks.ts`): `?type=&size=&seed=` (and optionally `recipe`, `encounter`, `battle`) for a generated map, `?map=<id>&recipe=<name>` for a saved one, `?replay=saved` for a saved replay. The address is the share identity.

**The menu's states.** `Skirmish` holds the map type (Open, Mixed, Metro), the size (Small, Medium, Large) and the seed, which opens on a fresh draw, is editable, and is redrawn by `New seed`. While the seed is not a u64 the field says so and `Deploy` leads nowhere; no seed is put in its place. `Deploy` opens `/battle?…`: a loading screen names the map and the stage (generating the map, placing forces, building the battlefield, starting the battle) with `Cancel`, which returns to the menu on the same choice. A refusal replaces the stages with what refused (the request, the map, the encounter), says the seed was not changed, and keeps the diagnostics behind `Details`. Leaving the page closes the worker, and a cancelled request's answer is never delivered, so no stale battle starts.

**Replay.** A prepared battle's saved file is `{ battle: { scenario, report }, replay }`: the exact scenario bytes retain its compiled map, prepared encounter and complete rules/catalogue input, while the preparation report retains request, physical identity and seeds. The simulation's `Replay` retains engine identity, scenario/config digests, battle seed and accepted commands. Playback builds the stored world through the existing preparation worker and reuses its replay constructor; it never reads current generation presets, recipes or saved map documents. The page's physical WorldView also uses the stored scenario rules. Current validated appearance assets remain outside simulation identity. The constructor refuses different engine, scenario or rules identities, and no cross-build compatibility or request-only replay fallback exists.

The last saved/imported replay occupies one IndexedDB record, replaced transactionally: a compiled large map exceeds localStorage's quota. Imports wait for the write before restarting or navigating, and failed writes leave the previous replay intact and report the error. Pending reads use the existing loading screen; the menu keeps its replay link disabled until the stored file is known, and the village viewer builds no default scenario during that wait. Download remains available if browser persistence fails.

**Startup**, one measurement each on this Mac, development build, from pressing Deploy (the battle page's navigation) to the first drawn frame and to the first observation:

| Map (seed 1) | Buildings | Map resolved | Encounter planned | Prepared | First frame | Playable |
|---|---|---|---|---|---|---|
| Mixed Small | 3,111 | 0.26 s | 0.50 s | 1.39 s | 3.11 s | 3.14 s |
| Mixed Medium | 3,957 | 0.33 s | 0.62 s | 1.54 s | 3.43 s | 3.46 s |
| Mixed Large | 6,422 | 0.48 s | 0.99 s | 2.09 s | 4.29 s | 4.33 s |
| Open Small | 758 | 0.09 s | 0.45 s | 1.12 s | 2.94 s | 2.97 s |
| Metro Small | 2,378 | 0.23 s | 0.29 s | 1.14 s | 2.68 s | 2.71 s |
| Metro Large | 9,836 | 0.78 s | 1.29 s | 2.67 s | 5.01 s | 5.06 s |

`STARTUP_MAP=type:size:seed bun run --cwd web scene -- generated` repeats one.

**Proof.**

- `crates/contract/tests/preparation.rs`: seeds above 2^53 survive the JSON boundary as text, and each kind of bad request is refused at its field.
- `web/tests/prepareBattle.test.ts`: a generated request prepares the requested map with a legal planned encounter that runs; map seed 2^53 + 1 and encounter seed 2^64 - 1 arrive as written; the request check, the generator (limits, and a request pinned to another generator) and the planner each refuse at their own stage; a catalogue map plays its saved encounter through the same function.
- `web/tests/battleStart.test.tsx`: the seed parser, the address round trip, the menu's deploy link for each choice and for a seed that is not one, and a cancelled or replaced preparation whose late answer is never delivered.
- The `generated` scene starts from the main menu (type, size, `New seed`, a typed seed, Deploy), checks the loading screen and its cancel address, the map the battle runs on, the existing drawing and camera checks, an ordered jeep driving up the road, a saved replay reaching the played battle's digest at the same tick, a replay from another engine build being refused, and a bad address being refused.
- One unprimed `screenshot-critique` of the menu, loading and refusal screens. Fixed from it: the chosen option is now the lit one and the others dim, the seed field's focus no longer looks like a chosen option, the note under the fields is the map type's alone, the refusal's title is in the failure colour with its `Details` under the way out, a bad address is "This link does not name a battle", and the section is `Skirmish` under the title `Battle`. Left standing: the menu is a small plate on an empty background with labelled rows, which the critique reads as a settings form (it was a plate of two cards before; a menu over a map preview is a look decision for the visual pass); the sound row's checkbox and slider; `Play village` and `Deploy` describe themselves in near-identical words.

**Replay closeout proof.** A focused simulation-backed test saves a prepared battle, changes the current rules fixture, demonstrates that regeneration refuses the old replay, then restores the captured scenario to the original digest. A real headless-browser worker test retains a 48 MiB scenario across the browser storage boundary and replays through the same preparation worker to the live digest; storage refusal cannot start a different replay. A page-world regression proves captured physical catalogue types reach WorldView rather than the current fixture catalogue.

**Current delivery.** Saved catalogue and runtime generation share the player acquisition path, and captured compiled-scenario replay is integrated. Final resource and integrated admission judgments remain with the current closeout evidence. The focused 48 MiB persistence proof establishes byte retention and replay restoration, not all-large-map browser resource admission. Historical startup/UI checks above retain their original scope.
