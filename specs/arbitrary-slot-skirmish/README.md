# Arbitrary-slot skirmish lobby

**Status:** planned; implementation has not started.
**Updated:** 2026-10-16.

## Next Agent Prompt

Start with slice 01. Read this README and its slice file, inspect the named owners, write red tests first, and update this section before ending your pass.

- [ ] 01 — slot contract and validation
- [ ] 02 — shared spawn groups and size admission
- [ ] 03 — lobby model, URL identity, and preview lifecycle
- [ ] 04 — desktop RTS lobby and generated 2D preview
- [ ] 05 — battle integration and closeout

## Goal and settled decisions

Build a desktop RTS lobby with Blue on the left, a real generated top-down map in the center, and Red on the right. Each side has one to five player slots. Presets seed 1v1, 3v3, or 5v5; Add/Delete creates asymmetric setups such as 3v4 or 2v3. Every slot is a player with a controller, independent wallet, roster, and spawn-group assignment. Single-player starts with exactly one Human slot on Blue and AI elsewhere; selecting Human in another Blue row transfers the role and changes the former row to AI. The reusable simulation setup must still represent all-AI benchmark and tournament matches.

The current game selects one faction per side. Slot data carries faction per slot so future custom and multiplayer modes can allow mixed factions; current preparation rejects mixed factions. Faction and slot configuration are admin-owned. Generated preview controls regenerate automatically, Reroll changes only the map seed, failures retry internally, and Deploy commits the exact admitted seed. Preview shows generated terrain, roads, water, forests, buildings, objectives, and up to three shared spawn groups per side. Ground uses the nearest legal road point; aircraft use existing `air::edge` derivation from the selected entry.

The explicit size-floor table is: one active slot → Small; two → Medium; three or four → Large; five → XL. The user chose Medium for 2v2.

This is a hard cutover for generated skirmishes: no compatibility shim or migration for old two-faction URLs, fixtures, or setup. Existing replay compatibility remains only where the independent replay contract requires it.

## Ownership and seams

`contract` owns canonical slot/setup validation. Reuse existing command/unit ownership seams (`CommandEnvelope.slot`, `UnitSetup.owner`, controller transitions); do not reuse the authored-bundle `PlayerSlot` type directly. Extract shared slot/controller vocabulary only if both owners can use it without a second concept. `mapgen` owns geometry and reserved sites. `sim` owns placement, wallets, rosters, AI turns, observations, and digests. `web` owns preview lifecycle and URL identity. `battle-lab` owns presentation. The preview never invents geometry and the UI never reimplements placement legality.

Required invariants: five-slot maximum per side; no empty deployed side; deterministic slot ordering; one faction per side in current preparation; one Human on Blue only in the single-player request; one to three shared spawn groups per side; active-slot size floor; exact seed identity; bounded hidden retry; stale preview results cannot replace current state.

## Slice graph

### 01 — Slot contract and validation

Update `crates/contract/src/skirmish.rs`, `preparation.rs`, relevant scenario ownership types, and simulation setup/publication. Replace `[Faction; 2]`, `ai_side`, and side-only economy ownership with validated ordered slot rows. Add controller, faction, wallet/roster, five-slot, duplicate-ID, and deterministic serialization tests. Keep side objective ownership derived. Do not enforce the single-player human rule in reusable simulation setup.

### 02 — Shared spawn groups and size admission

Update `crates/contract/src/encounter.rs`, `crates/mapgen/src/skirmish.rs`, map analysis, air-edge use, and preparation. Generate new edge-road arms for one to three groups per side; validate spacing, fairness, reachability, and finite geometry for every entry. Each slot references a shared group. Enforce the explicit size table at admission. Add focused mapgen, contract, air, and preparation tests.

### 03 — Lobby model, URL identity, and preview lifecycle

Add a pure lobby model under `apps/battle-lab/src/skirmish/`; update `MainMenu.tsx`, `battleLinks.ts`, and preparation orchestration. Implement presets, Add/Delete, human transfer, side faction controls, size bumping, exact map/deploy seeds, stale cancellation, and bounded hidden retries. Deploy consumes only the admitted preview. Test reducer/model behavior and URL round trips.

### 04 — Desktop lobby and generated preview

Build the Blue/map/Red RTS layout. Render actual generated geometry and admitted sites in lightweight SVG/canvas, with loading, retrying, ready, Reroll, Add/Delete, controller, faction, and Deploy states. Use `game-ui` before player-facing changes. Visual variable: desktop column balance, map legibility, marker hierarchy, and lobby state clarity; mobile polish and battle HUD are out of scope. Add browser scenes and run `compare-screenshots`, `screenshot-critique` last, and `preview-shots`.

### 05 — Battle integration and closeout

Expose the active human slot wallet and roster, route AI slot purchases through simulation, update publication/HUD and fixtures, remove old binary assumptions, run asymmetric battle scenes and digest/replay checks, then run the full Rust/browser suite once and archive with `close-spec`.

## Browser checkpoints

Register a lobby scene proving default 1v1, 3v3/5v5 presets, 3v4 and 2v3 Add/Delete, human transfer, side faction restriction, preview on open and change, Reroll, hidden retry, size bumping, exact Deploy identity, and shared spawn markers. Visual slices must use `game-ui`, `compare-screenshots`, `screenshot-critique`, and `preview-shots`; review is non-blocking.

## Choices ledger

- Arbitrary slots, maximum five per side; presets are conveniences.
- Per-slot wallet and roster; one faction per side in the current game, per-slot faction reserved for future modes.
- Automatic generation, explicit Reroll, hidden retry, exact Deploy.
- Blue/map/Red desktop RTS composition.
- Hard cutover, no generated-skirmish migration layer.
- User chose 2v2 floor: Medium.
