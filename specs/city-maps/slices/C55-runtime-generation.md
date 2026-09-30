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
- Preparation time/peak memory/downloads within G0 budgets; input/render remain responsive.
- Replace/cancel a pending request and switch maps without stale battle startup or leaked resources; bounded failure reports retain the requested seed.
- Same-build replay uses the stored compiled scenario/rules and matching digests; changed engine build or physical/config identity refuses clearly. A compatible art-only rebake leaves simulation replay identity unchanged; native/wasm battle parity holds.
- Menu/control/status variable only: compare with the existing menu's design conventions via compare-screenshots, run unprimed screenshot-critique last, and preview-shots non-blocking. Whole map appearance remains C54/C51's integration evidence.

## Delegated to the implementer
Preparation-worker wiring within G0's resource architecture, selector placement in the existing menu and status wording. Generation/identity rules and extra player controls are not delegated.

## Must stay green
One generator/compiler/resolver, immutable replay identity and no generation work in active simulation/render loops.

## Feedback that would change this slice
Rejected startup latency or workflow reopens preparation/residency or menu composition, preserving the required runtime outcome.
