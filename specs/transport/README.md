# Transport

**Status:** in progress. 01–08, 10 and 11 landed; 09 (helicopters) and 12 (voice packs) are in flight (Codex); 11b follows 09 (Codex, separate worktrees).
**Updated:** 2026-10-16.

Infantry squads ride in APCs, IFVs, light vehicles and helicopters. A carrier has 3, 8 or 16 seats. Squads board it in 1 s, ride hidden from the enemy, and get out on an explicit Unload order. When a carrier dies, its passengers bail out hurt, and some die. A building collapse now works the same way. The skirmish AI moves its infantry by transport.

[map.md](map.md) is the settled decision ledger (D1–D24) and the landmine cards (L1–L14) from the user walk on 2026-10-16. This README turns them into a build ladder. Where the drafting pass corrected the map, section "Corrections to the map" says so, and the map has been edited to match.

## Next Agent Prompt

You are implementing passenger transport. Start at the first unchecked slice below, read its file, and follow it. Before your first slice, read [map.md](map.md) (decisions and landmines), this README's contracts and invariants, `development.md` and the skills it names (`tweak-mechanics` for slices 04, 06–09; `game-ui` and `renderer` before slices 10–11; `write-tests` before any test).

**How the work runs (user, 2026-10-16):** Codex implements every non-visual slice (delegated with worktree isolation); the orchestrator reviews, integrates, and browser-verifies the visual slices 10 and 11 itself.

The user is not a blocker (2026-10-16: "Just do your best"). Every human checkpoint is non-blocking: open the shots with `preview-shots`, wait about 5 minutes, then decide on the evidence, record the call in [choices.md](choices.md), close the shots and continue. Record every decision the plan left to you in [choices.md](choices.md).

**Warnings:**
- Hard cutover. No backward compatibility, no `serde(default)` kept for old data, no migrations. Re-bless records and re-record captures instead.
- Battles without transport keep their digests, except the two named outcome changes: slice 04 (collapse becomes injury plus a death roll) and slice 05 (recon patrols drop to 3 soldiers). A digest change anywhere else is a bug.
- The main-menu reel must not change. Menu units extend test units (`fixtures/units/menu/units.json`), so never give an existing test unit seats; add new test carriers.
- Run the full check ladder once, at slice 15. Each slice runs only the narrowest rung that answers its question.

**Known environment gap:** `native/client/tests/audio.rs` fails until the audio clips are fetched from LFS (`assets/runtime/audio/clips`). It is unrelated to transport; fetch them before the closeout run.

Update this section, and tick the list, before you end each pass.

### TODO

- [x] [01](slices/01-crash-on-bridge-deck.md) A helicopter shot down over a bridge rests on the deck (L11). Skip if it stops being simple.
- [x] [02](slices/02-one-hold.md) One owner for a unit inside something: garrison becomes the building case of a general hold. No outcome change.
- [x] [03](slices/03-hold-and-refusal-contract.md) The observation, publication and refusal contract for holds. Captures are re-recorded once, here.
- [x] [04](slices/04-bail-out.md) The bail-out library, applied to building collapse (named outcome change).
- [x] [05](slices/05-recon-three.md) Recon patrols become 3 soldiers (named outcome change).
- [x] [06](slices/06-boarding.md) The carriage capability, boarding from both directions, and "on the field". Opens the transport lab.
- [x] [07](slices/07-unload.md) Unloading at a point, in place and queued; refused with no room.
- [x] [08](slices/08-carrier-destroyed.md) A destroyed carrier bails out its passengers; refunds take passengers along.
- [ ] [09](slices/09-helicopters.md) Helicopters board and unload from the low hover; a downed helicopter keeps its passengers until impact.
- [x] [10](slices/10-cards.md) The Seats row and passenger cards under their carrier.
- [x] [11](slices/11-controls.md) Right-click boarding, the board cursor, the Unload button and U key, the unload marker, refusal text.
- [ ] [11b](slices/11b-publish-unload-steps.md) The simulation publishes which queued step unloads; delete the browser's `unloadPoints.ts`. Captures are re-recorded once more.
- [ ] [12](slices/12-voices.md) Spoken lines for boarding, unloading and the new refusal.
- [ ] [13](slices/13-roster-seats.md) The roster's carriers get their seats.
- [ ] [14](slices/14-skirmish-ai.md) The skirmish AI pairs transports with squads.
- [ ] 15 Closeout: run the whole check ladder once, then `close-spec`.

## Slice graph

```
01 crash on deck ─────────────────────────────────────────────────────────────┐
02 one hold ── 03 contract ──┬── 04 bail-out ──┐                              │
05 recon three (any time)    │                 │                              │
                             └── 06 boarding ──┼── 07 unload ── 08 destroyed ─┴── 09 helicopters
                                               │        │                              │
                                               │        └── 10 cards ── 11 controls ── 12 voices
                                               └────────────────────────────── 13 roster seats ── 14 AI ── 15 closeout
```

