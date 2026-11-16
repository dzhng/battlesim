# S6 — step records end to end (no visual change)

**Unlocks.** C6, C7, C8. Every reader moves to `steps`; the old fields are deleted.

**The record** (`crates/contract/src/observation.rs`, tagged by `kind`):

| kind | carries |
|---|---|
| `move` | point, `attack_move` |
| `attack` | target (`TargetRef`), point (read-only: track position, else the current attack's `last_known`, else the contact's position, else the ground point; absent if the side knows none) |
| `garrison` | point (approach), host (building) |
| `board` | point (the carrier), host (carrier) |
| `collect` | point (the first squad) |
| `unload` | point (where it lets them out), optional host it unloads into |
| `exit` | point (the building) |
| `return_to_base` | point (`way_home` from where the previous step ends) |

`steps[0]` is the current order. A move that lets passengers out is a `move` step then an
`unload` step (the unload mark draws on the move's ring).

**Seams, pipeline order.**
1. Contract type; delete `queue`, `queue_unloads`, `goal_unloads`.
2. `crates/sim/src/battle/order_steps.rs` (new): walks `u.orders` with a read-only origin
   (where the previous step ends) and returns the steps. Never writes sim state. The
   publisher in `battle.rs` calls it (net-shrinks). `way_home` takes an origin point.
   `Unit::queued_steps` is deleted.
3. `crates/sim/src/publication/step_row.rs` (new): `STEP_FIELDS`, the row writer; target
   cells shared with `mount_row.rs` through one encoder, host cells through one encoder
   shared with the own row. `publication.rs`: section `steps`, own-row `stepCount`,
   `goalUnloads` gone, layout gains `stepKinds` (and the parity bless key list does too).
4. `crates/sim/src/publication/reader/step.rs` (new) decodes into the view; the target
   decoder is shared with `reader/mount.rs`.
5. TS: `OrderStepView` in `web/src/battle/sim/observationView.ts`, reusing
   `MountTargetView` (no third target union). `OwnUnitView.steps`.
6. `packages/battle-renderer/src/orderSteps.ts` (new): the step type the renderer takes,
   `ordersEnd(steps)` (the last move-like step's point, never an attack's), and the port
   of `queuedLegs` out of `orderOverlay.ts`, drawing exactly today's marks from steps
   (other kinds skipped until S9). The ghost (`orderOverlay.ts`, `unitGhosts.ts`) uses
   `ordersEnd`. `apps/battle-lab/src/previewRevision.ts` hashes step intent (kind, ids,
   authored points), never an attack step's moving point.
7. Hand-updated readers (they cast and won't fail typecheck): the `as unknown as
   OwnUnitView` web tests, the scenes reading `queue` (`movement.mjs`, `_battleLook.mjs`,
   `_contactSheet.mjs`, `street.mjs` incl. its leak guard, `_cursorOrders.mjs`,
   `transport.mjs`), lab routes, and the Rust tests asserting `queue`.

**Fixtures.** Hand-edit `fixtures/parity/publication/codec-vectors.json` frames (bless
deserializes them first); then bless the stream hashes, the run and publication captures
with their observations, the gzipped counts capture, and `fog-sight-vectors.json`.
Digests don't move in this slice, so native re-publication works. Fetch LFS only for
what these tests refuse.

**Tests.** `crates/sim/tests/order_markers.rs`: move → shift-attack → shift-garrison →
shift-return publishes four steps in order with the right kinds, and the attack step's
point follows the target, then holds its last spot once lost. `web/tests/observation.test.ts`
decodes the new steps from the codec vector; `web/tests/unitGhosts.test.ts`: the ghost
stands at `ordersEnd`.

**Visual.** No intended change: `compare-screenshots` of `journey-space-orders.png` and a
transport-unload shot, before against after, must match. Say so in the commit.

**Commit.** `feat(contract): the own unit publishes its orders as step records` (notes
that server and web deploy together).

**State.** Done on the presentation branch except the parity captures. Contract, publisher,
row, reader, TS view, renderer port, ghost, preview identity, scenes and tests moved to
`steps`; the old fields are deleted. No battle digest moved: the five paired stream records
re-blessed changed only their `publication_sha256` columns. `journey-space-orders` and
`journey-transport-arrived` are pixel-identical before and after (pixelmatch 0).

Left for the parent:
- **One fresh browser capture** (`native-parity-capture`, as `cd1297886` did), then
  `BLESS_PARITY=1` on `publication_reader`, presentation `effects`/`fog_sight`/`poses`, and
  `web/tests/captureObservations.test.ts`. Until then those capture-replay tests fail on
  the old layout. The capture's battle is already stale on the base commit (choices.md),
  and FX1/S5 may move digests again, so record it once, after the sim lane lands.
- **After S2**, extend `every_order_is_published_as_a_step_in_order` with a move queued
  behind the attack (move → attack → move → garrison → return to base), asserting that
  move's step follows the attack and the garrison's way home starts from it. **Done after
  the merge:** the move behind the attack publishes as its own step.
