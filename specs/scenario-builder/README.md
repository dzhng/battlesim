# Scenario builder

## Status and next pickup

**Status:** the side topology, bundle contract, offline admission, slot ownership, controller takeover, deterministic script runtime, modes, tactical builder, explicit scenario/replay file I/O, normal upload/play route, bundle-owned catalog admission, embedded audio seams, and renderer allocation fix are implemented. The uploaded package now reaches a real playable publication, supports AI takeover, keeps observation side-scoped, and round-trips a downloaded replay with a matching digest. Native contract checks, TypeScript checks, focused tests, builder/play scenes, and the mixed-player browser probe pass.

**Updated:** 2026-10-12.

**Next pickup:** none for this slice. Future work can add visual baselines and campaign promotion; keep the production topology at Blue, Red, and passive Neutral, with arbitrary side counts remaining a later architecture project.

**Topology verdict (2026-10-11):** keep a fixed Blue/Red/Neutral model for this feature. The probe ran the real scenario → Battle → observation/replay path, then added a disposable Neutral variant and the minimum compile adaptations. `sim` compiled only after changing side-owned arrays in movement/route planning; a Neutral battle then panicked at `battle.rs:2448` indexing fixed-two sightings. The inventory found 172 side-indexed/fixed-two owners before the browser UI, plus fixed skirmish/objective/publication assumptions. Arbitrary FFA is deferred; no dynamic-side compatibility layer is carried forward.

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
- **Sides:** fixed Blue, Red, and Neutral. Neutral is a first-class passive slot; arbitrary side ids/FFA are deferred because slice 00 found fixed-two assumptions across simulation, publication, economy, and browser consumers.
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
   └── 00b fixed Neutral slot
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

Slices 00 and 00b are closed. Slice 00 records the fixed three-slot verdict; slice 00b materializes it before any bundle or UI work. Slice 01 must not invent a second scenario schema. Slice 05 must freeze the script host before the UI exposes advanced source. Slices 07–09 are the only browser-heavy slices and each leaves a runnable checkpoint.

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

- Arbitrary side counts/FFA are deferred after slice 00; the production contract owns exactly Blue, Red, and Neutral for this feature.
- The bundle container (canonical JSON/base64 versus deterministic binary container) is chosen by slice 01's size and browser-admission probe; the semantic contract is fixed first.
- The sandbox runtime and host API are chosen by slice 05's portable deterministic probe; a failed probe removes authored code from the shipped runtime rather than adding an unsafe fallback.
- Economy semantics with arbitrary sides are resolved only after slice 00; unsupported combinations are refused explicitly.
- Built-in appearance/catalog dependencies may be referenced by content hash; user media must be embedded. Exact asset size/type limits are set by slice 01/02 measurements and recorded in `choices.md`.

## Global TODO checklist

- [x] Run and record the side-topology verdict — [`slices/00-side-topology-spike.md`](slices/00-side-topology-spike.md); fixed Blue/Red/Neutral, arbitrary FFA deferred.
- [x] Materialize fixed Blue/Red/Neutral state and publication — [`slices/00b-fixed-neutral-slot.md`](slices/00b-fixed-neutral-slot.md).
- [x] Freeze the self-contained bundle and asset container — [`slices/01-bundle-contract.md`](slices/01-bundle-contract.md).
- [x] Prove offline map/catalog/audio admission — [`slices/02-offline-admission.md`](slices/02-offline-admission.md).
- [x] Materialize player-slot ownership — [`slices/03-player-slots.md`](slices/03-player-slots.md).
- [x] Implement active/passive controllers and deterministic takeover — [`slices/04-controllers.md`](slices/04-controllers.md).
- [x] Freeze and prove the authored-code sandbox — [`slices/05-script-sandbox.md`](slices/05-script-sandbox.md).
- [x] Integrate Sandbox/Economy and script-defined victory — [`slices/06-modes-and-victory.md`](slices/06-modes-and-victory.md).
- [x] Build the tactical canvas and explicit file I/O — [`slices/07-builder-canvas.md`](slices/07-builder-canvas.md).
- [x] Add normal upload/play and embedded media playback — [`slices/08-runtime-upload-play.md`](slices/08-runtime-upload-play.md).
- [x] Pass the mixed-player/replay runtime closeout; visual baselines remain optional because this route has no approved screenshot target — [`slices/09-acceptance.md`](slices/09-acceptance.md).

## Next Agent Prompt

You are picking up the scenario-builder acceptance closeout. Read this README and `choices.md`, run the registered builder/play scenes against a warm Vite server, and record whether the uploaded package reaches normal BattleView. The runtime proof is complete: controller selection/takeover, side-scoped publication, and explicit replay download/upload with matching digests are recorded in `throwaway/evidence/scenario-play/mixed-player.json`. Embedded audio remains owned by the existing seam and is covered by the bundle contract; do not add persistence or fixture lookup.