- 05 is data only; land it whenever convenient before 13.
- 10–12 (the browser) can run in parallel with 08–09 (the sim) once 07 has landed.
- 13 needs 06–09, because the roster rules only allow a carriage claim once its behaviour tests exist (`specs/done/unit-roster/capability-admission.md:38`).
- 14 needs 13 (real carriers to buy) and 07 (unloading).

## Contracts

The interfaces between components, in their end state. Each slice says which part it introduces.

### Catalog (`crates/contract/src/catalog.rs`)
- `Capabilities.carriage: Option<Carriage { seats }>`. `seats` admits only 3, 8 or 16 (D1); anything else, or carriage on a squad, is refused at load. Absence means the unit carries no one.
- Mirrors kept in step: `packages/scene-assets/src/units.ts` and `apps/mechanics-editor/src/fields.ts`; `fixtures/catalog.json` is re-blessed with `BLESS_CATALOG=1`.

### Rules (`crates/contract/src/scenario.rs`, `fixtures/game.json`)
- `garrison.enter_exit_s` (2 s) and `garrison.entry_distance_m` (4 m) stay as the building row.
- New `carriage`: `board_s` = 1, `reach_m` (closer than 4 m), `unload_search_m` = 5.
- New `bail_out`, one row per kind of host:
  - `building`: `death` 0.25, `hp_lost` [20, 40], `search_m` 8;
  - `ground` (a vehicle): `death` 0.10, `hp_lost` [10, 30], `search_m` 5;
  - `air` (a helicopter): `squad_loss` 0.5, `death` 0.25, `hp_lost` [30, 60], `search_m` 5.

  The row is chosen by the host's physical kind (building, ground hull, airframe), never by role. Every number is a starting guess to tune.
- `garrison.survival_probability_on_collapse` and `garrison.exit_search_radius_m` are deleted. `GarrisonRules` gains `deny_unknown_fields`, so a stale field fails loudly instead of being ignored.

### Commands (`crates/contract/src/command.rs`)

| Order | Sent by | Meaning |
|---|---|---|
| `Board { units, carrier, gesture }` | squads, right-clicking their own carrier | The squads walk to the carrier, following it if it moves, and board after 1 s. |
| `Collect { units, squads, gesture }` | a carrier, right-clicking its own squads | The carrier drives to the squads, which hold where they are, then they board. |
| `Unload { units, goal: Option<[f64; 2]>, gesture, route }` | carriers | `Some(goal)`: drive there, then unload. `None` (U pressed twice): stop and unload where you stand, replacing the queue. |

- All three honour `queued` (Shift). Each lowers to queue steps in one place. For example, `Unload { goal: Some }` becomes `[Move(goal), UnitOrder::Unload]`.
- New `OrderError` reasons: `NotACarrier`, `NoRoomToUnload`, `Aboard { unit }` (an order aimed at a passenger). The existing `CapacityFull` and `NotInfantry` are reused, with their doc comments widened.
- **Refusals per unit.** `contract::command::Refusal { unit, reason }`:
  - `CommandAck.refused: Vec<Refusal>` carries per-unit refusals at the time of the order, for example the squads left over when seats run out (D7).
  - `ObservationFrame.refusals: Vec<Refusal>` (own side, the tick it happens) carries refusals that happen later, for example an unload with no room on arrival (D9).

  Today `CommandAck` has one `error` (`command.rs:271-281`), and nothing reports a refusal that happens on arrival.

### Observation and publication
- `OwnUnit.garrison` becomes `OwnUnit.hold: Option<HoldState { host, phase, progress }>`, with `host` being `Building { id, center, half }` or `Carrier { id }`.
- The publication's own row replaces its 8 `garrison*` columns (`crates/sim/src/publication.rs:364-371`) with host columns.
- **The passenger relation is published once, on the passenger.**
  - Seats used is derived: the living members of the squads whose host is that carrier.
  - Seats total comes from the catalog.
  - There is no separate passengers section; it would give the same fact two owners.
- `IDENTIFIED_FIELDS` and the enemy card never change. The enemy learns of passengers only when they are on the field again.
- The layout changes in slice 03, and once more in slice 11b (unload steps). That slice re-records every capture, native's included (`fixtures/README.md:250`).

## Single-owner invariants

