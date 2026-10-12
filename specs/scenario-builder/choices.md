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

## Slice 01 implementation choices

| Choice | ELI5 decision audit | Verdict |
|---|---|---|
| Use canonical JSON with base64 for the portable container | A download is ordinary text whose field order and embedded bytes are fixed. Upload parses it, validates hashes and topology, then requires the exact same bytes to serialize again. A binary archive would be smaller for large media, but the measured first artifact is bounded at 32 MiB and the JSON boundary keeps the browser and WASM owners aligned without a second parser. | **sound** — one strict boundary and byte-for-byte identity |
| Keep resolved `ScenarioDefinition` as the bundle's scenario payload | The bundle carries the map and rules already resolved by their existing contract owner, so an uploaded file can start a battle without asking a generator or fixture directory to recreate them. Rebuilding a parallel map/rules schema in the builder would let the two formats drift. | **sound** — preserves one owner for physical inputs |
| Index unit ownership through sorted player-slot unit lists | Each authored unit appears in exactly one slot list, and admission rejects duplicates, missing owners, side mismatches, and unsorted ids. A separate owner field on every unit would be more direct, but would duplicate the existing unit setup schema before the simulation command owner exists. | **needs-user** — provisional call is to keep slot-owned lists through slice 03, then move ownership into the authoritative command admission seam if that removes duplication |
| Make bundle upload require exact canonical JSON bytes | A file with the same meaning but different whitespace/order is refused before play, so bundle identity is stable across downloads and replay/config hashes. A normalizing reader could be friendlier, but would make two byte strings represent the same durable artifact. | **sound** — deterministic identity is the feature's reproducibility contract |
| Embed user media and allow hash-named built-in references | Uploaded audio travels inside the file and is checked against SHA-256 before admission; built-in content is represented by a named immutable reference and its hash. A bundle never stores a local filesystem path. | **sound** — self-contained user files with explicit external content identity |

## Slice 05 implementation choices

| Choice | ELI5 decision audit | Verdict |
|---|---|---|
| Use a small contract-owned interpreter instead of JavaScript/WASM eval | Authored scenario code needs conditions, bounded repetition, observation queries, and events, but browser evaluation would expose ambient capabilities and make native/WASM parity fragile. `CompiledScript` parses a closed expression language and emits only ordinary `Order`s, events, and a victory signal. The simulation remains the owner that admits and applies those orders. | **sound** — portable and auditable execution boundary |
| Make observation access explicit and side-scoped | The host takes only `(side, tick, ObservationFrame, seeded RNG value)`. Scripts can read scalar counts, visible unit positions/ids, and deterministic draws; they cannot name `Battle`, inspect hidden units, mutate rules, or access a clock or I/O. Indexed observation reads refuse outside the published list. | **sound** — visibility cannot be widened by script code |
| Bound all work and output with deterministic refusals | Every statement/expression consumes fuel; `repeat` has an iteration cap; variables, command output, and event output have fixed capacities. Exhaustion returns a named `ScriptRuntimeError` with a source span and instruction count. There is no silent fallback to a different controller. | **sound** — hostile or accidental loops cannot stall a battle |
| Include runtime version in script identity | A source hash alone would let a semantic interpreter change reuse old scenario/replay identities. The identity hashes `scenario-script-1` together with the source hash, and bundle admission compiles victory and authored scripts before accepting canonical bytes. | **sound** — semantic changes require a new durable identity |
| Keep script integration at the existing command/replay seam | Slice 05 proves the language and host in the shared contract crate; it does not add a parallel battle loop. Slice 06 will run the compiled program at authority tick boundaries, submit emitted commands through normal admission, and disable script execution while replay feeds accepted commands. | **sound** — one simulation authority and one replay log |

## Slice 07 implementation choices

| Choice | ELI5 decision audit | Verdict |
|---|---|---|
| Put the first builder at `/scenario-builder` in the battle lab | The route is a developer tool beside the existing workbenches, with the same catalog scope and HUD theme. It does not become a player-facing campaign page or a second app shell. | **sound** |
| Use an SVG tactical canvas for the first runnable checkpoint | Unit placement and inspection are the authoring question; a lightweight SVG makes coordinates, selection, and deterministic screenshots easy to inspect without duplicating the GPU renderer or inferring simulation state. The resolved map remains an opaque bundle payload; map geometry editing is still future work. | **sound** |
| Keep the editor state as a `ScenarioBundleDraft` that mirrors the canonical envelope | Slots, ownership, controllers, mode, victory source, scripts, and embedded assets are edited in one in-memory object. Download and upload are the only durable boundary; the editor does not add local storage or a parallel scenario file format. | **sound** |
| Let the Rust/WASM owner canonicalize and validate exports | Browser code can place units and edit common fields, but it does not reimplement map/rules hashes or topology admission. Uploads are refused atomically through `check_scenario_bundle`; a separate authoring entry computes identities for drafts before download. | **sound** |
