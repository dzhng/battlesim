# Scenario builder

## Status and next pickup

**Status:** planning complete; implementation has not started. The exploration interview and the write-spec interview are closed. The plan is ready for slice 00.

**Updated:** 2026-10-11.

**Next pickup:** implement [`slices/00-side-topology-spike.md`](slices/00-side-topology-spike.md). Do the end-to-end topology probe through the real contract, native/WASM battle, publication, browser decoder, controllers, and replay path before committing the production side model. Record the verdict and measured owner inventory in [`choices.md`](choices.md), then update this section before ending the pass.

**Active warning:** the current simulation is deeply fixed to Blue/Red. The user asked for a real arbitrary-side/FFA spike, not a paper estimate. Do not build the canvas or a compatibility bridge before that verdict.

## Goal

Build a developer-first tactical scenario builder that creates a self-contained downloadable scenario package. A user can upload that package through the normal runtime and play it without repository fixtures or generator lookups.

The first acceptance run authors a scenario on a tactical canvas, places units owned by distinct player slots, downloads it, reloads in a fresh browser context, uploads it, starts play with one active human at most, switches a human slot to active AI, leaves another human slot empty so it runs passive AI, verifies side-scoped observation, and confirms deterministic replay identity.

The scenario package may contain user assets such as audio. Those bytes, hashes, media types, and provenance travel with the export. Future campaign work may promote selected packages into code-owned content; campaign import is outside this feature.

## Scope decisions

- **Authoring:** tactical canvas first; common fields in the browser; advanced scripts hand-authored.
- **Persistence:** explicit Download/Upload only. No IndexedDB, localStorage, filesystem workspace, or checked-in scenario fixtures.
- **Runtime:** uploaded packages use the normal preparation/worker/BattleView path. This is local single-human control, not network multiplayer.
- **Ownership:** every unit belongs to one player slot. Controllers can command only their slot's units. Same-side knowledge may be shared separately from command authority.
- **Controllers:** at most one active human; human slots may switch to active AI; empty human slots use passive AI; AI-only slots remain active AI.
- **Sides:** run the arbitrary-side spike first. The fallback production model is Blue, Red, and passive Neutral. Keep arbitrary side ids/FFA only if the spike proves the generalization is a clean, maintainable owner shape through the full acceptance path.
- **Modes:** explicit Sandbox and optional Economy modes. Sandbox has authored forces and no wallet/purchase loop. Economy reuses the existing economy/objective owner after the topology verdict.
- **Neutral:** does not initiate combat, but returns fire and defends itself when threatened.
- **Advanced behavior:** authored sandboxed code with deterministic host APIs, bounded execution, no ambient I/O/time/network/DOM, and ordinary command/event emission through the authority.
- **Compatibility:** hard cutover. No migration framework, compatibility readers, or old-format shims.

## Why these owners are load-bearing

The simulation owns truth and observations. `ScenarioDefinition` already composes resolved map, rules, units, events and scripts (`crates/contract/src/scenario.rs`), while `PreparedWorld::from_scenario` can already construct a battle from resolved scenario text (`crates/game-wasm/src/lib.rs`). `Battle::observe` and publication filtering are the visibility boundary (`crates/sim/src/battle.rs`, `crates/sim/src/publication.rs`). Replay hashes the setup and records accepted commands (`crates/sim/src/battle.rs`).

The existing map workbench tunes generator documents and publishes fixtures (`apps/map-workbench/README.md`); it is not the owner of portable scenario packages. Map JSON and resolver bytes must remain owned by map contracts (`web/src/maps/resolve.ts`, `crates/contract/src/map.rs`). Appearance and audio remain owned by `packages/scene-assets` and `packages/battle-audio`; the bundle carries references to built-in content hashes and embeds user-supplied media through one asset manifest.

The current binary assumptions are explicit in `crates/contract/src/ids.rs`, `crates/contract/src/preparation.rs`, `crates/contract/src/skirmish.rs`, `web/src/battle/sim/protocol.ts`, and the fixed arrays in `crates/sim/src/battle.rs`. The first slice measures all of those owners instead of introducing a second parallel topology.

## Slice graph

```text
00 side topology spike
   ├── 01 canonical bundle + deterministic asset container
   │      └── 02 offline bundle admission and prepared-world startup
   ├── 03 player slots + controller ownership
   │      └── 04 active/passive AI and takeover
   ├── 05 deterministic authored-code sandbox spike
   │      └── 06 sandbox/economy/victory integration
   ├── 02 + 03 + 04 + 06
   │      └── 07 tactical canvas authoring and explicit file I/O
   │             └── 08 normal upload/play route and embedded audio
   │                    └── 09 full acceptance, replay, visual closeout
```

Slice 00 is a kill/keep gate. Slice 01 must not invent a second scenario schema. Slice 05 must freeze the script host before the UI exposes advanced source. Slices 07–09 are the only browser-heavy slices and each leaves a runnable checkpoint.

## Contract table

