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
| Arbitrary sides | Proper end-to-end spike; keep only if final complexity is acceptable | **user**; coding agent should find the correct answer by implementing the probe |
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
