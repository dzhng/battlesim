# Scenario builder choices ledger

This ledger carries decisions from the exploration and write-spec interviews. User decisions are marked **user**. Repository findings are marked **territory**. Open items name their unblock.

## Closed decisions

| Decision | Result | Source and reason |
|---|---|---|
| First slice | Saved developer scenario that can be played | **user**; campaign, multiplayer, and balance/testing all need a reusable artifact |
| Builder workflow | Tactical canvas first | **user**; exact placement and inspection are the center of authoring |
| Primary author | Developer first | **user**; optimize iteration speed, coordinates, diagnostics, reproducibility |
| Authoring surface | Hybrid | **user**; common fields in UI, advanced scripts hand-authored |
| Runtime use | Normal runtime upload/play; one active human at most | **user**; full network multiplayer is deferred |
| Persistence | Explicit Download/Upload only | **user**; no hidden local drafts or checked-in builder scenarios |
| Export | Self-contained package | **user**; resolved map, scripts, and user assets such as audio travel with it |
| Future campaign | Later campaign feature may import selected packages into the codebase | **user**; not part of this builder |
| Map | Resolved compiled snapshot | **user**; no regeneration controls in the saved artifact |
| Battle modes | Sandbox and optional Economy | **user**; testing needs authored forces, future scenarios need economy |
| Victory | Script-defined | **user**; campaign and specialized testing need arbitrary conditions |
| Controllers | Per-player ownership; at most one active human; human→AI takeover; empty human→passive AI; AI-only→active AI | **user**; multiple slots exist without full multi-human networking |
| AI | Per-player adaptive, observation-bound | **user**; faction, terrain, relations/side, force composition, and allies should influence behavior |
| Neutral | Passive; no aggression, but return fire/self-defense | **user** |
| Default side model | Blue/Red/Neutral fallback | **user**; arbitrary sides are a spike, not an assumed implementation |
| Arbitrary sides | Slice 00 spike rejected dynamic FFA for this feature; production is fixed Blue/Red/Neutral | **user + territory**; the real path reaches fixed-two state and requires a cross-owner rewrite, so no unbounded abstraction is carried forward |
| Compatibility | Hard cutover, no migrations or shims | **user** |
| Bundle location | Portable export, not a codebase fixture; earlier saved-map-folder choice superseded | **user**; scenario files are downloaded/uploaded, future campaign promotion is explicit |

## Territory givens

- Simulation, not the UI or renderer, owns truth and visibility (`README.md`, `crates/sim/src/publication.rs`).
- Current `Side`, preparation, skirmish, publication, and battle state are fixed to two combat sides (`crates/contract/src/ids.rs`, `preparation.rs`, `skirmish.rs`, `crates/sim/src/battle.rs`).
- Resolved map bytes preserve exact resolver output and physical identity (`web/src/maps/resolve.ts`, `crates/contract/src/identity.rs`).
- `PreparedWorld::from_scenario` can be the upload/start seam (`crates/game-wasm/src/lib.rs`).
- Replay records accepted commands and refuses build/scenario/config mismatches (`crates/sim/src/battle.rs`).
- Appearance and audio have existing owners (`packages/scene-assets`, `packages/battle-audio`); the bundle must not create parallel playback/catalog owners.

## Superseded choices

- **Standalone `fixtures/scenarios/<id>/`** was initially selected during the write-spec interview, then superseded when the user clarified that builder artifacts are downloadable/uploadable and not checked into the codebase. Future campaign import is a separate promotion path.
- **Static teams** and then **pairwise relations** were explored, then superseded by the user's simpler side model. The production contract does not carry alliance matrices unless the side spike proves a need.
- **Generated source plus frozen identity** was explored, then superseded by resolved snapshot-only persistence.

## Open decisions

| Open item | Unblock |
|---|---|
| Arbitrary side count versus Blue/Red/Neutral | Slice 00 end-to-end owner inventory and acceptance evidence |
| Bundle physical container | Slice 01 browser/native size and admission probe |
| Script runtime and host API | Slice 05 deterministic portability/fuel probe |
| Economy behavior after side verdict | Slice 06 against the accepted topology |
| Asset limits and built-in catalog references | Slices 01–02 measured startup/memory/admission evidence |

## Slice 00 evidence

- The committed native test `side_topology::existing_binary_path_proves_observation_and_replay_baseline` proves the current two-side observation and replay digest path.
- The committed refusal test and `side_topology_probe` report show the current contract rejects a four-side candidate with `unknown variant 'green', expected 'blue' or 'red'`.
- A disposable worktree added `Neutral` plus the smallest compile fixes in encounter, movement cover, and route planning. `cargo check -p sim` then passed, but constructing a Blue/Red/Neutral battle panicked at `crates/sim/src/battle.rs:2448` because `sightings` remained length two.
- The inventory found 172 side-indexed/fixed-two owners in the Rust simulation path before browser/UI consumers. Skirmish economy, objectives, publication layouts/readers, worker protocols, and UI types add further binary contracts.
- Verdict: implement a deliberate fixed three-slot model for this feature; arbitrary side ids/FFA are a later architecture project, not a hidden fallback.

## Slice 00 implementation choices

| Choice | ELI5 decision audit | Verdict |
|---|---|---|
| Keep the topology evidence as a native example plus one integration-test module | The probe runs the same `ScenarioDefinition` → `Battle` → side observation → replay path that the runtime owns. The example prints the refusal and digest evidence for a human or later agent; the test keeps the two-side baseline and four-side refusal from silently disappearing. A throwaway worktree was used for the attempted Neutral extension, so no broken experimental code entered the product tree. This gives future work a small executable witness instead of relying on a paragraph claiming the architecture is binary. | **sound** — one authority path, no second runtime implementation |
| Reslice before bundle work | The original plan said “Blue/Red/Neutral” after the spike, but the current code had no working Neutral slot. If bundle serialization started first, it would freeze a file format that the battle could not load. The plan therefore inserts `00b-fixed-neutral-slot` as the owner that must make the three-slot runtime real before `ScenarioBundle`. | **sound** — prevents a portable artifact that cannot be played |

## Slice 00b implementation choices

| Choice | ELI5 decision audit | Verdict |
|---|---|---|
| Use one fixed three-entry side owner across simulation and publication | Blue, Red, and Neutral now share the same indexed observation, fog, sighting, audible, navigation, movement, and publication arrays. This keeps one runtime shape and avoids a dynamic-side compatibility layer. | **sound** |
| Keep economy/encounter binary and refuse Neutral at their input boundary | Existing wallet, objective, entry-site, and encounter planners are two-side systems. Neutral sandbox units remain valid, while economy commands, Neutral AI-side setup, and Neutral encounter attackers receive named refusal diagnostics before any two-entry array is indexed. | **sound** |
| Force Neutral to `ReturnFireOnly` at spawn | A Neutral authored slot can defend itself after an incoming attack, but cannot start automatic aggression or override the passive-neutral rule with FireAtWill. | **sound** |
| Retain parity fixtures under the named topology change | Adding a third observation slot changes battle digests and publication stream identity even for binary scenarios. The native parity records were regenerated under the intentional fixed-topology decision. | **sound** |
| Mirror Neutral through the worker/client side type while keeping binary UI surfaces binary | The shared `SideName` accepts `neutral`, authority side switching can publish it, and economy/result widgets remain explicitly Blue/Red because binary skirmish refuses Neutral. | **sound** |