| Contract | Owner | First proof |
|---|---|---|
| `ScenarioBundle` canonical bytes, hashes, limits, diagnostics | `crates/contract` plus one TypeScript codec at the browser boundary | native/WASM round trip and refusal matrix |
| Resolved map snapshot and physical dependencies | map contract/resolver | upload resolves without generator or fixture lookup |
| Player slot, unit ownership, side identity, controller admission | contract + simulation | duplicate/unowned/one-human rules and command validation |
| Side-scoped observation/publication | simulation publication + worker/client decoders | pairwise hidden-state test with 3+ observers |
| Controller execution and replay | `sim::Battle` controller host | takeover/passive ticks replay identically |
| Deterministic script host | simulation-owned sandbox seam | two fresh runs and command-log replay match |
| User asset bytes and playback | bundle asset owner + `packages/battle-audio` | corrupted/missing/hash mismatch refusal and one played clip |
| Tactical canvas state | dedicated battle-lab scenario builder route | place/select/edit/download/upload screenshot checkpoint |

## Verification rules

Every slice has one contract, one runnable artifact, and focused tests at its owner. Use the narrowest check that answers the slice question; run `bun run check` and `bun run verify` only after slice 09. Native/WASM pairs must compare the same canonical bytes and digest. No test may re-derive map geometry or visibility independently of the owner that computes it.

Any slice that produces a visual shot must:

1. load `game-ui` before changing player-facing UI;
2. use `compare-screenshots` when a prior or reference target exists, naming the one visual variable and crop/mask;
3. run unprimed `screenshot-critique` as the last visual check;
4. open captures with `preview-shots` for a non-blocking human review window of about five minutes, then record the decision and close the shots if no response arrives.

## Scope firewalls

- No network multiplayer, server authority, rollback, or concurrent human session.
- No campaign progression or automatic promotion into repository fixtures.
- No direct map geometry editor; map editing is a later feature.
- No hidden browser persistence.
- No arbitrary JavaScript evaluation, browser APIs, wall clock, network, filesystem, or unbounded allocation in scripts.
- No appearance authoring or mutation of the shared audio catalog.
- No compatibility shim or migration framework.
- No renderer-side visibility inference; the simulation publication remains the sole observation authority.
- No second owner for map resolution, scenario serialization, controllers, audio playback, or replay identity.

## Known unknowns carried into implementation

- The side-cardinality verdict is intentionally deferred to slice 00.
- The bundle container (canonical JSON/base64 versus deterministic binary container) is chosen by slice 01's size and browser-admission probe; the semantic contract is fixed first.
- The sandbox runtime and host API are chosen by slice 05's portable deterministic probe; a failed probe removes authored code from the shipped runtime rather than adding an unsafe fallback.
- Economy semantics with arbitrary sides are resolved only after slice 00; unsupported combinations are refused explicitly.
- Built-in appearance/catalog dependencies may be referenced by content hash; user media must be embedded. Exact asset size/type limits are set by slice 01/02 measurements and recorded in `choices.md`.

## Global TODO checklist

- [ ] Run and record the side-topology verdict — [`slices/00-side-topology-spike.md`](slices/00-side-topology-spike.md).
- [ ] Freeze the self-contained bundle and asset container — [`slices/01-bundle-contract.md`](slices/01-bundle-contract.md).
- [ ] Prove offline map/catalog/audio admission — [`slices/02-offline-admission.md`](slices/02-offline-admission.md).
- [ ] Materialize player-slot ownership — [`slices/03-player-slots.md`](slices/03-player-slots.md).
- [ ] Implement active/passive controllers and deterministic takeover — [`slices/04-controllers.md`](slices/04-controllers.md).
- [ ] Freeze and prove the authored-code sandbox — [`slices/05-script-sandbox.md`](slices/05-script-sandbox.md).
- [ ] Integrate Sandbox/Economy and script-defined victory — [`slices/06-modes-and-victory.md`](slices/06-modes-and-victory.md).
- [ ] Build the tactical canvas and explicit file I/O — [`slices/07-builder-canvas.md`](slices/07-builder-canvas.md).
- [ ] Add normal upload/play and embedded media playback — [`slices/08-runtime-upload-play.md`](slices/08-runtime-upload-play.md).
- [ ] Pass the mixed-player/replay/visual closeout — [`slices/09-acceptance.md`](slices/09-acceptance.md).

## Next Agent Prompt

You are picking up the scenario-builder implementation plan. Read this README and `choices.md`, then implement slice 00 only. Build a real, disposable topology/controller probe through the existing contract → native/WASM Battle → side-scoped publication → browser decoder → replay path. Include Blue/Red/Neutral and a 4+ side FFA candidate. Inventory every fixed-two owner, run hidden-state isolation, per-player command ownership, passive neutral/empty-slot behavior, active AI takeover, and deterministic replay. Do not build the editor, bundle format, or compatibility bridge yet. Record the verdict, evidence, rejected branch, and exact next pickup in `choices.md` and this README before ending the pass.