- **One hold.** `sim::hold` alone owns a unit being inside something. It has one host per unit, one `Phase` (`Entering`, `Inside`, `Exiting`), one stationary timer whose length is data (2 s for buildings, 1 s for carriers), and one cancellation rule. `garrison` keeps only what is about buildings (seat plans, windows, firing, shelter); `carriage` keeps only what is about carriers (seats, riding along, unload spots). Neither runs its own timer.
- **Alive is accounting; on the field is the world.** `Unit::alive()` means not dead: the unit cap, casualties, death shouts and the referee's survival test. `Unit::on_field()` means alive and not riding in a carrier. Everything physical uses it: colliders, blasts, suppression, shoves, treads, sensing, fog, hearing, radar, shouts, objectives and supply.
- **One bail-out.** Every forced exit (building collapsed, carrier destroyed, helicopter crashed) goes through `sim::bail_out` with its row from `rules.bail_out`. Each event gets its own generator from `mix(seed ^ BAIL_OUT_STREAM ^ host ^ tick)`, never `damage_rng`. Nothing else places or kills soldiers thrown out of a host.
- **Only destruction kills.** An explicit unload is all or nothing, and is refused rather than fatal.
- **One spot search for forced and carrier exits.** It excludes props, live hulls and soldiers, and takes height from `surface_at` (bridges). It serves unloads and bail-outs. A building's ordinary exit (`garrison::exit_spots`) keeps its props-only check, so ordinary garrison exits don't change; merging the two would need a named decision.
- **Passenger positions come from one place**, every tick: the carrier, or `crash.position` while a downed helicopter falls.
- **`carriage.seats` is the only carrier test.** No role, family or name check anywhere, including the AI, which pairs squads by seats against squad size.
- **One low-hover predicate** (`movement::air`): resupply and holds only give it reasons.
- **Transport state folds into the digest only when present**, so battles without it keep their digests. The building hold folds exactly the bytes `garrison::digest` folds today.
- **The enemy never learns passengers:** no passenger field in identified units or contacts, no passenger sound, no passenger capture.
- **Test units belong to tests and labs.** Carriers for tests are new test units; the menu roster and reel stay separate.

## Proof

The user prefers a few high-level functional tests to many unit tests.
- **Sim:** about ten scenario tests in one new file, `crates/sim/tests/transport.rs` (registered in `crates/sim/tests/main.rs`), each asserting what the game does and how it fails. Each slice names its tests.
- **Browser:** one scene, fixture id `transport`, route `/labs/transport` (`apps/battle-lab/src/routes/transport.tsx`, registered in `router.tsx` and `apps/battle-lab/src/fixtures.json`), scene script `web/scenes/transport.mjs`, on a saved `test` map. Slice 06 creates it; later slices add variants.
- **Native:** the slice 03 re-record, and native's counts test.
- **Visual gates (every slice that produces a shot):**
  - Run an unprimed `screenshot-critique` as the last check.
  - Where there is a before shot, also run `compare-screenshots`.
  - Then a non-blocking checkpoint with `preview-shots`.

## Scope firewalls

- The main-menu reel and menu roster are out of scope.
- `Order::Garrison` and `Order::OccupyBuilding` both stay; retiring `Garrison` is not this feature. `ExitBuilding` keeps its button and gets no hotkey (D8).
- Air-defence vehicles stay without seats, even though two of them carry the stale "passenger carriage not implemented" text, which slice 13 removes.
- No firing from inside (D10), no bundled purchase (D16), no animation for getting in or out.
- The doc error in `specs/done/helicopters/README.md:98` (`/lab/air`, `/lab/air-closing` don't exist) is fixed in its own commit if met, not in a transport commit.

## Corrections to the map

The drafting pass checked the map against the code on 2026-10-16. These corrections are also edited into [map.md](map.md):
- **No passengers section** (L5): the relation lives on the passenger.
- **No `serde(default)` or migration** (L10): hard cutover. The collapse fields live only in `fixtures/game.json` and the parity records `exact-numbers.json`, `skirmish-purchases.json` and `skirmish-objectives.json`, which are re-blessed.
- **A golden is pinned** (L4): the menu reel's event logs (`crates/sim/tests/menu_reel.rs:220-238`).
- **`battle.rs:761` seeds a bought aircraft's arrival delay**, not a crash. The crash burst draws from `damage_rng` (`battle.rs:2702-2709`). The per-event pattern is real (`battle.rs:761`, `arrangement::rng` at `arrangement.rs:23`).
- **`Capabilities` has no `check` method.** Per-type checks are at `catalog.rs:1608`.
- **The low hover refuses holds** (new): `air::sinks` (`movement/air.rs:155-163`) needs an empty order queue, no engagement and a serving truck. Slice 09 generalises it.
- **Lab routes are `/labs/<id>`.** Slice 17's `/lab/transports` is stale.
- **L11 is more than one line.** The wreck is added with `base_z: None`, and `resting_place` searches the ground (`battle.rs:2689`, `crash.rs:172`). Slice 01 carries a skip rule.
